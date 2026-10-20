# Original street walk

Run from the repository with `cargo run --release --offline -p voxy_app --example street_walk`.
The original geometry is authored by the deterministic Rust generator. It does not use
Bistro/Amazon meshes. Surface scans and daylight are separately identified CC0 assets
from Poly Haven in `assets/environments/original-street/materials/provenance.json` and
`daylight-provenance.json`; these are external material data, not generated artwork.

The current quality pass varies building frontage widths, setbacks, floor and opening
counts, plaster/brick colours, balconies, shutters and shop entrances. Trees have
hierarchical tapered branches and individual geometric leaves. Every rendered triangle
is retained in the f64 collision world. Full 4K source maps retain their 8/16-bit precision
in linear RGBA32F GPU textures with complete mip chains and 16x anisotropic sampling.
There is no reduced rendering resolution, geometry decimation or distance LOD.

Lighting uses the existing Voxy HDR diffuse convolution, GGX environment prefilter and
DFG lookup, an HDR daylight sky, full-resolution contact occlusion and an 8192² cached
sun-shadow map. The shadow direction follows the brightest HDR sky source. Environment
integration and contact occlusion are approximations; this is not path-traced GI.
The photorealism target requires visual acceptance and must not be inferred from these
features alone. The current performance target is **at least 120 FPS at 1920×1080**.

The street continues through an arch in the end colonnade: a paved lane between
garden walls leads to a walled plot with a modern two-storey villa and a 4.8 × 14 m
swimming pool (shallow end 1.0 m, ramp, deep end 1.9 m, mosaic basin, steps, ladder and
underwater lights). The pool water is a `voxy_water` basin, drawn by the
scene's water pass on the FFT wave field shared with the `ocean` example: Fresnel
reflection of the screen and sky, refraction of the basin with Beer–Lambert absorption,
first-order caustics on whatever the refracted ray reaches, and an underwater view with
Snell's window when you jump in. The pass is skipped while the pool is off-screen.
`--spawn pool` starts at the garden gate. Wading leaves a ripple wake and jumping in
throws up a splash (`voxy_water` ripple field). An untextured preview of the generated geometry
renders headlessly with `VOXY_VILLA_PREVIEW=/tmp/villa.png cargo test -p voxy_app
--example street_walk render_villa_preview -- --ignored` (`VOXY_VILLA_VIEW=x,y,z,dx,dy,dz`).

Controls: click to capture the mouse, WASD/arrows, Shift to run, Space to jump,
V for smooth camera distance toggle, wheel for smooth zoom, Esc to release the mouse.
Day/night, seasons and weather: any of `--hour 18.5`, `--day 200`, `--season winter`,
`--time-scale 0.1` (simulated hours per real second, default 1/30) or `--latitude 48`
enables the calendar. The sun follows solar declination and hour angle (+X east, -Z north),
is reddened by air mass and dimmed by clouds, and exposure drops about 7 EV at night so the
street lamps carry the scene. Weather follows a seeded seasonal schedule of 4-hour spells
(no snow in summer, mostly snow in winter); wet ground dries over about three hours.
Keys: T pause, `[`/`]` ±1 hour, Y next season, `-`/`=` slower/faster, 1-4 force a weather
preset, 0 back to the seasonal schedule. Foliage colour and snow cover do not change with
the season yet. Benchmarks, snapshots and AA validation keep the calendar frozen.
`--seed 12345` chooses another reproducible variant. `--bistro` explicitly selects the
separately attributed reference scene; its source license remains intact.
`VOXY_STREET_MATERIALS` can point to the retained material directory.

`--snapshot /tmp/street.png` writes an SDR PNG preview, native HDR float32 PFM and
HDR pixel metadata. The PNG is not an HDR image. Default presentation uses extended
linear sRGB/float16 where supported, with live display headroom; `--sdr` selects SDR.

`--benchmark 20` measures presented window frames after 180 warmup frames. Capture
happens at frame 120 and is excluded from the measured interval. For GPU completion
throughput at the same resolution/materials/geometry, also use `--offscreen-benchmark`;
each submission is awaited, not merely counted. This diagnostic does not prove display
FPS. Timing queries are optional with `VOXY_STREET_GPU_TIMING=1` and must not substitute
for completed-frame or presented-frame measurements.

Actual GPU precision can be checked with `cargo run --release --offline -p voxy_app
--example qualify_float_material_upload`. The check reads back the actual top mip and
compares all float32 bits, including adjacent 16-bit source levels and HDR values.

Сглаживание по умолчанию использует 200% разрешения по каждой оси
(3840×2160 → 1920×1080) вместе с выбранным TAA/MetalFX. Усреднение выполняется
в линейном HDR до преобразования цвета. Это четыре пространственные выборки
на выходной пиксель, включая первый кадр после сброса истории; нагрузка и память
выше. `VOXY_STREET_RENDER_SCALE=1` возвращает обычное разрешение.


Current verified showcase (2026-10-15): the prepared CC0 panorama and image-based lighting
are the default sky; `VOXY_STREET_SKY=analytic` selects the procedural atmosphere.
Nine source maps pack into six RGBA32F textures without changing any used channel;
4096 filtered GPU samples, including mip interpolation, matched bit-for-bit.
CPU SIMD selection matches the scalar path and preserves ordering. All GPU
validation remains active. GPU compaction/subgroup experiments are opt-in.

The controlled benchmark uses a time-based 20-second forward/backward cycle.
Camera input is disabled only during measurement; Esc cancels it. Loading and
offscreen-test windows remain hidden. Frame-acquisition skips are reported.

Current evidence: `artifacts/original-street-realism-2026-10-15/status.json`.
Stable 120 FPS has not passed. Independently measured GPU load was 95–99% with
the street stopped; background qualifications were preserved. Earlier uncontrolled
131-FPS results do not certify the fixed route or a minimum frame rate.

## Exact screen-ray depth bounds

The opaque depth prepass now feeds an RG32Float conservative 8x8 min/max bound (the original march uses its unchanged minimum channel). Rays skip only impossible level-zero depth reads and reuse immutable tile/pixel depths within a ray; the complete 80-step, 60 m march and eight refinements remain enabled for GI and reflections. The qualifier checks the actual production owner and screen-ray function against the original march for 8192 rays, including both history modes and all fused tile minima.

Use `VOXY_STREET_RAY_REFERENCE=1` for the original full march. Pin `VOXY_STREET_RENDER_SCALE=2` to reproduce the 3840x2160 quality qualification and set `VOXY_STREET_BENCHMARK_REPORT` to a unique path when running concurrent work. The report includes the worst frame and minimum sampled FPS; offscreen completion timing cannot qualify presented 120 FPS. The current measurements under changing shared load do not establish a performance win or minimum 120 FPS.

The further refinement-cache candidate is opt-in via `VOXY_STREET_REFINE_CACHE=1`: expanded GPU checks compare each of 262144 individual ray outputs to the frozen previous kernel, but paired timings have not established a repeatable win. It is disabled by default. Benchmark completion now requires the requested fixed-step simulation duration as well as wall time; reports include measured physics steps and complete 2400-step motion cycles, so long frame stalls cannot falsely qualify a full cycle.

Candidate full-pixel traversal: `VOXY_STREET_PAIRED_DDA=1`. This uses the original
Depth32Float attachment with compensated camera projection, intersection and world
reconstruction. It is opt-in pending full-scene performance and history/visual
acceptance; it does not establish 120 FPS. See
`artifacts/original-street-realism-2026-10-15/combined-dda/status.json`.

`VOXY_STREET_PAIRED_DDA_TILES=1` additionally enables conservative min/max tile
rejection for paired DDA. It remains opt-in: synthetic correctness and fewer
visited cells do not establish a full-scene speedup. `VOXY_STREET_INIT_TRACE=1`
reports setup stages and completed full-quality light probes;
`VOXY_STREET_BENCHMARK_PROGRESS=1` logs every GPU-completed offscreen frame.
See `artifacts/original-street-realism-2026-10-15/tile-dda/status.json`.

Paired-ray candidates also expose `VOXY_STREET_DDA_CELL_BOUNDS=1` and
`VOXY_STREET_DDA_FAR_BOUND=1`; all survivors retain the original paired tests.
They remain opt-in pending scene performance acceptance. Irradiance baking now
yields between full-resolution faces and completes before benchmark warmup.
`qualify_probe_bake` checks exact SH coefficients with interleaved editor frames.
See `artifacts/original-street-realism-2026-10-15/ray-arithmetic/status.json`.

`VOXY_STREET_DDA_HIERARCHY=1` enables the same min/max bound owner with a
padded power-of-two base and all mip levels. With paired DDA and tiles enabled,
traversal ascends only after a conservative proven skip and descends near possible
geometry. GPU mip values and boundary ray outputs were checked against independent
references, including 3840x2160 and odd dimensions.

`VOXY_STREET_DDA_EDGE_CACHE=1` reuses an identical compensated boundary division
only when its integer cell axis has not changed. It remains opt-in pending full
scene timing. No approximate reciprocal or precision reduction is used.

With benchmark progress and GPU timing enabled, per-frame stage samples are logged
as belonging to the previous completed frame. New benchmark reports exclude probe
baking and warmup GPU samples. Stage intervals can overlap and must not be summed
as a serial frame duration.

`VOXY_STREET_EARLY_GI=1` is a scheduling-only candidate that evaluates the same
GI query immediately after the final wet/normal evaluation, before direct lighting.
It remains opt-in. The `qualify_ray_schedule` example captures native RGBA16F
frames for byte comparison across independent late/early process runs, with
procedural or `--hdr` lighting. This is native output equality, not a proof of
unrounded intermediate float32 values or full scene performance.

`VOXY_STREET_COMPUTE_GI=1` evaluates opaque GI in a separate full-resolution
compute pass. The shared material evaluator supplies world positions and final
normals through two RGBA32Float attachments; the RGBA32Float GI result is loaded
by opaque shading. Transparent layers retain their own queries. This adds 48
bytes per internal pixel of budgeted GPU textures and remains opt-in pending
full scene performance qualification.

`VOXY_STREET_GI_PRECISION_PROOF=1` additionally exports the original fragment GI
into a diagnostic RGBA32Float attachment. `qualify_ray_schedule` compares every
component against the compute result before native HDR rounding, and requires
real GI hits. `--maps` includes normal, packed, specular, emissive and masked
material textures. Keep this diagnostic flag unset during performance runs.

Paired DDA render pipelines now use the same precise Metal numerical contract
as compute, including explicit FMA contraction policy. This preserves the low
residual arithmetic. Performance reports made with the older fragment compiler
policy require a new baseline before attributing a speedup.

`VOXY_STREET_COMPUTE_SSR=1` together with `VOXY_STREET_COMPUTE_GI=1` moves
opaque reflections into a separate full-resolution compute dispatch. The same
Float32 guides encode unchanged mirror admission and history mode; transparent
layers retain their own queries. Combined guide/GI/reflection storage is 64 bytes
per internal pixel. Both flags remain opt-in pending full scene qualification.
The precision diagnostic uses separate raw GI and reflection references and also
compares complete native material frames. It does not prove every unrounded
full-material intermediate. Initial combined diagnostic differences are preserved
in `artifacts/original-street-realism-2026-10-15/compute-ssr/status.json`.

`VOXY_STREET_DDA_POINT_CACHE=1` caches the exact unsnapped entry depth pair and
only reconstructs XY for closed boundary touches when the existing exact depth
comparison can possibly match. Hierarchy jumps still reconstruct all coordinates.
Ordinary cell advancement, plane-hit tests and tie ordering are unchanged. Native
independent-reference and bitwise checks pass; full scene timing is pending.

`VOXY_STREET_DDA_PLANE_CACHE=1` reuses the exact finite plane parameter for
identical stored depth bits within one ray and rejects candidates outside the
existing exact parameter bounds before reconstructing XY. Signed zero depth
encodings stay distinct; nonfinite quotients retain the original calculation.
Native independent-reference and bitwise gates pass; full scene timing is pending.
The separate point-cache candidate showed no reliable full-cycle gain and is not
included in the plane-cache benchmark.

`VOXY_STREET_RAY_STAGES=1` with paired DDA and compute GI schedules projection,
depth traversal and history/radiance reconstruction as three exact kernels. A
bounded 1024x512 chunk retains all 176 segment bytes and 32 hit bytes; all pixels
and full-image ray paths remain covered. Frames without history retain the
original monolithic calculation. Native frame/raw-output, partial-edge,
unsupported-projection and probe-bake regressions pass. The isolated full scene
run did not establish a performance gain (0.812 FPS versus 0.909 FPS baseline),
so the mode remains disabled by default. GI/SSR query totals are available;
individual per-stage timing is not decoded yet.

Precision-only diagnostics `VOXY_STREET_RAY_CHUNK_TEST=1` and
`VOXY_STREET_RAY_FORCE_FALLBACK=1` require `VOXY_STREET_GI_PRECISION_PROOF=1`.
They exercise incomplete block edges and the complete original fallback with
positive hits; keep both unset for performance measurements.

### Exact ray stage profiling and local scheduling

With `VOXY_STREET_GPU_TIMING=1` and `VOXY_STREET_RAY_STAGES=1`, the existing timing owner records projection, traversal and resolve intervals for every chunk. Query slots are unique and bounded by the API limit. Use `tools/scene-research/summarize_ray_stage_profile.py LOG --report REPORT` to exclude probe/warmup counters and summarize only measured-cycle samples. Intervals may overlap; do not sum them as exclusive frame cost.

`VOXY_STREET_RAY_COHERENT=1` additionally sorts each 64-lane traversal workgroup by approximate scheduling keys. All original segments, ray IDs, exact traversal operations and destination hits are retained, including padded chunk edges and unsupported-ray fallbacks. It is opt-in. Native frame and raw Float32 reference fixtures passed; the completed full street cycle measured 0.985456 offscreen FPS versus 0.943819 in the preceding unsorted run. This is not a paired speedup qualification and does not meet 120 presented FPS. See `artifacts/original-street-realism-2026-10-15/ray-coherent/`.

### Exact per-ray interval caching

`VOXY_STREET_DDA_INTERVAL_CACHE=1` caches unchanged conservative begin/delta intervals per ray, instead of recomputing them at each hierarchy level. It applies to paired DDA with tile rejection, including opaque compute and independent transparent queries. It adds no global buffer or resource owner. It changes no ray, geometry, high/low component, reach, refinement or rejection tolerance.

The independent reference gate passed 132544 geometry cases; HDR frames and raw Float32 GI/SSR fixtures were bit-identical to their references. Alternating same-binary full cycles B/A/B measured 1.129857 / 0.953356 / 1.135140 completed offscreen FPS at internal 3840x2160, all effects and full collision geometry. The 18.5-19.1% observed gain is not a statistical confidence estimate or a 120 presented FPS result. Keep the experiment opt-in; artifacts are under `artifacts/original-street-realism-2026-10-15/interval-cache/`. Reports now record interval caching and local ray grouping explicitly.

### Paired division underflow correction

The strengthened numerical gate reproduced a Metal FTZ error for input bits `00800000 / 00800001`: the old paired quotient was approximately 2 instead of 0.99999988079072466. Tiny operands now receive an exact common 2^64 scale before the unchanged quotient/remainder/correction sequence. Integer normalization retains subnormal input bits; component bounds prevent scaling overflow.

The scaling helper passed all 16777216 signed subnormal/zero patterns and 1520 normal edge cases against independent F64. The actual paired quotient gate checks residuals lexicographically; its 5180 bounded cases and 4281 independent accuracy cases passed, including the original regression. Projected-depth outputs and native HDR/raw-query fixtures remained bit-identical in their tested scope. This is not a universal division proof.

Full corrected cycle measured 1.124294 mean offscreen FPS; 120 presented FPS remains unqualified. See `artifacts/original-street-realism-2026-10-15/division-underflow/`. The separate parameter-guard experiment had no observed gain and was reverted.

### Repeatable offscreen timing

`--offscreen-benchmark --benchmark 20` advances the unchanged 120 Hz physics integrator in fixed 0.25-second batches per rendered frame. Camera gait and weather use the same simulation clock. FPS uses real elapsed time, including all scheduling and rendering work. This full cycle normally measures 80 render frames and 2400 physics steps; it does not establish presented-window FPS. Reports include `measurement_start_simulation_steps`, `simulation_clock` and `replay_state_blake3`. The latter hashes exact camera/physics/weather input bits across every frame, including warmup; require equal hashes on the same pinned binary before treating paths as matching.

Offscreen comparison stops at the requested simulation duration once warmup is complete, even if a faster renderer finishes that motion cycle in less wall time. This keeps render-state transcripts and frame counts equal. Presented-window benchmarking retains its wall-and-simulation duration requirement. The stop policy is recorded in each report; neither mode substitutes simulation time for measured FPS.


### Software source traversal work diagnostics

`VOXY_STREET_WORLD_RAY_WORK_STATS=1` enables cumulative work counts for the guarded deferred nearest job (without UV output). These are diagnostic samples, excluding the first ray in each encode because its padding belongs to the exact queue. They count visited software BVH nodes and every software source-triangle certificate attempt, including failed previous-hit seeds. Isolated native traversal and exact-fallback work are excluded. GI and SSR must share the same prepared job owner.

The four-frame integration report includes `software_traversal_work`; set `VOXY_STREET_WORLD_RAY_WORK_RAW` to preserve its 48 raw bytes. Sums use explicit 64-bit carry; per-ray triangle counts saturate and the reader rejects saturation. Diagnostic fields use only reserved padding and add one reduction pass. Time measured with this option includes diagnostic overhead and is not a production FPS result. Default is off.

Validate coverage and raw counters with `tools/scene-research/audit_traversal_work.py REPORT --raw RAW --ray-kinds 2 --output PROOF`. This assumes the demo's single source owner was prepared before its full-frame encodes. Hardware candidate absence remains inconclusive and every original numeric predicate is unchanged.


### Full-resolution world-ray batch dimensions

Set `VOXY_STREET_WORLD_RAY_TILE_WIDTH` and `VOXY_STREET_WORLD_RAY_TILE_HEIGHT` before preparing the experimental world-ray frame. Defaults remain 512×256. The query owner is allocated at the matching capacity; nonzero dimensions, multiplication overflow, device/budget admission and prepare/connect capacity agreement are checked. These settings change scheduling only, preserving the complete source geometry, output/internal resolution, effects and every original ray.

On M4 Max / Metal, 1024×1024 reduced GI+SSR regions from 144 to 24 and active frame median from 2340.492 to 1197.349 ms in a same-executable three-frame comparison. It uses about 168 MiB more query/colour storage. Final PNG/PFM matched the original baseline, and a separate checked run reported zero sticky Float32 mismatches. This does not qualify a full movement cycle or 120 presented FPS. Evidence: `artifacts/original-street-realism-2026-10-15/world-ray-batching-v1/`.


The subsequent M4 Max screening pair retained **1280×1280** as a faster explicit setting: median 1036.674 ms versus a same-executable 1024×1024 control at 1153.130 ms (about 10.1% lower time). GI+SSR regions fall from 24 to 12. Input/output are 125 MiB each; two colour caches are 25 MiB each, adding 108 MiB relative to 1024×1024. The checked run reported zero sticky Float32 mismatches and PNG/PFM remained byte-identical. Device admission and nonzero/overflow checks still apply; the legacy default remains unchanged. This is not a full-cycle or 120 FPS qualification. Latest settings/evidence: `artifacts/original-street-realism-2026-10-15/world-ray-batching-v2/`.
