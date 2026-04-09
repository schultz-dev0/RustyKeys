use crate::config::{self, KeyClass};
use crate::hyprland;
use crate::sound::SoundEngine;
use crate::theme;
use gtk::prelude::*;
use gtk::{gio, glib};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

const APP_ID: &str = "dev.cloudyy.rusty_keys";

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

pub fn run() {
    let _ = adw::init();

    let app = adw::Application::builder().application_id(APP_ID).build();

    app.connect_activate(|app| {
        if let Some(existing) = app.windows().first() {
            existing.present();
            return;
        }
        build_ui(app);
    });
    app.run();
}

fn build_ui(app: &adw::Application) {
    let cfg = Rc::new(RefCell::new(config::load()));
    let asset_dir = theme::resolve_asset_dir();

    let mut sound = SoundEngine::new(&asset_dir);
    {
        let loaded = cfg.borrow().clone();
        sound.set_enabled(loaded.enabled);
        sound.set_volume(loaded.volume);
    }
    let sound = Rc::new(RefCell::new(sound));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Rusty Keys")
        .default_width(420)
        .default_height(220)
        .build();

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

    let log_row = adw::SwitchRow::builder()
        .title("Console key logs")
        .subtitle("Temporary debug output while sound assets are missing")
        .active(cfg.borrow().log_keys_to_console)
        .build();

    let theme_path = theme::resolve_matugen_css(cfg.borrow().matugen_css_path.as_deref())
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| String::from("none"));
    let theme_row = adw::ActionRow::builder()
        .title("Matugen CSS")
        .subtitle(theme_path)
        .build();

    let group = adw::PreferencesGroup::new();
    group.add(&enabled_row);
    group.add(&volume_row);
    group.add(&log_row);
    group.add(&theme_row);

    let page = adw::PreferencesPage::new();
    page.add(&group);

    let view = adw::ToolbarView::new();
    view.add_top_bar(&header);
    view.set_content(Some(&page));

    let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
    root.set_margin_start(12);
    root.set_margin_end(12);
    root.set_margin_top(12);
    root.set_margin_bottom(12);
    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    footer.set_halign(gtk::Align::End);

    let exit_button = gtk::Button::with_label("Exit");
    exit_button.add_css_class("destructive-action");
    footer.append(&exit_button);

    root.append(&status);
    root.append(&view);
    root.append(&footer);
    window.set_content(Some(&root));

    window.connect_close_request(|w| {
        w.set_visible(false);
        glib::Propagation::Stop
    });

    {
        let app = app.clone();
        exit_button.connect_clicked(move |_| {
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

    {
        let cfg = cfg.clone();
        log_row.connect_active_notify(move |row| {
            let state = row.is_active();
            cfg.borrow_mut().log_keys_to_console = state;
            if let Err(err) = config::save(&cfg.borrow()) {
                eprintln!("save config failed: {err}");
            }
        });
    }

    let key_controller = gtk::EventControllerKey::new();
    {
        let cfg = cfg.clone();
        let sound_for_keys = sound.clone();
        key_controller.connect_key_pressed(move |_, keyval, _keycode, state| {
            let class = classify_keyval(keyval);
            sound_for_keys.borrow().play_keyval(keyval, class);
            if cfg.borrow().log_keys_to_console {
                println!(
                    "[window-key] key={keyval:?} state={state:?} class={class:?}"
                );
            }
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
        Ok(_handle) => {}
        Err(err) => status.set_text(&format!("Bridge error: {err}")),
    }

    let sound_rx = sound.clone();
    let cfg_rx = cfg.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(8), move || {
        while let Ok(key) = rx.try_recv() {
            if cfg_rx.borrow().log_keys_to_console {
                println!("[bridge-key] class={key:?}");
            }
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
