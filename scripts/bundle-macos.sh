#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

if [ "$(uname)" != "Darwin" ]; then
  echo "Run this on a Mac; an app bundle needs a macOS build of glint" >&2
  exit 1
fi

cargo build --release --bin glint
version="$(cargo metadata --no-deps --format-version 1 \
  | sed -n 's/.*"name":"glint","version":"\([^"]*\)".*/\1/p')"

app="target/release/Glint.app"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp target/release/glint "$app/Contents/MacOS/glint"
cp assets/icons/glint.icns "$app/Contents/Resources/glint.icns"
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Glint</string>
  <key>CFBundleDisplayName</key><string>Glint</string>
  <key>CFBundleIdentifier</key><string>app.glint.launcher</string>
  <key>CFBundleExecutable</key><string>glint</string>
  <key>CFBundleIconFile</key><string>glint</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>${version:-0.1.0}</string>
  <key>CFBundleShortVersionString</key><string>${version:-0.1.0}</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>LSApplicationCategoryType</key><string>public.app-category.games</string>
</dict>
</plist>
PLIST

echo "Built $app"
