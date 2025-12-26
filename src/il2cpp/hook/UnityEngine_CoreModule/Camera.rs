use std::ptr::null_mut;

use crate::il2cpp::{api::il2cpp_resolve_icall, types::*};

static mut CLASS: *mut Il2CppClass = null_mut();
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

static mut SET_ALLOWMSAA_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_allowMSAA, SET_ALLOWMSAA_ADDR, (), this: *mut Il2CppObject, value: bool);

pub fn init(UnityEngine_CoreModule: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_CoreModule, UnityEngine, Camera);

    unsafe {
        CLASS = Camera;
        SET_ALLOWMSAA_ADDR = il2cpp_resolve_icall(
            c"UnityEngine.Camera::set_allowMSAA(System.Boolean)".as_ptr()
        );
    }
}