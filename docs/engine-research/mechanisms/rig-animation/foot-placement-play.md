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

## Mixed contact physical regression (2026-10-03)

A translating grounded character now has verified contact behavior with two independently phased authored clip curves during a fade: a positive source retains the initial x=.02 anchor through a target-only swing window, while coincident source/target zero windows release and replant at x=.08. This regression exposed an incorrect assumption that active_tick_fraction=1 always meant fade completion. Runtime now passes whether the source still contributes after advance; target-only tail events qualify only after actual completion. Separate tests cover this distinction for live and frozen sources, including exact completion boundaries. All 126 editor CPU tests and the real GPU planted-foot test passed. Physical interrupted variable-contact transitions and native Metal presentation still require further acceptance. Evidence: `artifacts/rig-mixed-physical-contact-2026-10-03/`.

## Interrupted variable contact physical acceptance (2026-10-03)

A grounded translating character starts a stance-to-air fade, then interrupts it with a third clip containing a zero-valued contact window. The accepted partial contact snapshot keeps the original x=.02 anchor while the new target is in swing. A forced contact-event budget failure at interruption retains the accepted frame Arc, serial and actor transform; the retry completes the transition without replanting at a later position. All 127 editor CPU tests passed. This adds physical coverage to the previously scalar-only frozen contact snapshot checks; physical zero-weight frozen-source replant, repeated interruptions and native Metal presentation remain open. Evidence: `artifacts/rig-interrupted-physical-contact-2026-10-03/`.

## Repeated interruption and zero-snapshot replant (2026-10-03)

The physical interrupted-contact regression now includes additional interrupts while the accepted contact is partial. The sole remains on its original x=.02 anchor after completion. It then fully releases into a zero-contact clip, starts a zero-to-zero fade and interrupts that fade with the authored stepping curve. A subsequent swing rearms planting, and the final sole matches the actor position at the renewed stance rather than the stale original anchor. All 127 editor CPU tests passed (the existing regression was expanded). Native Metal foot Play/Stop presentation, angular root trajectory fades and character carry on moving supports remain open. Evidence: `artifacts/rig-repeated-physical-contact-2026-10-03/`.

## Native Metal foot gate passed (2026-10-03)

The editor now creates its window hidden, initializes graphics, then shows, focuses and requests redraw. The real native foot fixture changed from zero presented frames to actual Presented outcomes without bypassing the Metal occlusion guard. The first running acceptance exposed an initial watcher invalidation replacing the model during motion; stable-rig acceptance now waits for import publication to settle before entering Play. This does not establish continuity during rig replacement. The final process exited 0 after 94 native frames: 12 physical ticks moved the actor center to x=.06 while the sole remained x=.0049999915, y=0; accepted palette signatures matched GPU ownership, two owners shared one source (1184 bytes). Stop restored authoring and released animated GPU bytes to zero. Evidence: `artifacts/rig-foot-window-order-2026-10-03/`. Earlier zero-frame reports are retained as historical failures. This is Apple Metal fixture evidence, not proof for all hardware or production rig reload continuity.

## Equivalent rig reload continuity (2026-10-03)

AnimationClip exposes exact authored equivalence excluding derived caches and allocation identity. A model revision with identical skeleton layout and all authored clip data retains the owner Animator, clocks, fade snapshots and foot contact authority while rebinding the imported model. Changes to any rig or authored clip data use fresh playback/contact state. An independently reconstructed rig/clip regression verifies fade weights 1/3 and 2/3, original anchor retention and changed-duration reset; all 128 editor CPU tests passed. The temporary stable-publication delay was removed from native foot acceptance: the actual model republished at frame 20 during Play, retained sole x=.0049999915 through the frame-27 check after 12 ticks (actor x=.06), then restored authoring and released GPU bytes at frame 30; process exit 0. Evidence: `artifacts/rig-equivalent-reload-2026-10-03/`. Animation edits and different skeleton layouts still reset rather than remap, and wider hardware coverage remains open.

## Phase-preserving authored clip revision (2026-10-03)

Animator::transition_to_at_phase starts the target at a validated normalized phase without extracting the sampling-origin offset as root displacement. Ordinary transition_to keeps its phase-zero behavior. Invalid phase/duration/rig commands retain existing playback. With an unchanged skeleton and selected clip index/name, a model revision uses the authored positive transition duration to transfer the accepted target phase and retain contact authority; zero duration remains an immediate reset. Existing interruption handling captures the displayed blended pose. CPU regressions cover duration changes, a +100 translation-origin offset with bounded root delta, repeated interruption pose continuity, invalid phase rollback and physical planted-anchor continuity for revised duration. All 68 animation and 128 editor CPU tests passed. A native changed-file animation gate, clip reorder/name remapping and different-rig retargeting remain pending; angular root trajectory fades retain their existing rejection. Evidence: `artifacts/rig-phase-reload-2026-10-03/`.

## Native watched clip revision (2026-10-03)

The existing native foot acceptance mode now optionally waits for a real watched GLB revision (VOXY_FOOT_RELOAD_SMOKE) and requires an observed intermediate fade before completion. The external driver copies the fixture into a temporary directory and atomically replaces its animation time accessor [0,1] with [0,2] after Play starts; it never writes the repository fixture. The imported revision published at frame 18, an active fade with weight .20000002 was observed at frame 26, and the frame-75 check after 36 physical ticks retained sole x=.0049999915 with actor x=.06, accepted GPU palette and two owners sharing one source (1184 bytes). Stop restored authoring and freed animated GPU bytes; process exit 0. All 128 editor CPU tests passed. Evidence and reproducible driver: `artifacts/rig-native-file-reload-2026-10-03/`. This proves duration revision for one constant-pose Metal fixture, not arbitrary edited motion, retargeting, angular trajectory fades or wider hardware support.

## Native changed-motion revision (2026-10-03)

The watched-file driver now supports --motion, changing both duration 1→2 and the hip translation endpoint from (0,.5,0) to (.15,.55,0). Native acceptance checks the independently specified linear reference hip=(.15*phase,.5+.05*phase,0), verifies the accepted GPU palette, then checks the planted sole. The process observed fade weight .20000002, sampled phase .3500000183 with hip (.052500006,.5175,0), and retained sole (.0049999915,5.96e-8,0) while actor x=.06 after 36 ticks. Stop at frame 79 restored authoring and freed animated GPU bytes; exit 0. All 128 editor CPU tests passed. Evidence: `artifacts/rig-native-motion-reload-2026-10-03/`. This is a bounded translation edit on Apple Metal; arbitrary motion edits, angular root trajectory fades, clip remapping and different-rig retargeting remain open.

## Named clip identity and reorder continuity (2026-10-03)

ModelAnimation.clip_name is an optional exact unique name; empty preserves legacy numeric selection. Names are bounded and validated at authoring when the model is published, then resolved again during runtime admission. The inspector exposes the name. For an unchanged named selection, equivalent authored clips may reorder across model reload without resetting target phase, active fade weight or foot authority. Numeric selection keeps order-based behavior. The expanded physical regression checks serialization and old missing-field compatibility, a reordered walk/run array during a fade, continued phase/weight increments, retained sole anchor, missing/ambiguous names and rejected reload publication preserving frame/serial. All 128 editor CPU tests passed. Native watched clip reordering, different-rig retargeting and angular root trajectory fades remain open. Evidence: `artifacts/rig-named-clips-2026-10-03/`.

## Native named clip reorder and inspector history (2026-10-03)

The watched-file driver now supports --motion --reorder: a temporary GLB starts with [move,other] (other has an independent constant negative hip translation), then changes to [other,move] while revising move duration and motion. Native Play explicitly selects move by name; acceptance verifies the resolved index is 1 and the actual target clip is still move, checks the analytic hip pose and accepted GPU palette, then verifies the original sole anchor. Fade weight .16666667 was observed; at frame 69 / tick 37, phase .3666666858 produced hip (.055000003,.5183333,0) and sole (.0049999915,0,0). Stop at frame 72 restored authoring and freed animated GPU bytes; exit 0. Inspector regression also checks rejected unknown names without history mutation, named edit undo/redo and scene JSON round trip. All 128 editor CPU tests passed. Evidence: `artifacts/rig-native-clip-reorder-2026-10-03/`. Different-rig retargeting, angular root trajectory fades and broader hardware evidence remain open.
