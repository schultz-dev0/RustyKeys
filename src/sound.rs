//! Sound playback
//! 
//! 1. Useroveride dir ~/.config/rustkeys/sounds is checked first
//! 2. Then bundled assets is checked
//! 3. Unknown keys fall back to `default.wav`

use crate::config::{self, KeyClass};
use gtk::gdk::Key;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};

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
            Ok(pair) => {
                info!("output stream initialized");
                (Some(pair.0), Some(pair.1))
            }
            Err(err) => {
                error!("failed to initialize output stream: {err}");
                (None, None)
            }
        };

        let bundled_sounds_dir = asset_dir.join("sounds");
        let override_sounds_dir = config::override_sounds_dir();
        if let Err(err) = fs::create_dir_all(&override_sounds_dir) {
            warn!(
                "failed to ensure override dir {}: {err}",
                override_sounds_dir.display()
            );
        }

        let default_sound = ensure_default_sound(&override_sounds_dir, &bundled_sounds_dir);

        let mut class_sounds = HashMap::new();
        class_sounds.insert(
            KeyClass::Normal,
            first_existing(
                &override_sounds_dir,
                &bundled_sounds_dir,
                &["q.wav", "a.wav", "normal.wav"],
            ),
        );
        class_sounds.insert(
            KeyClass::Space,
            first_existing(&override_sounds_dir, &bundled_sounds_dir, &["space.wav"]),
        );
        class_sounds.insert(
            KeyClass::Enter,
            first_existing(&override_sounds_dir, &bundled_sounds_dir, &["enter.wav"]),
        );
        class_sounds.insert(
            KeyClass::Backspace,
            first_existing(
                &override_sounds_dir,
                &bundled_sounds_dir,
                &["backspace.wav"],
            ),
        );
        class_sounds.insert(
            KeyClass::Modifier,
            first_existing(
                &override_sounds_dir,
                &bundled_sounds_dir,
                &["shift.wav", "caps lock.wav", "modifier.wav"],
            ),
        );

        let mut key_sounds = HashMap::new();
        for letter in 'a'..='z' {
            let key = letter.to_string();
            if let Some(path) = resolve_sound(
                &override_sounds_dir,
                &bundled_sounds_dir,
                &format!("{letter}.wav"),
            ) {
                key_sounds.insert(key, path);
            }
        }

        insert_if_exists(
            &mut key_sounds,
            &override_sounds_dir,
            &bundled_sounds_dir,
            "space",
            "space.wav",
        );
        insert_if_exists(
            &mut key_sounds,
            &override_sounds_dir,
            &bundled_sounds_dir,
            "enter",
            "enter.wav",
        );
        insert_if_exists(
            &mut key_sounds,
            &override_sounds_dir,
            &bundled_sounds_dir,
            "backspace",
            "backspace.wav",
        );
        insert_if_exists(
            &mut key_sounds,
            &override_sounds_dir,
            &bundled_sounds_dir,
            "tab",
            "tab.wav",
        );
        insert_if_exists(
            &mut key_sounds,
            &override_sounds_dir,
            &bundled_sounds_dir,
            "shift",
            "shift.wav",
        );
        insert_if_exists(
            &mut key_sounds,
            &override_sounds_dir,
            &bundled_sounds_dir,
            "caps_lock",
            "caps lock.wav",
        );
        insert_if_exists(
            &mut key_sounds,
            &override_sounds_dir,
            &bundled_sounds_dir,
            "bracketleft",
            "[.wav",
        );
        insert_if_exists(
            &mut key_sounds,
            &override_sounds_dir,
            &bundled_sounds_dir,
            "bracketright",
            "].wav",
        );

        debug!(
            "bundled_sounds={} override_sounds={} key_samples={} default={}",
            bundled_sounds_dir.display(),
            override_sounds_dir.display(),
            key_sounds.len(),
            default_sound
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| String::from("none"))
        );

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

    /// Enable or disable playback globally.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Set master output volume in range [0.0, 1.0].
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    /// Play by class when exact key information is unavailable.
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

    /// Play from a GTK key value with per-key mapping and default fallback.
    pub fn play_keyval(&self, keyval: Key, fallback_class: KeyClass) {
        if !self.enabled {
            return;
        }

        if let Some(path) = sample_name_for_key(keyval)
            .as_deref()
            .and_then(|n| self.key_sounds.get(n))
        {
            self.play_path(path);
            return;
        }

        self.play_default_or_class(fallback_class);
    }

    pub fn play_named(&self, sample_name: &str, fallback_class: KeyClass) {
        if !self.enabled {
            return;
        }

        if sample_name == "default" {
            self.play_default_or_class(fallback_class);
            return;
        }

        if let Some(path) = self.key_sounds.get(sample_name) {
            self.play_path(path);
            return;
        }

        self.play_default_or_class(fallback_class);
    }

    fn play_path(&self, path: &Path) {
        let Some(handle) = &self.handle else {
            warn!("no output stream handle; cannot play {}", path.display());
            return;
        };
        if !path.exists() {
            warn!("sample not found: {}", path.display());
            return;
        }

        let Ok(file) = File::open(path) else {
            error!("cannot open sample: {}", path.display());
            return;
        };
        let Ok(decoder) = Decoder::new(BufReader::new(file)) else {
            error!("cannot decode sample: {}", path.display());
            return;
        };
        let Ok(sink) = Sink::try_new(handle) else {
            error!("failed to create sink for sample: {}", path.display());
            return;
        };

        sink.append(decoder.amplify(self.volume));
        sink.detach();
    }

    /// Play default fallback sample if available.
    fn play_default(&self) {
        if let Some(path) = &self.default_sound {
            self.play_path(path);
        }
    }

    /// Prefer default fallback; otherwise use class fallback.
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

/// Create/resolve `default.wav` in the user override directory.
///
/// Priority:
/// 1) ~/.config/rustykeys/sounds/default.wav
/// 2) copy from ~/.config/rustykeys/sounds/a.wav
/// 3) copy from bundled assets/sounds/a.wav into override dir
/// 4) bundled assets/sounds/default.wav (if present)
fn ensure_default_sound(override_dir: &Path, bundled_dir: &Path) -> Option<PathBuf> {
    let override_default = override_dir.join("default.wav");
    if override_default.exists() {
        debug!("default sample present: {}", override_default.display());
        return Some(override_default);
    }

    let source = if override_dir.join("a.wav").exists() {
        override_dir.join("a.wav")
    } else {
        bundled_dir.join("a.wav")
    };

    if source.exists() {
        if fs::copy(&source, &override_default).is_ok() {
            debug!(
                "created default sample from a.wav: {}",
                override_default.display()
            );
            return Some(override_default);
        }
        warn!(
            "failed to copy {} to {}",
            source.display(),
            override_default.display()
        );
    }

    let bundled_default = bundled_dir.join("default.wav");
    if bundled_default.exists() {
        debug!("using bundled default sample: {}", bundled_default.display());
        return Some(bundled_default);
    }

    warn!(
        "default sample unavailable (missing a.wav/default.wav) in {}",
        override_dir.display()
    );

    None
}

/// Return first existing sample from override/bundled directories.
fn first_existing(override_dir: &Path, bundled_dir: &Path, names: &[&str]) -> PathBuf {
    for name in names {
        if let Some(path) = resolve_sound(override_dir, bundled_dir, name) {
            return path;
        }
    }

    bundled_dir.join(names.first().copied().unwrap_or("missing.wav"))
}

/// Insert a named key->path mapping when file exists in override or bundled kits.
fn insert_if_exists(
    map: &mut HashMap<String, PathBuf>,
    override_dir: &Path,
    bundled_dir: &Path,
    key: &str,
    file: &str,
) {
    if let Some(path) = resolve_sound(override_dir, bundled_dir, file) {
        map.insert(key.to_string(), path);
    }
}

/// Resolve one sample file from override first, bundled second.
fn resolve_sound(override_dir: &Path, bundled_dir: &Path, file: &str) -> Option<PathBuf> {
    let override_path = override_dir.join(file);
    if override_path.exists() {
        return Some(override_path);
    }

    let bundled_path = bundled_dir.join(file);
    if bundled_path.exists() {
        return Some(bundled_path);
    }

    None
}

/// Map GTK key values to sample names used by the sound kit.
fn sample_name_for_key(key: Key) -> Option<String> {
    use gtk::gdk::Key;

    match key {
        Key::space => Some("space".into()),
        Key::Return | Key::KP_Enter => Some("enter".into()),
        Key::BackSpace => Some("backspace".into()),
        Key::Tab => Some("tab".into()),
        Key::Shift_L
        | Key::Shift_R
        | Key::Control_L
        | Key::Control_R
        | Key::Alt_L
        | Key::Alt_R
        | Key::Meta_L
        | Key::Meta_R
        | Key::Super_L
        | Key::Super_R => Some("shift".into()),
        Key::Caps_Lock => Some("caps_lock".into()),
        Key::bracketleft => Some("bracketleft".into()),
        Key::bracketright => Some("bracketright".into()),
        _ => key.to_unicode().and_then(|ch| {
            if ch.is_ascii_alphabetic() {
                Some(ch.to_ascii_lowercase().to_string())
            } else {
                None
            }
        }),
    }
}
