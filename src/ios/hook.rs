use mach2::{dyld, loader::mach_header};
use crate::core::Hachimi;

// not exposed by mach2 crate
extern "C" {
    fn _dyld_register_func_for_add_image(
        func: Option<unsafe extern "C" fn(*const mach_header, libc::intptr_t)>,
    );
}

unsafe extern "C" fn on_image_added(mh: *const mach_header, slide: libc::intptr_t) {
    let count = dyld::_dyld_image_count();
    for i in 0..count {
        let img_mh = dyld::_dyld_get_image_header(i);
        if img_mh != mh { continue; }

        let raw = dyld::_dyld_get_image_name(i);
        if raw.is_null() { break; }
        let name = std::ffi::CStr::from_ptr(raw)
            .to_str()
            .unwrap_or("");

        if crate::ios::hachimi_impl::is_il2cpp_lib(name) {
            debug!("Image loaded: {}", name);
            debug!("Matched as IL2CPP lib, header={:#x} slide={:#x}", mh as usize, slide);

            crate::ios::hachimi_impl::on_il2cpp_loaded(mh as usize, slide as isize);
            Hachimi::instance().on_dlopen(name, mh as usize);
        }
        break;
    }
}

fn init_internal() {
    unsafe {
        _dyld_register_func_for_add_image(Some(on_image_added));
    }
}

pub fn init() {
    init_internal();
}