use crate::core::Hachimi;
use super::hook;

#[ctor::ctor]
fn hachimi_ios_init() {
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