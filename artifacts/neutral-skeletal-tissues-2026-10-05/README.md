# Neutral skeletal animation and soft tissue demo

The existing `body_motion` demo now uses `voxy_animation::Skeleton`, bind poses,
validated local joint rotations, and skin matrices for nine joints. Each limb
segment has one rigid joint influence. Shoulder/elbow and hip/knee hierarchies
retain shared pivots. Both mannequins have opaque blue suits.

The twelve-second cycle contains walking, jumping, then rest. Joint rotation
and root lift fade smoothly at stage boundaries. Four existing volumetric tissue
samples run at 240 Hz and their embedded boundary surfaces follow the solved
positions. Tissue inertia is driven by root translation; joint rotation does not
yet drive local tissue attachments. The tissue cages and mannequin geometry are
coarse technical specimens, not a calibrated anatomical character or a general
skinned asset pipeline.

Run the native demo:

```sh
cargo run -p voxy_app --example body_motion
```

Reproduce GPU frames (20 fps, 12 seconds, including the final endpoint):

```sh
cargo run -p voxy_app --example body_motion_snapshot -- artifacts/neutral-skeletal-tissues-2026-10-05/poses.png artifacts/neutral-skeletal-tissues-2026-10-05/frames
```

The strip shows 0.0, 0.7, 4.5 and 9.0 seconds. The offscreen example uses the same
`TissueDemo` source as the native example, real GPU rendering and readback.
The Mac was locked, preventing native-window inspection; these images are
GPU readback evidence, not a native-screen capture.

Verification:

- Three targeted app tests passed: hierarchical pivots and limb movement;
  continuous poses at 4, 8 and 12 seconds; four tissue responses and settling;
  and the existing ten-second general tissue regression.
- Tests also check stable vertex counts; generated meshes validate finite geometry.
- GPU validation and changing pixels are checked by the snapshot example.
- Actual adapter: Apple M4 Max, Metal. Other hardware was not tested here.

Local changes; no commit or push was performed for this refinement.
