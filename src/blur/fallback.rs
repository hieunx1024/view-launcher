use super::{BlurBackend, BlurResult, BlurStatus};
use i_slint_backend_winit::winit::window::Window;

#[derive(Default)]
pub struct FallbackBlurBackend;

impl FallbackBlurBackend {
    pub fn new() -> Self {
        Self
    }
}

impl BlurBackend for FallbackBlurBackend {
    fn apply(&self, _win: &Window, _radius: u32) -> BlurResult {
        Ok(())
    }

    fn update_region(&self, _win: &Window, _x: i32, _y: i32, _width: u32, _height: u32) {}

    fn status(&self) -> BlurStatus {
        BlurStatus::Fallback
    }
}
