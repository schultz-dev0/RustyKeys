use crate::config::KeyClass;
use gtk::gdk::Key;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

pub struct SoundEngine {
    enabled: bool,
    volume: f32,
    _stream: Option<OutputStream>,
    handle: Option<OutputStreamHandle>,
    class_sounds: HashMap<KeyClass, PathBuf>,
    key_sounds: HashMap<String, PathBuf>,
    default_sound: Option<PathBuf>,
}


impl SoundEngine {
    pub fn new(asset_dir: &Path) -> Self {
        let (stream, handle) = match OutputStream::try_default() {
            Ok(pair) => (Some(pair.0), Some(pair.1)),
            Err(_) => (None, None),
        };

        let sounds_dir = asset_dir.join("sounds");
        let default_sound = ensure_default_sound(&sounds_dir);

        let mut class_sounds = HashMap::new();
        class_sounds.insert(
            KeyClass::Normal,
            first_existing(&sounds_dir, &["q.wav", "a.wav", "normal.wav"]),
        );
        class_sounds.insert(KeyClass::Space, first_existing(&sounds_dir, &["space.wav"]));
        class_sounds.insert(KeyClass::Enter, first_existing(&sounds_dir, &["enter.wav"]));
        class_sounds.insert(
            KeyClass::Backspace,
            first_existing(&sounds_dir, &["backspace.wav"]),
        );
        class_sounds.insert(
            KeyClass::Modifier,
            first_existing(&sounds_dir, &["shift.wav", "caps lock.wav", "modifier.wav"]),
        );

        let mut key_sounds = HashMap::new();
        for letter in 'a'..='z' {
            let key = letter.to_string();
            let path = sounds_dir.join(format!("{letter}.wav"));
            if path.exists() {
                key_sounds.insert(key, path);
            }
        }

        insert_if_exists(&mut key_sounds, &sounds_dir, "space", "space.wav");
        insert_if_exists(&mut key_sounds, &sounds_dir, "enter", "enter.wav");
        insert_if_exists(&mut key_sounds, &sounds_dir, "backspace", "backspace.wav");
        insert_if_exists(&mut key_sounds, &sounds_dir, "tab", "tab.wav");
        insert_if_exists(&mut key_sounds, &sounds_dir, "shift", "shift.wav");
        insert_if_exists(&mut key_sounds, &sounds_dir, "caps_lock", "caps lock.wav");
        insert_if_exists(&mut key_sounds, &sounds_dir, "bracketleft", "[.wav");
        insert_if_exists(&mut key_sounds, &sounds_dir, "bracketright", "].wav");

        Self {
            enabled: true,
            volume: 0.45,
            _stream: stream,
            handle,
            class_sounds,
            key_sounds,
            default_sound,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    pub fn play_class(&self, class: KeyClass) {
        if !self.enabled {
            return;
        }

        let Some(path) = self.class_sounds.get(&class) else {
            self.play_default();
            return;
        };

        self.play_path(path);
    }

    pub fn play_keyval(&self, keyval: Key, fallback_class: KeyClass) {
        if !self.enabled {
            return;
        }

        if let Some(sample_name) = sample_name_for_key(keyval) {
            if let Some(path) = self.key_sounds.get(sample_name) {
                self.play_path(path);
                return;
            }
        }

        self.play_default_or_class(fallback_class);
    }

    fn play_path(&self, path: &Path) {
        let Some(handle) = &self.handle else {
            return;
        };
        if !path.exists() {
            return;
        }

        let Ok(file) = File::open(path) else {
            return;
        };
        let Ok(decoder) = Decoder::new(BufReader::new(file)) else {
            return;
        };
        let Ok(sink) = Sink::try_new(handle) else {
            return;
        };

        sink.append(decoder.amplify(self.volume));
        sink.detach();
    }

    fn play_default(&self) {
        if let Some(path) = &self.default_sound {
            self.play_path(path);
        }
    }

    fn play_default_or_class(&self, fallback_class: KeyClass) {
        if self.default_sound.is_some() {
            self.play_default();
            return;
        }

        if let Some(path) = self.class_sounds.get(&fallback_class) {
            self.play_path(path);
        }
    }
}

fn ensure_default_sound(sounds_dir: &Path) -> Option<PathBuf> {
    let default = sounds_dir.join("default.wav");
    if default.exists() {
        return Some(default);
    }

    let source = sounds_dir.join("a.wav");
    if source.exists() {
        if fs::copy(&source, &default).is_ok() {
            return Some(default);
        }
    }

    None
}

fn first_existing(dir: &Path, names: &[&str]) -> PathBuf {
    for name in names {
        let candidate = dir.join(name);
        if candidate.exists() {
            return candidate;
        }
    }
    dir.join(names.first().copied().unwrap_or("missing.wav"))
}

fn insert_if_exists(map: &mut HashMap<String, PathBuf>, dir: &Path, key: &str, file: &str) {
    let path = dir.join(file);
    if path.exists() {
        map.insert(key.to_string(), path);
    }
}

fn sample_name_for_key(key: Key) -> Option<&'static str> {
    use gtk::gdk::Key;

    match key {
        Key::space => Some("space"),
        Key::Return | Key::KP_Enter => Some("enter"),
        Key::BackSpace => Some("backspace"),
        Key::Tab => Some("tab"),
        Key::Shift_L
        | Key::Shift_R
        | Key::Control_L
        | Key::Control_R
        | Key::Alt_L
        | Key::Alt_R
        | Key::Meta_L
        | Key::Meta_R
        | Key::Super_L
        | Key::Super_R => Some("shift"),
        Key::Caps_Lock => Some("caps_lock"),
        Key::bracketleft => Some("bracketleft"),
        Key::bracketright => Some("bracketright"),
        _ => key.to_unicode().and_then(|ch| {
            if ch.is_ascii_alphabetic() {
                match ch.to_ascii_lowercase() {
                    'a' => Some("a"),
                    'b' => Some("b"),
                    'c' => Some("c"),
                    'd' => Some("d"),
                    'e' => Some("e"),
                    'f' => Some("f"),
                    'g' => Some("g"),
                    'h' => Some("h"),
                    'i' => Some("i"),
                    'j' => Some("j"),
                    'k' => Some("k"),
                    'l' => Some("l"),
                    'm' => Some("m"),
                    'n' => Some("n"),
                    'o' => Some("o"),
                    'p' => Some("p"),
                    'q' => Some("q"),
                    'r' => Some("r"),
                    's' => Some("s"),
                    't' => Some("t"),
                    'u' => Some("u"),
                    'v' => Some("v"),
                    'w' => Some("w"),
                    'x' => Some("x"),
                    'y' => Some("y"),
                    'z' => Some("z"),
                    _ => None,
                }
            } else {
                None
            }
        }),
    }
}
