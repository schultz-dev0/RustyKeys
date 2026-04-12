//! Local Unix-socket trigger bridge.
//!
//! This bridge lets external commands trigger key-class sounds via:
//! `rusty_keys trigger <class>`.

use crate::config::KeyClass;
use anyhow::{Context, Result};
use std::fs;
use std::io;
use std::os::unix::net::UnixDatagram;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;
use tracing::warn;

const SOCKET_NAME: &str = "rusty_keys.sock";

pub fn runtime_socket_path() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_RUNTIME_DIR") {
        return Path::new(&dir).join(SOCKET_NAME);
    }
    Path::new("/tmp").join(SOCKET_NAME)
}

pub fn send_trigger(class: &str) -> Result<()> {
    let socket = UnixDatagram::unbound().context("socket create failed")?;
    socket
        .send_to(class.as_bytes(), runtime_socket_path())
        .context("send trigger failed")?;
    Ok(())
}

pub fn start_bridge(tx: Sender<KeyClass>) -> Result<thread::JoinHandle<()>> {
    let path = runtime_socket_path();

    let sock = match UnixDatagram::bind(&path) {
        Ok(s) => s,
        Err(err) if err.kind() == io::ErrorKind::AddrInUse => {
            fs::remove_file(&path).context("remove stale socket failed")?;
            UnixDatagram::bind(&path).context("bind bridge socket failed")?
        }
        Err(err) => return Err(err).context("bind bridge socket failed"),
    };
    sock.set_nonblocking(true).context("set nonblocking failed")?;

    let handle = thread::spawn(move || {
        let mut buffer = [0_u8; 128];
        loop {
            match sock.recv_from(&mut buffer) {
                Ok((len, _addr)) => {
                    let value = String::from_utf8_lossy(&buffer[..len]).to_string();
                    let key = KeyClass::from_wire(&value);
                    if tx.send(key).is_err() {
                        warn!("receiver dropped, bridge thread exiting");
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
