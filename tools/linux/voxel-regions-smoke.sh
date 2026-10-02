#!/bin/sh
# Integer voxel broadphase API parity; Mesa llvmpipe is software execution.
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
    cargo build --offline --locked -p voxy_gpu --example voxel_regions_smoke
    run_probe() {
        task_backend=$1
        task_expected=$2
        task_log=/build/voxel-regions-$task_backend.log
        if timeout 120s xvfb-run -a /build/debug/examples/voxel_regions_smoke "$task_backend" > "$task_log" 2>&1; then task_status=0; else task_status=$?; fi
        cat "$task_log"
        test "$task_status" = 0
        grep -F "PASS: 240 nonblocking character tasks" "$task_log" > /dev/null
        grep -F "PASS: 240 nonblocking GPU vehicle ticks" "$task_log" > /dev/null
        grep -F "PASS: GPU vehicle stale rejection, consumed error and exact fresh recovery" "$task_log" > /dev/null
        grep -F "PASS: GPU 4913 obstacles, late nearest contact and canonical overlap tie" "$task_log" > /dev/null
        grep -F "PASS: pending character stale rejection, consumed errors and fresh recovery" "$task_log" > /dev/null
        grep -F "PASS: invalid GPU sweeps reject before world reads or dispatch" "$task_log" > /dev/null
        grep -F "PASS: GPU broadphase exact sweep parity, far/unloaded boundaries and stale rejection" "$task_log" > /dev/null
        grep -F "PASS: far integer world GPU fault candidate retains exact i64 coordinates" "$task_log" > /dev/null
        grep -F "PASS: frozen world GPU counts/candidates, stale edit rejection and fresh recovery" "$task_log" > /dev/null
        grep -F "PASS: concurrent nonblocking voxel regions, consumed and dropped readback recovery" "$task_log" > /dev/null
        grep -F "backend: $task_expected" "$task_log" > /dev/null
        grep -F "PASS: 257 parallel voxel regions, exact CPU counts/candidates/faults" "$task_log" > /dev/null
    }
    run_probe vulkan Vulkan
    run_probe gl Gl
    echo "PASS: explicit Vulkan/OpenGL voxel region classification"
'
