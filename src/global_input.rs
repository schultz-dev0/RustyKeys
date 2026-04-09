//! Global keyboard input backend based on Linux evdev.
//!
//! This listener feeds key events into the app even when the Rusty Keys window
//! is unfocused. It prefers physical keyboard devices and ignores obvious
//! virtual keyboard devices when possible.

use crate::config::KeyClass;
use evdev::{enumerate, Device, InputEventKind, Key};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

/// Global key event payload passed to the audio loop.
#[derive(Debug, Clone)]
pub struct GlobalKeyEvent {
    pub sample_name: Option<String>,
    pub fallback_class: KeyClass,
}

/// Start global input workers and return `(supervisor_handle, device_count)`.
pub fn start_global_listener(
    tx: Sender<GlobalKeyEvent>,
) -> Result<(thread::JoinHandle<()>, usize), String> {
    let mut candidates = pick_keyboard_devices();
    if candidates.is_empty() {
        return Err(String::from("no readable keyboard device found (evdev)"));
    }

    // Prefer physical keyboards. If none are available, fall back to virtual ones.
    let mut selected: Vec<(PathBuf, Device)> = candidates
        .drain(..)
        .filter(|(_, dev)| !is_virtual_keyboard(dev))
        .collect();
    if selected.is_empty() {
        selected = pick_keyboard_devices();
    }

    let selected_count = selected.len();
    eprintln!("[input] selected {selected_count} keyboard device(s)");

    let supervisor = thread::spawn(move || {
        for (path, mut keyboard) in selected {
            let tx = tx.clone();
            let name = keyboard
                .name()
                .map(ToString::to_string)
                .unwrap_or_else(|| String::from("unnamed"));

            eprintln!("[input] listening on {} ({name})", path.display());

            thread::spawn(move || loop {
                match keyboard.fetch_events() {
                    Ok(events) => {
                        for ev in events {
                            if let InputEventKind::Key(key) = ev.kind() {
                                if ev.value() != 1 {
                                    continue;
                                }

                                let event = map_key(key);
                                if tx.send(event).is_err() {
                                    return;
                                }
                            }
                        }
                    }
                    Err(_) => {
                        thread::sleep(Duration::from_millis(8));
                    }
                }
            });
        }

        // Keep the supervisor alive while worker threads run.
        loop {
            thread::sleep(Duration::from_secs(60));
        }
    });

    Ok((supervisor, selected_count))
}

/// Enumerate all keyboard-capable devices.
fn pick_keyboard_devices() -> Vec<(PathBuf, Device)> {
    let mut devices = Vec::new();
    for (path, dev) in enumerate() {
        if is_keyboard_candidate(&dev) {
            devices.push((path.to_path_buf(), dev));
        }
    }
    devices
}

/// Heuristic keyboard filter: must expose both alpha and enter keys.
fn is_keyboard_candidate(dev: &Device) -> bool {
    let keys = dev.supported_keys();
    keys.map(|k| k.contains(Key::KEY_A) && k.contains(Key::KEY_ENTER))
        .unwrap_or(false)
}

    /// Identify obvious virtual keyboard sources so physical devices are preferred.
fn is_virtual_keyboard(dev: &Device) -> bool {
    let name = dev
        .name()
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    name.contains("virtual") || name.contains("openlinkhub") || name.contains("uinput")
}

/// Convert evdev key codes into sample names and fallback classes.
fn map_key(key: Key) -> GlobalKeyEvent {
    let (sample_name, fallback_class) = match key {
        Key::KEY_SPACE => (Some("space".to_string()), KeyClass::Space),
        Key::KEY_ENTER | Key::KEY_KPENTER => (Some("enter".to_string()), KeyClass::Enter),
        Key::KEY_BACKSPACE => (Some("backspace".to_string()), KeyClass::Backspace),
        Key::KEY_TAB => (Some("tab".to_string()), KeyClass::Normal),
        Key::KEY_LEFTSHIFT
        | Key::KEY_RIGHTSHIFT
        | Key::KEY_LEFTCTRL
        | Key::KEY_RIGHTCTRL
        | Key::KEY_LEFTALT
        | Key::KEY_RIGHTALT
        | Key::KEY_LEFTMETA
        | Key::KEY_RIGHTMETA => {
            (Some("shift".to_string()), KeyClass::Modifier)
        }
        Key::KEY_CAPSLOCK => (Some("caps_lock".to_string()), KeyClass::Modifier),
        Key::KEY_LEFTBRACE => (Some("bracketleft".to_string()), KeyClass::Normal),
        Key::KEY_RIGHTBRACE => (Some("bracketright".to_string()), KeyClass::Normal),
        _ => {
            if let Some(ch) = alpha_key_to_char(key) {
                (Some(ch.to_string()), KeyClass::Normal)
            } else {
                // For obscure/unmapped keys, force default.wav path in the sound engine.
                (Some("default".to_string()), KeyClass::Normal)
            }
        }
    };

    GlobalKeyEvent {
        sample_name,
        fallback_class,
    }
}

/// Convert alpha evdev keys into lowercase sample names.
fn alpha_key_to_char(key: Key) -> Option<char> {
    match key {
        Key::KEY_A => Some('a'),
        Key::KEY_B => Some('b'),
        Key::KEY_C => Some('c'),
        Key::KEY_D => Some('d'),
        Key::KEY_E => Some('e'),
        Key::KEY_F => Some('f'),
        Key::KEY_G => Some('g'),
        Key::KEY_H => Some('h'),
        Key::KEY_I => Some('i'),
        Key::KEY_J => Some('j'),
        Key::KEY_K => Some('k'),
        Key::KEY_L => Some('l'),
        Key::KEY_M => Some('m'),
        Key::KEY_N => Some('n'),
        Key::KEY_O => Some('o'),
        Key::KEY_P => Some('p'),
        Key::KEY_Q => Some('q'),
        Key::KEY_R => Some('r'),
        Key::KEY_S => Some('s'),
        Key::KEY_T => Some('t'),
        Key::KEY_U => Some('u'),
        Key::KEY_V => Some('v'),
        Key::KEY_W => Some('w'),
        Key::KEY_X => Some('x'),
        Key::KEY_Y => Some('y'),
        Key::KEY_Z => Some('z'),
        _ => None,
    }
}
