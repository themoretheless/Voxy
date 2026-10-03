# Angular root motion in ordinary Play

The inspector's **Root motion rotation** field maps to the backward-compatible
`ModelAnimation.root_motion_rotation` boolean, off by default. The existing motion
bone name/index selects the same joint for translation and rotation. A CharacterBody
must belong to that model owner.

One fixed tick stages owner-local ModelPlayback clocks and a frame plus complete
RootRotationPath from the exact same phase interval. Pause produces no angular
path. Loop crossings retain their complete arcs before the clock folds; no winding
is inferred from endpoint quaternions. Path or pose failure leaves the prior clock.

A whole-clip translation-channel proof establishes a fixed selected-bone pivot.
It is independent of the rotation channel and rejects cubic translation excursions
between equal keys. Each bone ancestor must have a compiled constant TRS proof.
For angular motion every ancestor's absolute scale components must agree. Uniform
signed scales transform angular axes using the proper pseudovector basis det(S)*S;
a negative determinant reverses handedness. Nonuniform scale cannot be represented
by a rigid body rotation and explicitly rejects this mode.

The pivot is the in-place selected-bone translation transformed through those
constant model ancestors. Translation masks are consumed first, then root rotation
returns to bind and the skin palette rebuilds. Physics checks the entire moving
center path around that pivot, including normalized cubic spans and STEP events.
A collision admits only its prefix; the clip clock still advances through the tick,
like blocked translational locomotion. Later path spans are not applied to the body.

`fixed_step_with_motion_and_trajectories` combines translation-only owners and
trajectory owners in the existing character transaction. An owner present in both
lists must have exactly matching requested translation. Duplicate/conflicting or
invalid requests fail before publication. Successful physics consumes input once;
only then does the editor replace the animation runtime candidate. Rendering receives
the accepted immutable frame Arc and does not own a second animation clock.

Tests cover ordinary App Play/Stop, wall admission with a root pivot at x=.6,
mixed translation/rotation owners, physics failure and retry without clock drift,
reflection conversion, old settings, and explicit unsupported-pivot/parent rejection.
The opt-in real-device test compares exact GPU pixels with an independently baked
bind-pose CPU mesh at the physically accepted body transform, detects double rotation,
shares one source between two owners and releases all resources on clear.

The native diagnostic uses the same inspector, Play loop, imported model, GPU owner
frames and Stop path. It requires actual presented frames:

```sh
VOXY_ROOT_ROTATION_SMOKE=1 target/release/voxy_app \
  --model crates/voxy_render/examples/assets/root-pivot-turn.glb \
  --animation-native-smoke
```

The fixture has a cubic rotation with identical endpoint orientations and a
constant pivot at x=.6. A thin character body turns into a wall .25 along z;
first contact satisfies sin(angle)+.02*cos(angle)=.23. This is a controlled
rig/character collision fixture, not an automatic avatar collider fitting proof.
An occluded window with zero presented frames does not prove native admission.

Remaining production work includes moving-pivot simultaneous translation/rotation
trajectories, core crossfade angular velocity integration, animation transitions in
the editor, contact feedback/foot locking, and hardware coverage beyond the device
on which these checks run. The original broader engine goal remains unfinished.

The 2026-10-03 foreground native run admitted the expected angle
0.21203309612481186, pivot (.6,0,0), accepted world center
(.0134369545,0,.62626874), and serial 12. It presented 20 frames in
total, used two GPU primitives sharing one source (928 bytes), then reported
zero animation bytes and exact authoring restoration after Stop. The initial
background CLI attempt was occluded and is retained as a failed diagnostic, not
counted as native proof. Logs and source hashes are in
`artifacts/rig-play-root-rotation-2026-10-03/`.
