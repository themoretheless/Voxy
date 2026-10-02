#!/bin/sh
# Physical Linux NVIDIA acceptance; never replaces CUDA with CPU/graphics compute.
set -eu
cd "$(dirname "$0")/../.."
if [ "$(uname -s)" != Linux ]; then
    echo "CUDA/Vulkan hardware acceptance requires a Linux NVIDIA host" >&2
    exit 2
fi
if [ "$#" -gt 2 ]; then
    echo "usage: hardware-acceptance.sh [cuda-device-ordinal] [--ray-query]" >&2
    exit 2
fi
task_device=${1:-0}
task_require_ray=0
if [ "$#" -eq 2 ]; then
    if [ "$2" != --ray-query ]; then echo "invalid acceptance option" >&2; exit 2; fi
    task_require_ray=1
fi
case "$task_device" in
    ''|*[!0-9]*) echo "invalid CUDA device ordinal" >&2; exit 2 ;;
esac
command -v timeout > /dev/null
if [ -n "${CARGO_BUILD_TARGET:-}" ]; then
    echo "hardware acceptance requires native Cargo builds; unset CARGO_BUILD_TARGET" >&2
    exit 2
fi
mkdir -p target/cuda-hardware
timeout 900s cargo build --locked -p voxy_cuda --features cuda --example cuda_probe --example gravity_probe --example projectile_probe --example voxel_regions_probe --example box_sweep_probe --example vulkan_external
timeout 900s cargo build --locked -p voxy_gpu --features cuda --example terrain_smoke --example water_smoke --example voxel_regions_smoke --example cuda_voxel_world --example cuda_water_world
timeout 900s cargo build --locked -p voxy_app --features cuda --example cuda_character_probe --example cuda_vehicle_probe --bin voxy_app
timeout 900s cargo build --locked -p voxy_vulkan --features cuda --example cuda_gravity_render
timeout 900s cargo build --locked -p voxy_vulkan --features cuda --example cuda_gravity_window
run_probe() {
    task_label=$1
    shift
    task_log=target/cuda-hardware/$task_label.log
    if timeout 300s "$@" > "$task_log" 2>&1; then task_status=0; else task_status=$?; fi
    cat "$task_log"
    if [ "$task_status" != 0 ]; then
        echo "FAIL: $task_label (exit $task_status); log: $task_log" >&2
        exit "$task_status"
    fi
}
require_nvidia_graphics() {
    task_log=target/cuda-hardware/$1.log
    if ! grep -E '^(Water|Voxel regions|Device loss) GPU: .*vendor: 4318,.*device_type: (DiscreteGpu|IntegratedGpu),' "$task_log" > /dev/null; then
        echo "FAIL: $1 requires a physical NVIDIA graphics adapter; log: $task_log" >&2
        exit 1
    fi
}
timeout 900s cargo build --locked -p voxy_render --example voxel_diagonal --example compute_smoke --example device_loss_surface --example wgsl_inventory
run_probe wgsl-inventory cargo run --locked -p voxy_render --example wgsl_inventory
if ! grep -E "^WGSL inventory: [1-9][0-9]* files, [1-9][0-9]* variants, [1-9][0-9]* validated entrypoints, 0 failures; no GPU execution$" target/cuda-hardware/wgsl-inventory.log > /dev/null; then
    echo "FAIL: complete WGSL inventory proof missing" >&2
    exit 1
fi
if [ "$task_require_ray" = 1 ]; then
    timeout 900s cargo build --locked --no-default-features -p voxy_ray_probe --bin voxy_ray_probe --example animated_ray
    run_probe ray-query-vulkan cargo run --locked --no-default-features -p voxy_ray_probe --bin voxy_ray_probe -- --experimental --backend vulkan --require-nvidia
    if ! grep -E '^RAY GPU: .*vendor: 4318,.*device_type: (DiscreteGpu|IntegratedGpu),.*backend: Vulkan,' target/cuda-hardware/ray-query-vulkan.log > /dev/null; then
        echo "FAIL: Vulkan ray query requires physical NVIDIA proof" >&2; exit 1
    fi
    for task_marker in 'RAY SMOKE PASS:' 'PRIMARY BACKGROUND PASS:' 'GPU primary reflection -> HDR composition / material MRT:'; do
        if ! grep -F "$task_marker" target/cuda-hardware/ray-query-vulkan.log > /dev/null; then
            echo "FAIL: ray-query proof missing: $task_marker" >&2; exit 1
        fi
    done
    run_probe animated-ray-vulkan cargo run --locked --no-default-features -p voxy_ray_probe --example animated_ray -- --experimental --backend vulkan --require-nvidia --smoke
    if ! grep -E '^ANIMATED RAY GPU: .*vendor: 4318,.*device_type: (DiscreteGpu|IntegratedGpu),.*backend: Vulkan,' target/cuda-hardware/animated-ray-vulkan.log > /dev/null; then
        echo "FAIL: animated Vulkan ray query requires physical NVIDIA proof" >&2; exit 1
    fi
    if ! grep -F 'ANIMATED RAY PASS: 120 presentations' target/cuda-hardware/animated-ray-vulkan.log > /dev/null; then
        echo "FAIL: animated Vulkan ray presentation proof missing" >&2; exit 1
    fi
    echo "PASS: physical NVIDIA Vulkan ray-query and animated presentation gates"
fi
run_probe device-selection-tests cargo test --locked -p voxy_cuda --features cuda --example cuda_probe --example gravity_probe --example projectile_probe --example voxel_regions_probe --example box_sweep_probe device_selection_is_explicit_and_rejects_ignored_arguments
task_selection_passes=$(grep -c '^test device_argument::tests::device_selection_is_explicit_and_rejects_ignored_arguments \.\.\. ok$' target/cuda-hardware/device-selection-tests.log || true)
if [ "$task_selection_passes" != 5 ]; then
    echo "FAIL: expected five CUDA device-selection tests, observed $task_selection_passes" >&2
    exit 1
fi
run_probe buffers cargo run --locked -p voxy_cuda --features cuda --example cuda_probe -- "$task_device"
grep -F 'CUDA PASS: shared resident/transient budgets reject, preserve data and release for fresh work' target/cuda-hardware/buffers.log > /dev/null
run_probe cuda-regions cargo run --locked -p voxy_cuda --features cuda --example voxel_regions_probe -- "$task_device"
grep -F "PASS: CUDA 257 voxel regions exact CPU counts/candidates/faults and recovery" target/cuda-hardware/cuda-regions.log > /dev/null
run_probe cuda-box-sweeps cargo run --locked -p voxy_cuda --features cuda --example box_sweep_probe -- "$task_device"
grep -F "PASS: CUDA 267 exact f64 box sweeps, stationary/overlap/tie cases and invalid-input rejection" target/cuda-hardware/cuda-box-sweeps.log > /dev/null
run_probe cuda-world-regions cargo run --locked -p voxy_gpu --features cuda --example cuda_voxel_world -- "$task_device"
grep -F "PASS: CUDA world snapshot exact CPU classification, stale/fresh recovery and far anchors" target/cuda-hardware/cuda-world-regions.log > /dev/null
grep -F "PASS: CUDA broadphase 240 character ticks exact CPU state/contact parity" target/cuda-hardware/cuda-world-regions.log > /dev/null
grep -F "PASS: combined CUDA motion and CUDA collision broadphase exact CPU parity" target/cuda-hardware/cuda-world-regions.log > /dev/null
grep -F "PASS: CUDA 4913 obstacles across batches, late nearest contact and canonical overlap tie" target/cuda-hardware/cuda-world-regions.log > /dev/null
grep -F "PASS: CUDA exact sweeps including far anchors and unloaded boundaries" target/cuda-hardware/cuda-world-regions.log > /dev/null

run_probe terrain cargo run --locked -p voxy_gpu --features cuda --example terrain_smoke -- cuda "$task_device"
grep -F 'Terrain CUDA:' target/cuda-hardware/terrain.log > /dev/null
grep -F 'PASS: 502 chunks, 16449536 exact block comparisons, descriptor/seed/i64 bounds/cancellation parity' target/cuda-hardware/terrain.log > /dev/null
run_probe cuda-water cargo run --locked -p voxy_gpu --features cuda --example cuda_water_world -- "$task_device"
grep -F 'PASS: CUDA water 16 exact CPU world plans, revisions, writes and next-active parity' target/cuda-hardware/cuda-water.log > /dev/null
grep -F 'PASS: CUDA water stale revision rejection, no partial publication and fresh CPU parity' target/cuda-hardware/cuda-water.log > /dev/null
grep -F 'PASS: CUDA water lazy faults, read/write budgets, provenance and fresh recovery' target/cuda-hardware/cuda-water.log > /dev/null
grep -F 'PASS: CUDA water 648 exhaustive downward capacity, limit, volume and lazy-read cases' target/cuda-hardware/cuda-water.log > /dev/null
grep -F 'PASS: CUDA water ordered active cascade, lazy reads and final write accounting' target/cuda-hardware/cuda-water.log > /dev/null
grep -F 'PASS: CUDA water successful and failed world/graph requests release device reservations' target/cuda-hardware/cuda-water.log > /dev/null
run_probe projectiles cargo run --locked -p voxy_cuda --features cuda --example projectile_probe -- "$task_device"
grep -F 'CUDA PASS: 257 f64 projectiles x128 batches, exact Euler motion, overflow rejection and recovery' target/cuda-hardware/projectiles.log > /dev/null
run_probe character cargo run --locked -p voxy_app --features cuda --example cuda_character_probe -- "$task_device"
grep -F 'CUDA PASS: 240 character ticks, exact CPU state and voxel contacts, ground and jump exercised' target/cuda-hardware/character.log > /dev/null
run_probe vehicle cargo run --locked -p voxy_app --features cuda --example cuda_vehicle_probe -- "$task_device"
grep -F 'CUDA PASS: 240 vehicle ticks, exact CPU state and voxel contacts, forward and reverse exercised' target/cuda-hardware/vehicle.log > /dev/null
grep -F 'CUDA PASS: combined vehicle motion and CUDA contacts exact CPU parity' target/cuda-hardware/vehicle.log > /dev/null
run_probe gravity cargo run --locked -p voxy_cuda --features cuda --example gravity_probe -- "$task_device"
grep -F 'CUDA PASS: 257 f64 bodies x128 resident Verlet steps' target/cuda-hardware/gravity.log > /dev/null
grep -F 'repeated readback and owner-drop lifetime' target/cuda-hardware/gravity.log > /dev/null
grep -F 'CUDA PASS: singular/overflow sticky errors' target/cuda-hardware/gravity.log > /dev/null
grep -F 'CUDA PASS: gravity shared budget rejects competing allocations and releases for fresh work' target/cuda-hardware/gravity.log > /dev/null
run_probe vulkan-interop cargo run --locked -p voxy_cuda --features cuda --example vulkan_external -- --cuda-device "$task_device"
grep -F 'PASS: Vulkan CUDA import reserves full allocation and releases all gravity/import reservations' target/cuda-hardware/vulkan-interop.log > /dev/null
run_probe shader-pixels cargo run --locked -p voxy_vulkan --features cuda --example cuda_gravity_render -- "$task_device"
grep -F 'PASS: device-matched CUDA f64 gravity -> Vulkan export -> wgpu shader; initial and 64 evolved steps, exact pixels, no body readback or per-frame body upload' target/cuda-hardware/shader-pixels.log > /dev/null
run_probe voxel-diagonal cargo run --locked -p voxy_render --example voxel_diagonal -- vulkan --require-nvidia
grep -F 'PASS: production voxel Uv/Vu diagonals, 128 AO pixels match analytic interpolation' target/cuda-hardware/voxel-diagonal.log > /dev/null
run_probe compute-vulkan env VOXY_COMPUTE_BACKEND=vulkan cargo run --locked -p voxy_render --example compute_smoke -- --require-nvidia
grep -F 'backend: Vulkan' target/cuda-hardware/compute-vulkan.log > /dev/null
for task_marker in 'RESIDENT COMPUTE PASS:' 'DISPATCH LIMIT PASS:' 'COMPUTE RELOAD PASS:' 'PASS: WGSL compute, 1042 exact results'; do
    grep -F "$task_marker" target/cuda-hardware/compute-vulkan.log > /dev/null
done
run_probe device-loss-vulkan cargo run --locked -p voxy_render --example device_loss_surface -- vulkan --require-nvidia
require_nvidia_graphics device-loss-vulkan
grep -F 'backend: Vulkan' target/cuda-hardware/device-loss-vulkan.log > /dev/null
grep -F 'PASS: native device destruction retains diagnostic and guards render, scene and resize' target/cuda-hardware/device-loss-vulkan.log > /dev/null
grep -F 'PASS: native device loss rejects geometry, camera and skin writes' target/cuda-hardware/device-loss-vulkan.log > /dev/null
grep -F 'PASS: destroyed-device compute mapping terminates with a consumed error' target/cuda-hardware/device-loss-vulkan.log > /dev/null
grep -F 'PASS: native renderer recreation on the same window restores exact compute readback' target/cuda-hardware/device-loss-vulkan.log > /dev/null
grep -F 'PASS: recreated native surface presents a fresh frame on the same window' target/cuda-hardware/device-loss-vulkan.log > /dev/null
run_probe water-vulkan cargo run --locked -p voxy_gpu --example water_smoke -- vulkan --require-nvidia
require_nvidia_graphics water-vulkan
grep -F 'backend: Vulkan' target/cuda-hardware/water-vulkan.log > /dev/null
grep -F 'PASS: lazy unknown-node errors' target/cuda-hardware/water-vulkan.log > /dev/null
grep -F 'PASS: pending GPU water success/error isolation' target/cuda-hardware/water-vulkan.log > /dev/null
grep -F 'PASS: pending world plans retain revisions, reject stale commits and recover from fresh capture' target/cuda-hardware/water-vulkan.log > /dev/null
grep -F 'PASS: captured unavailable world chunks remain lazy and cannot enter successful transactions' target/cuda-hardware/water-vulkan.log > /dev/null
grep -F 'PASS: GPU missing sample identifies exact world position and consumes failed plan' target/cuda-hardware/water-vulkan.log > /dev/null
grep -F 'PASS: missing chunk load and fresh GPU retry match CPU without partial failed transfers' target/cuda-hardware/water-vulkan.log > /dev/null
grep -F 'PASS: 16 GPU water ticks, 524288 exact CPU cell comparisons' target/cuda-hardware/water-vulkan.log > /dev/null
run_probe collisions-vulkan cargo run --locked -p voxy_gpu --example voxel_regions_smoke -- vulkan --require-nvidia
require_nvidia_graphics collisions-vulkan
grep -F 'backend: Vulkan' target/cuda-hardware/collisions-vulkan.log > /dev/null
grep -F 'PASS: 240 nonblocking character tasks' target/cuda-hardware/collisions-vulkan.log > /dev/null
grep -F 'PASS: 240 nonblocking GPU vehicle ticks' target/cuda-hardware/collisions-vulkan.log > /dev/null
grep -F 'PASS: GPU vehicle stale rejection, consumed error and exact fresh recovery' target/cuda-hardware/collisions-vulkan.log > /dev/null
grep -F 'PASS: GPU 4913 obstacles, late nearest contact and canonical overlap tie' target/cuda-hardware/collisions-vulkan.log > /dev/null
grep -F 'PASS: pending character stale rejection, consumed errors and fresh recovery' target/cuda-hardware/collisions-vulkan.log > /dev/null

grep -F 'PASS: invalid GPU sweeps reject before world reads or dispatch' target/cuda-hardware/collisions-vulkan.log > /dev/null
grep -F 'PASS: frozen world GPU counts/candidates, stale edit rejection and fresh recovery' target/cuda-hardware/collisions-vulkan.log > /dev/null
grep -F 'PASS: GPU broadphase exact sweep parity, far/unloaded boundaries and stale rejection' target/cuda-hardware/collisions-vulkan.log > /dev/null
grep -F 'PASS: far integer world GPU fault candidate retains exact i64 coordinates' target/cuda-hardware/collisions-vulkan.log > /dev/null
grep -F 'PASS: concurrent nonblocking voxel regions, consumed and dropped readback recovery' target/cuda-hardware/collisions-vulkan.log > /dev/null
grep -F 'PASS: 257 parallel voxel regions, exact CPU counts/candidates/faults' target/cuda-hardware/collisions-vulkan.log > /dev/null
run_probe gameplay-vulkan-collisions cargo run --locked -p voxy_app --features cuda --bin voxy_app -- --backend vulkan --gpu-water --gpu-collisions --cuda-character-motion --cuda-terrain --cuda-projectiles --cuda-vehicle-motion --cuda-device "$task_device" --autopilot
grep -F "Voxy character collision: nonblocking GPU broadphase (Vulkan)" target/cuda-hardware/gameplay-vulkan-collisions.log > /dev/null
grep -E "Voxy GPU character ticks completed: [1-9][0-9]*" target/cuda-hardware/gameplay-vulkan-collisions.log > /dev/null
grep -E "Voxy GPU vehicle ticks completed: [1-9][0-9]*" target/cuda-hardware/gameplay-vulkan-collisions.log > /dev/null
grep -E "Voxy autopilot passed: water=[1-9][0-9]*," target/cuda-hardware/gameplay-vulkan-collisions.log > /dev/null
run_probe gameplay-vulkan-cuda-collisions cargo run --locked -p voxy_app --features cuda --bin voxy_app -- --backend vulkan --gpu-water --cuda-collisions --cuda-character-motion --cuda-terrain --cuda-projectiles --cuda-vehicle-motion --cuda-device "$task_device" --autopilot
grep -F 'Voxy character collision: CUDA broadphase and f64 contacts' target/cuda-hardware/gameplay-vulkan-cuda-collisions.log > /dev/null
grep -F 'CUDA motion integration:' target/cuda-hardware/gameplay-vulkan-cuda-collisions.log > /dev/null
grep -E 'Voxy CUDA collision ticks completed: [1-9][0-9]*' target/cuda-hardware/gameplay-vulkan-cuda-collisions.log > /dev/null
grep -E 'Voxy autopilot passed: water=[1-9][0-9]*,' target/cuda-hardware/gameplay-vulkan-cuda-collisions.log > /dev/null
run_probe gameplay-vulkan-cuda cargo run --locked -p voxy_app --features cuda --bin voxy_app -- --backend vulkan --gpu-water --cuda-terrain --cuda-projectiles --cuda-character-motion --cuda-vehicle-motion --cuda-device "$task_device" --autopilot
grep -F 'CUDA motion integration:' target/cuda-hardware/gameplay-vulkan-cuda.log > /dev/null
grep -F 'CUDA terrain:' target/cuda-hardware/gameplay-vulkan-cuda.log > /dev/null
grep -F 'Voxy water simulation: GPU ordered transfers (Vulkan)' target/cuda-hardware/gameplay-vulkan-cuda.log > /dev/null
grep -F 'Voxy first GPU water tick completed:' target/cuda-hardware/gameplay-vulkan-cuda.log > /dev/null
grep -F 'Voxy autopilot destroyed natural water bed' target/cuda-hardware/gameplay-vulkan-cuda.log > /dev/null
grep -E 'Voxy autopilot passed: water=[1-9][0-9]*,' target/cuda-hardware/gameplay-vulkan-cuda.log > /dev/null
run_probe window cargo run --locked -p voxy_vulkan --features cuda --example cuda_gravity_window -- --smoke --cuda-device "$task_device"

run_probe gameplay-vulkan-cuda-water cargo run --locked -p voxy_app --features cuda --bin voxy_app -- --backend vulkan --cuda-water --cuda-collisions --cuda-character-motion --cuda-terrain --cuda-projectiles --cuda-vehicle-motion --cuda-device "$task_device" --autopilot
grep -F 'Voxy water simulation: CUDA ordered transfers' target/cuda-hardware/gameplay-vulkan-cuda-water.log > /dev/null
grep -E 'Voxy CUDA water ticks completed: [1-9][0-9]*' target/cuda-hardware/gameplay-vulkan-cuda-water.log > /dev/null
grep -E 'Voxy autopilot passed: water=[1-9][0-9]*,' target/cuda-hardware/gameplay-vulkan-cuda-water.log > /dev/null
echo "PASS: physical CUDA buffers, terrain, projectiles, character/vehicle motion, gravity, Vulkan ownership, wgpu pixels, water/gameplay transactions, GPU-assisted collision parity and window lifecycle on device $task_device"
