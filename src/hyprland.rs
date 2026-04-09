//! Local Unix-socket trigger bridge.
//!
//! This bridge lets external commands trigger key-class sounds via:
//! `rusty_keys trigger <class>`.

use crate::config::KeyClass;
use std::fs;
use std::io;
use std::os::unix::net::UnixDatagram;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

const SOCKET_NAME: &str = "rusty_keys.sock";

/// Resolve socket path in XDG runtime dir (or /tmp fallback).
pub fn runtime_socket_path() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_RUNTIME_DIR") {
        return Path::new(&dir).join(SOCKET_NAME);
    }
    Path::new("/tmp").join(SOCKET_NAME)
}

/// Send a one-shot class trigger payload to the local bridge socket.
pub fn send_trigger(class: &str) -> Result<(), String> {
    let socket = UnixDatagram::unbound().map_err(|e| format!("socket create failed: {e}"))?;
    socket
        .send_to(class.as_bytes(), runtime_socket_path())
        .map_err(|e| format!("send trigger failed: {e}"))?;
    Ok(())
}

/// Start bridge listener thread and forward parsed classes to the app.
pub fn start_bridge(tx: Sender<KeyClass>) -> Result<thread::JoinHandle<()>, String> {
    let path = runtime_socket_path();
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("remove stale socket failed: {e}"))?;
    }

    let sock = UnixDatagram::bind(&path).map_err(|e| format!("bind bridge socket failed: {e}"))?;
    sock.set_nonblocking(true)
        .map_err(|e| format!("set nonblocking failed: {e}"))?;

    let handle = thread::spawn(move || {
        let mut buffer = [0_u8; 128];
        loop {
            match sock.recv_from(&mut buffer) {
                Ok((len, _addr)) => {
                    let value = String::from_utf8_lossy(&buffer[..len]).to_string();
                    let key = KeyClass::from_wire(&value);
                    if tx.send(key).is_err() {
                        break;
                    }
                }
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(12));
                }
                Err(_) => {
                    thread::sleep(Duration::from_millis(200));
                }
            }
        }
    });

    Ok(handle)
}
