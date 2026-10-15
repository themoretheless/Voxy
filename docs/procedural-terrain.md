# Procedural terrain

`ProceduralTerrainGenerator` implements `ChunkGenerator` without external noise dependencies.
World seed and absolute voxel coordinates determine every column; generation order and chunk boundaries do not affect the result. Integer value noise with smoothstep interpolation retains coordinate precision at both i64 limits.

Four independent noise fields describe continental elevation (128 blocks), mountain distribution (192), relief (32), and fine variation (8). Mountain strength blends continuously into plains. Heights lie in 0..=31, within the currently rendered vertical chunk. Lowlands fill with water up to y=6. Ocean beds use soil; plains use grass over soil; mountains above y=18 expose stone. These are three simple terrain biomes, not a temperature/humidity ecosystem.

`column(x, z, seed)` exposes height and biome for placement and inspection. The descriptor includes the block palette and a generator version. Changes to the noise algorithm or constants require a version bump for persistent worlds.

`build_procedural_scene(seed, radius)` generates and meshes this terrain. The desktop application's default scene uses it. `build_bootstrap_scene` retains the original fixtures used by gameplay tests. The actor starts above the maximum terrain height.

Run `cargo run -p voxy_app`. The current desktop scene uses a fixed seed and a 3 by 3 chunk area; biome regions extend beyond that area. Caves, ores, climate biomes, and infinite streaming are future work.

Validation: world sampling across positive and negative chunks, request-order independence, seed variation, biome coverage, height bounds, neighbor continuity, water layering, cancellation, coordinate overflow, and deterministic runtime meshing.

## Trees and shrubs

Desktop procedural scenes decorate terrain with `VegetationGenerator` on every
terrain backend. Vegetation placement runs on the CPU after terrain generation.
A seed determines candidates in 4 by 4 voxel cells: broadleaf trees have trunks
4–6 voxels tall and rounded crowns; shrubs have low woody stems and smaller
crowns. Plants grow on plains above sea level and only replace air. Wood is solid;
foliage has no collision. The current block materials use brown wood and green leaves.

Candidates outside each chunk are sampled too, so crowns continue across horizontal
and vertical boundaries independently of generation order. The wrapper has its own
versioned descriptor including the base generator, terrain profile and vegetation
palette. Library callers can keep bare terrain or supply their own vegetation block
IDs through `VegetationPalette`. Custom base generators passed to
`build_generated_scene` must retain the canonical procedural height profile.

## Accelerated generation

`voxy_gpu::GpuTerrainGenerator` implements the same `ChunkGenerator` and retains
procedural-terrain v1's descriptor, all 64 seed bits, signed i64 coordinate
semantics and block palette. The host prepares exact lattice cell addresses;
WGSL runs the hash, integer noise interpolation, biome selection and 32³ block
assignment. Two u32 limbs emulate u64 hash arithmetic without requiring optional
shader int64 support. Generated data is validated and palettized before returning.
Cancellation and coordinate overflow reject before device allocation; cancellation
is checked again before publishing results. Device waits have a 30-second timeout.
This synchronous generator is intended for native generation workers.

`build_generated_scene` accepts a generator factory using the actual scene registry
IDs. The desktop app selects accelerated generation explicitly:

```sh
cargo run -p voxy_app -- --gpu-terrain
cargo run -p voxy_app --features cuda -- --cuda-terrain
```

The CUDA path selects device ordinal 0 in the app. Library callers can choose
another ordinal through `CudaTerrainGenerator::new`. It uses private CUDA storage,
NVRTC-compiled fixed source and cached kernel/module ownership. An NVIDIA driver
and compatible NVRTC library are required; absent driver/compiler and kernel errors
are reported without CPU fallback. Default builds load neither CUDA nor NVRTC.
The flags are mutually exclusive. With neither flag, CPU generation remains the
existing default. These paths change generation, not the graphics API.

Verification commands:

```sh
cargo run -p voxy_gpu --example terrain_smoke -- metal
cargo run -p voxy_app --example gpu_terrain_smoke -- metal
cargo test -p voxy_gpu cuda_source_host_parity -- --ignored --nocapture
# On NVIDIA hardware with driver and NVRTC:
cargo run -p voxy_gpu --features cuda --example terrain_smoke -- cuda
cargo run -p voxy_app --features cuda --example gpu_terrain_smoke -- cuda
```

Verified on Apple M4 Max Metal: 502 chunks and 16,449,536 exact CPU block
comparisons, including different x/z coordinates, negative cell boundaries,
extreme i64 chunk extents, high-bit seeds, cancellation and coordinate overflow.
Integrated bootstrap produced the same nine rendered meshes and light volumes
as CPU generation. The desktop GPU-generation run presented its first voxel frame.

The C++ host harness compiles the same CUDA source with device qualifiers removed
and calls every column invocation serially. It matched CPU generation for 180
chunks (5,898,240 blocks). This verifies integer arithmetic and data layout only;
it does not prove NVRTC compilation, CUDA device scheduling, driver execution or
synchronization. CUDA-feature host/Windows/Linux checks passed. On this macOS host,
the actual CUDA probe returned `DriverUnavailable`; NVIDIA execution is unverified.

## Nonblocking browser jobs

`TerrainProgram` creates the same fixed pipeline on a caller-owned compute device.
`create_job` prepares a chunk, `TerrainJob::encode` records generation/readback,
and `TerrainDispatch::begin_read` starts mapping after queue submission.
`PendingTerrain::try_read` returns None while pending and publishes one validated
chunk on completion. Cancellation is checked before upload, encoding and result
publication; terminal results are consumed exactly once. Dropping pending work
releases its private readback resources. Native callers drive device polling;
browser callers yield to the event loop. The native generator now uses this same
job lifecycle, with its existing bounded worker wait.

WebGPU devices now request compute-capable baseline limits. WebGL2 retains its
previous graphics limits and reports compute unsupported explicitly. The browser
`WebEngine.generate_terrain(x, y, z, seed)` returns 32768 u32 block IDs through a
Promise. Its arguments use JS BigInt for exact i64 coordinates/u64 seeds; output
index is x+32*(z+32*y), with IDs air=0, surface=1, soil=2, stone=3, water=4.
This API generates data; the current browser scene still renders its existing
2D/3D demo rather than voxel chunks.

After building the browser package, `?backend=webgpu&terrain=1` runs a CPU/GPU
comparison before starting animation. Verified in the in-app browser: 98304
exact block comparisons over three chunks, including negative/extreme coordinates
and high-bit seeds; the animation continued afterwards. The corresponding WebGL2
page reported compute unsupported and continued rendering. WASM strict Clippy
passed. `terrain_async_smoke` on Metal checked 65536 exact blocks, independent
jobs, cancellation at three lifecycle stages and consumed-result rejection.


Compiler acceptance now has a separate real NVIDIA tooling check:
`sh tools/cuda/verify_compilation.sh`. NVRTC 12.6 compiled the terrain source,
and PTXAS assembled it for sm_52, sm_75 and sm_89. This verifies compiler
acceptance; NVIDIA driver/device execution remains unverified locally.


Native OpenGL parity: `cargo run -p voxy_gpu --example terrain_smoke -- gl`.
Actual Linux/Mesa 25.0.7 llvmpipe OpenGL 4.5 passed 502 chunks and 16,449,536 exact
CPU comparisons, including i64 extremes and seed bits. This verifies the software
OpenGL path; physical GPU execution on other drivers remains unverified.
`sh tools/linux/opengl-smoke.sh` includes this check with the scene/shader/physics
probes and retains per-probe logs in `target/linux-docker`.

## Botanical mesh generation

The voxel decorator above remains the gameplay terrain representation. For detailed
plants use the separate native `voxy_model::plants` generator: tapered, curved
branch hierarchies, flared tree roots, displaced bark, and thousands of individual
curved pointed leaves. Trees and multi-stem shrubs use metres and a reproducible
64-bit seed. Heights are bounded to 0.25–30 metres; geometry complexity is fixed.

```sh
cargo run -p voxy_app --bin voxy_app -- --vegetation tree --seed 42 --height 7
cargo run -p voxy_app --bin voxy_app -- --vegetation shrub --seed 42 --height 1.6
cargo run -p voxy_app --bin voxy_app -- --vegetation tree --output tree.glb --export-only
cargo run -p voxy_model --example plants
```

The GLB embeds six PNG maps: bark/leaf colour, normal and metallic/roughness.
It includes UVs, normals, separate PBR materials, and double-sided foliage. No external image assets
or downloads are needed. The model editor opens a separate `.preview.glb` with normal and roughness-map bindings removed
while retaining the full exported asset; it does not display the full normal-map/PBR
material. The `plants` example renders a native GPU preview through the scene PBR renderer
with GGX, normal maps, hemisphere lighting, shadow maps and tone mapping, and writes
`artifacts/vegetation/preview.png` plus tree/shrub GLBs.

This is detailed procedural vegetation, not a qualified photorealistic renderer.
Leaf backlighting and crown-density sky occlusion are approximations; measured leaf transmission, full indirect lighting and wind remain absent. Desktop startup now places mesh plants automatically from the seeded world markers.
The resident renderer uses shared instanced meshes, depth-correct PBR surfaces,
self-shadowing and shadow reception on terrain/actors in both normal and Bodycam
paths. Four shared world-detail variants keep geometry bounded. World startup builds raw meshes and six surface maps directly, avoiding a GLB encode/decode round trip. Root flares and a third grass primitive dress the ground around each plant. The HDR terrain shader treats IDs 1–3 as natural rock, grass and soil, without masonry seams. Gameplay retains
conservative voxel trunk collision; leaf and branch shapes have no fine collision.
Removing a root marker or its supporting ground retires that plant instance.
`--voxel-plants` selects the original block rendering. `--vegetation-check` captures
the same world/camera with and without the plant layer and writes evidence to
`artifacts/vegetation-world`. Captures use the offscreen GPU path; the report records actual presented frames separately, including when the window is occluded. Bark and leaf
textures are synthetic, not scanned botanical materials.


Native world inspection:

```sh
cargo run -p voxy_app --bin voxy_app
cargo run -p voxy_app --bin voxy_app -- --vegetation-check
cargo run -p voxy_app --bin voxy_app -- --voxel-plants
```

World instances currently have fixed detail, no wind, and no new-plant authoring.
The bounded startup world owns placement; streamed-world plant admission/eviction
is not implemented. Root removal also clears the remaining conservative trunk
markers, so retired models do not leave invisible trunk collision behind.

Vegetated-terrain descriptor v2 increases candidate density and gives wood priority
over every overlapping crown, preserving contiguous trunk markers at cell borders.
