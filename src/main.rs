//! Rusty Keys executable entrypoint.
//!
//! Modes:
//! - `rusty_keys` -> launch GUI/daemon app
//! - `rusty_keys trigger <class>` -> send local bridge trigger

mod app;
mod config;
mod global_input;
mod hyprland;
mod sound;
mod theme;

use std::env;
use tracing::error;

fn main() {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();
    if args.get(1).is_some_and(|arg| arg == "trigger") {
        let class = args.get(2).map(String::as_str).unwrap_or("normal");
        if let Err(err) = hyprland::send_trigger(class) {
            error!("trigger failed: {err:#}");
            std::process::exit(1);
        }
        return;
    }

    app::run();
}
