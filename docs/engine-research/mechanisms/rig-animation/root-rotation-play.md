# Composed root motion in ordinary Play

The inspector's **Root motion rotation** field maps to the backward-compatible
`ModelAnimation.root_motion_rotation` boolean, off by default. The selected motion
bone name/index selects both translation and rotation. A CharacterBody must belong
to that model owner.

One fixed tick stages an owner-local ModelPlayback clock, an AnimatorFrame and a
complete RootRigidPath over the same interval. Translation masks are compiled into
that trajectory. Cubic excursions, merged STEP bridges and ordered loop transforms
remain intact. Pause produces no movement. Path or pose failure preserves the clock.

Constant model ancestors require compiled whole-clip TRS proofs. Every ancestor's
absolute scale components must agree for angular extraction. Their product defines
one similarity frame A(x)=signed_scale*basis*x+origin. The basis is a proper rotation;
reflections use the pseudovector conversion det(S)*S and a negative scalar. Physics
conjugates the complete source transform as A M A^-1, including the translated
origin. Nonuniform scale and moving or unproved ancestors reject before publication.

The selected root translation can move during a turn. Selected translation axes
return to bind in the displayed pose, unselected axes retain their authored value,
and extracted rotation returns to bind before rebuilding the skin palette.
Physics samples translation and rotation together and admits only the collision-safe
prefix. It never adds the legacy translation delta for this owner. The clip clock
continues through a blocked tick, as in blocked translational locomotion; foot
placement feedback is not implemented yet.

`fixed_step_with_motion_and_rigid_trajectories` combines translation-only owners
and composed owners in the existing atomic character transaction. An owner cannot
appear in both lists. Duplicate or conflicting requests, invalid source frames and
exhausted budgets fail before scene, input, body cache or animation runtime publication.
Only successful physics consumes input and publishes the candidate animation runtime.
Rendering receives immutable accepted frames and owns no second playback clock.

Tests cover ordinary App Play/Stop for fixed and moving pivots, exact source-frame
conjugation with reflection and uniform scale, mixed owner types, physics failure
and retry without clock drift, old settings and unsupported ancestor rejection.
Opt-in GPU tests compare exact rendered pixels against an independently baked
bind-pose mesh at the accepted actor transform, detect retained authored motion,
share one source between two owners and release allocations on clear.

The native moving-root diagnostic uses ordinary inspector edits, Play, presented
GPU frames and Stop:

```sh
VOXY_COMPOSED_ROOT_SMOKE=1 target/release/voxy_app \
  --model crates/voxy_render/examples/assets/root-moving-turn.glb \
  --animation-native-smoke
```

This fixture has p(t)=(.6,0,2t(1-t)) and
q(t)=normalize(0,8t(1-t),0,1), with equal endpoint poses. All translation axes are
extracted. A thin body hits a wall .25 along z before the end of the first fixed
tick. The independent contact equation is
2t(1-t)+sin(angle)+.02*cos(angle)=.23, angle=2atan(8t(1-t)).
The fixed-pivot diagnostic remains available with `VOXY_ROOT_ROTATION_SMOKE=1`
and `root-pivot-turn.glb`; its historical evidence is in
`artifacts/rig-play-root-rotation-2026-10-03/`.

These fixtures prove controlled rig/character collision admission, not automatic
avatar collider fitting. Zero presented frames from an occluded window do not
prove native acceptance. Angular velocity crossfades, editor transitions, contact
feedback and foot locking, animated/nonuniform ancestor support and hardware
coverage beyond the tested device remain open. The broader engine goal is unfinished.
