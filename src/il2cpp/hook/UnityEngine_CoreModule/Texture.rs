use crate::il2cpp::{api::il2cpp_resolve_icall, types::*};

static mut GETDATAWIDTH_ADDR: usize = 0;
impl_addr_wrapper_fn!(GetDataWidth, GETDATAWIDTH_ADDR, i32, this: *mut Il2CppObject);

static mut GETDATAHEIGHT_ADDR: usize = 0;
impl_addr_wrapper_fn!(GetDataHeight, GETDATAHEIGHT_ADDR, i32, this: *mut Il2CppObject);

static mut SETANISOLEVEL_ADDR: usize = 0;
impl_addr_wrapper_fn!(SetAnisoLevel, SETANISOLEVEL_ADDR, (), this: *mut Il2CppObject, anisoLevel: i32);

#[repr(i32)]
enum FilterMode {
    Point,
    Bilinear,
    Trilinear
}

type set_filterModeFn = extern "C" fn(this: *mut Il2CppObject, filterMode: FilterMode);
extern "C" fn set_filterMode(this: *mut Il2CppObject, filterMode: FilterMode) {
    get_orig_fn!(set_filterMode, set_filterModeFn)(this, FilterMode::Trilinear);
    SetAnisoLevel(this, 16);
}

pub fn init(_UnityEngine_CoreModule: *const Il2CppImage) {
    let set_filterMode_addr = il2cpp_resolve_icall(
        c"UnityEngine.Texture::set_filterMode(System.Int32)".as_ptr()
    );

    unsafe {
        GETDATAWIDTH_ADDR = il2cpp_resolve_icall(c"UnityEngine.Texture::GetDataWidth()".as_ptr());
        GETDATAHEIGHT_ADDR = il2cpp_resolve_icall(c"UnityEngine.Texture::GetDataHeight()".as_ptr());
        SETANISOLEVEL_ADDR = il2cpp_resolve_icall(c"UnityEngine.Texture::set_anisoLevel(System.Int32)".as_ptr());
    }

    new_hook!(set_filterMode_addr, set_filterMode);
}