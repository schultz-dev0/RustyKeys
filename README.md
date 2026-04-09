# Rusty Keys

GTK4/Libadwaita keyboard sound daemon for Hyprland/Linux.

![](https://github.com/schultz-dev0/RustyKeys/blob/main/assets/rustykeys.png)

![](https://github.com/schultz-dev0/RustyKeys/blob/main/assets/Rustykeys_preview.png)
## Current Status

- Per-key sound kit playback works.
- Unknown/unmapped keys fall back to `default.wav`.
- Global key capture uses Linux `evdev` and prefers physical keyboards.
- Window close hides to background. Full quit is via the in-app `Exit` button.

## Hyprland App Class / Name

- App ID (window class/app_id): `org.cloudyy.rustykeys`
- Window title: `Rusty Keys`

Example Hyprland rule:

```ini
windowrule {
    name            = rustykeys
    match:class     = ^(org.cloudyy.rustykeys)$
    float           = on
    size            = 550 420
    center          = on
}
```

## Install

```bash
git clone https://github.com/schultz-dev0/RustyKeys.git
```
```bash
cd RustyKeys
```
```bash
./install.sh
```

Installer actions:

- Builds and installs `rusty_keys` to `~/.local/bin`
- Ensures `~/.config/rustykeys/sounds/` exists
- Seeds initial `.wav` files into the override directory when empty
- Creates desktop entry at `~/.local/share/applications/rusty_keys.desktop`
- Adds `~/.local/bin` to shell PATH if missing

## Run

```bash
cargo run
```

Optional local bridge trigger:

```bash
cargo run -- trigger enter
```

## Sound Kit Override (User Replaceable)

Drop replacement `.wav` files into:

`~/.config/rustykeys/sounds/`

Override files take precedence over bundled files in `assets/sounds/`.

As long as names match, the override kit is used automatically.

Common names:

- `a.wav ... z.wav`
- `space.wav`
- `enter.wav`
- `backspace.wav`
- `tab.wav`
- `shift.wav`
- `caps lock.wav`
- `[.wav`
- `].wav`
- `default.wav`

## Default Fallback Behavior

- If `~/.config/rustykeys/sounds/default.wav` exists, it is used for unknown keys.
- If not, the app tries to create it by copying `a.wav` into `default.wav`.
- If that fails, class fallback is used.

## Notes

- If no global input device can be read (permissions/device access), the app falls back to focused-window key events.
- Startup logs in terminal are intentional and help diagnose input/audio routing quickly.
