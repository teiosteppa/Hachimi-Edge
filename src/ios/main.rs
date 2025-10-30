use std::ffi::{c_void, CStr};
use std::sync::Once;
use std::fs::File;
use std::io::Write;
use ctor::ctor;

static STARTUP_ONCE: Once = Once::new();

#[no_mangle]
pub unsafe extern "C" fn dlopen(path: *const i8, mode: i32) -> *mut c_void {
    let test_path = std::panic::catch_unwind(|| {
        super::game_impl::get_data_dir("")
    });

    if let Ok(docs_dir) = test_path {
        let test_log_path = docs_dir.join("hachimi_dlopen_test.txt");
        if let Ok(mut file) = File::create(&test_log_path) {
            let _ = writeln!(file, "dlopen hook was executed!");
            let _ = file.flush();
        }
    }

    let real_dlopen: extern "C" fn(*const i8, i32) -> *mut c_void =
        std::mem::transmute(libc::dlsym(libc::RTLD_NEXT, b"dlopen\0".as_ptr() as _));

    STARTUP_ONCE.call_once(|| {
        eprintln!("[Hachimi-iOS] Intercepted dlopen, initializing synchronously...");
        initialize_hachimi();
    });

    let handle = real_dlopen(path, mode);
    handle
}

#[ctor]
unsafe fn hachimi_init_ctor() {
    let test_path = std::panic::catch_unwind(|| {
        super::game_impl::get_data_dir("")
    });

    if let Ok(docs_dir) = test_path {
        let test_log_path = docs_dir.join("hachimi_ctor_test.txt");
        if let Ok(mut file) = File::create(&test_log_path) {
            let _ = writeln!(file, "ctor hook was executed!");
            let _ = file.flush();
        }
    }

    STARTUP_ONCE.call_once(|| {
        eprintln!("[Hachimi-iOS] ctor hook fired, initializing synchronously...");
        initialize_hachimi();
    });
}

fn initialize_hachimi() {
    super::log_impl::init(log::LevelFilter::Info);

    info!("Hachimi synchronous initialization started...");

    crate::core::init(
        Box::new(super::log_impl::IosLog::new()),
        Box::new(super::hachimi_impl::IosHachimi),
        Box::new(super::game_impl::IosGame),
        Box::new(super::gui_impl::IosGui),
        Box::new(super::interceptor_impl::IosInterceptor),
        Box::new(super::symbols_impl::IosSymbols),
    );

    info!("Hachimi platform implementations set. Initializing Hachimi core...");
    if !crate::core::Hachimi::init() {
        error!("Failed to initialize Hachimi core");
        return;
    }

    info!("Hachimi core initialized. Initializing iOS GUI hooks...");
    super::gui_impl::init();

    info!("iOS initialization complete.");
}
