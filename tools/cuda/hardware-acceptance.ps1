# Native Windows/NVIDIA acceptance. Each gate requires real CUDA, never a fallback.
[CmdletBinding()]
param([ValidateRange(0, 2147483647)][int]$DeviceOrdinal = 0, [switch]$RequireRayQuery)

$ErrorActionPreference = 'Stop'
if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
    Write-Error 'CUDA/DX12 hardware acceptance requires a Windows NVIDIA host' -ErrorAction Continue
    exit 2
}
if ($env:CARGO_BUILD_TARGET) {
    Write-Error 'Hardware acceptance requires native Cargo builds; unset CARGO_BUILD_TARGET' -ErrorAction Continue
    exit 2
}
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$logs = Join-Path $root 'target/cuda-hardware-windows'
[IO.Directory]::CreateDirectory($logs) | Out-Null
$cargo = (Get-Command cargo -CommandType Application).Source

function Invoke-Gate {
    param([string]$Label, [string[]]$CargoArguments, [string[]]$RequiredText = @(), [int]$TimeoutSeconds = 300)
    $log = Join-Path $logs ($Label + '.log')
    $start = New-Object Diagnostics.ProcessStartInfo
    $start.FileName = $cargo
    # All arguments are fixed tokens or the validated numeric ordinal; no shell
    # evaluation or user-provided paths/text are passed to the child process.
    $start.Arguments = $CargoArguments -join ' '
    $start.WorkingDirectory = $root
    $start.UseShellExecute = $false
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $process = [Diagnostics.Process]::Start($start)
    $stdout = $process.StandardOutput.ReadToEndAsync()
    $stderr = $process.StandardError.ReadToEndAsync()
    if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
        # Terminate only the tree spawned for this gate, including its window.
        & taskkill.exe /PID $process.Id /T /F | Out-Null
        if (-not $process.WaitForExit(10000)) {
            [IO.File]::WriteAllText($log, "TIMEOUT: process tree termination could not be confirmed; PID $($process.Id)")
            $process.Dispose()
            throw "TIMEOUT: $Label; process termination unconfirmed; log: $log"
        }
        $contents = $stdout.GetAwaiter().GetResult() + $stderr.GetAwaiter().GetResult()
        [IO.File]::WriteAllText($log, $contents)
        $process.Dispose()
        throw "TIMEOUT: $Label; log: $log"
    }
    $process.WaitForExit()
    $status = $process.ExitCode
    $contents = $stdout.GetAwaiter().GetResult() + $stderr.GetAwaiter().GetResult()
    [IO.File]::WriteAllText($log, $contents)
    $process.Dispose()
    Write-Host $contents
    if ($status -ne 0) { throw "FAIL: $Label (exit $status); log: $log" }
    foreach ($required in $RequiredText) {
        if (-not $contents.Contains($required)) { throw "FAIL: $Label missing '$required'; log: $log" }
    }
}

function Confirm-NvidiaGraphics([string]$Label) {
    $log = Join-Path $logs "$Label.log"
    $contents = Get-Content $log -Raw
    if ($contents -notmatch '(?m)^(Water|Voxel regions|Device loss) GPU: .*vendor: 4318,.*device_type: (DiscreteGpu|IntegratedGpu),') {
        throw "FAIL: $Label requires a physical NVIDIA graphics adapter; log: $log"
    }
}

try {
    Invoke-Gate 'build-cuda' @('build', '--locked', '-p', 'voxy_cuda', '--features', 'cuda', '--example', 'cuda_probe', '--example', 'gravity_probe', '--example', 'projectile_probe', '--example', 'voxel_regions_probe', '--example', 'box_sweep_probe') @() 900
    Invoke-Gate 'build-terrain' @('build', '--locked', '-p', 'voxy_gpu', '--features', 'cuda', '--example', 'terrain_smoke', '--example', 'water_smoke', '--example', 'voxel_regions_smoke', '--example', 'cuda_voxel_world', '--example', 'cuda_water_world') @() 900
    Invoke-Gate 'build-character' @('build', '--locked', '-p', 'voxy_app', '--features', 'cuda', '--example', 'cuda_character_probe', '--example', 'cuda_vehicle_probe', '--bin', 'voxy_app') @() 900
    Invoke-Gate 'build-dx12' @('build', '--locked', '-p', 'voxy_vulkan', '--features', 'cuda', '--example', 'd3d12_export', '--example', 'cuda_gravity_render', '--example', 'cuda_gravity_window') @() 900
    Invoke-Gate 'device-selection-tests' @('test', '--locked', '-p', 'voxy_cuda', '--features', 'cuda', '--example', 'cuda_probe', '--example', 'gravity_probe', '--example', 'projectile_probe', '--example', 'voxel_regions_probe', '--example', 'box_sweep_probe', 'device_selection_is_explicit_and_rejects_ignored_arguments') @('device_selection_is_explicit_and_rejects_ignored_arguments ... ok') 900
    $selectionContents = Get-Content (Join-Path $logs 'device-selection-tests.log') -Raw
    $selectionPasses = [regex]::Matches($selectionContents, '(?m)^test device_argument::tests::device_selection_is_explicit_and_rejects_ignored_arguments \.\.\. ok\r?$').Count
    if ($selectionPasses -ne 5) { throw "FAIL: expected five CUDA device-selection tests, observed $selectionPasses" }
    Invoke-Gate 'd3d12-resource-descriptor' @('test', '--locked', '-p', 'voxy_cuda', '--features', 'cuda', '--lib', 'committed_resource_descriptor_is_dedicated_and_not_opaque') @('committed_resource_descriptor_is_dedicated_and_not_opaque ... ok') 900
    $ordinal = $DeviceOrdinal.ToString([Globalization.CultureInfo]::InvariantCulture)
    Invoke-Gate 'buffers' @('run', '--locked', '-p', 'voxy_cuda', '--features', 'cuda', '--example', 'cuda_probe', '--', $ordinal) @("CUDA PASS: device $ordinal")
    if ((Get-Content (Join-Path $logs 'buffers.log') -Raw) -notmatch 'CUDA PASS: shared resident/transient budgets reject, preserve data and release for fresh work') { throw 'CUDA shared budget proof missing' }
    Invoke-Gate 'cuda-regions' @('run', '--locked', '-p', 'voxy_cuda', '--features', 'cuda', '--example', 'voxel_regions_probe', '--', $ordinal) @('PASS: CUDA 257 voxel regions exact CPU counts/candidates/faults and recovery')
    Invoke-Gate 'cuda-box-sweeps' @('run', '--locked', '-p', 'voxy_cuda', '--features', 'cuda', '--example', 'box_sweep_probe', '--', $ordinal) @('PASS: CUDA 267 exact f64 box sweeps, stationary/overlap/tie cases and invalid-input rejection')
    Invoke-Gate 'cuda-world-regions' @('run', '--locked', '-p', 'voxy_gpu', '--features', 'cuda', '--example', 'cuda_voxel_world', '--', $ordinal) @('PASS: CUDA world snapshot exact CPU classification, stale/fresh recovery and far anchors', 'PASS: CUDA broadphase 240 character ticks exact CPU state/contact parity', 'PASS: combined CUDA motion and CUDA collision broadphase exact CPU parity', 'PASS: CUDA exact sweeps including far anchors and unloaded boundaries', 'PASS: CUDA 4913 obstacles across batches, late nearest contact and canonical overlap tie')
    Invoke-Gate 'terrain' @('run', '--locked', '-p', 'voxy_gpu', '--features', 'cuda', '--example', 'terrain_smoke', '--', 'cuda', $ordinal) @('Terrain CUDA:', 'PASS: 502 chunks, 16449536 exact block comparisons, descriptor/seed/i64 bounds/cancellation parity')
    Invoke-Gate 'cuda-water' @('run', '--locked', '-p', 'voxy_gpu', '--features', 'cuda', '--example', 'cuda_water_world', '--', $ordinal) @('PASS: CUDA water 16 exact CPU world plans, revisions, writes and next-active parity', 'PASS: CUDA water stale revision rejection, no partial publication and fresh CPU parity', 'PASS: CUDA water lazy faults, read/write budgets, provenance and fresh recovery', 'PASS: CUDA water 648 exhaustive downward capacity, limit, volume and lazy-read cases', 'PASS: CUDA water ordered active cascade, lazy reads and final write accounting', 'PASS: CUDA water successful and failed world/graph requests release device reservations')
    Invoke-Gate 'projectiles' @('run', '--locked', '-p', 'voxy_cuda', '--features', 'cuda', '--example', 'projectile_probe', '--', $ordinal) @('CUDA PASS: 257 f64 projectiles x128 batches, exact Euler motion, overflow rejection and recovery')
    Invoke-Gate 'character' @('run', '--locked', '-p', 'voxy_app', '--features', 'cuda', '--example', 'cuda_character_probe', '--', $ordinal) @('CUDA PASS: 240 character ticks, exact CPU state and voxel contacts, ground and jump exercised')
    Invoke-Gate 'vehicle' @('run', '--locked', '-p', 'voxy_app', '--features', 'cuda', '--example', 'cuda_vehicle_probe', '--', $ordinal) @('CUDA PASS: 240 vehicle ticks, exact CPU state and voxel contacts, forward and reverse exercised', 'CUDA PASS: combined vehicle motion and CUDA contacts exact CPU parity')
    Invoke-Gate 'gravity' @('run', '--locked', '-p', 'voxy_cuda', '--features', 'cuda', '--example', 'gravity_probe', '--', $ordinal) @('CUDA PASS: 257 f64 bodies x128 resident Verlet steps', 'CUDA PASS: singular/overflow sticky errors')
    if ((Get-Content (Join-Path $logs 'gravity.log') -Raw) -notmatch 'CUDA PASS: gravity shared budget rejects competing allocations and releases for fresh work') { throw 'CUDA gravity shared budget proof missing' }
    Invoke-Gate 'dx12-interop' @('run', '--locked', '-p', 'voxy_vulkan', '--features', 'cuda', '--example', 'd3d12_export', '--', '--cuda-device', $ordinal) @("CUDA/DX12 selected device $ordinal, LUID", 'backend: Dx12', 'PASS: CUDA writes in shared D3D12 committed storage visible to wgpu; 24 exact words')
    if ((Get-Content (Join-Path $logs 'dx12-interop.log') -Raw) -notmatch 'PASS: DX12 CUDA import reserves full allocation and releases reservation') { throw 'DX12 CUDA import budget proof missing' }
    Invoke-Gate 'build-render-probes' @('build', '--locked', '-p', 'voxy_render', '--example', 'voxel_diagonal', '--example', 'compute_smoke', '--example', 'device_loss_surface', '--example', 'wgsl_inventory') @() 900
    Invoke-Gate 'wgsl-inventory' @('run', '--locked', '-p', 'voxy_render', '--example', 'wgsl_inventory') @('WGSL inventory:', '0 failures; no GPU execution')
    if ((Get-Content (Join-Path $logs 'wgsl-inventory.log') -Raw) -notmatch '(?m)^WGSL inventory: [1-9][0-9]* files, [1-9][0-9]* variants, [1-9][0-9]* validated entrypoints, 0 failures; no GPU execution\r?$') { throw 'Complete WGSL inventory proof missing' }
    if ($RequireRayQuery) {
        Invoke-Gate 'build-ray-probe' @('build', '--locked', '--no-default-features', '-p', 'voxy_ray_probe', '--bin', 'voxy_ray_probe', '--example', 'animated_ray') @() 900
        Invoke-Gate 'ray-query-dx12' @('run', '--locked', '--no-default-features', '-p', 'voxy_ray_probe', '--bin', 'voxy_ray_probe', '--', '--experimental', '--backend', 'dx12', '--require-nvidia') @('RAY SMOKE PASS:', 'PRIMARY BACKGROUND PASS:', 'GPU primary reflection -> HDR composition / material MRT:')
        if ((Get-Content (Join-Path $logs 'ray-query-dx12.log') -Raw) -notmatch '(?m)^RAY GPU: .*vendor: 4318,.*device_type: (DiscreteGpu|IntegratedGpu),.*backend: Dx12,') { throw 'DX12 ray query requires physical NVIDIA proof' }
        Invoke-Gate 'animated-ray-dx12' @('run', '--locked', '--no-default-features', '-p', 'voxy_ray_probe', '--example', 'animated_ray', '--', '--experimental', '--backend', 'dx12', '--require-nvidia', '--smoke') @('ANIMATED RAY PASS: 120 presentations')
        if ((Get-Content (Join-Path $logs 'animated-ray-dx12.log') -Raw) -notmatch '(?m)^ANIMATED RAY GPU: .*vendor: 4318,.*device_type: (DiscreteGpu|IntegratedGpu),.*backend: Dx12,') { throw 'Animated DX12 ray query requires physical NVIDIA proof' }
        Write-Host 'PASS: physical NVIDIA DX12 ray-query gate'
    }
    Invoke-Gate 'voxel-diagonal' @('run', '--locked', '-p', 'voxy_render', '--example', 'voxel_diagonal', '--', 'dx12', '--require-nvidia') @('PASS: production voxel Uv/Vu diagonals, 128 AO pixels match analytic interpolation')
    Invoke-Gate 'shader-pixels' @('run', '--locked', '-p', 'voxy_vulkan', '--features', 'cuda', '--example', 'cuda_gravity_render', '--', $ordinal, '--dx12') @('CUDA render DX12:', 'PASS: device-matched CUDA f64 gravity -> DX12 export -> wgpu shader; initial and 64 evolved steps, exact pixels, no body readback or per-frame body upload')
    $savedComputeBackend = $env:VOXY_COMPUTE_BACKEND
    try {
        $env:VOXY_COMPUTE_BACKEND = 'dx12'
        Invoke-Gate 'compute-dx12' @('run', '--locked', '-p', 'voxy_render', '--example', 'compute_smoke', '--', '--require-nvidia') @('backend: Dx12', 'RESIDENT COMPUTE PASS:', 'DISPATCH LIMIT PASS:', 'COMPUTE RELOAD PASS:', 'PASS: WGSL compute, 1042 exact results')
    } finally {
        $env:VOXY_COMPUTE_BACKEND = $savedComputeBackend
    }
    Invoke-Gate 'device-loss-dx12' @('run', '--locked', '-p', 'voxy_render', '--example', 'device_loss_surface', '--', 'dx12', '--require-nvidia') @('backend: Dx12', 'PASS: native device destruction retains diagnostic and guards render, scene and resize', 'PASS: native device loss rejects geometry, camera and skin writes', 'PASS: destroyed-device compute mapping terminates with a consumed error', 'PASS: native renderer recreation on the same window restores exact compute readback', 'PASS: recreated native surface presents a fresh frame on the same window')
    Confirm-NvidiaGraphics 'device-loss-dx12'
    Invoke-Gate 'water-dx12' @('run', '--locked', '-p', 'voxy_gpu', '--example', 'water_smoke', '--', 'dx12', '--require-nvidia') @('backend: Dx12', 'PASS: pending GPU water success/error isolation', 'PASS: pending world plans retain revisions, reject stale commits and recover from fresh capture', 'PASS: lazy unknown-node errors', 'PASS: 16 GPU water ticks, 524288 exact CPU cell comparisons', 'PASS: GPU missing sample identifies exact world position and consumes failed plan', 'PASS: missing chunk load and fresh GPU retry match CPU without partial failed transfers', 'PASS: captured unavailable world chunks remain lazy and cannot enter successful transactions')
    Confirm-NvidiaGraphics 'water-dx12'
    Invoke-Gate 'collisions-dx12' @('run', '--locked', '-p', 'voxy_gpu', '--example', 'voxel_regions_smoke', '--', 'dx12', '--require-nvidia') @('backend: Dx12', 'PASS: frozen world GPU counts/candidates, stale edit rejection and fresh recovery', 'PASS: GPU broadphase exact sweep parity, far/unloaded boundaries and stale rejection', 'PASS: invalid GPU sweeps reject before world reads or dispatch', 'PASS: 240 nonblocking character tasks', 'PASS: 240 nonblocking GPU vehicle ticks', 'PASS: GPU vehicle stale rejection, consumed error and exact fresh recovery', 'PASS: GPU 4913 obstacles, late nearest contact and canonical overlap tie', 'PASS: pending character stale rejection, consumed errors and fresh recovery', 'PASS: far integer world GPU fault candidate retains exact i64 coordinates', 'PASS: concurrent nonblocking voxel regions, consumed and dropped readback recovery', 'PASS: 257 parallel voxel regions, exact CPU counts/candidates/faults')
    Confirm-NvidiaGraphics 'collisions-dx12'
    Invoke-Gate 'gameplay-dx12-collisions' @('run', '--locked', '-p', 'voxy_app', '--features', 'cuda', '--bin', 'voxy_app', '--', '--backend', 'dx12', '--gpu-water', '--gpu-collisions', '--cuda-character-motion', '--cuda-terrain', '--cuda-projectiles', '--cuda-vehicle-motion', '--cuda-device', $ordinal, '--autopilot') @('Voxy character collision: nonblocking GPU broadphase (Dx12)', 'Voxy GPU character ticks completed:', 'Voxy first GPU water tick completed:', 'Voxy autopilot passed: water=')
    if ((Get-Content (Join-Path $logs 'gameplay-dx12-collisions.log') -Raw) -notmatch 'Voxy GPU character ticks completed: [1-9][0-9]*') {
        throw 'GPU character gameplay completed no collision ticks'
    }
    if ((Get-Content (Join-Path $logs 'gameplay-dx12-collisions.log') -Raw) -notmatch 'Voxy GPU vehicle ticks completed: [1-9][0-9]*') {
        throw 'Combined CUDA motion/GPU vehicle collision completed no ticks'
    }
    Invoke-Gate 'gameplay-dx12-cuda-collisions' @('run', '--locked', '-p', 'voxy_app', '--features', 'cuda', '--bin', 'voxy_app', '--', '--backend', 'dx12', '--gpu-water', '--cuda-collisions', '--cuda-character-motion', '--cuda-terrain', '--cuda-projectiles', '--cuda-vehicle-motion', '--cuda-device', $ordinal, '--autopilot') @('Voxy character collision: CUDA broadphase and f64 contacts', 'CUDA motion integration:', 'Voxy first GPU water tick completed:', 'Voxy autopilot passed: water=')
    Invoke-Gate 'gameplay-dx12-cuda-water' @('run', '--locked', '-p', 'voxy_app', '--features', 'cuda', '--bin', 'voxy_app', '--', '--backend', 'dx12', '--cuda-water', '--cuda-collisions', '--cuda-character-motion', '--cuda-terrain', '--cuda-projectiles', '--cuda-vehicle-motion', '--cuda-device', $ordinal, '--autopilot') @('Voxy water simulation: CUDA ordered transfers', 'Voxy CUDA water ticks completed:', 'Voxy autopilot passed: water=')
    $cudaWaterGameplay = Get-Content (Join-Path $logs 'gameplay-dx12-cuda-water.log') -Raw
    if ($cudaWaterGameplay -notmatch 'Voxy CUDA water ticks completed: [1-9][0-9]*' -or $cudaWaterGameplay -notmatch 'Voxy autopilot passed: water=[1-9][0-9]*,') {
        throw 'CUDA water gameplay completed no CUDA water ticks or committed transactions'
    }
    $cudaCollisionLog = Get-Content (Join-Path $logs 'gameplay-dx12-cuda-collisions.log') -Raw
    if ($cudaCollisionLog -notmatch 'Voxy CUDA collision ticks completed: [1-9][0-9]*' -or $cudaCollisionLog -notmatch 'Voxy autopilot passed: water=[1-9][0-9]*,') {
        throw 'CUDA collision gameplay completed no collision ticks or water transactions'
    }
    Invoke-Gate 'gameplay-dx12-cuda' @('run', '--locked', '-p', 'voxy_app', '--features', 'cuda', '--bin', 'voxy_app', '--', '--backend', 'dx12', '--gpu-water', '--cuda-terrain', '--cuda-projectiles', '--cuda-character-motion', '--cuda-vehicle-motion', '--cuda-device', $ordinal, '--autopilot') @('CUDA motion integration:', 'CUDA terrain:', 'Voxy water simulation: GPU ordered transfers (Dx12)', 'Voxy first GPU water tick completed:', 'Voxy autopilot destroyed natural water bed', 'Voxy autopilot passed: water=')
    Invoke-Gate 'window' @('run', '--locked', '-p', 'voxy_vulkan', '--features', 'cuda', '--example', 'cuda_gravity_window', '--', '--smoke', '--cuda-device', $ordinal, '--dx12') @('export_only=false', 'dx12=true', '200 resident steps, no body readback')
    Write-Host "PASS: physical CUDA buffers, terrain, projectiles, character/vehicle motion, gravity, DX12 memory, shader pixels, water/gameplay transactions, GPU-assisted collision parity and window lifecycle on device $ordinal"
} catch {
    Write-Error $_ -ErrorAction Continue
    exit 1
}
