#!/usr/bin/env bash
set -euo pipefail

APP_NAME="rusty_keys"
PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TMP_ROOT="/dev/shm/${APP_NAME}_${UID}"
RUNTIME_DIR="${TMP_ROOT}/runtime"
ASSET_DIR="${RUNTIME_DIR}/assets"

mkdir -p "${RUNTIME_DIR}"
rm -rf "${RUNTIME_DIR:?}"/*

if [[ ! -x "${PROJECT_DIR}/target/release/${APP_NAME}" ]]; then
  echo "Building release binary..."
  cargo -C "${PROJECT_DIR}" build --release
fi

cp "${PROJECT_DIR}/target/release/${APP_NAME}" "${RUNTIME_DIR}/${APP_NAME}"
mkdir -p "${ASSET_DIR}"
if [[ -d "${PROJECT_DIR}/assets" ]]; then
  cp -r "${PROJECT_DIR}/assets/." "${ASSET_DIR}/"
fi

export RUSTY_KEYS_ASSET_DIR="${ASSET_DIR}"
exec "${RUNTIME_DIR}/${APP_NAME}" "$@"
