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

This is the rotation extraction and trajectory-query foundation. It is not yet
connected to AnimatorFrame, ModelAnimation or ordinary animation-driven character
physics. Remaining work includes transactional multi-span/cubic CCD, in-place pose
and palette removal, constant-parent frame conversion, blended angular velocities
during crossfades, coupled translation/rotation, and native presented-frame proof.
The existing authored AngularMotion path uses single-axis angular CCD separately.
