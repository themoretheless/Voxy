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
