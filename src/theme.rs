//! Theme management for app CSS and Matugen integration.

use gtk::prelude::*;
use gtk::{gdk, gio, style_context_add_provider_for_display, CssProvider};
use std::path::{Path, PathBuf};

/// Runtime CSS state and optional file monitor for live Matugen reloads.
pub struct ThemeRuntime {
    monitor: Option<gio::FileMonitor>,
    matugen_provider: CssProvider,
}

impl ThemeRuntime {
    pub fn setup(display: &gdk::Display, asset_dir: &Path, configured_matugen: Option<&str>) -> Self {
        let app_provider = CssProvider::new();
        let app_css = asset_dir.join("style.css");
        if app_css.exists() {
            app_provider.load_from_path(app_css);
            style_context_add_provider_for_display(
                display,
                &app_provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        let matugen_provider = CssProvider::new();
        if let Some(path) = resolve_matugen_css(configured_matugen)
            && path.exists()
        {
            matugen_provider.load_from_path(&path);
            style_context_add_provider_for_display(
                display,
                &matugen_provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
            );
        }

        Self {
            monitor: None,
            matugen_provider,
        }
    }

    /// Watch Matugen CSS file and hot-reload on change.
    pub fn watch_matugen(&mut self, css_path: Option<PathBuf>) {
        let Some(path) = css_path else {
            return;
        };
        if !path.exists() {
            return;
        }

        let file = gio::File::for_path(&path);
        let Ok(monitor) = file.monitor_file(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE) else {
            return;
        };

        let provider = self.matugen_provider.clone();
        monitor.connect_changed(move |_m, _f, _o, ev| {
            if matches!(ev, gio::FileMonitorEvent::Changed | gio::FileMonitorEvent::Created) {
                provider.load_from_path(&path);
            }
        });

        self.monitor = Some(monitor);
    }
}

pub fn resolve_matugen_css(configured: Option<&str>) -> Option<PathBuf> {
    if let Some(path) = configured {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }

    if let Ok(path) = std::env::var("MATUGEN_GTK4_CSS") {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }

    let home = std::env::var("HOME").ok()?;
    let candidates = [
        format!("{home}/.config/matugen/generated/gtk-4.css"),
        format!("{home}/.config/matugen/generated/colors.css"),
    ];

    candidates
        .iter()
        .map(PathBuf::from)
        .find(|path| path.exists())
}

/// Resolve runtime assets directory.
pub fn resolve_asset_dir() -> PathBuf {
    if let Ok(path) = std::env::var("RUSTY_KEYS_ASSET_DIR") {
        return PathBuf::from(path);
    }

    let exe = std::env::current_exe().ok();
    if let Some(parent) = exe.and_then(|p| p.parent().map(|x| x.to_path_buf())) {
        let candidate = parent.join("assets");
        if candidate.exists() {
            return candidate;
        }
    }

    let system_path = PathBuf::from("/usr/share/rusty_keys/assets");
    if system_path.exists() {
        return system_path;
    }

    PathBuf::from("./assets")
}
