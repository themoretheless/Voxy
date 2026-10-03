# Authored foot IK in ordinary Play

The scene component `editor.foot-placement.v1` stores `ModelFootPlacement`: up to
eight named root/middle/tip chains with tip-local sole offset, sole plane normal,
model-space pole, explicit plant intent, blend weight and contact settings. Empty
feet disable correction. Names must resolve uniquely against the imported rig.
Chains are directly connected and may share ancestors above their independent
roots; chains that affect each other's joints/ancestors are rejected. CharacterBody
must belong to the logical model owner. Live collider IDs are never serialized.

AnimationRuntime binds chains to its immutable model and retains per-owner foot
contact state. Asset replacement, clip changes or binding changes rebuild those
contacts. Each fixed tick starts from the newly sampled animation pose, not the
previous tick's corrected pose. Imported hierarchy parts do not own foot clocks.

Ordinary App Play invokes `fixed_step_with_preparation` when foot bindings exist.
The correction uses the accepted renderer-facing actor matrix and grounded state,
and the same immutable static-collider snapshot as physics. The sole is evaluated
from the complete joint hierarchy. Contact positions are converted back to model
space; normals use the corresponding covector transform. A minimum-arc rotation
aligns the sole plane normal, preserves authored foot twist as far as that alignment
allows, and rotates the sole offset before solving the ankle target. Signed scales
use the same proper pseudovector basis as the analytical IK kernel. Uniform signed
ancestors and a signed nonuniform tip are supported; nonuniform ancestors remain
unsupported by the IK solver.

IK rebuilds skin palettes and preserves root-motion metadata. An unreachable full
IK target releases that foot until swing/landing rather than publishing a clamped
pose with a falsely planted contact. Preparation errors preserve physics state,
scene transforms, pending input, animation clock/frame and foot contacts together.
Stop clears runtime state and restores authored settings. Legacy owners without
foot bindings retain the existing tick path.

## Current scope and acceptance

Explicit stance and smooth normalized clip contact curves are implemented.
Automatic gait classification, imported contact event tracks, pelvis adjustment
and transported platform foot twist remain pending. Platform
anchors follow transforms; this does not implement dynamic platform body carry.

Tests independently rebuild joint globals from local transforms, verify the sole
world point and plane normal, and preserve joint translations/scales and motion
metadata. They cover moving actors with a retained contact, slope normals with
reflected/nonuniform tip scale, invalid/dependent bindings, shared-budget rollback,
and ordinary App Play/Stop. Foot-specific GPU pixel/reference acceptance now passes on Apple M4 Max/Metal.
Native presented-frame acceptance remains open; this does not prove hardware-wide
support.

## Clip contact curves and event intervals

`plant` enables stance. Optional `contact_curve` keys use normalized target-clip
phase and weight in [0,1]; an empty curve preserves fixed stance. Nonempty curves
require at least two strictly ordered finite keys spanning phases 0 and 1, at most
4096 keys per foot, and an active clip. Smoothstep interpolation gives zero slope
at authored keys; its result multiplies the binding weight. Zero weight releases
the foot. Authors should match weights at the loop seam for continuous blending.

Phase and traversed interval come from the existing animator's bounded f64 clock,
including speed, pause, loop and clamp playback. The interval detects zero-weight
swing keys crossed between ticks, across loop seams and even multiple complete
loops without enumerating cycles. Crossing swing rearms contact before acquiring
the final tick's support; an old anchor cannot survive an entirely skipped swing.
Failed publication retains clock, interval and foot state. Curve settings are
shared immutably across candidate runtime clones.

This is an authored normalized curve for the selected clip. Per-clip curve
selection and crossfade contact blending remain pending. Tests cover smooth values,
admission, speed/pause, loop/clamp endpoints, skipped swing keys and failed
preparation with a successful retry at the newly acquired sole point.

## GPU acceptance and current native gate

`foot-contact.glb` binds every visible vertex to the foot and includes explicit
inverse bind matrices. The hardware test compares its corrected GPU image with an
independent three-vertex world-space reference at locked x=.03, without deriving
that reference from the solved palette. The uncorrected owner produces a different
image. Two owners share one source; 1184 retained bytes clear to zero, with no
validation error. This is a flat full-weight stance check on Apple M4 Max/Metal.
GPU slope/reflection/contact-curve coverage is still required.

`VOXY_FOOT_CONTACT_SMOKE=1` exercises ordinary native Play/Stop; optional
`VOXY_FOOT_REVIEW_SMOKE=1` holds accepted Play briefly for visual inspection. Current
native attempts did not present any frames: Metal returned `SkippedOccluded` even
while the window was visible and key. The diagnostic occlusion state was 8192,
without the visible flag used by the existing backend guard. CUA raising/clicking
focused the window but its contents remained blank. This native gate is not
accepted; the backend guard was not bypassed to manufacture a passing result.
The initial Documents asset-directory wait was avoided with a temporary asset copy.
