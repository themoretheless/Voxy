# Full imported contact CPU qualification and frame diagnosis

The release CPU regression `wide_imported_contact_completes_full_clip` passed all 480 steps of the two-second CesiumMan clip at 240 Hz, including the final cumulative energy receipt assertions. The test exited 0 and took 1221.58 seconds while a separate GPU render ran. All 40 progress reports are present. The previous step-131 failure is passed. Physical thresholds, CCD, quadrature/subdivision limits and force laws were not relaxed. This qualifies this fixture only; it does not prove real-time performance or general character-contact robustness.

138 focused physics tests passed (93 library, 8 static L-BFGS, 29 prescribed contact, 8 viscoelastic; 3 manual library tests ignored). `implicit-qualified-source.rs` is the exact solver source for this CPU result. The diagnostic tracing compares local and canonical world contact frames on nonlinear exhaustion without affecting the solve or error.

`full-clip-pass.log` is terminal CPU evidence. Earlier progress files are historical partial snapshots. A separate native GPU render is still live in exec session 12276, log `/tmp/voxy-contact-frame-gpu-render.log`; poll that same handle. It uses Metal on Apple M4 Max. Existing frames demonstrate only their saved prefix; full GPU completion remains unconfirmed.

The old `maximum_drift_roundoff_m` trace field measures the full kinematic equality residual including nonlinear solver error. The current source renames it `maximum_kinematic_residual_m`; this diagnostic-only name change follows the qualified binary. Frame-force comparisons must be read alongside same-step residuals, not treated as proof of a causal rounding defect.

The renderer displays an imported rig and separate coarse volumetric specimens. Integrated anatomical skin, calibrated tissue parameters, real-time physics, other assets/platforms, and the full engine parity objective remain unfinished.

Reproduce the CPU gate:

```sh
VOXY_CONTACT_REJECTION_TRACE=1 cargo test --release -p voxy_app --example body_motion_snapshot collision_tests::wide_imported_contact_completes_full_clip -- --ignored --exact --nocapture
```

GPU completed: exit 0, 41 frames (0..2 seconds at 20 fps) and `secondary-motion.csv`; `gpu-full-pass.log` records Metal on Apple M4 Max and 1357.988445542 seconds for simulation plus rendering. `imported-full-motion.gif` assembles all frames. This is full visual evidence for the prior qualified binary, not a performance result for the later optimizations. Historical paragraphs describing a live render are superseded by this terminal result.
