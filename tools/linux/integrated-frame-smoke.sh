#!/bin/sh
# Integrated scene, compute, HDR and exposure frame checks; Mesa llvmpipe is CPU software rendering.
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
    cargo build --offline --locked -p voxy_render --example temporal_surface_smoke
    xvfb-run -a sh -eu -c '\''
        for backend in vulkan gl; do
            export VOXY_TEMPORAL_BACKEND=$backend
            VOXY_TEMPORAL_INTEGRATED=1 /build/debug/examples/temporal_surface_smoke
            VOXY_TEMPORAL_INTEGRATED=1 VOXY_TEMPORAL_AUTO_EXPOSURE=1 /build/debug/examples/temporal_surface_smoke
        done
    '\''
'
