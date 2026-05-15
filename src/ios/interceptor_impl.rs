use crate::core::{interceptor::HookHandle, Error};
use once_cell::sync::OnceCell;
use std::os::raw::c_void;

type MsHookFn = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut *mut c_void);

enum HookBackend {
    Substrate { hook_fn: MsHookFn },
    Dobby,
}

static BACKEND: OnceCell<HookBackend> = OnceCell::new();

fn is_livecontainer() -> bool {
    unsafe {
        let count = libc::_dyld_image_count();
        for i in 0..count {
            let name_ptr = libc::_dyld_get_image_name(i);
            if !name_ptr.is_null() {
                let name = std::ffi::CStr::from_ptr(name_ptr).to_string_lossy();
                if name.contains("LiveContainer") {
                    return true;
                }
            }
        }
        false
    }
}

fn backend() -> &'static HookBackend {
    BACKEND.get_or_init(|| {
        const RTLD_DEFAULT: *mut c_void = std::ptr::null_mut::<c_void>().wrapping_sub(2)
            as *mut c_void;
        const HOOK_SYM: &[u8] = b"MSHookFunction\0";

        unsafe {
            if is_livecontainer() {
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