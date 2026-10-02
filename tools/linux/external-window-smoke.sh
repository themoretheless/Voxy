#!/bin/sh
# Software Vulkan window shell; explicit export-only mode, not CUDA execution.
set -eu
cd "$(dirname "$0")/../.."
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
    -v "$HOME/.cargo/registry:/usr/local/cargo/registry:ro" \
    -e CARGO_TARGET_DIR=/build -e VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation \
    -e VK_LOADER_DEBUG=layer -w /workspace \
    "${VOXY_LINUX_IMAGE:-voxy-linux-validation:latest}" sh -eu -c '
    RUSTUP_TOOLCHAIN=$(rustup default | cut -d " " -f1)
    export RUSTUP_TOOLCHAIN
    test -f /usr/share/vulkan/explicit_layer.d/VkLayer_khronos_validation.json
    cargo build --offline --locked -p voxy_vulkan --features cuda --example cuda_gravity_window
    task_log=/build/external-window.log
    if timeout 40s xvfb-run -a /build/debug/examples/cuda_gravity_window --export-only --smoke > "$task_log" 2>&1; then task_status=0; else task_status=$?; fi
    if [ "$task_status" != 0 ]; then cat "$task_log"; exit "$task_status"; fi
    grep -F "export_only=true" "$task_log" > /dev/null
    grep -F "backend: Vulkan" "$task_log" > /dev/null
    grep -F "Inserted device layer \"VK_LAYER_KHRONOS_validation\"" "$task_log" > /dev/null
    if grep -E "Validation Error|VUID-" "$task_log"; then exit 1; fi
    grep -F "200 resident steps" "$task_log" > /dev/null
    grep -E "Gravity window|PASS:" "$task_log"
    task_log=/build/cuda-window-no-driver.log
    if timeout 40s xvfb-run -a /build/debug/examples/cuda_gravity_window --smoke > "$task_log" 2>&1; then
        echo "CUDA unexpectedly succeeded in software-only container" >&2; exit 1
    else task_status=$?; fi
    test "$task_status" = 1
    grep -F "DriverUnavailable" "$task_log"
    echo "PASS: Vulkan window lifecycle; unavailable CUDA rejects initialization without switching mode"
    '
