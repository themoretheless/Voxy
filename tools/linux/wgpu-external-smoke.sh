#!/bin/sh
# wgpu Vulkan export/acquire followed by shader pixel readback. No CUDA writer.
set -eu
cd "$(dirname "$0")/../.."
task_image=${VOXY_LINUX_IMAGE:-voxy-linux-validation:latest}
mkdir -p target/linux-docker
task_log=target/linux-docker/wgpu-external.log
if # Cargo parses every workspace member, including the sibling Rush path dependency.
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
    -e VK_LOADER_DEBUG=layer -w /workspace "$task_image" sh -eu -c '
    RUSTUP_TOOLCHAIN=$(rustup default | cut -d " " -f1)
    export RUSTUP_TOOLCHAIN
    test -f /usr/share/vulkan/explicit_layer.d/VkLayer_khronos_validation.json
    cargo run --offline --locked -p voxy_vulkan --example external_gravity
    ' > "$task_log" 2>&1; then task_status=0; else task_status=$?; fi
if [ "$task_status" != 0 ]; then cat "$task_log"; exit "$task_status"; fi
grep -F "backend: Vulkan" "$task_log" > /dev/null
grep -F 'Inserted device layer "VK_LAYER_KHRONOS_validation"' "$task_log" > /dev/null
if grep -E 'Validation Error|VUID-' "$task_log"; then exit 1; fi
grep -F 'frames after external acquire' "$task_log" > /dev/null
grep -E 'Gravity render:|Inserted device layer|PASS:' "$task_log"
echo "PASS: wgpu Vulkan external memory, ownership and shader pixels with validation"
