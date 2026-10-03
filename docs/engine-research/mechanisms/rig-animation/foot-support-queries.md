# Foot support queries and immutable anchors

`SupportWorld` snapshots active affine BoxCollider geometry, authored extents and
scene identity together. It uses the same collider validation as character physics;
colliders attached to characters or beneath dynamic character parents are rejected.
Translation, rotation, signed scale and shear are supported. Mesh surfaces are not.

`probe` returns the nearest physical ray hit with a world point, unit face normal,
distance and opaque local anchor. Walkability is checked only after selecting the
nearest hit, so a steep obstacle cannot expose a lower floor through itself.
Origins strictly inside a solid do not produce support. Equal-distance colliders
use stable node identity; corner entry ties prefer the face best aligned with up.

`resolve` reconstructs an anchor from a current snapshot. Anchors follow surface
transforms, but release when their node is inactive, removed, recycled or its face
no longer contains the captured authored point. Even a one-ULP change in face
thickness releases the old face anchor. Foreign-scene anchors are rejected.
An old snapshot deliberately retains its old geometry and authored metadata.

Each scanned surface consumes one query; resolving an anchor consumes one query.
An exhausted budget returns an error, never a partial previously found contact.
Snapshot capacity and query limits are bounded. Finite extreme normal vectors are
normalized without overflowing or underflowing their squared length.

## Verification and integration boundary

Ten tests cover nearest hits and ties, slopes, reflected and sheared surfaces,
platform transforms, source snapshot consistency, dimension changes, stale IDs,
foreign scenes, invalid input, capacities, budget exhaustion and normal extremes.

This API supplies contact data. [Authored foot placement in ordinary Play](foot-placement-play.md) uses this API;
per-foot contact state and staged publication are described below. A moving anchor does not carry the character or
provide moving-platform collision dynamics. Foot IK must be staged against the
physically accepted actor transform before the physics/scene/input/frame transaction
publishes. Native foot-locking and GPU acceptance have not been performed.

## Staged character preparation

`CharacterPhysics::fixed_step_with_preparation` invokes a fallible callback after
all character motion is accepted and before scene transforms, physics state and
pending input publish. The preview contains deterministic owner-ordered character
poses: the exact renderer-facing world matrix, solver-precision center/rotation,
velocity and grounded state. Its support snapshot comes from that same tick's
collision world. Support queries receive the remaining curved-collision query
budget, capped at the support API limit, rather than a fresh independent budget.

The callback returns a candidate owned by the caller. That candidate is returned
only after the tick commits; preparation errors retain their original error type.
Physics errors skip preparation. A preparation failure preserves all character
states, scene transforms and pending input. External callback side effects are not
rolled back: callers must build local candidate state and publish the returned
value. Legacy tick APIs do not construct the preview or run preparation.

This transaction boundary now hosts the editor foot correction callback.
Smooth clip-phase stance curves are implemented; imported event tracks and
native/GPU foot acceptance remain pending.

## Per-foot plant/release authority

`FootContactState::prepare` is immutable and returns a candidate state/contact.
The sole is the animated pre-IK position in the accepted actor world frame; up is
unit world up. Grounding comes from the character preview. Plant intent comes from
authored stance/contact logic, not an inference that every low foot is in stance.

Acquisition casts down from a configurable lift and admits only contacts within
`plant_distance`. A planted foot resolves its original object-local anchor, keeping
its surface point while animation drifts. The wider `release_distance` provides
hysteresis. A missing support, excessive sole-to-anchor distance or insufficient
normal alignment releases the foot and suppresses acquisition until swing or a new
airborne landing. Airborne and swing states clear contact immediately. Resolving
an existing anchor never transfers it to a different collider.

All settings/input are validated before querying. Failed preparation leaves the
published foot state intact. Query budget consumption belongs to the staged tick.
A caller must publish candidate foot states together with successfully prepared
animation frames, using `fixed_step_with_preparation`. Tests include platform
transform following, drift, reach release, rearming, missing/tilted support,
acquisition distance, admission/budget errors, and contact rollback when a later
preparation stage rejects the whole character tick.

The editor now selects named foot chains, evaluates sole offsets and rebuilds skin
palettes in ordinary Play and applies smooth clip-phase contact weights. Per-clip
curve selection, crossfade blending, moving-platform body carry and native/GPU
foot-lock acceptance remain required for
production locomotion.
