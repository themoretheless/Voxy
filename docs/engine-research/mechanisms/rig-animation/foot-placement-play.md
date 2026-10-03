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

### Native window visibility follow-up (2026-10-03)

The active-Space diagnostic and a fullscreen transition both retained occlusion state 8192 and presented zero frames; both acceptance processes exited 1. Focus, active-Space membership and fullscreen alone did not satisfy the native gate. The Metal occlusion guard remains intact. A separate default launch visibly rendered geometry, but it does not prove foot-contact Play/Stop acceptance. See `artifacts/rig-foot-native-space-2026-10-03/report.json`.

### Authoring admission

Foot settings now use the same numeric and contact-curve validation before authoring history commit and runtime binding. Loaded rig revisions also validate named chains before commit; asynchronous imports are checked again at runtime admission. Empty bindings remain disabled. Regression coverage rejects invalid weight, sole axis, support distances and missing bone names while preserving the scene and history, then verifies ordinary Play/Stop. All 113 editor CPU tests passed; this does not close the native presentation gate.

### Named clip contact curves

Each foot can optionally serialize `clip_contact_curves`, a map from imported clip name to normalized contact keys. When the map is empty, the existing `contact_curve` remains authoritative. With named mappings, missing active clips or unmapped selected clips fail candidate preparation rather than silently applying fixed stance. Rig binding requires each mapped name to resolve uniquely. The mapping is bounded to 64 clips per foot and 4096 ordered keys in total per foot, including the common and all named curves. Selected-clip phase and crossed-swing intervals use the same transactional playback clock. Clip changes rebuild the foot runtime, avoiding old anchors across clips.

Verification: 114 editor CPU tests passed, including serialized named mappings and the accepted-character-tick skipped-swing/failed-tick rollback regression using a named curve. This does not prove blended contacts across animation transitions or native window presentation; both remain open.

When the imported rig is available, authoring validation also requires a contact mapping for the currently selected clip before committing history. An ordinary inspector regression rejects switching from walk to jump without a jump curve, preserves scene/history, and accepts the same switch after adding the curve. Positive two-clip rig binding and duplicate-name rejection are covered. Final editor CPU suite: 115 passed; 10 GPU tests ignored.

Aggregate contact-key budget regression passed at 4096 keys and rejected 4097 keys; common and named curves share the budget. Final editor CPU suite: 116 passed, 10 GPU tests ignored.

### Pose blend phase foundation

`Animator::pose_blend_phases` exposes the target clip and phase, target blend weight, and either a live source clip/phase or explicit `FrozenPose` for interrupted fades. It borrows existing state and does not allocate. These clocks are the same clocks used to sample the pose. A frozen blended pose cannot be assigned a single clip phase; contact integration must preserve its own displayed contact snapshot at interruption. This API is verified by 66 animation tests and an editor build, but source/target contact blending is not yet integrated.

### Editor pose transition admission

`ModelAnimation.transition_seconds` defaults to zero (immediate clip switch), and accepts finite values from 0 to 60. Nonzero transitions between playing clips retain the current Animator, so live sources keep advancing and interrupted transitions begin from the displayed pose snapshot. Switches to/from bind pose remain immediate. Invalid durations are rejected at authoring and fixed-tick admission. Models with active foot bindings explicitly reject fades until contact blending is integrated, including attaching feet during an existing fade; candidate failure preserves the accepted frame. Existing angular root trajectory admission also rejects active fades. CPU editor suite: 117 passed; 10 GPU tests ignored. Native presentation remains open.

### Contact blend state foundation

Foot candidates now retain accepted scalar contact weights and a bounded frozen-source snapshot identity. Live sources sample their own clip curves and normalized clocks, using the same Animator blend weight as the pose. Interrupted fades capture the last accepted contact weight once per pose snapshot and retain it across subsequent fade samples; repeated interruptions recapture the displayed contact value. Candidate cloning keeps unpublished weights isolated. Tests cover live weights, repeated interruptions, missing accepted snapshots and staged-state isolation: 118 editor CPU tests and 66 animation tests passed. The active-foot transition admission guard remains in place until mixed-curve crossed-swing events and physical anchor/reach behavior are verified.

### Source contact phase interval

`Animator::source_phase_interval(dt)` predicts live source phase travel and its active fraction of the fixed tick using the source clock and fade remainder. The interval stops when the fade completes, while target phase travel includes the tick tail. Looping intervals retain an unwrapped end so skipped contact events can be detected. Zero speed produces a stationary phase interval; zero dt has zero active fraction. Frozen interrupted poses and completed fades expose no source clip interval. The returned source Arc is borrowed without allocation. All 67 animation tests passed. Contact-event integration must capture this interval before advance and publish it transactionally; mixed-curve zero-window handling and foot transition admission remain open.

### Transactional source interval retention

ModelPlayback captures the live source clip Arc, phase interval and active tick fraction before advancing. It publishes them together with the accepted Animator candidate. A failed frame publication retains the previous clock and source interval. Completion within a tick still retains source metadata for that tick; the next accepted nonfade tick clears it. Foot correction preflights source curve admission and interval sanity before support queries. Regression coverage includes failure before completion, accepted completion, failure after completion and accepted cleanup; 119 editor CPU tests passed, 10 GPU tests ignored. Mixed-curve swing-event detection and physical transition admission remain open.

### Mixed contact zero windows

The event detector intersects source and target zero-valued curve windows in common fixed-tick coordinates. A source with positive contact prevents target-only swing events during the fade; after fade completion, target-only events qualify, including the exact completion boundary. Frozen sources use their captured scalar contact. Start-only events do not release again. Enumerated looping windows and intersections consume a bounded operation budget; oversized traversal returns an error. The ordinary single-clip path preserves its non-enumerating crossed-swing behavior. Tests cover coincident/disjoint zero events, frozen sources, tick tails, loops and budget exhaustion: 121 editor CPU tests passed. Mixed transition context and shared per-physics-tick budget still need runtime integration before admitting active-foot fades.

### Shared contact event work budget

`SupportQueryBudget::charge_work` reserves preparation work from the same bounded budget used by support queries. Contact event traversal no longer creates a per-foot private allowance. Single-clip scans charge their key count; mixed-window traversal and intersection charge work as they proceed. Foot correction passes the remaining accepted-character-tick budget through all feet. Tests verify successive contact operations and prior support work exhausting one limit without underflow. Live source interval and active frozen snapshot branches are passed to mixed event detection, but frozen completion metadata and physical fade-contact admission remain open. Final CPU checks: 122 editor, 43 gameplay library and 41 gameplay integration tests passed; 10 GPU tests ignored.

### Frozen source completion context

`Animator::frozen_source_tick(dt)` exposes the interrupted source snapshot and the active fraction of the tick without fabricating a clip phase. ModelPlayback captures and publishes this context transactionally alongside live source intervals. Contact correction captures the accepted scalar snapshot weight before sampling a completed target (which clears retained blend state), then detects events with the exact fade prefix. Rejected publication retains previous context; the next accepted nonfade tick releases the snapshot. A Weak-reference regression confirms cleanup. All 123 editor CPU tests passed, 10 GPU tests ignored. Active-foot fade admission remains guarded pending physical anchor continuity and accepted-tick tests.

## Physical clip fade admission (2026-10-03)

Active foot bindings now retain their runtime across live and interrupted clip fades, including completion inside a fixed tick. The physical regression verifies full-weight anchor continuity while the character translates, rejection on explicit contact-event budget exhaustion without publishing pose or scene movement, and successful retry of frozen-source completion. The original failing test assumed distant colliders would force a fresh query, but an existing valid anchor requires no such probe; explicit authored curves now force measurable event work and the test checks the specific budget error. All 124 editor CPU tests and the real GPU planted-foot geometry/resource cleanup test passed. Variable-weight mixed swing behavior still requires physical integration coverage. Native Metal presentation and angular root trajectory fades remain open. Evidence: `artifacts/rig-physical-fade-2026-10-03/`.
