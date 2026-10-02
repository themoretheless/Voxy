#!/bin/sh
# Portable correspondence/reprojection/temporal proof; llvmpipe is software GPU.
set -eu
cd "$(dirname "$0")/../.."
task_image=${VOXY_LINUX_IMAGE:-voxy-linux-smoke:latest}
task_registry=${VOXY_CARGO_REGISTRY:-$HOME/.cargo/registry}
task_git=${VOXY_CARGO_GIT:-$HOME/.cargo/git}
task_tokenizer=${VOXY_TOKENIZER_ROOT:-$PWD/../../Sources/tokenizer}
mkdir -p target/linux-docker
set --
if [ -f crates/voxy_rush/Cargo.toml ]; then
    if [ ! -f "$task_tokenizer/crates/tokenizer-rush/Cargo.toml" ]; then
        echo 'Missing Rush dependency; set VOXY_TOKENIZER_ROOT' >&2
        exit 2
    fi
    set -- -v "$task_tokenizer:/Sources/tokenizer:ro"
fi
docker run --rm --init "$@" --network none --cpus 2 --memory 4g \
    -v "$PWD:/workspace:ro" -v "$PWD/target/linux-docker:/build" \
    -v "$task_registry:/usr/local/cargo/registry:ro" \
    -v "$task_git:/usr/local/cargo/git:ro" \
    -e CARGO_TARGET_DIR=/build -w /workspace "$task_image" sh -eu -c '
    RUSTUP_TOOLCHAIN=$(rustup default | cut -d " " -f 1)
    export RUSTUP_TOOLCHAIN
    cargo build --offline --locked -p voxy_render --example planar_reflection
    for backend in vulkan gl; do
        task_log=/build/planar-reflection-$backend.log
        if timeout 60s xvfb-run -a /build/debug/examples/planar_reflection --backend "$backend" > "$task_log" 2>&1; then task_status=0; else task_status=$?; fi
        cat "$task_log"
        [ "$task_status" -eq 0 ] || exit "$task_status"
        case "$backend" in vulkan) task_expected=Vulkan ;; gl) task_expected=Gl ;; esac
        grep -q "backend: $task_expected" "$task_log"
        grep -q "PLANAR REFLECTION PASS:" "$task_log"
    done
'
