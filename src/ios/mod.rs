pub mod game_impl;
pub mod gui_impl;
pub mod hachimi_impl;
pub mod hook;
pub mod interceptor_impl;
pub mod log_impl;
pub mod symbols_impl;
pub mod utils;
pub mod plugin_loader;

mod main;

#[cfg(target_os = "ios")]
#[link(name = "c++")]
extern "C" {}
