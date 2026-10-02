#!/bin/sh
# Linux temporal rendering proof. Mesa llvmpipe is CPU software rendering.
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
    cargo build --offline --locked -p voxy_render --example previous_position_probe --example relative_motion --example perspective_motion --example temporal_resolve --example motion_composition --example skinned_history_surface --example xr_motion
    WGPU_BACKEND=vulkan /build/debug/examples/perspective_motion
    xvfb-run -a sh -eu -c '\''
        for backend in vulkan gl; do
            export WGPU_BACKEND=$backend VOXY_MOTION_BACKEND=$backend VOXY_XR_BACKEND=$backend
            for example in temporal_resolve previous_position_probe relative_motion motion_composition skinned_history_surface xr_motion; do
                /build/debug/examples/$example
            done
        done
    '\''
'
