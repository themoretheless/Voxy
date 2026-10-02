#!/bin/sh
# Graphical gameplay with GPU water; Mesa llvmpipe is software execution.
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
    cargo build --offline --locked -p voxy_app --bin voxy_app
    run_probe() {
        task_backend=$1
        task_expected=$2
        task_log=/build/gameplay-water-$task_backend.log
        if timeout 180s xvfb-run -a /build/debug/voxy_app --backend "$task_backend" --gpu-water --gpu-collisions --autopilot > "$task_log" 2>&1; then task_status=0; else task_status=$?; fi
        cat "$task_log"
        test "$task_status" = 0
        grep -F "Voxy character collision: nonblocking GPU broadphase ($task_expected)" "$task_log" > /dev/null
        grep -E "Voxy GPU character ticks completed: [1-9][0-9]*" "$task_log" > /dev/null
        grep -E "Voxy GPU vehicle ticks completed: [1-9][0-9]*" "$task_log" > /dev/null
        grep -F "($task_expected)" "$task_log" > /dev/null
        grep -F "Voxy first GPU water tick completed:" "$task_log" > /dev/null
        grep -F "Voxy autopilot destroyed natural water bed" "$task_log" > /dev/null
        grep -E "Voxy autopilot passed: water=[1-9][0-9]*," "$task_log" > /dev/null
    }
    run_probe vulkan Vulkan
    run_probe gl Gl
    echo "PASS: explicit Vulkan/OpenGL graphical gameplay, GPU character collision and GPU water commits"
'
