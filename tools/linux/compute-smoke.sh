#!/bin/sh
# Actual shader/reload compute checks; Mesa llvmpipe is CPU software rendering.
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
    cargo build --offline --locked -p voxy_render --example compute_smoke
    xvfb-run -a sh -eu -c '\''
        for backend in vulkan gl; do
            export VOXY_COMPUTE_BACKEND=$backend
            task_log=/build/compute-$backend.log
            if /build/debug/examples/compute_smoke > "$task_log" 2>&1; then task_status=0; else task_status=$?; fi
            cat "$task_log"
            if [ "$task_status" != 0 ]; then exit "$task_status"; fi
            case "$backend" in vulkan) task_api=Vulkan ;; gl) task_api=Gl ;; esac
            grep -E "^Compute on .*backend: $task_api," "$task_log" > /dev/null
            grep -F "DISPATCH LIMIT PASS: zero and over-limit X/Y/Z rejected; valid submissions remain clean" "$task_log" > /dev/null
            grep -F "RESIDENT COMPUTE PASS: four submissions, independent snapshots, job continued during mapping" "$task_log" > /dev/null
            grep -F "COMPUTE RELOAD PASS: old jobs retained, new shader active, invalid replacement rejected" "$task_log" > /dev/null
            grep -F "PASS: WGSL compute, 1042 exact results, partial workgroups, independent jobs, readback, cancellation and validation failures" "$task_log" > /dev/null
        done
    '\''
'
