#!/bin/sh
# Actual Vulkan rendering/compute on the image's adapter; llvmpipe is software.
set -eu
cd "$(dirname "$0")/../.."
task_image=${VOXY_LINUX_IMAGE:-voxy-linux-validation:latest}
task_registry=${VOXY_CARGO_REGISTRY:-$HOME/.cargo/registry}
mkdir -p target/linux-docker
# Cargo parses every workspace member, including the sibling Rush path dependency.
task_tokenizer=${VOXY_TOKENIZER_ROOT:-$PWD/../../Sources/tokenizer}
set --
if [ -f crates/voxy_rush/Cargo.toml ]; then
    if [ ! -f "$task_tokenizer/crates/tokenizer-rush/Cargo.toml" ]; then
        echo "Missing Rush dependency; set VOXY_TOKENIZER_ROOT to the tokenizer checkout" >&2
        exit 2
    fi
    set -- -v "$task_tokenizer:/Sources/tokenizer:ro"
fi
docker run --rm "$@" --network none --cpus 2 --memory 4g \
    -v "$PWD:/workspace:ro" -v "$PWD/target/linux-docker:/build" \
    -v "$task_registry:/usr/local/cargo/registry:ro" \
    -e CARGO_TARGET_DIR=/build -w /workspace "$task_image" sh -eu -c '
    RUSTUP_TOOLCHAIN=$(rustup default | cut -d " " -f 1)
    export RUSTUP_TOOLCHAIN
    test -f /usr/share/vulkan/explicit_layer.d/VkLayer_khronos_validation.json
    cargo build --offline --locked -p voxy_render --example scene_smoke --example xray_smoke --example compute_smoke
    export VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation VK_LOADER_DEBUG=layer VOXY_SCENE_BACKEND=vulkan
    run_probe() {
        task_label=$1
        shift
        task_log=/build/renderer-$task_label.log
        if "$@" > "$task_log" 2>&1; then task_status=0; else task_status=$?; fi
        if [ "$task_status" != 0 ]; then cat "$task_log"; exit "$task_status"; fi
        grep -E "PASS:|GPU:|Rendering on|Compute on|Inserted device layer" "$task_log"
        grep -F "Inserted device layer" "$task_log" | grep -F VK_LAYER_KHRONOS_validation > /dev/null
        grep -F "backend: Vulkan" "$task_log" > /dev/null
        if grep -E "Validation Error|VUID-" "$task_log"; then exit 1; fi
    }
    run_probe scene /build/debug/examples/scene_smoke
    run_probe shaders /build/debug/examples/scene_smoke --shader-test
    run_probe xray /build/debug/examples/xray_smoke
    run_probe compute /build/debug/examples/compute_smoke
    echo "PASS: Linux Vulkan scene, shader reload, X-ray and compute with active Khronos validation"
'
