#!/bin/sh
# Linux ARM64 CPU-only NVRTC/PTX assembly check using pinned NVIDIA tooling.
set -eu
cd "$(dirname "$0")/../.."
python3 - <<'PY_INVENTORY'
from pathlib import Path
root = Path("crates/voxy_cuda/src")
expected = {"gravity.cu", "terrain.cu", "projectile.cu", "box_sweep.cu", "voxel_regions.cu", "water.cu", "affine.ptx"}
actual = {p.relative_to(root).as_posix() for p in root.rglob("*") if p.is_file() and p.suffix in {".cu", ".ptx"}}
if actual != expected:
    raise SystemExit(f"CUDA compiler coverage mismatch: uncovered={sorted(actual - expected)}, missing={sorted(expected - actual)}")
print("PASS: compilation matrix covers every CUDA/PTX source")
PY_INVENTORY
python3 tools/cuda/fetch_tooling.py
mkdir -p target/cuda-verified
docker run --rm --network none --read-only --tmpfs /tmp --cpus 2 --memory 1g \
    -v "$PWD:/workspace:ro" \
    -v "$PWD/target/cuda-tooling:/tooling:ro" \
    -v "$PWD/target/cuda-verified:/out" \
    voxy-linux-smoke:latest sh -eu -c '
        cc -std=c11 -Wall -Wextra -Werror /workspace/tools/cuda/nvrtc_probe.c \
            -I/tooling/nvidia/cuda_nvrtc/include -L/tooling/nvidia/cuda_nvrtc/lib \
            -Wl,-rpath,/tooling/nvidia/cuda_nvrtc/lib -l:libnvrtc.so.12 -o /out/nvrtc-probe
        /out/nvrtc-probe --recovery-check
        capabilities=$(/out/nvrtc-probe --list-architectures)
        test -n "$capabilities"
        printf "Compiler-supported architecture matrix: %s\n" "$capabilities"
        for capability in $capabilities; do
            export VOXY_NVRTC_ARCH=$capability
            architecture=sm_$capability
            /out/nvrtc-probe /workspace/crates/voxy_cuda/src/gravity.cu /out/gravity-$architecture.ptx \
                gravity_predict gravity_correct gravity_commit gravity_view_validate gravity_view_commit
            /out/nvrtc-probe /workspace/crates/voxy_cuda/src/terrain.cu /out/terrain-$architecture.ptx procedural_terrain
            /out/nvrtc-probe /workspace/crates/voxy_cuda/src/projectile.cu /out/projectile-$architecture.ptx projectile_motion
            /out/nvrtc-probe /workspace/crates/voxy_cuda/src/box_sweep.cu /out/box-sweep-$architecture.ptx box_sweep
            /tooling/nvidia/cuda_nvcc/bin/ptxas -arch="$architecture" "/out/box-sweep-$architecture.ptx" -o "/out/box-sweep-$architecture.cubin"
            /out/nvrtc-probe /workspace/crates/voxy_cuda/src/water.cu /out/water-$architecture.ptx water_transfer
            /tooling/nvidia/cuda_nvcc/bin/ptxas -arch="$architecture" "/out/water-$architecture.ptx" -o "/out/water-$architecture.cubin"
            /out/nvrtc-probe /workspace/crates/voxy_cuda/src/voxel_regions.cu /out/voxel-regions-$architecture.ptx voxel_regions
            /tooling/nvidia/cuda_nvcc/bin/ptxas -arch="$architecture" "/out/voxel-regions-$architecture.ptx" -o "/out/voxel-regions-$architecture.cubin"
            /tooling/nvidia/cuda_nvcc/bin/ptxas -arch="$architecture" "/out/projectile-$architecture.ptx" -o "/out/projectile-$architecture.cubin"
            /tooling/nvidia/cuda_nvcc/bin/ptxas -arch="$architecture" "/out/gravity-$architecture.ptx" -o "/out/gravity-$architecture.cubin"
            /tooling/nvidia/cuda_nvcc/bin/ptxas -arch="$architecture" "/out/terrain-$architecture.ptx" -o "/out/terrain-$architecture.cubin"
            /tooling/nvidia/cuda_nvcc/bin/ptxas -arch="$architecture" /workspace/crates/voxy_cuda/src/affine.ptx -o "/out/affine-$architecture.cubin"
            printf "PTXAS PASS: gravity/terrain/projectile/voxel-regions/box-sweep/water/affine for %s; no GPU execution\n" "$architecture"
        done
        if VOXY_NVRTC_ARCH=999 /out/nvrtc-probe /workspace/crates/voxy_cuda/src/terrain.cu /out/rejected.ptx procedural_terrain; then
            echo "Unsupported compiler architecture was accepted" >&2
            exit 1
        else
            test "$?" = 2
        fi
        echo "PASS: unsupported compiler architecture rejected before compilation"
        if VOXY_NVRTC_ARCH=52 /out/nvrtc-probe /workspace/crates/voxy_cuda/src/terrain.cu /tmp/rejected-entry.ptx voxy_missing_kernel_entry; then
            echo "Missing kernel entrypoint was accepted" >&2
            exit 1
        else
            test "$?" = 1
            test ! -e /tmp/rejected-entry.ptx
        fi
        echo "PASS: missing kernel entrypoint rejected without publishing PTX"

    '
