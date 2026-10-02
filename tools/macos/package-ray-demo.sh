#!/bin/sh
# Local macOS application bundle; no signing/notarization or installer implied.
set -eu
case "$(uname -s)" in Darwin) ;; *) echo 'Requires macOS' >&2; exit 1 ;; esac
mode=interactive
example=animated_ray
for argument in "$@"; do
    case "$argument" in
        --smoke) [ "$mode" = interactive ] || { echo 'Duplicate --smoke' >&2; exit 1; }; mode=smoke ;;
        --planar) [ "$example" = animated_ray ] || { echo 'Duplicate --planar' >&2; exit 1; }; example=planar_scene ;;
        *) echo 'usage: package-ray-demo.sh [--smoke] [--planar]' >&2; exit 1 ;;
    esac
done
repo=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
cd "$repo"
cargo build --locked --offline --no-default-features -p voxy_ray_probe --example "$example"
arguments='--experimental --backend metal'
if [ "$mode" = smoke ]; then arguments="$arguments --smoke"; fi
case "$example:$mode" in
    animated_ray:interactive) name=VoxyRay; identifier=io.voxy.ray.interactive ;;
    animated_ray:smoke) name=VoxyRaySmoke; identifier=io.voxy.ray.smoke ;;
    planar_scene:interactive) name=VoxyPlanar; identifier=io.voxy.ray.planar.interactive ;;
    planar_scene:smoke) name=VoxyPlanarSmoke; identifier=io.voxy.ray.planar.smoke ;;
esac
bundle="$repo/target/macos/$name.app"
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
cp "target/debug/examples/$example" "$bundle/Contents/MacOS/ray_engine"
cat > "$bundle/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>$identifier</string>
<key>CFBundleName</key><string>$name</string>
<key>CFBundleExecutable</key><string>launch</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleVersion</key><string>1</string>
<key>CFBundleShortVersionString</key><string>0.1.0</string>
<key>NSHighResolutionCapable</key><true/>
<key>LSUIElement</key><false/>
</dict></plist>
EOF
cat > "$bundle/Contents/MacOS/launch" <<EOF
#!/bin/sh
set -eu
cd -- "\$(dirname -- "\$0")"
exec ./ray_engine $arguments > ../Resources/runtime.log 2>&1
EOF
chmod +x "$bundle/Contents/MacOS/launch" "$bundle/Contents/MacOS/ray_engine"
plutil -lint "$bundle/Contents/Info.plist"
printf '%s\n' "$bundle"
