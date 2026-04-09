#!/usr/bin/env bash
set -euo pipefail

APP_NAME="rusty_keys"
APP_ID="org.cloudyy.rustykeys"
PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="${HOME}/.local/bin"
CONFIG_DIR="${HOME}/.config/rustykeys"
SOUNDS_DIR="${CONFIG_DIR}/sounds"
DESKTOP_DIR="${HOME}/.local/share/applications"
DESKTOP_FILE="${DESKTOP_DIR}/${APP_NAME}.desktop"

log() {
  printf '[install] %s\n' "$*"
}

append_path_if_missing() {
  local rc_file="$1"
  local marker="# Added by Rusty Keys installer"
  local export_line='export PATH="$HOME/.local/bin:$PATH"'

  [[ -f "$rc_file" ]] || touch "$rc_file"

  if grep -Fq "$marker" "$rc_file"; then
    return
  fi

  {
    echo
    echo "$marker"
    echo "$export_line"
  } >>"$rc_file"

  log "Updated PATH in ${rc_file}"
}

mkdir -p "$BIN_DIR" "$CONFIG_DIR" "$SOUNDS_DIR" "$DESKTOP_DIR"

log "Building release binary"
(
  cd "$PROJECT_DIR"
  cargo build --release
)

install -m 0755 "$PROJECT_DIR/target/release/${APP_NAME}" "$BIN_DIR/${APP_NAME}"
log "Installed binary to ${BIN_DIR}/${APP_NAME}"

if ! compgen -G "$SOUNDS_DIR/*.wav" >/dev/null; then
  log "No user override kit found, seeding sounds from bundled assets"
  cp -n "$PROJECT_DIR"/assets/sounds/*.wav "$SOUNDS_DIR"/ 2>/dev/null || true
fi

cat >"$DESKTOP_FILE" <<EOF
[Desktop Entry]
Version=1.0
Type=Application
Name=Rusty Keys
Comment=Mechanical keyboard sound daemon
Exec=${APP_NAME}
Terminal=false
Categories=Utility;
StartupNotify=false
StartupWMClass=${APP_ID}
X-GNOME-WMClass=${APP_ID}
EOF
log "Wrote desktop entry to ${DESKTOP_FILE}"

if [[ ":$PATH:" != *":${BIN_DIR}:"* ]]; then
  append_path_if_missing "${HOME}/.zshrc"
  append_path_if_missing "${HOME}/.bashrc"
  log "Open a new shell session to refresh PATH"
else
  log "PATH already contains ${BIN_DIR}"
fi

log "Done"
