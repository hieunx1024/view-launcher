#[cfg(unix)]
use std::path::PathBuf;

#[cfg(unix)]
pub fn get_socket_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir).join("view-launcher.sock")
    } else {
        let user = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
        PathBuf::from(format!("/tmp/view-launcher-{}.sock", user))
    }
}

#[cfg(windows)]
pub fn get_socket_addr() -> &'static str {
    "127.0.0.1:42425"
}

pub fn send_toggle_message() -> bool {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::net::UnixStream;
        let path = get_socket_path();
        if let Ok(mut stream) = UnixStream::connect(&path) {
            let _ = stream.write_all(b"toggle");
            return true;
        }
        false
    }
    #[cfg(windows)]
    {
        use std::io::Write;
        use std::net::TcpStream;
        if let Ok(mut stream) = TcpStream::connect(get_socket_addr()) {
            let _ = stream.write_all(b"toggle");
            return true;
        }
        false
    }
}
