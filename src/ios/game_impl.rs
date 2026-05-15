use std::path::PathBuf;

use crate::core::game::Region;

extern "C" {
    fn objc_getClass(name: *const u8) -> *mut std::ffi::c_void;
    fn sel_registerName(name: *const u8) -> *mut std::ffi::c_void;
    fn objc_msgSend(receiver: *mut std::ffi::c_void, sel: *mut std::ffi::c_void, ...) -> *mut std::ffi::c_void;
    fn NSHomeDirectory() -> *mut std::ffi::c_void;
}

pub fn get_package_name() -> String {
    unsafe {
        let cls = objc_getClass(b"NSBundle\0".as_ptr());
        let sel_main = sel_registerName(b"mainBundle\0".as_ptr());
        let bundle = objc_msgSend(cls, sel_main);

        if bundle.is_null() {
            return "unknown".to_string();
        }

        let sel_id = sel_registerName(b"bundleIdentifier\0".as_ptr());
        let bundle_id = objc_msgSend(bundle, sel_id);

        if bundle_id.is_null() {
            return "unknown".to_string();
        }

        let sel_utf8 = sel_registerName(b"UTF8String\0".as_ptr());
        let utf8_ptr = objc_msgSend(bundle_id, sel_utf8) as *const std::os::raw::c_char;

        if utf8_ptr.is_null() {
            return "unknown".to_string();
        }

        std::ffi::CStr::from_ptr(utf8_ptr)
            .to_string_lossy()
            .into_owned()
    }
}

pub fn get_region(package_name: &str) -> Region {
    if package_name.starts_with("jp.co.cygames.umamusume") || package_name.starts_with("app.papaya2933.cheetah1054") {
        Region::Japan
    } else if package_name.starts_with("com.komoe.kmumamusumegp") || package_name.starts_with("com.komoe.umamusumeofficial") {
        Region::Taiwan
    } else if package_name.starts_with("com.kakaogames.umamusume") {
        Region::Korea
    } else if package_name.starts_with("com.bilibili.umamusu") {
        Region::China
    } else if package_name.starts_with("com.cygames.umamusume") {
        Region::Global
    } else {
        Region::Unknown
    }
}

pub fn get_data_dir(_package_name: &str) -> PathBuf {
    get_game_documents_dir()
}

fn get_game_documents_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| {
        unsafe {
            let ns_str = NSHomeDirectory();
            if !ns_str.is_null() {
                let sel_utf8 = sel_registerName(b"UTF8String\0".as_ptr());
                let utf8_ptr = objc_msgSend(ns_str, sel_utf8) as *const std::os::raw::c_char;

                if !utf8_ptr.is_null() {
                    return std::ffi::CStr::from_ptr(utf8_ptr)
                        .to_string_lossy()
                        .into_owned();
                }
            }
        }

        "/var/mobile".to_string()
    });

    PathBuf::from(home).join("Documents").join("hachimi")
}