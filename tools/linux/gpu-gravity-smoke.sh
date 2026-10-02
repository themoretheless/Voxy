#!/bin/sh
# Linux API acceptance using an existing image; llvmpipe is software, not hardware proof.
set -eu
cd "$(dirname "$0")/../.."
task_image=${VOXY_LINUX_IMAGE:-voxy-linux-smoke:latest}
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
    -v "$PWD:/workspace:ro" \
    -v "$PWD/target/linux-docker:/build" \
    -v "$task_registry:/usr/local/cargo/registry:ro" \
    -e CARGO_TARGET_DIR=/build -w /workspace "$task_image" sh -eu -c '
    RUSTUP_TOOLCHAIN=$(rustup default | cut -d " " -f 1)
    export RUSTUP_TOOLCHAIN
    cargo build --offline --locked -p voxy_app --example gpu_gravity
    cargo build --offline --locked -p voxy_gpu --example gravity_smoke --example gravity_render
    timeout 60s xvfb-run -a /build/debug/examples/gpu_gravity --smoke --backend vulkan > /build/gravity-vulkan-window.log 2>&1
    cat /build/gravity-vulkan-window.log
    grep -q "backend: Vulkan" /build/gravity-vulkan-window.log
    timeout 120s /build/debug/examples/gravity_smoke vulkan > /build/gravity-vulkan-math.log 2>&1
    cat /build/gravity-vulkan-math.log
    grep -q "backend: Vulkan" /build/gravity-vulkan-math.log
    timeout 60s /build/debug/examples/gravity_render vulkan > /build/gravity-vulkan-pixels.log 2>&1
    cat /build/gravity-vulkan-pixels.log
    grep -q "backend: Vulkan" /build/gravity-vulkan-pixels.log
'
