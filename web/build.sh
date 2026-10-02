#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
task_bindgen="${VOXY_WASM_BINDGEN:-wasm-bindgen}"
task_profile="${VOXY_WEB_PROFILE:-release}"
case "$task_profile" in
    release) task_directory=release ;;
    dev) task_directory=debug ;;
    *) echo 'VOXY_WEB_PROFILE must be dev or release' >&2; exit 1 ;;
esac
if [ "$("$task_bindgen" --version)" != 'wasm-bindgen 0.2.127' ]; then
    echo 'Use wasm-bindgen-cli 0.2.127 (set VOXY_WASM_BINDGEN to its executable)' >&2
    exit 1
fi
cargo build -p voxy_web --target wasm32-unknown-unknown --profile "$task_profile"
"$task_bindgen" --target web --out-dir web/pkg "target/wasm32-unknown-unknown/$task_directory/voxy_web.wasm"
