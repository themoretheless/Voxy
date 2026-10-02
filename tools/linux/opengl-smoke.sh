#!/bin/sh
# Native OpenGL API acceptance; Mesa llvmpipe is software execution.
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
    -v "$PWD:/workspace:ro" -v "$PWD/target/linux-docker:/build" \
    -v "$task_registry:/usr/local/cargo/registry:ro" \
    -e CARGO_TARGET_DIR=/build -w /workspace "$task_image" sh -eu -c '
    RUSTUP_TOOLCHAIN=$(rustup default | cut -d " " -f 1)
    export RUSTUP_TOOLCHAIN
    cargo build --offline --locked -p voxy_render --example scene_smoke
    cargo build --offline --locked -p voxy_app --example gpu_gravity
    cargo build --offline --locked -p voxy_gpu --example terrain_smoke --example gravity_smoke --example gravity_render
    export VOXY_SCENE_BACKEND=gl
    run_probe() {
        task_label=$1
        shift
        task_log=/build/opengl-$task_label.log
        task_timeout=30s
        if [ "$task_label" = terrain ]; then task_timeout=90s; fi
        if [ "$task_label" = gravity-math ]; then task_timeout=120s; fi
        if timeout "$task_timeout" xvfb-run -a "$@" > "$task_log" 2>&1; then task_status=0; else task_status=$?; fi
        cat "$task_log"
        test "$task_status" = 0
        grep -F "backend: Gl" "$task_log" > /dev/null
        if [ "$task_label" = terrain ]; then
            grep -F "PASS: 502 chunks," "$task_log" > /dev/null
        fi
    }
    run_probe scene /build/debug/examples/scene_smoke
    run_probe shaders /build/debug/examples/scene_smoke --shader-test
    run_probe gravity /build/debug/examples/gpu_gravity --smoke --backend gl
    run_probe gravity-math /build/debug/examples/gravity_smoke gl
    run_probe gravity-pixels /build/debug/examples/gravity_render gl
    run_probe terrain /build/debug/examples/terrain_smoke gl
    echo "PASS: strict OpenGL scene, shaders, resident physics window, trajectory/orbit and physics pixels, exact terrain parity"
'
