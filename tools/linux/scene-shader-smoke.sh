#!/bin/sh
# Actual scene shader reload and device ownership checks; Mesa llvmpipe is CPU software rendering.
set -eu
cd "$(dirname "$0")/../.."
task_registry=${VOXY_CARGO_REGISTRY:-$HOME/.cargo/registry}
task_git=${VOXY_CARGO_GIT:-$HOME/.cargo/git}
mkdir -p target/linux-docker
docker run --rm --init --network none --cpus 2 --memory 4g \
    -v "$PWD:/workspace:ro" -v "$PWD/target/linux-docker:/build" \
    -v "$task_registry:/usr/local/cargo/registry:ro" \
    -v "$task_git:/usr/local/cargo/git:ro" \
    -e CARGO_TARGET_DIR=/build -w /workspace voxy-linux-smoke:latest sh -eu -c '
    RUSTUP_TOOLCHAIN=$(rustup default | cut -d " " -f 1)
    export RUSTUP_TOOLCHAIN
    cargo build --offline --locked -p voxy_render --example scene_smoke --example voxel_diagonal --example wgsl_inventory
    /build/debug/examples/wgsl_inventory
    xvfb-run -a sh -eu -c '\''
        for backend in vulkan gl; do
            export VOXY_SCENE_BACKEND=$backend
            /build/debug/examples/scene_smoke --shader-test
            /build/debug/examples/scene_smoke --shader-test --msaa
            /build/debug/examples/voxel_diagonal "$backend"
        done
    '\''
'
