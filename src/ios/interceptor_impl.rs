use crate::core::{interceptor::HookHandle, Error};
use once_cell::sync::OnceCell;
use std::os::raw::c_void;

type MsHookFn = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut *mut c_void);

enum HookBackend {
    Substrate { hook_fn: MsHookFn },
    Dobby,
}

static BACKEND: OnceCell<HookBackend> = OnceCell::new();

fn get_os_major_version() -> i32 {
    unsafe {
        let mut os_version = [0u8; 32];
        let mut size = std::mem::size_of_val(&os_version);
        if libc::sysctlbyname(
            b"kern.osproductversion\0".as_ptr() as *const _,
            os_version.as_mut_ptr() as *mut _,
            &mut size,
            std::ptr::null_mut(),
            0,
        ) == 0 {
            let version_str = std::ffi::CStr::from_ptr(os_version.as_ptr() as *const _).to_string_lossy();
            if let Some(major_str) = version_str.split('.').next() {
                return major_str.parse().unwrap_or(0);
            }
        }
        0
    }
}

fn is_macos_hardware() -> bool {
    type ObjcGetClassFn = unsafe extern "C" fn(*const u8) -> *mut c_void;
    type SelRegisterNameFn = unsafe extern "C" fn(*const u8) -> *mut c_void;
    type ObjcMsgSendFn = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
    type ObjcMsgSendBoolFn = unsafe extern "C" fn(*mut c_void, *mut c_void) -> bool;
    type ClassRespondsToSelectorFn = unsafe extern "C" fn(*mut c_void, *mut c_void) -> bool;
    type ObjectGetClassFn = unsafe extern "C" fn(*mut c_void) -> *mut c_void;

    unsafe {
        let objc = libc::dlopen(b"/usr/lib/libobjc.A.dylib\0".as_ptr() as *const _, libc::RTLD_LAZY);
        if objc.is_null() {
            return false;
        }

        let mut is_mac = false;

        let sym_get_class = libc::dlsym(objc, b"objc_getClass\0".as_ptr() as *const _);
        let sym_sel_reg = libc::dlsym(objc, b"sel_registerName\0".as_ptr() as *const _);
        let sym_msg_send = libc::dlsym(objc, b"objc_msgSend\0".as_ptr() as *const _);
        let sym_responds = libc::dlsym(objc, b"class_respondsToSelector\0".as_ptr() as *const _);
        let sym_obj_class = libc::dlsym(objc, b"object_getClass\0".as_ptr() as *const _);

        if !sym_get_class.is_null() && !sym_sel_reg.is_null() && !sym_msg_send.is_null()
            && !sym_responds.is_null() && !sym_obj_class.is_null()
        {
            let objc_get_class: ObjcGetClassFn = std::mem::transmute(sym_get_class);
            let sel_register_name: SelRegisterNameFn = std::mem::transmute(sym_sel_reg);
            let objc_msg_send: ObjcMsgSendFn = std::mem::transmute(sym_msg_send);
            let objc_msg_send_bool: ObjcMsgSendBoolFn = std::mem::transmute(sym_msg_send);
            let class_responds_to_selector: ClassRespondsToSelectorFn = std::mem::transmute(sym_responds);
            let object_get_class: ObjectGetClassFn = std::mem::transmute(sym_obj_class);

            let cls = objc_get_class(b"NSProcessInfo\0".as_ptr());
            if !cls.is_null() {
                let sel_process_info = sel_register_name(b"processInfo\0".as_ptr());
                let process_info = objc_msg_send(cls, sel_process_info);

                if !process_info.is_null() {
                    let sel_is_mac = sel_register_name(b"isiOSAppOnMac\0".as_ptr());
                    let obj_cls = object_get_class(process_info);

                    if class_responds_to_selector(obj_cls, sel_is_mac) {
                        is_mac = objc_msg_send_bool(process_info, sel_is_mac);
                    }
                }
            }
        }

        libc::dlclose(objc);
        is_mac
    }
}

fn backend() -> &'static HookBackend {
    BACKEND.get_or_init(|| {
        const RTLD_DEFAULT: *mut c_void = std::ptr::null_mut::<c_void>().wrapping_sub(2)
            as *mut c_void;
        const HOOK_SYM: &[u8] = b"MSHookFunction\0";

        unsafe {
            let is_mac = is_macos_hardware();
            let major_version = get_os_major_version();

            if !is_mac && major_version >= 26 {
                info!("iOS: using Dobby for hooking");
                return HookBackend::Dobby;
            }

            let sym = libc::dlsym(RTLD_DEFAULT, HOOK_SYM.as_ptr() as *const _);
            if !sym.is_null() {
                info!("iOS: using Substrate/Ellekit (MSHookFunction from RTLD_DEFAULT)");
                return HookBackend::Substrate {
                    hook_fn: std::mem::transmute(sym),
                };
            }

            const PATHS: &[&[u8]] = &[
                b"/var/jb/usr/lib/libellekit.dylib\0",
                b"/usr/lib/libsubstrate.dylib\0",
                b"/usr/lib/libhooker.dylib\0",
            ];
            for &path in PATHS {
                let handle = libc::dlopen(path.as_ptr() as *const _, libc::RTLD_LAZY | libc::RTLD_GLOBAL);
                if handle.is_null() { continue; }
                let sym = libc::dlsym(handle, HOOK_SYM.as_ptr() as *const _);
                if !sym.is_null() {
                    info!("iOS: using Substrate/Ellekit from {}", std::str::from_utf8(path).unwrap_or("?"));
                    return HookBackend::Substrate {
                        hook_fn: std::mem::transmute(sym),
                    };
                }
                libc::dlclose(handle);
            }

            info!("iOS: falling back to Dobby for hooking");
            HookBackend::Dobby
        }
    })
}

pub unsafe fn hook(orig_addr: usize, hook_addr: usize) -> Result<usize, Error> {
    match backend() {
        HookBackend::Substrate { hook_fn } => {
            let mut trampoline: *mut c_void = std::ptr::null_mut();
            hook_fn(orig_addr as *mut c_void, hook_addr as *mut c_void, &mut trampoline);
            if trampoline.is_null() {
                Err(Error::HookingError("MSHookFunction returned null trampoline".into()))
            } else {
                Ok(trampoline as usize)
            }
        }
        HookBackend::Dobby => {
            Ok(dobby_rs::hook(orig_addr as *mut c_void, hook_addr as *mut c_void)? as usize)
        }
    }
}

pub unsafe fn unhook(hook: &HookHandle) -> Result<(), Error> {
    match backend() {
        HookBackend::Substrate { .. } => {
            Ok(())
        }
        HookBackend::Dobby => {
            dobby_rs::unhook(hook.orig_addr as *mut c_void)?;
            Ok(())
        }
    }
}

impl From<dobby_rs::DobbyHookError> for Error {
    fn from(e: dobby_rs::DobbyHookError) -> Self {
        Error::HookingError(e.to_string())
    }
}

pub unsafe fn find_symbol_by_name(module: &str, symbol: &str) -> Result<usize, Error> {
    if let Some(addr) = dobby_rs::resolve_symbol(module, symbol) {
        return Ok(addr as usize);
    }
    Err(Error::SymbolNotFound(module.to_owned(), symbol.to_owned()))
}

pub unsafe fn get_vtable_from_instance(_instance_addr: usize) -> *mut usize {
    unimplemented!("vtable hooking not used on iOS")
}
pub unsafe fn hook_vtable(
    _vtable: *mut usize,
    _vtable_index: usize,
    _hook_addr: usize,
) -> Result<HookHandle, Error> {
    unimplemented!("vtable hooking not used on iOS")
}
pub unsafe fn unhook_vtable(_hook: &HookHandle) -> Result<(), Error> {
    unimplemented!("vtable hooking not used on iOS")
}