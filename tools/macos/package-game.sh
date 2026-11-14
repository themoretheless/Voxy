#!/bin/sh
# Bundle the existing game entrypoint. Local build; no notarization implied.
set -eu
[ "$(uname -s)" = Darwin ] || { echo 'Requires macOS' >&2; exit 1; }
usage() {
    echo 'usage: package-game.sh BINARY PACKAGE OUTPUT.app [--smoke | --direct] [--name NAME] [--bundle-id ID] [--sign IDENTITY]' >&2
    exit 1
}
[ "$#" -ge 3 ] || usage
binary=$1
package=$2
output=$3
shift 3
mode=--game
direct=false
sign_identity=
name='Voxy Game'
bundle_id=io.voxy.game.local
while [ "$#" -gt 0 ]; do
    case "$1" in
        --direct) direct=true; shift ;;
        --sign) [ "$#" -ge 2 ] && [ -n "$2" ] || usage; sign_identity=$2; shift 2 ;;
        --smoke) mode=--game-native-smoke; shift ;;
        --name) [ "$#" -ge 2 ] && [ -n "$2" ] || usage; name=$2; shift 2 ;;
        --bundle-id) [ "$#" -ge 2 ] || usage; bundle_id=$2; shift 2 ;;
        *) usage ;;
    esac
done
if [ "$direct" = true ] && [ "$mode" != --game ]; then usage; fi
if [ -n "$sign_identity" ] && [ "$direct" != true ]; then
    echo 'Signing requires --direct and a compatible Voxy editor binary' >&2
    exit 1
fi
case "$bundle_id" in
    .*|*.|*..*|*[!A-Za-z0-9.-]*|'') echo 'Invalid bundle identifier' >&2; exit 1 ;;
esac
[ -f "$binary" ] && [ -x "$binary" ] || { echo 'Missing executable' >&2; exit 1; }
[ -f "$package" ] || { echo 'Missing resource package' >&2; exit 1; }
case "$output" in *.app) ;; *) echo 'Output must end in .app' >&2; exit 1 ;; esac
[ ! -e "$output" ] && [ ! -L "$output" ] || { echo 'Output already exists; preserve it' >&2; exit 1; }
parent=$(dirname -- "$output")
mkdir -p "$parent"
lock="$output.voxy-build-lock"
mkdir -- "$lock" || { echo 'Output is being packaged' >&2; exit 1; }
staging=
cleanup() {
    if [ -n "$staging" ]; then rm -rf -- "$staging"; fi
    rmdir -- "$lock"
}
trap cleanup EXIT
trap 'exit 1' HUP INT TERM
[ ! -e "$output" ] && [ ! -L "$output" ] || { echo 'Output already exists; preserve it' >&2; exit 1; }
staging=$(mktemp -d "$parent/.voxy-game.XXXXXX")
app="$staging/game.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
executable=game_engine
if [ "$direct" = true ]; then executable=voxy_game; fi
cp "$binary" "$app/Contents/MacOS/$executable"
cp "$package" "$app/Contents/Resources/game.vpak"
cat > "$app/Contents/Info.plist" <<'PLIST'
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
if [ "$direct" = true ]; then
    plutil -replace CFBundleExecutable -string voxy_game "$app/Contents/Info.plist"
else
cat > "$app/Contents/MacOS/launch" <<EOF_LAUNCH
#!/bin/sh
set -eu
location=\$(CDPATH= cd -- "\$(dirname -- "\$0")" && pwd)
logs=\${VOXY_GAME_LOG_DIR:-"\$HOME/Library/Logs/Voxy/Game"}
mkdir -p "\$logs"
log=\$(mktemp "\$logs/launch.XXXXXX")
exec "\$location/game_engine" --game-package "\$location/../Resources/game.vpak" $mode > "\$log" 2>&1
EOF_LAUNCH
    chmod +x "$app/Contents/MacOS/launch"
fi
chmod +x "$app/Contents/MacOS/$executable"
plutil -replace CFBundleIdentifier -string "$bundle_id" "$app/Contents/Info.plist"
plutil -replace CFBundleName -string "$name" "$app/Contents/Info.plist"
plutil -insert CFBundleDisplayName -string "$name" "$app/Contents/Info.plist"
plutil -lint "$app/Contents/Info.plist"
if [ -n "$sign_identity" ]; then
    timestamp=--timestamp
    if [ "$sign_identity" = - ]; then timestamp=--timestamp=none; fi
    /usr/bin/codesign --force --options runtime "$timestamp" --sign "$sign_identity" "$app"
    /usr/bin/codesign --verify --strict "$app"
fi
[ ! -e "$output" ] && [ ! -L "$output" ] || { echo 'Output already exists; preserve it' >&2; exit 1; }
mv "$app" "$output"
printf '%s\n' "$output"
