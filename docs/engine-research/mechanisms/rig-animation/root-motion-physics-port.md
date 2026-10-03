# Root translation extraction and character physics port

## Translation ownership

`AnimatorFrame::into_in_place_translation(skeleton, axes)` consumes an owned frame. Selected translation coordinates of the chosen motion joint return to that rig's bind translation; rotation, signed scale, unselected axes and other joints remain unchanged. The skin palette is rebuilt and admitted before the frame is returned. Extracted axes are cleared from the frame's displacement, preventing repeated consumption. The returned vector remains parent-local, with the same integration semantics as Animator root motion. It does not include animated ancestors or root rotation.

`CharacterPhysics::fixed_step_with_motion(scene, input, dt, requests)` accepts one explicit world-space displacement per active character. Input/gravity runs first. Each requested displacement then sweeps and slides through the existing convex controller against the same active static world. This is a defined sequential motion policy, not a force or a change to gravity integration. Ground snap follows the existing policy, but upward persistent velocity prevents snapping a jumping character back down. Contacts also project persistent body velocity out of obstacle interiors. The method returns the displacement accepted by this motion stage, including its ground snap, excluding preceding input/gravity motion. Requests are not stored or repeated next tick.

A request requires a live, active CharacterBody in the same scene, unique owner, finite vector and component magnitude at most 1e6. Validation and solver/publication errors preserve every character pose, runtime state and input edge. Scene positions, dynamic ancestor restrictions and collider capacities retain their existing limits. Character-character contacts remain unsupported.

## Staging across domains

Stage an Animator clone, advance exactly once per fixed tick, extract selected axes, convert the request through a valid locomotion basis, call physics, then publish the staged Animator after physics accepts. On failure retain the previous Animator for retry. Do not advance a separate render clock. The integration regression demonstrates this order with an identity locomotion/world basis: 125 ticks cross two loop boundaries, request 25 units of travel, stop at the wall after 0.9 units, and keep the skin palette in place. A duplicate-owner physics rejection preserves the animation clock for retry.

## Editor integration

Fixed-tick PlaySession application is now implemented; see [fixed-animation-play](fixed-animation-play.md). The render subsystem holds immutable accepted frames and never advances a ModelPlayback. Root motion is opt-in through ModelAnimation axis fields and requires a CharacterBody on the same model owner. The current conversion accepts bind-only ancestors and proved constant authored channels, using their authored transform; moving or unproved ancestors are rejected. Root rotation, collision feedback/foot sliding, moving platforms, skeletal presentation interpolation and continuous integration through animated ancestors remain open.

No CUDA, new native animation/physics scene or GPU throughput claim is made by these CPU tests.
