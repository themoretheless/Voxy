# Composed root trajectories and character collision admission

`RootRigidCurve` preserves simultaneous translation and rotation, including
cubic excursions with equal endpoints, ordered loop transforms and merged STEP
bridges. `CharacterPhysics::fixed_step_with_rigid_trajectories` admits these paths
through the existing fixed-step transaction and conservative advancement kernel.
It does not apply the legacy additive translation delta a second time.

## Coordinates

A request maps source coordinates into initial body-local coordinates through
`A(x) = signed_scale * basis * x + origin`. For the authored relative transform `M(u)`, physics
uses `A M(u) A^-1`. If the initial body orientation is `W`, its center is
`c0 + W * (signed_scale * basis * M.translation + origin - delta_rotation * origin)` and its
orientation is `W * delta_rotation`, where
`delta_rotation = basis * M.rotation * basis^-1`.

Both values are sampled at the same curve fraction. The translated source frame
must be included in the conjugation; conjugating only the quaternion gives an
incorrect center trajectory. Uniform signed scales are supported, with reflections represented by a negative
scalar and a proper rotation basis. Nonuniform or animated model ancestors still
require a further coordinate adapter.

## Continuous collision admission

Each body corner is mapped into the source frame. Certified span projection
bounds enclose that corner's entire trajectory along each candidate separating
axis. These bounds can certify continued separation from a touching floor while
a moving pivot turns a tall body. Remaining obstacles use the existing SAT
conservative advancement kernel, with a bound on full corner velocity over the
normalized span. Translation and rotation are never swept as separate stages.

Iteration counts are shared across spans. Query counts are shared across all
bodies in the tick. Duplicate owners, conflicting authored angular motion,
invalid frames and exhausted budgets fail before scene, input or cached body
state is published. The admission receipt identifies the completed spans and
fraction of the first blocked span; a STEP event can be blocked at a time
fraction of one.

## Integration boundary

The physics entry point is connected to ordinary editor Play through ModelPlayback
and AnimationRuntime. Translation masks and constant similarity-frame ancestors
are admitted together with the in-place pose; see [root-rotation-play.md](root-rotation-play.md). Angular crossfades and collision feedback into foot placement
also remain open.

Regression cases are in `crates/voxy_gameplay/tests/integration.rs`:
closed moving-pivot curves with independent analytic first contact, atomic budget
failure, source-frame conjugation, and grounded tall-body yaw.
