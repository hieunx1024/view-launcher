use i_slint_backend_winit::winit::window::Window;
use std::fmt;

pub mod fallback;
#[cfg(feature = "wayland")]
pub mod wayland_kde;
#[cfg(feature = "x11")]
pub mod x11;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlurStatus {
    Native,
    RuleRequired,
    Fallback,
}

#[derive(Debug)]
pub enum BlurError {
    Unsupported(String),
    BackendFailed(String),
}

impl fmt::Display for BlurError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(msg) => write!(f, "Unsupported blur: {}", msg),
            Self::BackendFailed(msg) => write!(f, "Blur backend failed: {}", msg),
        }
    }
}

impl std::error::Error for BlurError {}

pub type BlurResult = Result<(), BlurError>;

pub trait BlurBackend: Send + Sync {
    fn apply(&self, win: &Window, radius: u32) -> BlurResult;
    fn update_region(&self, win: &Window, x: i32, y: i32, width: u32, height: u32);
    fn status(&self) -> BlurStatus;
}

pub struct RuleRequiredBlurBackend;

impl BlurBackend for RuleRequiredBlurBackend {
    fn apply(&self, _win: &Window, _radius: u32) -> BlurResult {
        Ok(())
    }

    fn update_region(&self, _win: &Window, _x: i32, _y: i32, _width: u32, _height: u32) {}

    fn status(&self) -> BlurStatus {
        BlurStatus::RuleRequired
    }
}

pub fn detect_blur_backend(win: &Window) -> (Box<dyn BlurBackend>, BlurStatus, String) {
    let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_default().to_lowercase();
    let current_desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().to_lowercase();
    let is_wayland = session_type == "wayland" || std::env::var("WAYLAND_DISPLAY").is_ok();
    let is_kde = current_desktop.contains("kde") || current_desktop.contains("plasma");
    let is_hyprland = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() || current_desktop.contains("hyprland");
    let is_sway = std::env::var("SWAYSOCK").is_ok() || current_desktop.contains("sway");

    // Tier 1: X11
    #[cfg(feature = "x11")]
    if !is_wayland {
        let backend = x11::X11BlurBackend::new();
        if backend.apply(win, 32).is_ok() {
            return (
                Box::new(backend),
                BlurStatus::Native,
                "Tier 1 (X11 _KDE_NET_WM_BLUR_BEHIND_REGION)".to_string(),
            );
        }
    }

    // Tier 2: Wayland KDE
    #[cfg(feature = "wayland")]
    if is_wayland && is_kde {
        if let Ok(backend) = wayland_kde::WaylandKdeBlurBackend::new(win) {
            if backend.apply(win, 32).is_ok() {
                return (
                    Box::new(backend),
                    BlurStatus::Native,
                    "Tier 2 (Wayland KDE blur protocol)".to_string(),
                );
            }
        }
    }

    // Tier 3: Hyprland / SwayFX
    let is_swayfx_active = is_sway && is_swayfx();
    if is_hyprland || is_swayfx_active {
        let comp_name = if is_hyprland { "Hyprland" } else { "SwayFX" };
        print_tier3_compositor_rules(is_hyprland);
        return (
            Box::new(RuleRequiredBlurBackend),
            BlurStatus::RuleRequired,
            format!("Tier 3 ({} compositor blur rules)", comp_name),
        );
    }

    if is_sway {
        println!("[view-launcher] Vanilla Sway does not support backdrop blur (use SwayFX for compositor blur).");
        println!("[view-launcher] Activating Tier 4 Fallback: high-opacity frosted glass (~95% opacity).");
    }

    // Tier 4: GNOME / Mutter / unknown -> Fallback
    (
        Box::new(fallback::FallbackBlurBackend::new()),
        BlurStatus::Fallback,
        "Tier 4 (Fallback high-opacity glass)".to_string(),
    )
}

#[cfg(all(unix, not(target_os = "macos")))]
fn is_swayfx() -> bool {
    if let Ok(sway_sock) = std::env::var("SWAYSOCK") {
        use std::io::{Read, Write};
        use std::os::unix::net::UnixStream;
        use std::time::Duration;
        if let Ok(mut stream) = UnixStream::connect(&sway_sock) {
            let _ = stream.set_read_timeout(Some(Duration::from_millis(50)));
            let _ = stream.set_write_timeout(Some(Duration::from_millis(50)));
            let mut msg = Vec::with_capacity(14);
            msg.extend_from_slice(b"i3-ipc");
            msg.extend_from_slice(&0u32.to_le_bytes());
            msg.extend_from_slice(&7u32.to_le_bytes()); // Type 7: GET_VERSION
            if stream.write_all(&msg).is_ok() {
                let mut header = [0u8; 14];
                if stream.read_exact(&mut header).is_ok() {
                    let len = u32::from_le_bytes([header[6], header[7], header[8], header[9]]) as usize;
                    let mut payload = vec![0u8; len];
                    if stream.read_exact(&mut payload).is_ok() {
                        let text = String::from_utf8_lossy(&payload).to_lowercase();
                        return text.contains("swayfx");
                    }
                }
            }
        }
    }
    false
}

#[cfg(not(all(unix, not(target_os = "macos"))))]
fn is_swayfx() -> bool {
    false
}

fn print_tier3_compositor_rules(is_hyprland: bool) {
    if is_hyprland {
        println!("[view-launcher] Hyprland blur rules (add to ~/.config/hypr/hyprland.conf):");
        println!("  windowrulev2 = blur, class:^(view-launcher)$");
        println!("  windowrulev2 = ignorealpha 0.2, class:^(view-launcher)$");
        println!("  windowrulev2 = float, class:^(view-launcher)$");
        println!("  windowrulev2 = center, class:^(view-launcher)$");
        println!("  windowrulev2 = noshadow, class:^(view-launcher)$");
    } else {
        println!("[view-launcher] SwayFX blur rules (add to ~/.config/sway/config):");
        println!("  for_window [app_id=\"view-launcher\"] blur enable, floating enable");
    }
}
