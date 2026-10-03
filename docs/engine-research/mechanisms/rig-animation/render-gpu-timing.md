# Rig compute and render GPU intervals

`SceneRenderer::encode_profiled` attaches boundary timestamps to the ordinary
single-sample scene pass. It rejects missing TIMESTAMP_QUERY before encoding.
The original encode and MSAA paths continue using no timestamps or completion
waits. The caller owns query resolution and must satisfy wgpu query validation.

The upstream RiggedFigure regression now draws each of 65 poses with the editor
Lambert shader into a 128x128 color/depth target. Each image must contain more
than 100 colored pixels. Position and normal CPU/GPU comparisons remain active.
The midpoint image was read back and visually inspected: the complete articulated
figure is visible. This fixture has 370 vertices, 768 indices, 22 palette nodes,
one draw and no overlays or shadow/composition passes.

An initial Metal run resolving counters in the drawing command buffer returned
render_begin nonzero and render_end zero. Its strict timestamp assertion failed;
this is retained as evidence. Resolving after GPU completion in a second command
buffer produced valid monotonic intervals on all 65 samples. This serialization
is confined to diagnostics and adds CPU waiting/readback overhead. Profiling
callers should check actual written samples, not infer support from feature flags.
No timing is calculated from an unwritten or backwards interval.

Measured GPU medians/p95 (microseconds): compute 18.959/19.667, scene pass
56.416/58.667, interval from compute beginning to render end 80.291/83.875.
The combined interval is measured directly; it is not a sum of percentiles.
Samples include the initial pose. Host code is debug; its reported preflight
interval includes camera fitting and transform upload. These CPU intervals are
not comparable with previous pure-preflight diagnostics.

This is the GPU compute-plus-draw portion of a small offscreen frame, not the
complete native editor frame, throughput, FPS or presentation latency. Other
hardware, complex rigs, LOD appearance and CUDA remain separate acceptance gates.

A related upstream report describes zero Metal counters on macOS 26:
https://github.com/gfx-rs/wgpu/issues/9414 . Our compute samples were valid and
separate resolution restored render samples; a common root cause is unproven.
Evidence, failed and successful logs, hashes and image:
`artifacts/rig-render-gpu-2026-10-03/`.
