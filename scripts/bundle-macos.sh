#!/bin/sh
# Assemble target/bundle/Cub.app from a release GUI build and zip it.
# Usage: scripts/bundle-macos.sh   (run from the repo root, on macOS)
set -eu

cargo build --release --no-default-features --features gui --locked

APP="target/bundle/Cub.app"
VERSION=$(cargo pkgid | sed 's/.*[#@]//')
rm -rf target/bundle
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/cub "$APP/Contents/Resources/cub"

# Finder launches the bundle executable with no arguments; start the GUI.
cat > "$APP/Contents/MacOS/cub" <<'LAUNCH'
#!/bin/sh
exec "$(dirname "$0")/../Resources/cub" gui "$@"
LAUNCH
chmod +x "$APP/Contents/MacOS/cub"

# App icon: assets/icon.png (1024x1024, transparent) -> Cub.icns
ICONSET=target/bundle/Cub.iconset
mkdir -p "$ICONSET"
for size in 16 32 128 256 512; do
  sips -z $size $size assets/icon.png --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
  sips -z $((size * 2)) $((size * 2)) assets/icon.png --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/Cub.icns"
rm -rf "$ICONSET"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>Cub</string>
  <key>CFBundleDisplayName</key><string>Cub</string>
  <key>CFBundleIdentifier</key><string>io.github.vivainio.cub</string>
  <key>CFBundleExecutable</key><string>cub</string>
  <key>CFBundleIconFile</key><string>Cub</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>CFBundleDocumentTypes</key><array><dict>
    <key>CFBundleTypeExtensions</key><array><string>leo</string></array>
    <key>CFBundleTypeRole</key><string>Editor</string>
  </dict></array>
</dict></plist>
PLIST

# Ad-hoc sign so Gatekeeper treats it as a coherent (if unnotarized) bundle.
codesign --force --deep --sign - "$APP"
(cd target/bundle && ditto -c -k --keepParent "Cub.app" cub-macos.zip)
echo "target/bundle/cub-macos.zip"
