# Bone-driven neutral tissue specimens

This extends the existing `body_motion` example, retaining its nine-joint shared
animation skeleton and four volumetric tissue samples. Bind-space pin positions
are captured from each actual simulation sample when the demo is built.

At each 240 Hz step, the same skin matrices used to render the skeleton map fixed
pins to world space. Chest samples follow the torso joint; rear samples follow
the left and right hip joints. Torso yaw is added to the walking cycle. Free
nodes are advanced by the tissue solver, not transformed as rigid geometry.
Embedded surfaces render the solved world positions without a second bone
transform.

All four bodies are staged together. A failed pin target, missing bone, or solver
step preserves the previous cohort. Animation time is published only after the
body step succeeds. The subsequent frame-level staging also preserves all four
bodies, animation time and the step accumulator if any substep fails; see
`../tissue-frame-atomic-2026-10-05/report.json` for the current qualification.

The regression comparison keeps root translation, gravity and solver settings
identical while removing bone rotations in the control. Each of the four free
center nodes must differ by more than 1 mm after two seconds. Pin positions must
match their assigned matrices at every step. Other checks cover invalid final
bone data and a truncated palette, preservation of all body states and successful
recovery, joint pivots, continuity, stable geometry and settling.

Reproduce:

```sh
cargo test -p voxy_app --lib tissue_demo::tests
cargo run -p voxy_app --example body_motion_snapshot -- artifacts/bone-driven-tissues-2026-10-05/poses.png artifacts/bone-driven-tissues-2026-10-05/frames
cargo run -p voxy_app --example body_motion
```

The image strip uses 0.0, 0.7, 4.5 and 9.0 seconds. Animation frames are real GPU
readback at 20 fps, including the twelve-second endpoint. Native-screen visibility
is unverified because the Mac was locked. Actual GPU coverage is Metal on Apple
M4 Max; this does not establish other backend or hardware support.

Geometry and tissue cages remain coarse. Material parameters are illustrative,
not calibrated physiology. Limbs use one rigid joint influence per segment;
this demo does not establish a general production skinned-character pipeline.
Changes are local and unpushed.
