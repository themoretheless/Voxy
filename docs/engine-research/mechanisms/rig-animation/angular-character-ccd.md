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

This is a physics API foundation. Animation root rotation extraction and routing
authored AngularMotion on character owners remain unfinished. The native oriented
character smoke validates translation collisions and Play/Stop; it does not exercise
the new angular request API.
