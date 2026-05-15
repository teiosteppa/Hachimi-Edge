use crate::core::Hachimi;
use super::hook;

// sets RUST_BACKTRACE=full so Rust panics emit full backtraces, & redirects stderr to capture Rust/Unity log output
// uses dup2 because spawning threads inside a ctor is unsafe on Apple platforms
fn pre_init() {
    unsafe {
        libc::setenv(
            c"RUST_BACKTRACE".as_ptr() as *const libc::c_char,
            c"full".as_ptr() as *const libc::c_char,
            1,
        );
    }

    let log_path = super::utils::get_log_path();
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    if let Some(path_str) = log_path.to_str() {
        if let Ok(cpath) = std::ffi::CString::new(path_str) {
            unsafe {
                let fd = libc::open(
                    cpath.as_ptr(),
                    libc::O_WRONLY | libc::O_CREAT | libc::O_APPEND,
                    0o644,
                );
                if fd >= 0 {
                    libc::dup2(fd, libc::STDERR_FILENO);
                    libc::close(fd);
                }
            }
        }
    }
}

/// iOS entry point via the `#[ctor]` constructor attribute.
///
/// This function is called by dyld **before** the app's `main()` runs,
/// giving Hachimi a chance to install its hooks early.
#[ctor::ctor]
fn hachimi_ios_init() {
    pre_init();

    if !Hachimi::init() {
        unsafe {
            libc::syslog(
                libc::LOG_ERR,
                b"[Hachimi] Hachimi::init() FAILED\0".as_ptr() as *const _,
            );
        }
        return;
    }

    let pkg = super::game_impl::get_package_name();
    let region = super::game_impl::get_region(&pkg);
    let data_dir = super::game_impl::get_data_dir(&pkg);
    info!("Hachimi::init() OK");
    info!("Bundle ID: {}", pkg);
    info!("Region: {}", region);
    debug!("Data dir: {:?}", data_dir);

    hook::init();
    debug!("dyld callback registered");
}