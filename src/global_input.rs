use crate::config::KeyClass;
use anyhow::{bail, Result};
use evdev::{enumerate, Device, InputEventKind, Key};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;
use tracing::{error, info};

#[derive(Debug, Clone)]
pub struct GlobalKeyEvent {
    pub sample_name: Option<String>,
    pub fallback_class: KeyClass,
}

pub fn start_global_listener(
    tx: Sender<GlobalKeyEvent>,
) -> Result<(thread::JoinHandle<()>, usize)> {
    let mut candidates = pick_keyboard_devices();
    if candidates.is_empty() {
        bail!("No readable keyboard devices found");
    }

    let mut selected: Vec<(PathBuf, Device)> = candidates
        .drain(..)
        .filter(|(_, dev)| !is_virtual_keyboard(dev))
        .collect();
    if selected.is_empty() {
        selected = pick_keyboard_devices();
    }

    let selected_count = selected.len();
    info!("Monitoring {selected_count} keyboard(s)");

    let supervisor = thread::spawn(move || {
        for (path, mut keyboard) in selected {
            let tx = tx.clone();
            let name = keyboard
                .name()
                .map(ToString::to_string)
                .unwrap_or_else(|| String::from("unnamed"));

            info!("listening on {} ({name})", path.display());

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
                    Err(err) => {
                        error!(
                            "fetch_events error on {} ({name}): {err}",
                            path.display()
                        );
                        thread::sleep(Duration::from_millis(10));
                    }
                }
            });
        }

        loop {
            thread::sleep(Duration::from_secs(60));
        }
    });

    Ok((supervisor, selected_count))
}

fn pick_keyboard_devices() -> Vec<(PathBuf, Device)> {
    let mut devices = Vec::new();
    for (path, dev) in enumerate() {
        if is_keyboard_candidate(&dev) {
            devices.push((path.to_path_buf(), dev));
        }
    }
    devices
}

// Check if a device looks like a real keyboard by checking for common keys
fn is_keyboard_candidate(dev: &Device) -> bool {
    let Some(keys) = dev.supported_keys() else {
        return false;
    };
    [Key::KEY_A, Key::KEY_S, Key::KEY_ENTER, Key::KEY_SPACE, Key::KEY_BACKSPACE]
        .iter()
        .all(|&k| keys.contains(k))
}

// Filter out common virtual/software devices
fn is_virtual_keyboard(dev: &Device) -> bool {
    let name = dev
        .name()
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    name.contains("virtual") || name.contains("openlinkhub") || name.contains("uinput")
}

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
                (Some("default".to_string()), KeyClass::Normal)
            }
        }
    };

    GlobalKeyEvent {
        sample_name,
        fallback_class,
    }
}

fn alpha_key_to_char(key: Key) -> Option<char> {
    // Basic QWERTY mapping
    const KEYS: [(Key, char); 26] = [
        (Key::KEY_Q, 'q'), (Key::KEY_W, 'w'), (Key::KEY_E, 'e'), (Key::KEY_R, 'r'), (Key::KEY_T, 't'),
        (Key::KEY_Y, 'y'), (Key::KEY_I, 'i'), (Key::KEY_O, 'o'), (Key::KEY_P, 'p'), (Key::KEY_U, 'u'),
        (Key::KEY_A, 'a'), (Key::KEY_S, 's'), (Key::KEY_D, 'd'), (Key::KEY_F, 'f'), (Key::KEY_G, 'g'),
        (Key::KEY_H, 'h'), (Key::KEY_J, 'j'), (Key::KEY_K, 'k'), (Key::KEY_L, 'l'),
        (Key::KEY_Z, 'z'), (Key::KEY_X, 'x'), (Key::KEY_C, 'c'), (Key::KEY_V, 'v'), (Key::KEY_B, 'b'),
        (Key::KEY_N, 'n'), (Key::KEY_M, 'm'),
    ];
    KEYS.iter().find(|&&(k, _)| k == key).map(|&(_, c)| c)
}
