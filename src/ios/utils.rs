use std::path::PathBuf;

/// Returns the path for the Hachimi log file, inside the game sandbox Documents dir.
pub fn get_log_path() -> PathBuf {
    crate::ios::game_impl::get_data_dir("").join("hachimi.log")
}