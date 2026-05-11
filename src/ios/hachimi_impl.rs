use serde::{Deserialize, Serialize};
use crate::core::Hachimi;

pub fn is_il2cpp_lib(filename: &str) -> bool {
    filename.contains("UnityFramework")
        || filename.contains("GameAssembly")
        || filename.ends_with("libil2cpp.dylib")
}

pub fn on_il2cpp_loaded(header_addr: usize, slide: isize) {
    info!("═══ STAGE 3: ACQUIRING IL2CPP HANDLE ═══");

    let mut info: libc::Dl_info = unsafe { std::mem::zeroed() };
    let handle = unsafe {
        if libc::dladdr(header_addr as *const _, &mut info) != 0 && !info.dli_fname.is_null() {
            let path_str = std::ffi::CStr::from_ptr(info.dli_fname).to_string_lossy();
            libc::dlopen(info.dli_fname, libc::RTLD_LAZY | libc::RTLD_NOLOAD)
        } else {
            std::ptr::null_mut()
        }
    };

    if handle.is_null() {
        error!("iOS: Failed to acquire genuine UnityFramework handle!");
        return;
    }

    crate::il2cpp::symbols::set_handle(handle as usize);

    super::symbols_impl::init_exports(header_addr, slide);

    info!("═══ STAGE 3: DONE ═══");

    info!("═══ STAGE 4: IL2CPP_INIT HOOK ═══");
    let il2cpp_init_addr = unsafe { crate::il2cpp::symbols::dlsym("il2cpp_init") };

    if il2cpp_init_addr != 0 {
        info!("il2cpp_init found at {:#x}", il2cpp_init_addr);
        install_il2cpp_init_hook(il2cpp_init_addr);
    } else {
        error!("il2cpp_init symbol not found — hooking will not fire");
        error!("═══ STAGE 4: FAILED ═══");
    }
}

static ORIG_IL2CPP_INIT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

unsafe extern "C" fn hooked_il2cpp_init(domain_name: *const std::os::raw::c_char) -> i32 {
    info!("═══ STAGE 5: IL2CPP_INIT FIRED (via hook) ═══");

    let name_str = if !domain_name.is_null() {
        std::ffi::CStr::from_ptr(domain_name).to_string_lossy().to_string()
    } else {
        "(null)".to_string()
    };
    info!("il2cpp_init called with domain: {}", name_str);

    let trampoline = ORIG_IL2CPP_INIT.load(std::sync::atomic::Ordering::Relaxed);
    if trampoline == 0 {
        error!("FATAL: trampoline is null! Cannot call original il2cpp_init");
        return -1;
    }

    let orig: extern "C" fn(*const std::os::raw::c_char) -> i32 =
        std::mem::transmute(trampoline);
    let result = orig(domain_name);
    info!("Original il2cpp_init returned: {}", result);

    post_il2cpp_init();

    result
}

unsafe fn post_il2cpp_init() {
    info!("[post-init] symbols::init()...");
    crate::il2cpp::symbols::init();
    info!("[post-init] symbols::init() OK");

    info!("[post-init] on_hooking_finished()...");
    crate::core::Hachimi::instance().on_hooking_finished();
    info!("═══ STAGE 5: DONE ═══");
}

fn install_il2cpp_init_hook(addr: usize) {
    let hachimi = crate::core::Hachimi::instance();
    info!("Installing il2cpp_init hook: target={:#x} hook={:#x}",
        addr, hooked_il2cpp_init as usize);

    unsafe {
        let pre_bytes = std::slice::from_raw_parts(addr as *const u32, 4);
        info!("PRE-HOOK  bytes @ {:#x}: {:08x} {:08x} {:08x} {:08x}",
            addr, pre_bytes[0], pre_bytes[1], pre_bytes[2], pre_bytes[3]);
    }

    match hachimi.interceptor.hook(addr, hooked_il2cpp_init as usize) {
        Ok(trampoline) => {
            ORIG_IL2CPP_INIT.store(trampoline, std::sync::atomic::Ordering::Release);
            info!("Trampoline at {:#x}", trampoline);

            unsafe {
                let post_bytes = std::slice::from_raw_parts(addr as *const u32, 4);
                info!("POST-HOOK bytes @ {:#x}: {:08x} {:08x} {:08x} {:08x}",
                    addr, post_bytes[0], post_bytes[1], post_bytes[2], post_bytes[3]);
            }

            info!("═══ STAGE 4: DONE — waiting for il2cpp_init() call ═══");
        }
        Err(e) => {
            error!("Failed to hook il2cpp_init: {}", e);
            error!("═══ STAGE 4: FAILED ═══");
        }
    }
}

pub fn is_criware_lib(filename: &str) -> bool {
    filename.contains("cri_ware") || filename.ends_with("libcri_ware_unity.dylib")
}

pub fn on_hooking_finished(_hachimi: &Hachimi) {
    info!("iOS hooking finished");
}