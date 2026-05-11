pub mod render_hook;
pub mod input_hook;
pub mod metal_painter;

pub fn init() {
    render_hook::init();
    input_hook::init();
}
