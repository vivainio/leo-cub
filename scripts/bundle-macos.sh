#!/bin/sh
# Assemble target/bundle/Leo Cub.app from a release GUI build and zip it.
# Usage: scripts/bundle-macos.sh   (run from the repo root, on macOS)
set -eu

cargo build --release --no-default-features --features gui --locked

APP="target/bundle/Leo Cub.app"
VERSION=$(cargo pkgid | sed 's/.*[#@]//')
rm -rf target/bundle
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/cub "$APP/Contents/Resources/cub"

# Finder launches the bundle executable with no arguments; start the GUI.
cat > "$APP/Contents/MacOS/leo-cub" <<'LAUNCH'
#!/bin/sh
exec "$(dirname "$0")/../Resources/cub" gui "$@"
LAUNCH
chmod +x "$APP/Contents/MacOS/leo-cub"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>Leo Cub</string>
  <key>CFBundleDisplayName</key><string>Leo Cub</string>
  <key>CFBundleIdentifier</key><string>io.github.vivainio.leo-cub</string>
  <key>CFBundleExecutable</key><string>leo-cub</string>
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
(cd target/bundle && ditto -c -k --keepParent "Leo Cub.app" leo-cub-macos.zip)
echo "target/bundle/leo-cub-macos.zip"
