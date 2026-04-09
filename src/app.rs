//! Main GTK/libadwaita application shell for Rusty Keys.
//!
//! This module owns daemon-style lifecycle behavior:
//! - close window => hide to background
//! - explicit Exit button => full process quit
//! - unexpected window removal => auto-recreate hidden window

use crate::config::{self, KeyClass};
use crate::global_input::{self, GlobalKeyEvent};
use crate::hyprland;
use crate::sound::SoundEngine;
use crate::theme;
use gtk::prelude::*;
use gtk::{gio, glib};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc;

/// Hyprland-visible app_id/class for window rules.
pub const APP_ID: &str = "org.cloudyy.rustykeys";

/// User-facing window title.
pub const APP_TITLE: &str = "Rusty Keys";

/// Lightweight class fallback for focused window key events.
fn classify_keyval(keyval: gtk::gdk::Key) -> KeyClass {
    use gtk::gdk::Key;

    match keyval {
        Key::space => KeyClass::Space,
        Key::Return | Key::KP_Enter => KeyClass::Enter,
        Key::BackSpace => KeyClass::Backspace,
        Key::Shift_L
        | Key::Shift_R
        | Key::Control_L
        | Key::Control_R
        | Key::Alt_L
        | Key::Alt_R
        | Key::Meta_L
        | Key::Meta_R
        | Key::Super_L
        | Key::Super_R => KeyClass::Modifier,
        _ => KeyClass::Normal,
    }
}

/// Run the Libadwaita application event loop.
pub fn run() {
    let _ = adw::init();

    let app = adw::Application::builder().application_id(APP_ID).build();
    eprintln!("[app] app_id/class: {APP_ID}");
    app.set_accels_for_action("app.quit", &[]);

    let exit_requested = Rc::new(Cell::new(false));

    {
        let exit_requested = exit_requested.clone();
        app.connect_activate(move |app| {
            if let Some(existing) = app.windows().first() {
                existing.present();
                return;
            }
            build_ui(app, exit_requested.clone());
        });
    }

    {
        let exit_requested = exit_requested.clone();
        app.connect_window_removed(move |app, _window| {
            if exit_requested.get() {
                return;
            }
            if app.windows().is_empty() {
                // If compositor/window manager destroys our window, recreate hidden daemon window.
                build_ui(app, exit_requested.clone());
                if let Some(win) = app.windows().first() {
                    win.set_visible(false);
                }
            }
        });
    }

    {
        let exit_requested = exit_requested.clone();
        app.connect_shutdown(move |_| {
            if !exit_requested.get() {
                eprintln!(
                    "rusty_keys: shutdown happened without Exit button; compositor may have forced close"
                );
            }
        });
    }

    app.run();
}

/// Construct the application window and wire all runtime subsystems.
fn build_ui(app: &adw::Application, exit_requested: Rc<Cell<bool>>) {
    let cfg = Rc::new(RefCell::new(config::load()));
    let asset_dir = theme::resolve_asset_dir();
    eprintln!("[app] starting Rusty Keys");
    eprintln!("[app] asset dir: {}", asset_dir.display());
    eprintln!(
        "[app] config: enabled={} volume={:.2}",
        cfg.borrow().enabled,
        cfg.borrow().volume
    );

    let mut sound = SoundEngine::new(&asset_dir);
    {
        let loaded = cfg.borrow().clone();
        sound.set_enabled(loaded.enabled);
        sound.set_volume(loaded.volume);
    }
    let sound = Rc::new(RefCell::new(sound));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title(APP_TITLE)
        .build();
    window.set_hide_on_close(true);

    let header = adw::HeaderBar::new();
    header.set_show_end_title_buttons(true);

    let status = gtk::Label::new(Some("Bridge: listening"));
    status.set_halign(gtk::Align::Start);

    let enabled_row = adw::SwitchRow::builder()
        .title("Enable sounds")
        .active(cfg.borrow().enabled)
        .build();

    let slider = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 1.0, 0.01);
    slider.set_draw_value(true);
    slider.set_value(cfg.borrow().volume as f64);

    let volume_row = adw::ActionRow::builder().title("Volume").build();
    volume_row.add_suffix(&slider);

    let group = adw::PreferencesGroup::new();
    group.add(&enabled_row);
    group.add(&volume_row);

    let page = adw::PreferencesPage::new();
    page.add(&group);

    let view = adw::ToolbarView::new();
    view.add_top_bar(&header);
    view.set_content(Some(&page));
    view.set_vexpand(true);

    let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
    root.set_margin_start(12);
    root.set_margin_end(12);
    root.set_margin_top(12);
    root.set_margin_bottom(12);

    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    footer.set_halign(gtk::Align::End);
    footer.set_valign(gtk::Align::End);

    let exit_button = gtk::Button::with_label("Exit");
    exit_button.add_css_class("destructive-action");
    footer.append(&exit_button);

    root.append(&status);
    root.append(&view);
    root.append(&footer);
    window.set_content(Some(&root));

    window.connect_close_request(|w| {
        // Hard rule: normal window close hides only; daemon continues running.
        w.set_visible(false);
        glib::Propagation::Stop
    });

    {
        let app = app.clone();
        let exit_requested = exit_requested.clone();
        exit_button.connect_clicked(move |_| {
            // Full process termination is allowed only from this explicit action.
            exit_requested.set(true);
            app.quit();
        });
    }

    if let Some(display) = gtk::gdk::Display::default() {
        let mut theme_runtime = theme::ThemeRuntime::setup(
            &display,
            &asset_dir,
            cfg.borrow().matugen_css_path.as_deref(),
        );
        theme_runtime.watch_matugen(theme::resolve_matugen_css(
            cfg.borrow().matugen_css_path.as_deref(),
        ));
        let _ = Box::leak(Box::new(theme_runtime));
    }

    {
        let cfg = cfg.clone();
        let sound = sound.clone();
        enabled_row.connect_active_notify(move |row| {
            let state = row.is_active();
            sound.borrow_mut().set_enabled(state);
            cfg.borrow_mut().enabled = state;
            if let Err(err) = config::save(&cfg.borrow()) {
                eprintln!("save config failed: {err}");
            }
        });
    }

    // Window-focused key fallback path (useful when evdev global input is unavailable).
    let key_controller = gtk::EventControllerKey::new();
    {
        let sound_for_keys = sound.clone();
        key_controller.connect_key_pressed(move |_, keyval, _keycode, _state| {
            let class = classify_keyval(keyval);
            sound_for_keys.borrow().play_keyval(keyval, class);
            glib::Propagation::Proceed
        });
    }
    window.add_controller(key_controller);

    {
        let cfg = cfg.clone();
        let sound = sound.clone();
        slider.connect_value_changed(move |s| {
            let value = s.value().clamp(0.0, 1.0) as f32;
            sound.borrow_mut().set_volume(value);
            cfg.borrow_mut().volume = value;
            if let Err(err) = config::save(&cfg.borrow()) {
                eprintln!("save config failed: {err}");
            }
        });
    }

    let (tx, rx) = mpsc::channel::<KeyClass>();
    match hyprland::start_bridge(tx) {
        Ok(_handle) => eprintln!("[bridge] local trigger socket active"),
        Err(err) => {
            eprintln!("[bridge] failed to start: {err}");
            status.set_text(&format!("Bridge error: {err}"));
        }
    }

    let (global_tx, global_rx) = mpsc::channel::<GlobalKeyEvent>();
    match global_input::start_global_listener(global_tx) {
        Ok((_handle, count)) => {
            eprintln!("[input] global input active (evdev), devices={count}");
            status.set_text(&format!("Global input: active (evdev, {count} device(s))"));
        }
        Err(err) => {
            eprintln!("[input] global input unavailable: {err}");
            status.set_text(&format!(
                "Global input unavailable ({err}); window focus fallback active"
            ));
        }
    }

    let sound_rx = sound.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(8), move || {
        while let Ok(event) = global_rx.try_recv() {
            if let Some(sample_name) = event.sample_name.as_deref() {
                sound_rx
                    .borrow()
                    .play_named(sample_name, event.fallback_class);
            } else {
                sound_rx.borrow().play_class(event.fallback_class);
            }
        }

        while let Ok(key) = rx.try_recv() {
            sound_rx.borrow().play_class(key);
        }

        glib::ControlFlow::Continue
    });

    let present = gio::SimpleAction::new("present", None);
    {
        let window = window.clone();
        present.connect_activate(move |_, _| {
            window.present();
        });
    }
    app.add_action(&present);
    app.set_accels_for_action("app.present", &["<Primary>k"]);

    let hold_guard = app.hold();
    std::mem::forget(hold_guard);
    window.present();
}
