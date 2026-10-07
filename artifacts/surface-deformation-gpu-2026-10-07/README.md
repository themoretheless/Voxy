GPU surface deformation and authored-normal transport

Actual prepared body: 60866 vertices, 636 shell controls, 182598 sparse influences. Production SkinEmbedding is the independent CPU displacement reference. The existing voxy_app surface_normals implementation is the independent CPU normal reference. Tested nonzero synthetic control displacements and return to rest, static UV/color preservation, missing initialization, malformed CSR/control indices, NaN controls and finite overflow rejection.

Maximum position error: 3.10e-8 m; maximum normal-vector error: 1.20e-4. Draws use the same GPU vertex and normal allocations that compute writes. The owner uses managed memory and exposes immutable geometry; no CPU mesh rebuild/readback is required by this path. Fixed topology only; colors/material attributes are unchanged.

120 stage samples after 10 warmup updates: mean 2.7325 ms, p95 6.7617 ms, maximum 25.1878 ms. Includes control upload, CPU encoding, GPU submission/work and blocking wait. Previous sample had mean 1.9633 ms, p95 4.6900 ms, maximum 12.2313 ms; variability is retained. These are stage timings, not presented FPS. Simulation, lighting, scene draw and presentation are excluded. GPU stage does not guarantee every-frame 8.3333 ms.

Native body demo remains on CPU. Required integration: replace FemaleDemo per-frame mesh preparation with resident controls; transfer facial/feature geometry and dynamic material inputs; generate irradiance and deformed-geometry diffusion coefficients on GPU; retain the diffusion solution between frames with parity/residual admission; encode compute before drawing in the existing SceneApp/Renderer entrypoint; measure full presented-frame cadence and CPU/GPU costs under the shown settings. Do not switch the default demo until feature and full-frame gates pass.

Reproduce: cargo run -p voxy_render --example surface_deformation -- /absolute/output/result.json. Build and existing female_motion check passed; result metadata explicitly keeps target_120fps_achieved=false.
