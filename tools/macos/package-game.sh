#!/bin/sh
# Bundle the existing game entrypoint. Local build; no notarization implied.
set -eu
[ "$(uname -s)" = Darwin ] || { echo 'Requires macOS' >&2; exit 1; }
[ "$#" -ge 3 ] && [ "$#" -le 4 ] || { echo 'usage: package-game.sh BINARY PACKAGE OUTPUT.app [--smoke]' >&2; exit 1; }
binary=$1
package=$2
output=$3
mode=--game
if [ "$#" = 4 ]; then
    [ "$4" = --smoke ] || { echo 'Unknown option' >&2; exit 1; }
    mode=--game-native-smoke
fi
[ -f "$binary" ] && [ -x "$binary" ] || { echo 'Missing executable' >&2; exit 1; }
[ -f "$package" ] || { echo 'Missing resource package' >&2; exit 1; }
case "$output" in *.app) ;; *) echo 'Output must end in .app' >&2; exit 1 ;; esac
[ ! -e "$output" ] || { echo 'Output already exists; preserve it' >&2; exit 1; }
parent=$(dirname -- "$output")
mkdir -p "$parent"
staging=$(mktemp -d "$parent/.voxy-game.XXXXXX")
trap 'rm -rf -- "$staging"' EXIT HUP INT TERM
mkdir -p "$staging/Contents/MacOS" "$staging/Contents/Resources"
cp "$binary" "$staging/Contents/MacOS/game_engine"
cp "$package" "$staging/Contents/Resources/game.vpak"
cat > "$staging/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>io.voxy.game.local</string>
<key>CFBundleName</key><string>Voxy Game</string>
<key>CFBundleExecutable</key><string>launch</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleVersion</key><string>1</string>
<key>CFBundleShortVersionString</key><string>0.1.0</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
cat > "$staging/Contents/MacOS/launch" <<EOF_LAUNCH
#!/bin/sh
set -eu
location=\$(CDPATH= cd -- "\$(dirname -- "\$0")" && pwd)
logs=\${VOXY_GAME_LOG_DIR:-"\$HOME/Library/Logs/Voxy/Game"}
mkdir -p "\$logs"
log=\$(mktemp "\$logs/launch.XXXXXX")
exec "\$location/game_engine" --game-package "\$location/../Resources/game.vpak" $mode > "\$log" 2>&1
EOF_LAUNCH
chmod +x "$staging/Contents/MacOS/launch" "$staging/Contents/MacOS/game_engine"
plutil -lint "$staging/Contents/Info.plist"
mv "$staging" "$output"
trap - EXIT HUP INT TERM
printf '%s\n' "$output"
