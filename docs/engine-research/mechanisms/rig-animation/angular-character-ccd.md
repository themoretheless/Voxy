# Character angular sweep foundation

`CharacterPhysics::fixed_step_with_rigid_motion` accepts world translation followed
by a body-local scaled-axis rotation in radians. Angular magnitude preserves
winding, including full turns with identical quaternion endpoints. Requests are
bounded to four turns per tick. The existing translation-only API remains available.

The solver first certifies separation over the whole arc using projection extrema.
Remaining obstacles use conservative advancement with SAT projection gaps and a
maximum perpendicular corner travel bound. Contact clips the angular fraction;
accepted orientation persists without resetting velocity on the following tick.
Grounding is refreshed after rotation, while upward velocity suppresses snap.
Budget exhaustion and invalid requests reject the staged tick without publishing
scene, runtime body, or input changes. The default angular budget is 256 iterations.

Tests cover collision between clear endpoints for half, full and four turns,
projection bounds against independent quaternion samples, grounded yaw, turning
away from contact, tall bodies, retained orientation and velocity, jump continuity,
and atomic failure after staged translation.

Active AngularMotion on a character owner is consumed by CharacterPhysics in the
ordinary fixed loop. Its axis retains parent-local semantics; the nonphysics batch
and legacy behavior skip character owners. A simultaneous explicit nonzero angular
request is rejected atomically. Inactive owners do not turn, live descriptors are
read every tick, and Stop restores authoring. Rotating physics ancestors and static
colliders remain rejected. Tests exercise mid-arc collision through the existing
translation-only API and ordinary editor Play/Stop.

Animation-driven root rotation remains unfinished; its [ordered trajectory
foundation](root-rotation-path.md) now preserves LINEAR/STEP/CUBICSPLINE paths.
The native oriented
character smoke validates translation collisions and Play/Stop; it does not exercise
the new angular request API.
