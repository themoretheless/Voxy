#!/bin/sh
# Integer water shader API parity; Mesa llvmpipe is software execution.
set -eu
cd "$(dirname "$0")/../.."
task_image=${VOXY_LINUX_IMAGE:-voxy-linux-smoke:latest}
task_registry=${VOXY_CARGO_REGISTRY:-$HOME/.cargo/registry}
task_git_cache=${VOXY_CARGO_GIT_CACHE:-$HOME/.cargo/git}
mkdir -p target/linux-docker
# Cargo resolves the locked Rush Git dependency from its mounted cache.
set --
if [ -d "$task_git_cache" ]; then
    set -- "$@" -v "$task_git_cache:/usr/local/cargo/git:ro"
fi
docker run --rm "$@" --network none --cpus 2 --memory 4g \
    -v "$PWD:/workspace:ro" -v "$PWD/target/linux-docker:/build" \
    -v "$task_registry:/usr/local/cargo/registry:ro" \
    -e CARGO_TARGET_DIR=/build -w /workspace "$task_image" sh -eu -c '
    RUSTUP_TOOLCHAIN=$(rustup default | cut -d " " -f 1)
    export RUSTUP_TOOLCHAIN
    cargo build --offline --locked -p voxy_gpu --example water_smoke
    run_probe() {
        task_backend=$1
        task_expected=$2
        task_log=/build/water-$task_backend.log
        if timeout 120s xvfb-run -a /build/debug/examples/water_smoke "$task_backend" > "$task_log" 2>&1; then task_status=0; else task_status=$?; fi
        cat "$task_log"
        test "$task_status" = 0
        grep -F "backend: $task_expected" "$task_log" > /dev/null
        grep -F "PASS: 16 GPU water ticks, 524288 exact CPU cell comparisons" "$task_log" > /dev/null
        grep -F "PASS: GPU water lazy reads" "$task_log" > /dev/null
        grep -F "PASS: pending GPU water success/error isolation" "$task_log" > /dev/null
        grep -F "PASS: pending world plans retain revisions" "$task_log" > /dev/null
        grep -F "PASS: captured unavailable world chunks remain lazy and cannot enter successful transactions" "$task_log" > /dev/null
        grep -F "PASS: GPU missing sample identifies exact world position and consumes failed plan" "$task_log" > /dev/null
        grep -F "PASS: missing chunk load and fresh GPU retry match CPU without partial failed transfers" "$task_log" > /dev/null
    }
    run_probe vulkan Vulkan
    run_probe gl Gl
    echo "PASS: explicit Vulkan/OpenGL integer water transfers and error contracts"
'
