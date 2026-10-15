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


Current verified showcase (2026-10-15): set `VOXY_STREET_SKY=hdri` for the prepared
CC0 panorama and image-based lighting. The procedural default is retained.
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
