# Double precision authored rig and imported contact

The existing rig/clip source now exposes Pose64, f64 STEP/LINEAR/CUBICSPLINE sampling, hierarchy evaluation and skin palettes. ModelSurface64 uses the original imported vertices, UNORM16 weights and indices. No second parser or animation clock is introduced.

The imported contact snapshot provider supplies both relative support palettes and collision geometry in f64 when explicitly selected with `--cesium --contact --wide-contact`, through the existing atomic tissue stepping implementation. Default contact retains legacy sampling. Legacy Mat4 callers convert at its boundary. Rendering remains f32. Authoring masks, contact floors and energy admission are unchanged.

Verified: 224 animation tests; 19 tissue tests plus the sub-render-scale support test; 19 model tests; editor 168 passed, 12 ignored. Model and coupled-contact checks are recorded separately when terminal. The f64 analytic hierarchy test detects a 1 ns phase difference, and imported geometry retains motion where f32 output freezes. This does not establish real-time performance or physiological calibration.

An initial experiment rounding shared f64 interpolation into the legacy render path regressed the existing imported contact test at step 55. Legacy linear interpolation and quaternion SLERP were restored; one intermediate compatibility run passed all 68 steps (464.77 s). A separate subtraction-based interpolation experiment rejected step 65 (206.89 s); it was reverted to the tested `Vec3::lerp` compatibility formula. Both logs are retained. The full 480-step contact clip is not qualified. The wide coupled provider passed its initial eight steps but rejected step 65 (nonlinear nonconvergence, 298.15 s). The explicit 68-step wide qualification test is ignored by default with this known reason; it remains runnable and is not counted as passed.


Current-source neutral visual proof: `motion.gif`, 41 frames over the full 2 s
Cesium clip, 480 physical steps; Apple M4 Max/Metal rendering, CPU physics.
The four blue regions are illustrative FEM pads, not anatomical skin or a
physiologically calibrated body. Contact is disabled in this visual proof.
The exact snapshot/render duration is recorded in `render.log`; this is not real-time qualification.

The rendered trace records maximum secondary displacement relative to the
bone-driven reference of 3.766, 3.764, 8.932 and 7.300 mm across the four regions
at the 41 sampled frames. These observed values describe this illustrative
fixture only. Full trace and final energy receipts are in
`frames/secondary-motion.csv` and `visual-motion-metrics.json`.
