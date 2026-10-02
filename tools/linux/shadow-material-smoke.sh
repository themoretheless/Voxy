#!/bin/sh
# Linux software Vulkan acceptance for common HDR shadow/PBR uniforms and readback.
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
    export WGPU_BACKEND=vulkan
    cargo build --offline --locked -p voxy_render --example shadow_visibility --example auto_exposure --example planar_capture --example hdr_mips --example environment_ggx --example dfg --example environment_lighting --example environment_diffuse --example imported_environment
    cargo test --offline --locked -p voxy_render --lib hdr_asset::tests
    /build/debug/examples/shadow_visibility
    /build/debug/examples/auto_exposure
    /build/debug/examples/planar_capture
    /build/debug/examples/hdr_mips
    /build/debug/examples/environment_ggx
    /build/debug/examples/dfg
    /build/debug/examples/environment_lighting
    /build/debug/examples/environment_diffuse
    /build/debug/examples/imported_environment
    cargo run --offline --locked -p voxy_render --example hdr_probe
'
