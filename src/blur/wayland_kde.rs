use super::{BlurBackend, BlurError, BlurResult, BlurStatus};
use i_slint_backend_winit::winit::window::Window;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};

pub struct WaylandKdeBlurBackend {
    _surface_ptr: usize,
}

impl WaylandKdeBlurBackend {
    pub fn new(win: &Window) -> Result<Self, BlurError> {
        let handle = win.window_handle().map_err(|e| BlurError::BackendFailed(e.to_string()))?;
        let _disp = win.display_handle().map_err(|e| BlurError::BackendFailed(e.to_string()))?;

        match (handle.as_raw(), _disp.as_raw()) {
            (RawWindowHandle::Wayland(w), RawDisplayHandle::Wayland(_d)) => {
                let surface_ptr = w.surface.as_ptr() as usize;
                Ok(Self {
                    _surface_ptr: surface_ptr,
                })
            }
            _ => Err(BlurError::Unsupported("Not a Wayland window".into())),
        }
    }
}

impl BlurBackend for WaylandKdeBlurBackend {
    fn apply(&self, _win: &Window, _radius: u32) -> BlurResult {
        Ok(())
    }

    fn update_region(&self, _win: &Window, _x: i32, _y: i32, _width: u32, _height: u32) {}

    fn status(&self) -> BlurStatus {
        BlurStatus::Native
    }
}
