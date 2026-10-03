# Rig-bound two-bone IK

`TwoBoneChain::new` compiles one directly connected root/middle/tip chain against
an exact skeleton layout. It shares the immutable rig, stores no playback clock,
and rejects foreign layouts even when joint counts agree.

`Pose::solve_two_bone` returns a candidate pose and a reach report. Targets and pole
positions are in model coordinates. The analytical triangle solve clamps targets
to the geometric reach interval without stretching either link. The pole selects
the knee side; a degenerate pole uses the current knee direction, then a deterministic
perpendicular axis. Coincident targets and antiparallel rotations remain defined.
Triangle arithmetic is scaled before squaring link lengths.

All local translations and scales remain unchanged. Only root, middle and optional
tip rotations change; unrelated local rotations remain unchanged. Local-rotation
weighting in [0,1] is explicitly not linear interpolation of the endpoint. Zero
weight returns the original pose. A requested tip orientation denotes the proper
global rotation of the signed-scale decomposition, including a reflected tip.

Ancestors through the middle joint require uniform absolute scales. Signed uniform
scale is supported by the same proper pseudovector basis used for root motion.
The tip itself may retain signed nonuniform scale. Nonuniform ancestors require a
more general constrained solve and currently produce `UnsupportedIkScale`.
Collapsed links, invalid targets, unsupported scales, foreign rigs and overflowing
matrices fail before any source pose changes.

`AnimatorFrame::with_two_bone_ik` rebuilds skin matrices from the solved pose and
preserves extracted root motion, selected motion joint and transition weight. It
fits the existing immutable-frame publication path; it neither advances a clock
nor writes actor transforms.

## Verification and integration boundary

Tests use independently evaluated f32 skin matrices to check reachable endpoints,
link lengths, pole side, proper tip orientation, uniform reflection and signed
nonuniform tip scale. They also cover both unreachable reach limits, complete
folding at a coincident target, degenerate poles, antiparallel targets, blend weights,
metadata/palette preservation, foreign rigs and failed admission.

This is the IK kernel. Automatic foot placement and foot locking in ordinary Play
are not implemented yet. [Support queries](foot-support-queries.md) now provide contact normals and stable
local anchors. Authored chain selection, plant/release state and integration are
still required. The
world contact anchor must be converted through the physically accepted actor
transform. Corrections must be staged before the existing physics/scene/input/frame
transaction publishes; applying a fallible correction after physics publication
would violate tick atomicity. Angular crossfades, pelvis adjustments and locomotion
contact feedback remain part of the broader production animation work.
