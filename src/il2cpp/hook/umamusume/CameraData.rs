use std::ptr::null_mut;
use serde::{Serialize, Deserialize};

use crate::il2cpp::{symbols::{get_method, get_method_addr}, types::*};

#[derive(Default, Copy, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[repr(i32)]
pub enum ShadowResolution {
    #[default] Default,
    _256 = 0x100,
    _512 = 0x200,
    _1024 = 0x400,
    _2048 = 0x800,
    _4096 = 0x1000
}

static mut CLASS: *mut Il2CppClass = null_mut();
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

static mut SET_RENDERINGANTIALIASING_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_RenderingAntiAliasing, SET_RENDERINGANTIALIASING_ADDR, (), this: *mut Il2CppObject, value: i32);

static mut SET_ISCREATEANTIALIASTEXTURE_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_IsCreateAntialiasTexture, SET_ISCREATEANTIALIASTEXTURE_ADDR, (), this: *mut Il2CppObject, value: bool);

static mut GET_ISUIRENDERING_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_IsUIRendering, GET_ISUIRENDERING_ADDR, bool, this: *mut Il2CppObject);

static mut GET_CAMERA_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_Camera, GET_CAMERA_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut SET_ISOVERRIDESHADOWRESOLUTION_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_IsOverrideShadowResolution, SET_ISOVERRIDESHADOWRESOLUTION_ADDR, (), this: *mut Il2CppObject, value: bool);

static mut SET_OVERRIDESHADOWRESOLUTION_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_OverrideShadowResolution, SET_OVERRIDESHADOWRESOLUTION_ADDR, (), this: *mut Il2CppObject, value: ShadowResolution);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, "Gallop.RenderPipeline", CameraData);
    unsafe {
        CLASS = CameraData;
        GET_CAMERA_ADDR = get_method_addr(CameraData, c"get_Camera", 0);
        GET_ISUIRENDERING_ADDR = get_method_addr(CameraData, c"get_IsUIRendering", 0);
        SET_RENDERINGANTIALIASING_ADDR = get_method_addr(CameraData, c"set_RenderingAntiAliasing", 1);
        SET_ISCREATEANTIALIASTEXTURE_ADDR = get_method_addr(CameraData, c"set_IsCreateAntialiasTexture", 1);
        SET_ISOVERRIDESHADOWRESOLUTION_ADDR = get_method_addr(CameraData, c"set_IsOverrideShadowResolution", 1);
        SET_OVERRIDESHADOWRESOLUTION_ADDR = get_method_addr(CameraData, c"set_OverrideShadowResolution", 1);
    }
}