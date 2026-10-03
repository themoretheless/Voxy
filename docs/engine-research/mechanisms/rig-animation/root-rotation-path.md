# Ordered root rotation trajectories

`AnimationClip::root_rotation_curve(joint)` compiles only the selected locomotion
rotation channel. A thread-safe lazy cache belongs to the immutable clip; repeated
selections, clip clones and concurrent owners share the same coefficients. The curve supports
LINEAR quaternion interpolation, right-continuous STEP events, normalized
CUBICSPLINE polynomials, held key ranges and Loop/Clamp playback. The interpolation
semantics follow [glTF Appendix C](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#appendix-c-animation-sampler-interpolation-modes).

`curve.path(start, end, max_spans)` returns the ordered trajectory, not just a
quaternion between its endpoints. A complete turn with equal endpoint orientations
still contains every key arc. Noncommuting axes remain ordered. A loop composes
its cycle transform without adding a seam reset. STEP events belong to
`(start,end]`; they have zero duration, a separate shortest event arc and no finite
angular speed. Cubic spans retain the polynomial even when their endpoints agree.

Let `U(t)` be the unwrapped orientation relative to the first authored rotation.
An interval returns `U(start)^-1 * U(t)`. Adjacent intervals therefore compose on
the right, in time order. For each linear/STEP span,
`span_start * exp(body_angular_displacement) == span_end`. Do not sum the axes or
use `path.end_rotation()` alone for collision admission. No initial authored
rotation offset is emitted as locomotion.

Cubic Hermite data is converted to Bezier controls. The control hull supplies a
lower quaternion norm and an upper derivative norm over the entire interval.
Subdividing continues until a positive norm and at most pi/2 radians of bounded
angular travel are established. `angular_speed_bound()` bounds speed per clip
second; `angular_velocity()` evaluates the analytic velocity in the interval-start
frame. A zero or unproved quaternion cannot produce an accepted prefix.

`projection_bounds(vector, normal)` encloses the rotated point projection over the
whole span. Arcs use trigonometric extrema; cubic rotation uses a degree-six
rational Bezier hull with numerical guards. An inconclusive denominator hull
returns the enclosing sphere bound. This preserves a tight grounded-yaw bound and
provides the data needed for conservative curved collision advancement.

Compilation admits at most 65,536 keys in total across all selected channels of
one clip; selecting another bone cannot multiply that cache budget. A request admits 1..=4096 spans,
with a bounded subdivision depth. Failure discards the whole path. Loop indices
outside the exact integer range of f64 are rejected. Constant channels can span
large elapsed times without walking their cycles. Small increments after a large
clock and near-singular, nonzero cubic curves are covered by tests.

`point_speed_bound(vector)` bounds the speed of each body vertex over a whole
span. Cubic spans use the degree-five numerator of the normalized quaternion
angular velocity and the proved lower norm. This avoids using a tall body's
full sphere radius as its speed when it turns around the vertical axis.

`CharacterPhysics::fixed_step_with_trajectory_motion` consumes immutable ordered
paths after translation in one staged fixed tick. Each request supplies a
body-local basis and a body-local pivot offset. For a pivot `p`, initial body
orientation `W` and accepted path delta `D(u)`, the center follows
`c(u) = c(0) + W * (p - D(u) * p)`. Every corner is therefore a rotated
`corner - p` around the stationary world anchor. Point-speed bounds and whole-span
projection hulls use these shifted corners, and SAT queries use the moving center.
Per-span projection hulls can certify an obstacle as separated;
remaining candidates use SAT conservative advancement on the actual curve. The
iteration budget is shared across spans and the query budget across bodies. A
failure preserves all scene poses, velocities and input edges. Normal grounding
refresh can subsequently adjust the accepted center; the receipt includes that
adjustment. A collision returns
the accepted span, fraction and rotation, stops before later spans, and retains the
accepted orientation on subsequent ticks. Explicit paths and authored AngularMotion
cannot both write one character's rotation.

`AnimatorFrame::without_root_rotation` restores only the selected joint's bind
rotation and rebuilds its skin palette. It checks the exact rig binding and all
pose transforms. Translation, signed scale, other joints and translation motion
remain intact. The caller owns extraction and physics admission separately.

`Animator::advance_with_root_rotation` samples the displayed frame and selected
rotation path from the same bounded phase interval, retaining complete loop winding
before folding the clock. Pose, path and clock publish together only when all
admission succeeds. Active crossfades currently return an explicit unsupported
error; ordinary Animator pose/translation crossfades remain available.

`ModelAnimation.root_motion_rotation` opts a model owner into this path. Existing
scene documents default it off. AnimationRuntime stages the selected path,
constant-parent basis and pivot, removes the selected bone's displayed rotation,
and forwards the immutable frame to the existing renderer. The editor admits
translation-only and trajectory owners in one character transaction, then publishes
the candidate animation clocks. Rejected physics preserves the earlier frame Arc,
clock serial, body poses, velocity and input. Play/Stop restores the authoring
scene normally. See [ordinary Play admission](root-rotation-play.md).

A skeletal root whose translation changes the pivot uses the composed
RootRigidPath in ordinary Play. Translation and rotation are sampled together by
physics; the legacy RootRotationPath remains available for explicit angular paths.
Constant parent TRS proofs admit uniform signed scale and reflection. Nonuniform
scale, moving or unproved ancestors and active angular crossfades remain unsupported.

GPU admission is covered by an opt-in real-device editor test:
`curved_root_rotation_collision_renders_once_with_in_place_gpu_palette`. It
submits frames produced by the actual AnimationRuntime to AnimatedModels after
curved physics collision, compares exact rendered pixels with an independently baked bind-pose
mesh at the accepted body transform, and checks that retaining authored bone
rotation gives different pixels. Two owners share one source, repeated publication
does not grow allocation, and clearing releases all owner resources. This is an
offscreen fixed-tick runtime/renderer/physics proof; native presented-frame
admission is exercised separately by the ordinary Play diagnostic.

The composed translation/rotation physics entry point and its coordinate mapping
are described in [root-rigid-physics.md](root-rigid-physics.md). Ordinary Play now
uses that path for both fixed and moving root pivots.
