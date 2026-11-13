use super::{BlurBackend, BlurError, BlurResult, BlurStatus};
use i_slint_backend_winit::winit::window::Window;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::sync::atomic::{AtomicU32, Ordering};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt, PropMode};
use x11rb::wrapper::ConnectionExt as _;

pub struct X11BlurBackend {
    cached_xid: AtomicU32,
}

impl Default for X11BlurBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl X11BlurBackend {
    pub fn new() -> Self {
        Self {
            cached_xid: AtomicU32::new(0),
        }
    }

    fn get_xid(&self, win: &Window) -> Option<u32> {
        let cached = self.cached_xid.load(Ordering::Relaxed);
        if cached != 0 {
            return Some(cached);
        }
        if let Ok(handle) = win.window_handle() {
            let xid = match handle.as_raw() {
                RawWindowHandle::Xlib(h) => Some(h.window as u32),
                RawWindowHandle::Xcb(h) => Some(h.window.get()),
                _ => None,
            };
            if let Some(id) = xid {
                self.cached_xid.store(id, Ordering::Relaxed);
                return Some(id);
            }
        }
        None
    }
}

impl BlurBackend for X11BlurBackend {
    fn apply(&self, win: &Window, _radius: u32) -> BlurResult {
        let xid = self.get_xid(win).ok_or_else(|| BlurError::BackendFailed("Failed to get X11 window id".into()))?;
        let (conn, _) = x11rb::connect(None).map_err(|e| BlurError::BackendFailed(e.to_string()))?;

        // 1. Set _NET_WM_WINDOW_TYPE to _NET_WM_WINDOW_TYPE_UTILITY / _NET_WM_WINDOW_TYPE_DOCK
        if let Ok(reply) = conn.intern_atom(false, b"_NET_WM_WINDOW_TYPE") {
            if let Ok(atom_type) = reply.reply() {
                if let Ok(dock_reply) = conn.intern_atom(false, b"_NET_WM_WINDOW_TYPE_UTILITY") {
                    if let Ok(atom_dock) = dock_reply.reply() {
                        let _ = conn.change_property32(
                            PropMode::REPLACE,
                            xid,
                            atom_type.atom,
                            AtomEnum::ATOM,
                            &[atom_dock.atom],
                        );
                    }
                }
            }
        }

        // 2. Set _NET_WM_STATE to _NET_WM_STATE_ABOVE and _NET_WM_STATE_SKIP_TASKBAR
        if let Ok(state_reply) = conn.intern_atom(false, b"_NET_WM_STATE") {
            if let Ok(atom_state) = state_reply.reply() {
                let above = conn.intern_atom(false, b"_NET_WM_STATE_ABOVE").ok().and_then(|r| r.reply().ok()).map(|r| r.atom);
                let skip = conn.intern_atom(false, b"_NET_WM_STATE_SKIP_TASKBAR").ok().and_then(|r| r.reply().ok()).map(|r| r.atom);
                let mut states = Vec::new();
                if let Some(a) = above { states.push(a); }
                if let Some(s) = skip { states.push(s); }
                if !states.is_empty() {
                    let _ = conn.change_property32(
                        PropMode::REPLACE,
                        xid,
                        atom_state.atom,
                        AtomEnum::ATOM,
                        &states,
                    );
                }
            }
        }

        // 3. Set _KDE_NET_WM_BLUR_BEHIND_REGION with initial visible pill region
        if let Ok(blur_reply) = conn.intern_atom(false, b"_KDE_NET_WM_BLUR_BEHIND_REGION") {
            if let Ok(atom_blur) = blur_reply.reply() {
                let rect: [u32; 4] = [179, 40, 470, 40];
                let _ = conn.change_property32(
                    PropMode::REPLACE,
                    xid,
                    atom_blur.atom,
                    AtomEnum::CARDINAL,
                    &rect,
                );
            }
        }

        let _ = conn.flush();
        Ok(())
    }

    fn update_region(&self, win: &Window, x: i32, y: i32, width: u32, height: u32) {
        if let Some(xid) = self.get_xid(win) {
            if let Ok((conn, _)) = x11rb::connect(None) {
                if let Ok(reply) = conn.intern_atom(false, b"_KDE_NET_WM_BLUR_BEHIND_REGION") {
                    if let Ok(atom_kde_blur) = reply.reply() {
                        let rect = [x.max(0) as u32, y.max(0) as u32, width, height];
                        let _ = conn.change_property32(
                            PropMode::REPLACE,
                            xid,
                            atom_kde_blur.atom,
                            AtomEnum::CARDINAL,
                            &rect,
                        );
                        let _ = conn.flush();
                    }
                }
            }
        }
    }

    fn status(&self) -> BlurStatus {
        BlurStatus::Native
    }
}
