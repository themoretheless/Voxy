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

This API supplies contact data. Automatic foot planting, release state and ordinary
Play integration remain pending. A moving anchor does not carry the character or
provide moving-platform collision dynamics. Foot IK must be staged against the
physically accepted actor transform before the physics/scene/input/frame transaction
publishes. Native foot-locking and GPU acceptance have not been performed.
