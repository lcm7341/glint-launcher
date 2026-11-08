#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

if [ $# -eq 0 ]; then
  cargo build --release --bin glint
fi
binary="${1:-$PWD/target/release/glint}"
binary="$(realpath "$binary")"

data="${XDG_DATA_HOME:-$HOME/.local/share}"
for size in 16 32 64 128 256 512; do
  folder="$data/icons/hicolor/${size}x${size}/apps"
  mkdir -p "$folder"
  cp "assets/icons/glint-$size.png" "$folder/glint.png"
done

mkdir -p "$data/applications"
sed "s|^Exec=.*|Exec=\"$binary\"|" assets/glint.desktop > "$data/applications/glint.desktop"

command -v update-desktop-database >/dev/null && update-desktop-database -q "$data/applications" || true
command -v kbuildsycoca6 >/dev/null && kbuildsycoca6 --noincremental >/dev/null 2>&1 || true

echo "Installed Glint's icon and menu entry for $binary"
