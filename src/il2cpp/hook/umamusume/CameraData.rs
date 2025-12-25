use std::ptr::null_mut;

use crate::il2cpp::{symbols::get_method_addr, types::*};

static mut CLASS: *mut Il2CppClass = null_mut();
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

static mut SET_RENDERINGANTIALIASING_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_RenderingAntiAliasing, SET_RENDERINGANTIALIASING_ADDR, (), this: *mut Il2CppObject, value: i32);

static mut SET_ISCREATEANTIALIASTEXTURE_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_IsCreateAntialiasTexture, SET_ISCREATEANTIALIASTEXTURE_ADDR, (), this: *mut Il2CppObject, value: bool);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, "Gallop.RenderPipeline", CameraData);
    unsafe {
        CLASS = CameraData;
        SET_RENDERINGANTIALIASING_ADDR = get_method_addr(CameraData, c"set_RenderingAntiAliasing", 1);
        SET_ISCREATEANTIALIASTEXTURE_ADDR = get_method_addr(CameraData, c"set_IsCreateAntialiasTexture", 1);
    }
}