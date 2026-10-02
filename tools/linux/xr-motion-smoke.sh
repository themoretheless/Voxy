#!/bin/sh
# Stereo motion GPU proof without headset/runtime emulation.
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
    cargo build --offline --locked -p voxy_render --example xr_motion
    xvfb-run -a sh -eu -c '\''
        for backend in vulkan gl; do
            export VOXY_XR_BACKEND=$backend
            task_log=/build/xr-motion-$backend.log
            if /build/debug/examples/xr_motion > "$task_log" 2>&1; then task_status=0; else task_status=$?; fi
            cat "$task_log"
            if [ "$task_status" != 0 ]; then exit "$task_status"; fi
            grep -F "XR MOTION PASS: asymmetric eyes, opposite motion, skipped frame, tracking recovery, clipping changes, teleport" "$task_log" > /dev/null
        done
    '\''
'
