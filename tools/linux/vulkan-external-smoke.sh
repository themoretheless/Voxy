#!/bin/sh
# Vulkan export proof; no CUDA device execution or graphics ownership handoff.
set -eu
cd "$(dirname "$0")/../.."
task_image=${VOXY_LINUX_IMAGE:-voxy-linux-smoke:latest}
task_registry=${VOXY_CARGO_REGISTRY:-$HOME/.cargo/registry}
task_validation=${VOXY_VULKAN_VALIDATION:-0}
case "$task_validation" in
    0|1) ;;
    *) echo "VOXY_VULKAN_VALIDATION must be 0 or 1" >&2; exit 1 ;;
esac
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
    -e VOXY_VULKAN_VALIDATION="$task_validation" -e CARGO_TARGET_DIR=/build -w /workspace "$task_image" sh -eu -c '
    RUSTUP_TOOLCHAIN=$(rustup default | cut -d " " -f 1)
    export RUSTUP_TOOLCHAIN
    if [ "$VOXY_VULKAN_VALIDATION" = 1 ]; then
        if [ ! -f /usr/share/vulkan/explicit_layer.d/VkLayer_khronos_validation.json ]; then
            echo "Requested Khronos validation layer is missing from the Linux image" >&2
            exit 1
        fi
        export VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation VK_LOADER_DEBUG=layer
    fi
    if cargo run --offline --locked -p voxy_cuda --example vulkan_external -- --export-only > /build/vulkan-external.log 2>&1; then
        task_status=0
    else
        task_status=$?
    fi
    cat /build/vulkan-external.log
    test "$task_status" = 0
    if [ "$VOXY_VULKAN_VALIDATION" = 1 ]; then
        grep -F "Inserted device layer" /build/vulkan-external.log | grep -F VK_LAYER_KHRONOS_validation > /dev/null
    fi
    if grep -E "Validation Error|VUID-" /build/vulkan-external.log; then
        exit 1
    fi
    if [ "$VOXY_VULKAN_VALIDATION" = 1 ]; then
        if /build/debug/examples/vulkan_external --validation-negative > /build/vulkan-validation-negative.log 2>&1; then
            echo "Injected validation fault unexpectedly succeeded" >&2
            exit 1
        fi
        grep -F VUID-vkCmdFillBuffer-dstOffset-00025 /build/vulkan-validation-negative.log
        grep -F "command buffer was not submitted" /build/vulkan-validation-negative.log > /dev/null
        echo "PASS: Khronos validation detected the unsubmitted invalid-fill control"
    fi
'
