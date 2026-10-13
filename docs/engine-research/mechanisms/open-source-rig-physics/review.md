# Source-backed rig and secondary-physics adoption review

Reviewed 2026-10-10. The pinned source copies, SHA-256 hashes and URLs are recorded in `sources.json`; repository revisions are in `repository-pins.json`. These are source inspections, not upstream runtime benchmarks or integrated Voxy features. Upstream comments and claimed speedups are not Voxy measurements.

## Selected implementations

| Repository / pinned revision | Licence inspected | Concrete mechanism | Decision for Voxy |
| --- | --- | --- | --- |
| [TressFX](https://github.com/GPUOpen-Effects/TressFX/tree/ba0bdacdfb964e38522fda812bf23169bc5fa603) | MIT, license.txt | Integration, local/global shape, length/collision and follow-hair dispatches; persistent GPU buffers and UAV ordering | Use as a scheduling and follower-generation reference. Do not replace Cosserat/contact admission with its visual constraints. |
| [PositionBasedDynamics](https://github.com/InteractiveComputerGraphics/PositionBasedDynamics/tree/10a70bc146a97873dc3c8fef372f5217e010542e) | MIT, LICENSE | Cosserat stretch/shear and bend/twist constraints; direct stiff-rod tree solver; substep integration | Compare rod operators and independent constraint groups. Voxy already has a rod solver; adopting another solver is not itself a feature. |
| [ozz-animation](https://github.com/guillaumeblanc/ozz-animation/tree/744eb9d99f606eda849acb0b1204f7a3dc20bca1) | MIT in LICENSE.md, despite GitHub NOASSERTION metadata | Two-bone IK, pole plane, continuous reach softening; foot raycasts, ankle correction, pelvis offset and foot alignment | Reference for a Rust implementation of full leg/foot placement. Keep planted ankle targets exact; reach softening must not introduce grounded-foot sliding. |
| [PPF Contact Solver](https://github.com/st-tech/ppf-contact-solver/tree/618f85b667b7f211f762908690bc46f65b578ae0) | Apache-2.0, LICENSE | Rust driver, Metal/CUDA/ROCm backend sources, device scalar recurrences, batched iteration submission, sticky numerical-breakdown latch | First reference for eliminating host synchronization. Its float32 PCG is not a replacement for Voxy equality QR and original-f64 admission. |
| [Newton VBD](https://github.com/newton-physics/newton/tree/a6e1649b112e3d962b35f7dad66780dc55150588) | Apache-2.0, LICENSE.md | Graph-coloured particle/body updates, separate input/output state, preallocated contact pipeline and capture-aware construction | Reference for parallel local blocks and persistent state. No Metal portability or 160 FPS result established by this review. |

## Important source findings

PPF `driver/pcg.rs` batches four iterations while far from convergence, checks the first iteration and switches to checks each iteration near tolerance. A device-side sticky latch preserves a breakdown across delayed host checks. Submission is batched as well as readback: omitting only the read does not remove all host stalls. Its backend contract requires dependency ordering for queued dispatches and explicitly specifies float32. Preserve its distinction between queued and completed work when adapting scheduling.

PPF `rod_strain.kernel.cpp` applies a barrier only for positive tensile strain. Compression returns no term. Therefore this kernel cannot replace Voxy's two-sided axial-strain admission: our rejected compressed segments would not be covered. `collision_window.rs` controls authored time intervals in which elements collide; it is not a collision-safe adaptive timestep algorithm and must not be presented as one.

TressFX stages guide simulation before follow-hair generation. This is useful for keeping follower geometry resident, but does not prove equivalence to Voxy's rod/contact model. Newton VBD requires graph colouring and separate states; those ownership boundaries are useful independently of its solver equations.

## Implementation priority and acceptance

1. **Reproduce the current contact divergence as a small replay.** Capture the actual rod-232, frame-3, substep-0, iteration-3 projection input and connected constraints. The existing full capture identifies the first over-gate phase but its operator capture targets rod 53, so it is not the required replay. Compare the same frozen operator and branch decisions before changing GPU scheduling.
2. **Adapt PPF's resident-state scheduling pattern to Voxy's existing operator.** Persistent scratch/scalars and bounded dispatch batches; sticky failure status cleared per solve; dependency barriers; retain original-input f64 residual/admission checks and publish only admitted states. Do not swap in PCG merely because it is available. Validate seeded residuals, near-zero denominators, breakdown between host probes, rejected-state isolation and CPU/GPU replay parity.
3. **Use ozz foot IK as the rig reference.** Raycast targets, pelvis compensation, knee pole, sole alignment and planted-foot locking; walk/run/jump transitions must use the imported rig. Validate unreachable targets, nearly straight knees, slopes, landing and both planted feet. Current jump tests do not establish a production walk/run system.
4. **Use TressFX's guide/follower pipeline as the rendering reference.** Keep current groom density and physical guide count. Measure rendered frame time separately from physics throughput, with the full scene and continuous camera motion.
5. **Evaluate PBD/Newton operators only through isolated fixtures.** Preserve material, contact and strain requirements. A faster solver with different admissible motion is not an equivalent optimization.

## Evidence and delivery limits

Current local full GPU qualification fails at frame 3: rod 232 first exceeds the parity gates in contact-positions iteration 3 after a preceding structural-mesh phase that still agrees closely. This is a correctness blocker, not evidence of achieving the requested FPS. See `artifacts/articulated-jump-2026-10-10/frame-three-rod-232-full-capture/phase-comparison.json`.

No upstream dependency has been installed and no upstream algorithm has been integrated by this review. Reused code must retain its licence/attribution; inspect any selected dependency and asset licences separately. Pinned source snapshots are audit material, not a new runtime ownership layer.

## Scheduling boundary verified in Voxy

`gpu_hair_joint.rs::run_equality_tasks` already batches independent ready tasks before a shared submission/wait. `EqualityTask::consume` computes the next residual and original-f64 backward-error admission on the host, then `encode_with_timestamps` uploads the next RHS. Therefore PPF-style batching across refinement iterations requires a resident residual/admission design; simply queueing repeated solves would reuse stale RHS and is invalid. Keep the host original-input check as final authority until a mathematically qualified replacement exists.

## First native rig mechanism implemented

`voxy_animation::Pose::solve_two_bone_softened` adds opt-in extension softening inspired by ozz's `SoftenTarget`, with an independently written scalar exponential rather than its SIMD rational approximation. Below the extension threshold it uses exact IK; at the threshold the endpoint distance and its derivative are continuous. Ratio 1 takes the original exact path. Geometric reach reporting remains separate from intentional softening; local translations/scales and unrelated joints remain unchanged.

This is a reusable rig primitive, not an integrated walk/run controller. Grounded jump targets still use exact IK. Focused tests cover all eight IK cases including extension continuity/monotonicity, disabled-mode identity, geometric reach reporting and invalid ratios. No new visual or FPS result is implied.

The mechanism is also exposed through `AnimatorFrame::with_softened_two_bone_ik`, rebuilding the palette while preserving extracted root motion and transition weight. Full animation-library validation passed 239 tests; mirrored uniform scales, unchanged segment lengths, foreign rigs and invalid ratios are covered. The existing exact frame API delegates with ratio 1. Evidence: `artifacts/articulated-jump-2026-10-10/ozz-inspired-soft-ik/validation.json`.

## Support-relative foot planting

`voxy_animation::FootPlant` stores the admitted ankle pose relative to a caller-owned rigid support. Each tick it resolves that anchor into current actor model coordinates for exact IK. Scene/support identity, contact acquisition, release events and pelvis correction stay with the locomotion controller. The anchor follows a moving platform without recapturing its animated ankle position. World deltas are subtracted before rotation in f64 to avoid unnecessary loss of precision at distant origins.

Full library validation passed 243 tests. New cases cover actor motion, common-frame invariance, moving/rotating support, distant origins, invalid transforms/weights, unrepresentable targets, and a 60-step exact-IK integration checking world ankle position/orientation. This is the stance primitive, not yet a walk/run state machine or a rendered demonstration. Evidence: `artifacts/articulated-jump-2026-10-10/support-relative-foot-plant/validation.json`.

## Shared pelvis correction implemented

`Pose::solve_planted_legs` intersects the permitted pelvis translation interval with each leg's reachable spherical shell. Both outer extension and inner fold limits participate; the closest feasible offset to zero is selected, followed by independent exact IK. This extends the pelvis-offset idea in ozz's foot example without copying its heuristic lowest-foot rule. Both chains must be independent descendants of joint 0, the actor's shared skeleton root; targets have full weight. No common feasible offset returns `UnreachableFootPlant` without publishing a partially solved pose.

`AnimatorFrame::with_planted_legs` rebuilds the palette but keeps extracted actor root motion and transition metadata unchanged. Validation passed 246 animation-library tests, including uneven ankle heights, bounded correction rejection, inner fold limits, zero correction for feasible stance, independent-chain validation and frame metadata. This remains library integration: terrain queries, gait contact events, support identity and visual walk/run acceptance are unfinished. Results expose individual reach reports; f32 pose storage retains its existing numerical limits. Evidence: `artifacts/articulated-jump-2026-10-10/shared-pelvis-foot-ik/validation.json`.

## First application integration

`FemaleRig` now captures immutable flat-ground stance anchors at construction. During grounded crouch/landing, `jump_pose` resolves both anchors through the shared vertical actor transform and applies shared pelvis/exact-leg IK. During flight it retains the authored tuck and ankle orientation targets. Arm/spine motion stays in the same pose publication. This integrates support-relative targets into the existing actual-model demonstration; terrain acquisition and walk/run contact state machines are still unfinished.

The first application jump suite passed 6 tests with 2 GPU tests ignored. A denser full-loop regression is running separately; do not infer its outcome from the earlier suite. The long GPU divergence capture intentionally uses its frozen baseline binary, predating this rig integration, and cannot qualify these changed poses.

The dense jump suite has now completed: 6 passed, 2 ignored, with 721 uniform loop samples plus 11 event/peak times. The separate 8-step coupled full-groom skin/hair regression passed. Actual-body GPU skinning also passed on Apple M4 Max Metal: 60866 body vertices, 6 sampled poses, maximum CPU/GPU position difference 1.9687948e-7 m. This is not full GPU hair physics qualification or rendered FPS evidence. See `artifacts/articulated-jump-2026-10-10/integrated-jump-foot-support/validation.json`.

## Real walk/run source clips acquired and imported

Quaternius Universal Animation Library Standard is CC0 according to the publisher and accompanying licence. The J-Ponzo glTF mirror is pinned at `e24c23cf2a1323488a3faa226ea7ea21f644b73e`; unmodified document/buffer, licence, provenance hashes and source clip/node catalogue are in `assets/animation/quaternius-universal`. This specific snapshot contains 46 clips including Walk_Loop, Jog_Fwd_Loop, Sprint_Loop and three jump phases; do not report the publisher's full-collection count as imported.

The native ModelAsset loader imported all 46 clips without code changes. The real-fixture integration test passed 5566 pose/palette samples and 230 f64 surface samples and verified actual Walk_Loop mesh motion. Contact events, loop/clamp policy, explicit source/target axis corrections and gait transition integration remain unfinished. Evidence: `artifacts/articulated-jump-2026-10-10/quaternius-library-import/validation.json`.

## Retarget reference-pose calibration

The source character's T-pose and FemaleRig's lowered arms require explicit reference-pose calibration before transferring locomotion. Godot's primary retargeting documentation describes normalization of rest orientation/silhouette: https://godotengine.org/article/animation-retargeting-in-godot-4-0/ . This informs the requirement, not an imported engine ownership design.

`RetargetBinding::with_target_reference_pose` now calibrates mapped target rotations while retaining the canonical target skeleton and inverse binds. Reference translation/scale edits are rejected; unmapped target joints retain bind channels. All pose-field/enclosure/evaluation paths use the same compiled target reference. Exact rigid-root eligibility is recomputed, so a changed rotation cannot keep an incoherent translation/rotation certificate.

The library passed 248 tests; the final focused calibration/enclosure tests passed 2. They check canonical ownership, unchanged bind mapping when calibration is disabled, foreign/proportion rejection, invalidated rigid-root eligibility and enclosure containment of the calibrated ideal rotation. Evidence: `artifacts/articulated-jump-2026-10-10/retarget-reference-pose/validation.json`.

Remaining integration issue: the source has extra shoulder, neck and spine ancestors compared with FemaleRig. The current RetargetBinding explicitly transports local channels, so omitting those ancestors cannot be presented as a qualified whole-body retarget. A source/target reference profile and preservation of the omitted ancestors' global rotation contribution are still required before visual gait acceptance.

## Model-space authored rotation transport

Inspected and pinned Godot `scene/3d/retarget_modifier_3d.cpp` at `232e6b14abf6e590b41d60f11704e42de22bff72`, including its MIT licence. Its global-pose path and cached rest correction inform the missing-ancestor requirement. Source/PIN.json are under `sources/godotengine__godot`; this is a mechanism reference, not a Godot runtime dependency.

`RetargetBinding::compile_model_rotations` now creates a pose-only `ModelRotationRetarget`. It composes source authored quaternion frames including unmapped ancestors, removes the compiled source rest frame, applies the explicit model-axis conversion and target reference frame, then resolves local target rotations parent-before-child. Per-joint local `rotation_basis` is not silently reused as a model-space correction. Canonical local translation/scale policy remains explicit. No local-channel root-motion certificate is exposed through this distinct pose-only type.

The first real-library test exposed small exported anisotropic scales. They are preserved, not rounded: final rotation-frame semantics compose authored quaternions and scale handedness, retaining magnitude channels. This does not claim polar rotation extraction from a sheared matrix, nor full global position/scale retargeting. The initial rejection log is preserved.

Final evidence: 250 animation tests passed; 2 actual-library integration tests passed, including 484 identity-retarget pose/palette samples across Idle_Loop, Walk_Loop, Jog_Fwd_Loop and Sprint_Loop. Synthetic cases explicitly exercise an omitted animated shoulder, unordered mapping, calibrated reference pose, signed scale and immutable failure handling. This is not yet the source-to-FemaleRig profile or gait contact integration. See `artifacts/articulated-jump-2026-10-10/model-rotation-retarget/validation.json`.

## Quaternius-to-Voxy humanoid profile

`voxy_app::humanoid_locomotion::VoxyHumanoidLocomotion` now compiles an explicit 46-bone mapping, including all 30 finger joints, with T-pose calibration of the canonical target arms. It retains caller-owned Arc<ModelAsset> source ownership and the existing target rig identity. The pelvis translation unit conversion uses the measured leg-length ratio and the source hip-parent rotation; other mapped translations retain target bone proportions. This named preset validates target topology, not merely bone names. Source toe landmarks confirm matching +Z forward axes; their source-only joints remain outside the current target map.

The actual 60866-vertex body fixture passed 484 pose samples across idle/walk/jog/sprint and 20 DQ-skinned mesh samples. Leg lengths remain within 1e-5 m, non-root local translations remain canonical, invalid phases and wrongly parented target legs reject. These are pose/kinematic checks, not visual gait quality or physical foot contacts. The source still needs stance events and a motion driver before enabling coupled gait physics. Evidence: `artifacts/articulated-jump-2026-10-10/humanoid-locomotion-profile/validation.json`.

## Heel/toe contact pivots

FootPlant now supports `capture_pivot` and `target_with_rotation`: a support-relative heel/toe point stays fixed while the ankle follows an explicitly supplied world orientation. This extends the existing contact owner rather than creating a separate gait physics system. The zero-pivot default retains ankle-lock semantics; callers still own support identity, contact phases and pivot transitions.

253 animation-library tests passed. New regressions verify 60 roll steps for heel and toe on a moving/rotating support, invalid pivot/orientation rejection, and 60 exact-IK steps keeping the toe contact within 3e-6 m during actor movement. This is the contact primitive, not yet an authored heel-strike/toe-off gait controller. Evidence: `artifacts/articulated-jump-2026-10-10/heel-toe-contact-pivot/validation.json`.

## Authored jump clip profile

The existing Quaternius-to-Voxy profile also exposes JumpStart, JumpLoop and JumpLand, resolving the original Jump_Start, Jump_Loop and Jump_Land clips without modifying upstream animation bytes. They share the existing model-space rotation retarget and canonical target rig identity. The actual-body regression includes all seven locomotion/jump clips, 121 phases each. This does not yet replace the scene jump clock, integrate phase transitions or qualify rendered jump quality and FPS.

Validation passed: 847 pose/palette samples and 35 full-mesh skinning samples across seven clips, including all three authored jump phases, on 60,866 body vertices. Evidence: artifacts/articulated-jump-2026-10-10/authored-jump-profile/validation.json.

## Actual-mesh authored jump boundary audit

Direct clip switches are not qualified: Idle end to JumpStart start has maximum vertex displacement 0.43599334 m and pelvis displacement 0.28068048 m; JumpStart end to JumpLoop start matches exactly; JumpLoop end to JumpLand start has vertex displacement 0.3172377 m and pelvis displacement 0.044993516 m; JumpLand end to Idle start has vertex displacement 0.07637988 m. These are endpoint differences, not measured rendered jitter. Source clip durations are 1.3333334 s start, 2.5 s flight loop, 1.25 s land, unlike the current physical jump clock. Before enabling authored playback, separate pelvis animation displacement from the physical actor trajectory, define phase synchronization and admit transitions/ground contacts. Existing Animator transition APIs should own blending. Evidence: artifacts/articulated-jump-2026-10-10/authored-jump-seams/seams.json. The actual-body regression passed.

## Physically owned actor translation

VoxyHumanoidLocomotion::pose_phase_in_place reuses AnimatorFrame::into_in_place_translation to reset the target pelvis translation to its canonical bind coordinates and return the absolute authored offset separately. The offset is not an interval delta and must not be accumulated or blindly added to the physical jump trajectory. Rig identity is checked by the shared extraction API; rotations, scales and non-pelvis joints are retained. This enables physical actor ownership without silently applying the authored pelvis displacement twice. The existing scene driver is not switched by this API addition; phase timing, contact admission and transitions remain required.

Extraction validation passed across 847 authored pose samples and 35 full-mesh samples of 60,866 vertices. Adding the separately returned pelvis offset to in-place skinned vertices reconstructs original authored deformation within 2e-6 m. Other joints, pelvis orientation/scale and canonical in-place translation are checked, with foreign-rig rejection. Evidence: artifacts/articulated-jump-2026-10-10/in-place-authored-jump/validation.json.

## Authored jump synchronized phase sampling

The profile jump_pose sampler uses the existing jump_motion takeoff, flight and landing constants and existing Pose::blend rather than a second mutable playback clock. Source start, flight loop and landing are sampled by normalized phases; entry and recovery blend with a fixed idle reference, while the final 0.10 seconds of flight blend toward the landing entry pose. Pelvis translation remains canonical, with actor displacement externally owned. This is stateless animation phase sampling, not a force integrator or a contact solver. Source authored timing is time-warped to the physical demo clock; velocity/acceleration quality and contact IK acceptance still need qualification. The scene driver is not switched until those checks pass.

Clock sampler validation passed: 721 samples over the six-second loop, finite skin palettes and canonical pelvis translation. Ten event boundaries are probed at +/-1e-7 seconds for local pose continuity (translation <2e-6 m; absolute quaternion dot >1-2e-6). This is C0 pose qualification, not joint velocity, contact or rendered quality qualification. Evidence: artifacts/articulated-jump-2026-10-10/authored-jump-clock/validation.json.

## Authored jump support integration candidate

FemaleRig::authored_jump_pose now applies existing two-leg shared-pelvis IK and support-relative FootPlant targets to authored retargeted jump poses. Grounded targets include the physical actor height; preparation in the first/last fifth of flight targets the future support pose without duplicating actor translation. Middle flight retains the authored leg animation. Knee poles come from the authored knee positions, preserving the animation bend plane. This candidate is not enabled in the scene; a full-cycle support/clearance/boundary regression is required before publication. The existing maximum 0.10 m pelvis correction is retained.

The first support integration run was rejected by UnsupportedIkScale. Source exporter scale channels cannot define the target physical limb metric. The named humanoid preset now explicitly retains every target bind scale while transferring rotations and pelvis displacement; original glTF scale channels remain unmodified and the general retarget implementation still preserves its original scale behavior. This is a preset ownership policy, not epsilon-based source normalization or relaxed IK admission. A validated Pose::set_joint_scale setter supports the policy.

Support integration candidate passed 721 samples over the six-second cycle: both grounded ankles stay within 3e-6 m of support and airborne ankles remain above the support plane within the same tolerance. Seven contact/cycle boundaries have local pose continuity checked at +/-1e-7 seconds. This does not prove full-foot surface collision, joint velocity quality, full skin/hair coupling, rendered quality or FPS. Evidence and original UnsupportedIkScale rejection are preserved in artifacts/articulated-jump-2026-10-10/authored-jump-support/.

The complete animation library regression passed 253 tests after adding the validated local-scale setter and target-metric profile policy. Log: artifacts/articulated-jump-2026-10-10/authored-jump-support/animation-tests.log.

## Ready inertialization source for velocity transitions

Pinned orangeduck/Motion-Matching at 57b7250e0d34a4e456a34d47e24c2f05fdcc711e: controller.cpp, spring.h, quat.h, README.md and MIT LICENSE (Daniel Holden, 2021). No motion databases or training data were downloaded; README distinguishes the restricted animation dataset from MIT code. controller.cpp contact_update preserves position/velocity offsets when locking and releasing foot contacts, and spring.h implements decaying positional/rotational transition offsets. These mechanisms are relevant to the measured C0-only jump path, but are not yet integrated.

Do not copy quaternion velocity handling unchanged: the source transition subtracts destination angular velocity without rotating it by the new offset, while update rotates destination velocity by the offset. For a nonidentity initial offset these expressions do not generally reproduce the source angular velocity. Its rotational log-vector decay also requires explicit SO(3) tangent/Jacobian treatment if angular velocity must equal the derivative of the output orientation. The source contact offsets decay toward the lock target rather than proving exact nonpenetrating support. Keep strict contact admission after free-pose smoothing; do not relax planted support gates to imitate this demo.

## Sole surface and velocity audit

Baseline exact-support candidate passed 721 samples of 8,280 sole vertices (foot weight >0.9999, rest y<-0.755 m): grounded reconstruction error below 4e-6 m and no sole vertex below the rest sole minimum plane -0.82004285 m beyond 3e-6 m. This is the selected rigid sole surface, not mixed ankle skin or arbitrary terrain. However, finite differences at 1e-4 s found local angular velocity mismatch 2.2179152 rad/s at takeoff and 10.6059323 rad/s at landing, so C1 qualification failed despite the positional checks passing. Baseline artifacts: artifacts/articulated-jump-2026-10-10/authored-jump-sole-velocity/.

The next candidate changes airborne support preparation from zero actor height to physical actor height multiplied by contact preparation weight. As weight approaches one at contact boundaries, both the target pose and actor-relative height derivative approach the grounded solve. Middle flight remains authored. This change must retain the original sole clearance/support tolerances; it is not admitted merely from its algebraic motivation.

The height-aware support trial retained the sole checks, but landing local angular velocity mismatch remained 10.5131365 rad/s (baseline 10.6059323); takeoff remained 2.2179152 rad/s. It did not resolve the required velocity transition and was reverted, preserving both its method source and measurements under authored-jump-sole-velocity/. The current support candidate retains the previously validated zero-height preparation. Next diagnostic must separate source pose angular derivatives from root/pelvis/contact IK effects, rather than treating the positional gate as production animation acceptance.

## Localized phase-rate discontinuity

A raw-versus-support-IK diagnostic at 1e-3, 1e-4 and 1e-5 s confirms a source sampling discontinuity: raw landing-entry knee angular velocity changes from approximately zero to 35.394 rad/s, with resolved IK mismatch approximately 10.606 rad/s. Raw takeoff mismatch is approximately 1.46 rad/s; support pelvis velocity mismatch is approximately 2.49 m/s at takeoff and 2.62 m/s at landing. These are finite-difference measurements, not proof of derivatives of all joints. Evidence: artifacts/articulated-jump-2026-10-10/authored-jump-derivative-localization/.

The next candidate uses a 0.10 s quintic phase-rate ramp at the start of flight/landing and the end of takeoff: source phase and phase rate reconnect to the ordinary phase schedule at the edge of each window, while the held-pose endpoints have zero phase rate. This changes the phase schedule rather than rounding keys or filtering the rendered mesh. Height-aware support preparation is re-evaluated together with this source-rate correction; neither component is accepted merely from its motivation.

The phase-rate plus height-aware candidate passes existing support and sole gates. Raw landing velocity mismatch at 1e-4 s drops from 35.393 to about 0.001 rad/s; resolved mismatch drops from 10.606 to about 0.241 rad/s. At takeoff the residual resolved mismatch grows from 1.74 to 17.38 rad/s as the step shrinks from 1e-4 to 1e-5 s, indicating conditioning/storage noise near straight knees rather than a remaining large raw source velocity jump.

Motion-Matching controller.cpp uses a 0.015 m max_length_buffer to avoid full extension by clamping the foot target. The Voxy candidate instead adds opt-in solve_planted_legs_with_extension_margin: intersect both original reachable shells with a reduced outer reach, preserve the inner fold limit, choose a common bounded pelvis displacement and solve both exact foot targets. Zero margin preserves the existing API behavior. The full animation suite passed 254 tests including preserved contact, bone lengths, margin equivalence, invalid margin rejection and failure when pelvis travel cannot provide the reserve. Actual-body velocity/support admission is pending.

## Conditioned authored jump admission

The combined phase-rate correction, height-aware preparation and opt-in 15 mm extension reserve passed the actual-body regression. At 1e-4 s, maximum measured local angular velocity mismatch over pelvis/spine/leg joints drops to 0.003998 rad/s at takeoff and 0.000674 rad/s at landing (previous 2.218 and 10.606). At 1e-5 s the maxima are 0.02532 and 0.007855 rad/s, without the previous straight-knee amplification. Regression gates require selected joint mismatches <0.2 rad/s and pelvis velocity mismatch <0.02 m/s at both events for all three step sizes. All 8,280 rigid sole vertices retain the original 4e-6 m grounded and 3e-6 m floor tolerances across 721 samples. The full animation suite passed 254 tests. Evidence and source hashes: artifacts/articulated-jump-2026-10-10/conditioned-authored-jump/validation.json.

This admission is limited to the selected derivatives, event boundaries and sole surface. It is not a whole-trajectory C1 proof, mixed ankle/body collision test, full coupled skin/hair qualification, rendered visual acceptance or FPS result. The source-authored path remains disabled in the scene until initialization and coupled simulation ownership are connected and checked.

## Runtime authored jump selection and initialization

VOXY_AUTHORED_JUMP=1 selects the source-authored path in SceneApp::with_female_secondary / female_motion. FemaleRig routes all existing jump palette, render, physical-shell point and head queries through its installed immutable profile, so hair roots/colliders and skin targets share the same clock. Render and shell bindings compile separately over one shared Arc<ModelAsset>. Background view replication and replacement body construction retain the shared source while compiling against their own target layouts. Selection on an advancing model is rejected before mutation.

The jump neutral pose now matches canonical target bind, and the extension reserve follows the existing jump limb envelope. Exact rest initialization is retained instead of imposing source idle offsets or a permanent pelvis reserve on initialized physical state. The complete pose/sole/derivative cycle test passes. A new actual-model integration test verifies initial render palettes and skin targets, shared asset ownership in the background replica, identical replica palettes, eight coupled skin/hair steps with all 469 guides x20 segments, and late-switch rejection. This covers only 0.0667 s of dynamics, not a whole coupled jump. Checks passed for female_motion, female_render, hair_render and skin_render. Timings are functional-test observations under concurrent build work, not FPS benchmarks; observed CPU physics remains slower than real time. Evidence: artifacts/articulated-jump-2026-10-10/authored-jump-scene-integration/.

The release female_motion executable built successfully. Launch from the repository with `VOXY_AUTHORED_JUMP=1 cargo run --release -p voxy_app --example female_motion`. Build log, binary SHA-256 and unchanged source hash audit are recorded in the scene-integration artifact. No native visual or FPS result is claimed by this build.

## Full authored coupled jump: reproduced frame-15 admission failure (2026-10-11)

The original 720-frame, 120 Hz coupled fixture stops after 14 admitted frames. A separate diagnostic rerun reproduces the same frame-15 failure with all 469 guides and 20 segments retained. After temporal refinement to 32 physical substeps (h = 0.00026041666666666666 s), rejected guide 66, segment 16 has positive axial strain 0.07672856851738663 against the unchanged 0.05 limit. Segment 18 is also over the limit. Published time, step count, skin positions and guide position/orientation frames remain at the prior admitted state. This does not certify all internal caches.

The rejected rod endpoint contact diagnostics are empty; that alone does not exclude earlier mesh or inter-strand contact contributions. This is a tensile failure, unlike the earlier compression fixture. PPF's tensile barrier is therefore a relevant mechanism to evaluate on a frozen input, but is not yet integrated or proved equivalent. Increasing timestep subdivisions alone did not establish admission. Preserve material and contact requirements; capture the actual prediction/contact state before choosing a solver change. Full trajectory and rendered FPS remain unqualified.

Evidence: `artifacts/articulated-jump-2026-10-10/authored-jump-full-cycle-2026-10-11/` and `authored-jump-failure-diagnostic-2026-10-11/diagnosis.json`; raw rejected rod: `authored-jump-rejected-guide.json`.

The full frame-15 endpoint replay has now been captured in `authored-jump-frame15-admission.vhr` (VHR2, original prediction/inertia and last-substep mesh motion interval). A new opt-in `captured_rejected_rod_structural_response` diagnostic retains rod 66 material, rest state, prediction, root and physical timestep, but removes contact terms for isolation only. Its 64 iterations pass root/finite checks: strain falls from 0.07672856851738663 to 0.0016766106410979198 after the first correction and approximately 0.0001259905985 at convergence. This demonstrates that the captured structural solve can relieve this tensile strain without a new barrier; it does not qualify that correction with contacts or swept motion. The next experiment reuses the full coupled elastic/contact replay with whole-step strand sweep auditing.

### Replay fidelity correction under validation

Correction to the preceding capture description: the actual authored demo uses sampled collider motion, not continuous mesh admission. The saved file header is VHR1, not VHR2. The existing full elastic/contact replay refuses its contact-merit fidelity check before doing corrections: stored merit 1.09045668793513e-11 versus rebuilt merit 4.509051844708857e-3. This is substantial and cannot be excused as roundoff or used to qualify a production fix. Preserve this original binary.

The exporter now samples the last collider interval whenever either sampled motion or continuous admission is enabled, and clears live nearest-triangle/clearance caches before measuring serialized contact merit. This is a diagnostic capture change, not a production solver change. A separate fresh capture is running; cache/order fidelity remains unproved until replay passes the unchanged comparison. The isolated structural result remains evidence only about the no-contact rod correction.

Fresh capture validation now passes the unchanged geometry-merit comparison. The complete coupled elastic/contact replay reaches the unchanged 5% endpoint strain gate, but its required whole-step strand sweep audit fails with `swept strand motion starts in penetration`. This confirms export fidelity improved, not production physics completion. The legacy sampled-motion demo admits endpoints without establishing nonintersecting strand trajectories; endpoint-only terminal iterations cannot be promoted as the solution. Next qualification must address initial/substep strand feasibility and continuous coupled motion. See `authored-jump-fresh-admission-capture-2026-10-11/validation.json` for capture/binary hashes and per-iteration strain.

The captured strand-refresh reference audit passes on the fresh authored frame-15 state: all 109 pair witnesses, normals, impulses, contact planes and ordering match, and refresh preserves guide positions. This does not prove absence of penetration. The whole-step audit reports initial penetrations including (208,7)/(238,9), (10,13)/(45,12), (423,11)/(463,9), and (68,7)/(70,6). Reference and filtered refresh medians in this diagnostic run are 9.170/10.225 ms; these are contact discovery timings, not frame rates.

The full authored coupled fixture now has an explicitly recorded `VOXY_AUTHORED_JUMP_CONTINUOUS_CONTACTS=1` qualification mode. It enables the existing joint position/velocity and swept strand/continuous mesh operators, preserves sample motion, all rods/material/gates, and disables endpoint-only terminal bypasses. Production defaults remain unchanged until the complete 720-frame trajectory qualifies. The mode is test-only and its status is recorded in the summary.

The continuous authored coupled run admits frame 1 with the unchanged full groom. This step takes 85963 ms (hair 85954 ms, skin 164 ms); it includes the first continuous-contact admission and is not a steady-state benchmark. A three-second native live sample identifies EqualityQr::solve, segment_pair, swept pair_fraction and polynomial subdivision as computation sites, with thread joins also visible. Preserve cross-thread sample counts as occurrence counts, not wall-time percentages. Subsequent frame/full-cycle qualification remains live; neither rendering nor 160 FPS is measured. See `authored-jump-continuous-full-cycle-2026-10-11/live-profile-analysis.json`.

## QR finite-validation traversal optimization

The live profile led to a bounded native optimization in `EqualityQr::solve`: sparse-basis orthogonalization validates finiteness while writing its existing FMA update, eliminating the subsequent scan over the same vector. Every coefficient, including exact zeros, still participates; arithmetic, original-input residuals and admission tolerance are unchanged. Dense-basis behavior is unchanged.

A paired standalone harness extracts baseline/candidate source and loads the historical rod-232 VQC1 operator (86 rows, width 1638). The first 20 active prefixes all solve and match coordinates/reactions bitwise. With 200 runs per timed batch, all seven alternating-order pairs improve; median candidate/baseline ratio 0.7288. Concurrent system work was not controlled, so this supports a local optimization, not a controlled application speedup. All 16 nonignored QR tests and 370 library physics tests pass (40 library tests ignored). The already-running continuous full-cycle process uses its earlier frozen binary and cannot qualify the newly edited QR implementation. Evidence: `qr-fused-finite-validation/`.

## Representable-progress guard for continuous component search

The live frozen run repeatedly reports rejected component scales shrinking to subnormal values. `component_scales_from` now checks the fixed contact graph after merging: if every rounded candidate endpoint in a collision-limited component equals its staged base, any smaller positive scale produces the same endpoint. Return the existing no-admissible-progress error so `advance_mode` activates swept constraints and recomputes its direction. This does not accept a blocked path, move roots, add a tolerance, or publish a frozen state.

The new regression constructs a collision-inducing prescribed-root trajectory with stationary free corrections, checks the expected rejection and unchanged input. Existing root-plus-free coherent motion still passes. All 29 nonignored safe-motion tests and 371 library physics tests pass (2/40 ignored respectively). No full-scene timing improvement or full authored jump qualification is claimed. The pre-existing live full-cycle process remains on its frozen earlier binary. Evidence: `stationary-contact-component/validation.json`.

The historical `flat-face-manifold-batched-full-capture/root-motion.vjr` replay was additionally tested against the progress guard. Serial/cached/parallel physical clock comparisons agree, but final component admission rejects with no-admissible-progress. This fixture is not qualified by the 371 passing ordinary tests. An isolated copied physics source with only the new guard removed is compiling to distinguish a pre-existing rejected historical trajectory from a regression. The updated full demo test binary is also compiling; the original full-cycle job remains live. Do not claim either pending build or baseline result as passed.

The isolated baseline comparison has completed: without the new guard, the same historical root-motion fixture also rejects, with `swept strand component admission did not converge` after exhausting its iteration budget. The candidate reports no-admissible-progress earlier. Thus rejection of this fixture predates the guard, although the error reason changes. This is rejection equivalence on one captured operator, not proof of full trajectory equivalence or a repaired motion. Both logs are preserved in `stationary-contact-component/`.

## Historical component 166/198 localization

The captured clock at zero free correction is limited by rods 166/198. Independent sampled segment-distance analysis localizes segment 0 against segment 2: initial gap +9.272e-8 m, gap -1.958e-8 m at tau=0.125, endpoint gap -7.791e-7 m for zero free correction. This is a sampled penetration counterexample, not merely an exhausted query. Root displacements are approximately 0.199 mm each; zero free correction includes all captured staged motion, not just roots, so this does not prove incompatible prescribed roots.

Full free correction improves the sampled pair distances, but public raw `sweep_capsules` queries still exhaust their 1024-iteration budget on two segment pairs. IterationLimit means unknown, not proven penetration. Production pair_fraction already includes selective outward-rounded line certificates and interval-plane certification; do not duplicate or bypass that mechanism based on the raw-query result. The captured full multi-component path still rejects and needs joint constraint refinement. Analysis scripts and logs: `root-pair-166-198/`.

The updated authored demo test binary now compiles successfully with both QR and representable-progress changes (`stationary-contact-component/updated-demo-build.log`). This build is not a new trajectory qualification; the pre-existing full run remains on its frozen earlier code.

## Rejected Bernstein-half micro-optimization

An isolated trial specializes interval multiplication by positive 0.5 during polynomial subdivision. All 2639 tested splits match baseline bounds bitwise, including signed zeros, subnormal/extreme/nonfinite/reversed interval inputs. However the long alternating paired benchmark consistently regresses, so no production polynomial code is changed. The benchmark includes edge inputs and concurrent machine load, and is not a full trajectory timing result. Preserve `bernstein-half-trial/validation.json` and the standalone source rather than retaining an unhelpful optimization.

## Updated continuous authored run

The prior frozen process was deliberately interrupted after 23 minutes with one completed frame because validated solver changes were now available. SIGINT termination and Cargo exit 101 were verified; `terminal.json` labels it interrupted, not passed or naturally failed. Its partial evidence and original running summary are preserved. A separate updated binary run starts from frame zero with the same all-guide 720-frame requirements, source/binary hashes and rejected-root-motion export enabled.

The updated run admits frame 1: elapsed 70543.498 ms, hair 70540.058 ms, skin 178.392 ms. Different uncontrolled machine load and initial-contact costs preclude an end-to-end speedup claim relative to the previous first frame. The run remains live; this is neither full trajectory admission nor rendered FPS evidence. See `authored-jump-continuous-updated-2026-10-11/run-manifest.json`.

## Frozen proposal from the updated authored run

A complete finite VJR1 snapshot from the updated live run is now frozen separately from its rotating exporter output (469 guides, 21 points each; full length/trailing-byte validation and SHA256). Replaying the component clock reproduces its rejection in 0.80 seconds without body import, full groom warmup or preceding frames. This is a geometric proposal capture, not a complete material/inertia checkpoint.

The updated captured limiter is pair 124/166, segments 0/4. Sampled initial gap is approximately 1.0e-17 m (touching); zero correction ends at -3.597e-6 m penetration. Full correction improves sampled separation, but does not qualify all other pairs or the whole continuous path. This explains why arbitrarily shrinking the correction cannot cure this proposal: separation supplied by the correction is required. Joint constraint refinement, with the original prescribed/staged clock, remains the next operator to qualify. Evidence: `authored-jump-continuous-updated-2026-10-11/frozen-root-motion-01-diagnosis.json` and `updated-pair-124-166/`.
# Additional ready-made hair integration review (2026-10-11)

Reviewed upstream README and repository metadata for [TressFX-Godot](https://github.com/ismailivanov/TressFX-Godot/tree/ad3d1faad2338a386e4baf4e0b00a560aab4dd0d), pinned commit `ad3d1faad2338a386e4baf4e0b00a560aab4dd0d`. MIT port of AMD TressFX; demo assets have separate licenses. This is an additional integration reference, not a validated Voxy dependency.

Useful ready mechanisms: GPU simulation, skeleton-weighted hair, per-frame distance-field collision, strand material, Blender groom exporter and cost display. Upstream ships macOS binaries; this does not establish a direct Metal backend or performance on our scene.

Adoption constraints explicitly documented upstream: simulation is once per rendered frame with a 50 ms timestep cap; colliders must be closed and the rest groom outside them; supported strand vertex counts are 4/8/16/32/64, so Voxy's current 21-point guides cannot be imported unchanged. ShortCut/PPLL transparency and deep shadow maps were not ported. Do not trade away hair density, shadow quality, fixed-step behavior or continuous-contact admission by blindly replacing our pipeline with this port. Prefer upstream AMD rendering mechanisms where the port omits them. No upstream timing is evidence of Voxy exceeding 160 FPS.

Seven pinned review files, including both licenses, compute/render shaders and upstream lighting/strand shaders, are stored under `sources/ismailivanov__TressFX-Godot/`; `source-manifest.json` records URL, commit, size and SHA-256 for every file. No demo assets were imported.

Source-level adoption decision: `tressfx_sim.glsl::update_follow_hair_vertices` adds a world-space follower offset scaled by tip separation. Voxy `female_hair.rs::append_surface` instead uses `p + basis * render_offsets[...]`, preserving the rod's local orientation. Transfer the persistent buffers and guide-then-follower dispatch ordering, not the port's offset formula. The GPU extraction acceptance case must compare every retained follower vertex against this existing CPU formula for rotated, bent and twisted guides, including the terminal point; preserve the 60 followers per guide and 21 guide points. Upstream `write_out_pass` also normalizes an unchecked adjacent-position difference; handle coincident endpoints explicitly rather than introducing non-finite tangents. These are implementation requirements, not completed GPU integration.

Current-code follow-up: Voxy already has resident fibre extraction in `voxy_render::FiberSurfaceInput`, `fiber_surface.wgsl`, `FiberSurfaceTransfer` and the `SceneApp` resident-hair branch. Do not build a duplicate system. Added `crates/voxy_render/tests/fiber_surface.rs` to qualify the existing compute path at 469 × 21 × 60 × 4 = 2,363,760 vertices on a real adapter, comparing all positions, local-frame normals, UVs, colours and terminal frames against a rotated/bent/twisted CPU reference. This synthetic full-size extraction test does not qualify authored simulation, the final scatter/draw path, camera smoothness or frame rate.

The full-density extraction test passed on Apple M4 Max / Metal: maximum position error 5.9606464e-8 m across all 2,363,760 vertices, with UV, colour and normal checks passing. Command: `cargo test --release -p voxy_render --test fiber_surface -- --nocapture`. Evidence and source hashes: `artifacts/articulated-jump-2026-10-10/full-density-fibre-gpu-2026-10-11/`. The test explicitly budgets one full-output readback; runtime resident rendering does not need that diagnostic transfer. This result establishes neither authored-cycle contact admission nor 160 FPS.

Draw-transfer follow-up passed: `scene::fiber_surface_draw_tests::fibre_scatter_preserves_draw_streams_and_neighbouring_geometry` checks 240 generated vertices at destination base 3, bit-exact separate vertex/normal streams, unchanged three-vertex prefix/suffix and rejection of an overflowing partition. It exercises the actual `FiberSurfaceTransfer` constructor and dispatch. This is boundary/ABI evidence, not full-size rendering evidence; log and manifest are beside the extraction artifacts.

The updated full coupled continuous-contact run is now terminal, exit 101 after 1065.96 s: 3/720 frames completed; frame 4 failed with `joint square-root active contacts did not converge` (2359 constraints, residual 2.6016433320517574, tolerance 1e-14). Source summary and terminal log are in `authored-jump-continuous-updated-2026-10-11`. Do not restart the same job or relax admission as a performance fix. Next diagnostic must preserve actual contact rows, native operator and timestep, because geometric VJR1 alone cannot replay this constrained solve.

Exact-operator diagnostic follow-up: the existing VQC1 exporter already records original physical matrices, zero-base RHS, contact load rows and original/effective bounds, sufficient to replay the constrained physical response without reconstructing rods. A separate diagnostic run enables `VOXY_HAIR_QR_INPUT_EXPORT` with the same binary/physics, under `authored-jump-frame4-exact-operator-2026-10-11`; it remains live and is not a successful trajectory claim. VQC1 is an operator snapshot, not a whole simulation checkpoint or an explicit timestep archive.

Independent OSS comparison: [OSQP](https://github.com/osqp/osqp) is Apache-2.0 and [documents numerical infeasibility witnesses](https://osqp.org/docs/solver/). `tools/compare_hair_qr_osqp.py` uses OSQP Python 1.0.5 in an isolated environment, audits the frozen VQC1 and solves the original-bound whitened QP, retaining every contact row and removing only exactly zero coordinates. It never publishes a physical state; a numerical infeasibility status is not a rigorous certificate. The historical 60-row capture returned solved after 2275 iterations, inequality violation 8.131516293641283e-20. The 513-row duplicate-load infrastructure case returned solved inaccurate after 20000 iterations, violation 7.172040739078511e-14, above the original 1e-14 tolerance; native replay passes that case. This contradicts treating an off-the-shelf solver as an automatic strict-admission replacement. Reports and exact Python package versions are in the frame4 diagnostic directory. No result exists yet for the pending frame4 physical operator.

Fixed a separately reproduced native active-set size defect: `UnilateralContinuation::poll` had a constant 512-iteration budget while adding at most one contact per iteration, including an initial zero-state check. Standalone builds of the actual old/new production source show old code admitting 511 identity inequalities but rejecting 512/513, while new code admits 512/513 with exactly unit coordinates and reactions. The bounded budget is now `max(512, rows + 1)`; no contact row, residual, material or admission tolerance is changed. This provides room for each independent row to enter; it is not a general convergence guarantee for cycling/releasing sets. Added the 513-row regression. All 372 nonignored physics library tests passed (41 ignored), and historical physical-operator plus duplicate-load replays passed. Evidence: `artifacts/articulated-jump-2026-10-10/contact-active-budget/`. The live frame4 capture intentionally uses the original app binary to retain the original failure; its cause remains unproven until the exact operator is available and replayed with the fix.

Frame4 capture is now terminal (exit 101, 960.22 s, 3 completed frames): the failing island has 8 systems, 141 rows and 1008 coordinates, refinement 1, tolerance 1e-14. Frozen VQC1 SHA-256: `7f133fb9d00f2e97c6700d767fad4b68f9f5bb654abb0131be27d701087d36cb`. Fixed-budget native replay still fails in 0.18 s, so that independent defect does not resolve this island. OSQP reports primal infeasible for the rounded whitened problem, with only an approximate witness (transpose residual 8.3974e-5); do not infer incompatibility of original physical loads from it.

HiGHS feasibility proposals against original physical load rows were verified with exact rational arithmetic on all binary64 coefficients. Both the initial proposal and a numerical minimum-max-coordinate proposal satisfy every original inequality strictly (minimum exact gaps approximately 9.988e-11 and 9.994e-11). The latter has maximum translational coordinate 40.9807114 m and zero angular coordinates. This is a feasibility witness, not a physically acceptable hair correction, an exact optimum/lower-bound certificate, force balance, energy minimization or continuous-motion admission. `tools/verify_hair_qr_feasibility.py` independently checks the JSON coordinate witness, fixed coordinates, every exact original load product and capture hashes; zero-coordinate counterexample is rejected. Evidence and witnesses are in the frame4 diagnostic directory. Next inspect continuous-cut trust scaling and linearization validity, without relaxing admission. Updated app build with the iteration-budget fix succeeded; no new full-scene qualification was claimed.

Shared contact-clock correction: swept-cut refinement previously rebuilt base endpoint planes for `staged + free`, while its swept cuts and continuous guard evaluated `staged + scale * free`. Added canonical `position_constraints_on_clock`; cut rebuilds now apply the current per-rod trust scale to both mesh and strand base Jacobians. Original geometric bounds stay unchanged, and normal assembly recomputes diagonals/aliases rather than mutating an already prepared response. Ordinary unscaled contact construction remains on its existing path. The new regression demonstrates that an unscaled 20 micrometre correction satisfies a base plane but leaves 15 micrometres penetration after a 0.25 trust scale; the rebuilt row rejects it and matches the actual endpoint residual. Production and serial reference cut assembly both use the corrected clock. All 373 nonignored physics tests passed (41 ignored), including worker/reference comparisons. Evidence: `artifacts/articulated-jump-2026-10-10/shared-contact-clock/`. Initial compilation hit a transient full disk; only our clean temporary TressFX clone was removed, pinned sources retained, and retry passed. App rebuild/full coupled trajectory qualification remain pending; no claim that this resolves frame4 or establishes FPS.

Retained-clock follow-up: a new Newton increment can change trust scales without creating a new geometric witness. Both production and reference activation now rebuild retained rows in that case. The regression retains a clear endpoint witness, changes the component clock from 1 to 0.5, verifies base/witness exact aliasing and the recomputed diagonal, then verifies an unchanged clock settles without another rebuild. All 374 nonignored physics tests passed (41 ignored). The actual GPU coordinate backend passed seeded release, dependent-hint cold retry and exact serial/batch agreement on the prior shared-clock binary; that test does not exercise the retained-clock geometry update. Final app rebuild followed conditionally by a fresh full authored-cycle qualification is scheduled under `authored-jump-shared-contact-clock-2026-10-11`; the shell aborts if build or fresh-manifest creation fails. No full-cycle or FPS success is claimed before its terminal result.

Small-coefficient diagnostic, not yet integrated: extracted the actual `add_constraint` owner with the original math helpers into `artifacts/articulated-jump-2026-10-10/contact-small-coefficients/`. Current code silently omits a nonzero gradient 1e-20 because diagonal 1e-40 is below its 1e-30 cutoff, and omits 1e-200 because squaring underflows before structural-zero detection. An isolated candidate checks exact coefficient zeros, retains every positive representable diagonal and explicitly rejects movable-support diagonal underflow; it retains 1e-20, rejects 1e-200 and preserves zero-row omission. These are assembler counterexamples, not proof of a feasible physical correction or the cause of frame4. Keep production sources unchanged until the in-flight shared-clock build records its manifest, then integrate and run the complete physics gate; do not misattribute a new source hash to an already compiling binary.

Added `captured_full_joint_operator_preserves_physical_admission`: supports up to 4096 rows, streams original systems, skips serialized whitened columns and recomputes them through the canonical native solver, then validates original loads and inequalities. This avoids retaining raw file bytes and duplicate captured whitening matrices. File-length/dimension/finite-load checks precede physical replay. It passed a synthetic 513-row duplicate-load case and an existing real 20-system/60-row capture (2520 coordinates), both at tolerance 1e-14. This validates replay infrastructure, not the pending 2359-constraint failure. `tools/audit_hair_qr_input.py` separately checks all serialized finite values and SHA-256 without allocating dense matrices. Logs/manifests are in the diagnostic directory.

### Small nonzero contact coefficients (2026-10-11)

An isolated replay of the actual constraint assembly showed that `g=1e-20`, mobility 1 produced a finite positive diagonal of `1e-40` but was silently discarded by the `1e-30` cutoff. Squared-norm zero tests also mistook `g=1e-200` for structural zero. Constraint assembly now retains every positive diagonal, tests structural zero componentwise, and uses a weighted hypot fallback only when the original diagonal is zero or nonfinite. This preserves representable weighted diagonals for `(g,mobility)=(1e-200,1e200)` and `(1e200,1e-200)`. Genuine weighted diagonal underflow returns an explicit error. Invalid mobility is rejected. Native and batched response membership use the same exact componentwise test.

The regression checks original gradient bits, weighted extremes, explicit underflow, root/pinned/zero entries, exact cancellation, and invalid mobility. This addresses numerical row preservation, not proof of full contact convergence, GPU equivalence, or real-time performance. The already-running shared-clock 720-frame qualification uses its separately hashed older binary and does not validate this change.

The shared-clock full qualification is terminal: 3/720 frames completed, frame 4 failed with `swept contact refinement budget exhausted` after 306.36 seconds. This replaces the prior QR failure on this particular trajectory but is not successful full-cycle qualification. Its VJR1 captures geometry and candidate increments, not a physical Hessian checkpoint. The final log, binary/source manifest, summary and terminal status are preserved under `artifacts/articulated-jump-2026-10-10/authored-jump-shared-contact-clock-2026-10-11/`. Do not increase the refinement cap without inspecting how retained witness planes and trust clocks evolve.

Small-load physical response regression: native and batched responses at load scales 1e-20 and 1e-155 retain positive compliance and linear/angular response. Normalize the response back to a unit load before original-system H*x=b admission, so the absolute residual floor cannot hide a missing tiny response. Forward comparison uses 1e-8 relative to the full response (the physical validator tolerance); an initial 1e-12 criterion exposed approximately 2e-11 forward rounding sensitivity. Final physics gate: 376 passed, 41 ignored, zero failures.

### IPC Toolkit trust-region review (2026-10-11)

Pinned `ipc-sim/ipc-toolkit` at `0985885fecf157ac7c065c5f461028cc0aebe228`, MIT. Five files are retained with SHA256 provenance in `sources/ipc-sim__ipc-toolkit/source-manifest.json`: license, TrustRegion implementation/header, tests, OGC tutorial. Source review only; no IPC build or runtime qualification performed.

`TrustRegion::update_if_needed` refreshes geometric regions around the current positions, then clears its update flag. `filter_step` clips the proposed displacement against fixed region centers/radii and requests a region refresh when enough vertices were restricted. `planar_filter_step` computes crossing-based truncation ratios per candidate, reduces them per vertex, retains an isotropic inflation-radius fallback for distant primitives, and then scales each affected displacement. Thus the implementation still scales full per-vertex displacement; the header's direction-aware wording must not be read as an implementation of a free tangential projection.

Relevant architectural lesson: geometric admissible regions have explicit frozen centers and refresh ownership. This differs from repeatedly putting a proposal-dependent trust scale into a linear contact row and then recomputing that scale from the solution of the scaled row. The retained Voxy scalar analogue demonstrates that this latter scheme has no positive fixed point when the required scaled correction exceeds the trust bound. It does not prove which exact full-model planes cause the observed failure. The terminal capture shows guide 421 with an unaccepted 224151 m candidate translation and 14006853 rad candidate rotation, yielding trust scale 2.4988e-8. Trace qualification is needed to establish the iteration history.

No IPC code copied into the production rod solver: its positional regions, barriers, codimensional offsets and treatment of prescribed motion require an explicit adaptation and preservation of Voxy's Cosserat rotations, original physical force balance, and whole-path CCD.

### Witness-query worker qualification (2026-10-11)

Six immutable full-groom VJR1 captures were replayed through the original witness builder, current serial builder, and explicit eight-worker builder. Three alternating-order repetitions per capture preserve exact changed flags, constraint gradient/bound bits and group identities. All six passed. Parallel median activation time was lower for all six (1.33x–4.64x speedup versus current serial), but paired samples were faster in only 14/18 comparisons and host contention caused substantial variance. This does not establish universal latency reduction or total frame FPS. Evidence: `artifacts/articulated-jump-2026-10-10/witness-worker-qualification-2026-10-11/`.

Production now selects up to eight workers only for at least 1024 broad-phase pairs, matching the existing narrow-phase query policy. Workers read immutable geometry and publish results canonically only after all queries succeed. Small queries remain serial. The captured regression also compares the automatic production selection with explicit serial/eight-worker/reference outputs. The already-running full-cycle trace binary predates this worker-policy change.

Worker policy final validation: 376 physics tests passed (41 ignored); all six captured witnesses passed reference/serial/eight-worker/automatic-worker equivalence, three repeats each. Full trace qualification is now terminal: frame 4 refinement exhaustion after 3/720 completed frames, 534.99 seconds. Late iterations 28–32 each add one witness while refreshing 49–53 retained clocks; minimum trust shrinks geometrically to 4.2221e-8. Thus zero clock-only events does NOT rule out scale feedback when a new witness is also added. See the final trace-analysis and failure-interpretation JSON for observations and limits. This full trace predates the production worker-policy change.

### Bounded temporal refinement, single ownership (2026-10-11)

FemaleHair already owns transactional temporal retries for final axial-strain admission. Extend that existing owner to `swept contact refinement budget exhausted` rather than adding a second nested retry loop inside HairSystem. Each retry clones the original admitted HairSystem, doubles the effective substep count (including the implicit 240 Hz minimum) up to 32, and resamples the same root/collider interval. Material parameters, guide density, force-balance admission and continuous path guards remain unchanged. Exhaustion preserves the original error and public state. Accelerator observations remain external to rollback. The original configured substep count is restored only after success.

HairSystem now records `last_step_substeps()` independently of its configured substep count. The public clock is committed only with an admitted state; a rejected body-admission callback cannot publish the attempted clock. FemaleHair validates with this actual clock. Physics gate: 377 passed, 41 ignored. The app integration test injects a known contact rejection after one real structural substep, then compares the retried groom to a direct finer-step run; it isolates orchestration from geometry, and is not a replay of the frame-4 collision. App test and subsequent full-cycle qualification are pending.

This is a bounded integration retry, not proof that the proposal-dependent scaled-contact fixed point itself is repaired and not a real-time performance qualification.

Temporal refinement integration gate passed: the actual 469-guide groom after an injected rejection in structural substep two matches the direct four-substep baseline, including all rod debug-state fields and targets; source state/configuration remain unchanged. Test duration 4.08 s after a successful release build. This isolates retry orchestration, not the frame-4 collision. The new full-density 720-frame qualification is running under `artifacts/articulated-jump-2026-10-10/authored-jump-adaptive-substeps-2026-10-11/`, with binary and source hashes.

Temporal exhaustion regression passed on the actual 469-guide groom: an unrelated accelerator failure makes exactly one call; contact-refinement and axial-strain failures each make five calls at 2/4/8/16/32 substeps. Targets, collider surface query and entire HairSystem debug state remain unchanged for all three cases. Test runtime 13.15 s. Initial attempt never executed because disk exhaustion truncated the stripped executable; the verified broken generated file and one inactive ignored debug incremental cache were removed, then the rebuild/test passed. Running full-cycle binary was separately frozen with matching SHA256 before rebuilding.

### PPF rod and driver compatibility boundary (2026-10-11)

Added three Apache-2.0 source files at existing pin `618f85b667b7f211f762908690bc46f65b578ae0`: Rust driver/step.rs, neutral rod_force.kernel.cpp and energy/model/rod_bend.kernel.cpp. The reviewed bending interface takes three Vec3f positions and a rest angle; stretching takes endpoint positions/rest length. These specific kernels do not take independent material-frame orientations and therefore do not establish equivalence to Voxy's rotational Cosserat state, torsion or bend/twist coupling. This is a statement about the inspected interfaces, not an exhaustive claim that every PPF branch lacks those features. Replacing the full Voxy rod model with these terms would leave explicit fidelity requirements unqualified.

The step driver confirms an iteration-cap branch only when `max_newton_steps > 0` (and not final_step), rather than treating a tiny positive time of impact as guaranteed physical progress. Its reviewed host path uses fp32 per-element state and host scalar/reduction stages; copying that loop does not prove a GPU-resident implementation. Retain canonical physical admission and explicit bounded failure in any port. No PPF runtime build or numerical-parity claim was made here. Normal/optimized source-integrity audits agree: ten mechanism manifests, 63 verified files. Evidence: `artifacts/engine-corpus-audit-2026-10-11/ppf-rod-driver/`.


### Adaptive full-cycle terminal failure and exact original-load check (2026-10-11)

The full-density run is terminal after 3/720 frames, 767.78 s, not still running. Frame 4 actually retries two to four substeps, then fails square-root contact convergence on one rod with 14 rows and 126 coordinates (VQC1 SHA256 `9a7f84a54e50f2dd7c1cccc8e47d63aaf99a25189a2664597c320fde6182767b`). Native frozen replay reproduces the failure.

OSQP/HiGHS numerical infeasibility statuses do not establish original-load incompatibility. Rational simplex with explicitly split positive/negative variables proposes a feasible vector; the independent standard-library verifier checks the rounded binary64 JSON vector against every exact original load and fixed coordinate. All inequalities hold with minimum exact gap 9.761803810491436e-9. Maximum translation is approximately 4.282 million metres: this is not a physically admitted correction or a minimum-norm result. Rows 6/13 are nearly opposing with normal sum approximately 1e-11 and positive bound sum 8e-5; their numerical half/half dual witness fails exact transpose cancellation. SymPy unrestricted-variable and dual simplex attempts failed and were not used as proof.

`tools/verify_hair_qr_infeasibility.py` checks nonnegative rational weights, exact cancellation at every original coordinate and strictly positive weighted bounds. Positive synthetic, perturbed, zero and negative witnesses were checked; the actual approximate dual witness is rejected. Evidence is in `authored-jump-adaptive-substeps-2026-10-11/{original-feasibility-proof,exact-primal-lp-result,rejected-numerical-dual-witness,failure-interpretation}.json`. Next capture contact-row provenance and compare linearization/refresh ownership to the reviewed IPC OGC frozen-region mechanism; no production fix or FPS claim follows from this diagnostic.


### Failed-island provenance capture (2026-10-11)

Added opt-in `VOXY_HAIR_FAILED_ISLAND_CONTEXT=<directory>` at the native per-island solve owner. On failure it captures that specific original island input and a matching sidecar with global rod/row identities, every gradient/mobility/bound bit, frozen positions and available frozen contact source, normal, target, barycentric fraction, metric and trajectory time. No target rod or observation ordinal needs to be guessed beforehand. The captured bounds are explicitly the original island input before numerical refinement, not a claim to capture any later adjusted RHS. Batched backend failures are not covered by this hook.

A synthetic two-plane incompatible contact regression rejects the correction and preserves all rod debug state and zero published multipliers. The emitted VQC1 passed structural audit; both metadata rows reconstruct the original physical load vectors exactly; mesh source IDs 7/8 and frozen positions survive. Full physics gate: 378 passed, 41 ignored. Evidence: `artifacts/articulated-jump-2026-10-10/failed-island-context-regression/`. Actual frame4 provenance still requires a new coupled capture; this diagnostics change does not repair the contact model or qualify rendering FPS.


### Necessary translation bound for the actual 14-row failure (2026-10-11)

`tools/bound_hair_contact_translation.py` derives a necessary bound using original physical load rows 6/13. Adding the two inequalities and applying the exact L1 coefficient bound proves that ANY feasible correction requires at least one translation coordinate with absolute value >= `368934881472237/136350208` metres (approximately 2,705,788.916 m). Both rows contain only translational loads; all original coordinates are included. Thus the earlier million-metre feasible witness is not merely an unfortunate LP proposal: the current affine model itself excludes every correction bounded below this scale. This is not proof that nonlinear scene geometry is infeasible or that the unrestricted problem is globally physically meaningful. Evidence: `exact-translation-lower-bound.json` in the adaptive full-cycle directory. A known exact synthetic bound, angular-load rejection and zero-slope rejection passed. Next obtain source/model ownership of these planes before changing their validity/refresh policy.


Full coupled provenance qualification launched after successful app release test build (2m01s), frozen binary and source hashes recorded in `artifacts/articulated-jump-2026-10-10/authored-jump-failed-island-provenance-2026-10-11/run-manifest.json`. Process 43670 was verified running at 32 seconds with advancing nonlinear contact diagnostics. Exec session 18122 owns the run and records its terminal exit; no completion is implied. It uses the original 469-guide groom and captures failed island context plus nonlinear contact replay. Poll this existing process/session rather than restart on observation timeout.


Added `tools/analyze_hair_island_context.py`: checks sidecar dimensions and reconstructs every original physical load row against its paired VQC1 before reporting exact unscaled frozen-contact endpoint-Jacobian matches. Synthetic source IDs 7/8 matched separately; a one-bit row corruption was rejected. Matches are candidate frozen contacts, not proof of triangle-feature origin or historical plane creation. Clock-scaled/unmatched rows remain untouched. The existing provenance qualification process was verified still running with advancing nonlinear diagnostics at 1m50s; no restart or completion claim.


Persistent exact-evidence regressions: `python3 -W error::ResourceWarning -m unittest discover -s tools -p test_hair_contact_evidence.py -v` passes five tests. These distinguish true opposing-plane infeasibility from nearly opposing normals, independently check an exact necessary translation bound of 2**41 for a synthetic near-opposition, reject angular loads/zero slope/invalid rows and invalid dual weights. Fixed an unclosed witness-file handle exposed by these checks. The existing full provenance run was verified live at 2m47s with advancing nonlinear diagnostics; no failure context had yet been produced.


### Actual failed-island provenance, terminal (2026-10-11)

The provenance run is now terminal exit101 after 656.18 s, 3/720 frames, frame4 same square-root failure following the actual 2-to-4 substep retry. Frozen failed-island VQC1 has exactly the prior capture SHA256 `9a7f84a54e50f2dd7c1cccc8e47d63aaf99a25189a2664597c320fde6182767b`. Sidecar original loads audited successfully; all14 rows match frozen mesh0 contacts of rod199. Local rows6/13 are global rows215/222, both segment19/fraction1 at the terminal point. Row6 is endpoint geometry (trajectory_time=None), physical gap ~-4.72e-17; row13 is trajectory_time=1 with physical gap ~-8e-5 and nearly opposite normal. Their targets differ by approximately 80um along the normal. This establishes a conflicting endpoint/trajectory linearization, not yet its triangle-feature or historical creation provenance. A 9.8MiB nonlinear replay was saved. `provenance-findings.json` and `failed-islands/{context-analysis,opposing-contact-geometry}.json` retain exact source associations and decoded geometry. Next examine unsigned trajectory witness orientation versus the initial safe side and signed closed-mesh geometry. No contact model fix, full cycle or FPS qualification follows yet.


### Current trajectory refresh does not reproduce the opposing terminal plane (2026-10-11)

The ignored native `captured_terminal_trajectory_side_probe` loads the exact saved nonlinear replay and asserts rod199 terminal position equals the failed frozen pose. Current final-segment trajectory discovery returns one witness at time1/fraction-0 (the other endpoint of segment19), normal aligned +0.9928378733 with captured endpoint row6. It returns zero opposing time1/fraction1 witnesses. The initial hypothesis-check test expected an opposing witness and failed; retained as diagnostic evidence, not a passing gate. The corrected reporting probe compares against the captured normal and passes the exact-pose assertion. Thus current trajectory discovery does not recreate row13; next inspect retained tangent-model lifetime/installation and fresh static rows, rather than immediately change triangle normal orientation. Evidence: `terminal-side-probe.log`, `terminal-side-initial-hypothesis-rejected.log`, `trajectory-refresh-findings.json`. No production physics change or cycle qualification claimed.


### Pose-scoped speculative contact model (2026-10-11)

Implemented explicit invalidation of rejected-probe planes after an accepted nonlinear pose change (positions or rotations). Same-frozen-pose retries retain their planes and exact alias semantics. The next iteration rebuilds signed mesh and whole-path trajectory contacts; CCD, material/strain/force admission and published-state transactions remain in place. This follows the reviewed IPC principle that local contact regions have explicit refresh ownership; no IPC code or solver was copied. Retaining every speculative halfspace across changed poses had no valid global nonconvex-surface interpretation.

New regression retains a rejected opposing plane in a same-pose retry, then invalidates it without modifying a newly queried outward contact or pose after movement. Three model tests and the full physics gate pass:379 passed,42 ignored. Actual frozen full469-guide nonlinear replay launched (process61700,exec64081), with immutable binary/source/fixture hashes under `contact-model-pose-lifetime-2026-10-11/`. It reconstructs fresh geometry from the saved pose, not the unsaved original speculative model history. No replay completion, full authored trajectory or FPS claim yet.


Pose-lifetime replay is TERMINAL exit101 after12.27s, not running. Rebuilt initial merit exactly equals captured2.9367614931589525e-8. Iteration0 rejects a candidate and adds301 speculative rows; iteration1 fails the same14-row island before any accepted movement. Thus post-move invalidation alone does not repair this case. Keep its local model-lifetime invariant separate from the unresolved first rejected-probe plane orientation/validity. Next capture that rejected candidate and determine a safe-side/CCD-entry contact mechanism instead of accepting all unsigned deep-penetration planes.


### Local geometry descent before speculative enrichment (2026-10-11)

Moved tangent-model enrichment after the existing bounded geometry line search. A far rejected probe no longer automatically supplies planes before testing smaller real-geometry steps. Only the last failed nearby trial may enrich the same-pose model; accepted moves still invalidate speculative planes. Clearance, constitutive/strain admission and whole-path guards remain unchanged.379 physics tests pass,42 ignored.

Full469-guide frozen replay now accepts iteration0 at scale0.04420399375 (merit2.936761493e-8 ->2.777616447e-8), then iteration1 reaches4.795694593e-9. It still fails at iteration2 after11.57s, but now a fresh7-row island of rod199 exposes endpoint row0 and time1 trajectory row5 with nearly opposing normals. No speculative enrichment occurred before this failure. Therefore stale rejected planes were not the sole mechanism; the current geometric path can itself generate opposing signed-endpoint and trajectory-side planes. The earlier fresh-trajectory probe covered only the original pose, not these subsequently accepted poses. Evidence: `contact-local-descent-2026-10-11/{qualification.log,context-analysis.json,translation-lower-bound.json,validation.json}`. Next inspect the new frozen endpoint pose and trajectory witness face side against the saved mesh before modifying orientation.


### Signed closed-face trajectory witness correction (2026-10-11)

Native probe on the exact new7-row frozen rod199 pose identifies offending triangle vertex IDs[8320,8334,8837], closest feature Face, signed separation approximately-8.936208764e-7m. Its unsigned trajectory normal had dot=-1 with face winding, while endpoint geometry used the outward winding normal. This is an actual side-branch discrepancy, not inferred solver conditioning.

`trajectory_contact_oriented` now applies validated closed-face winding at nonzero separation too, and reports the selected witness signed physical gap dot(p-q,n)-radius. Unsigned discovery still finds candidate witnesses only; full CCD remains admission authority. Open sheets and nonzero edge/vertex witnesses retain their existing behavior; codimensional feature orientation is not broadly solved by this change. Regression covers both winding directions, nonzero back-side face, signed gap, endpoint row and outward correction.380 physics tests pass,42 ignored. Actual face probe now has dot=+1 and zero opposing terminal witnesses. Before/after logs, source/binary/fixture hashes are in `contact-oriented-face-2026-10-11/`. Full469-guide nonlinear replay process69642/session20983 was verified live at35s and has advanced beyond previous iteration2 failure to iteration7; not yet a converged/admitted replay or full720-frame/FPS qualification.


Oriented-face nonlinear replay is TERMINAL exit101 after98.56s. It no longer fails square-root contacts, but reaches iteration12 `joint hair contact direction has no descending geometry step`, final merit1.923492150922055e-11 and preceding worst penetration~4.0123um. No failed-operator capture was created because linear solves complete. A separate direction/geometry diagnostic run uses the same frozen binary and fixture, with existing rejected-direction/contact-replay exports and contact-parts trace, under `contact-oriented-face-direction-2026-10-11/`; this is a terminal-result-driven diagnostic, not a timeout restart. Inspect physical directional derivatives before changing tolerances or iteration budget.


### No-descent directional capture terminal (2026-10-11)

Same frozen binary/fixture diagnostic is terminal exit101 after38.60s (different host timing), same iteration12 no-descent. Saved1.3MiB rejected-direction JSON plus nonlinear failure replay. All169 violated captured mesh rows have nonnegative directional gap; dominant rod291 depth4.01232339um has predicted correction+4.01232339um. The last scale1e-12 moves any coordinate at most2.427040086e-16m, yet refreshed rod409/segment19 worst depth changes by2.47805999e-10m. Its normal also changes from approximately[.21318,.07994,-.97374] to[.21012,.07837,-.97453] at effectively identical barycentric fraction. Resulting mesh merit increase3.14228e-17 dominates the expected tiny descent. This establishes a captured geometric-refresh discrepancy, not its root cause: inspect closest-feature/support witness selection, nearest cache and segment/triangle distance stability on the new saved replay before relaxing any gate. Evidence: `directional-merit-analysis.json` and `rejected-direction.json`. No remaining live qualification from this run.


### Native rod409 query isolation and IPC distance reference (2026-10-11)

An ignored native probe loads the new no-descent replay, repeats rod409 static refresh at the same pose and overlays exactly the final captured trial coordinates. Static refresh repeats identically, nearest face59447/Edge(1,2) remains stable and its point-distance gap stays about-6.33781186e-8m. Adding whole-path trajectory refresh reproduces the anomalous time1 row: trial gap-6.362593451e-8 with normal[.21012,.07837,-.97453], whereas a second time1 row and static query remain stable near-6.33781186e-8. This isolates the discrepancy to a trajectory witness, not broad nearest-cache nondeterminism. Next inspect that witness's own triangle/closest-point feature and normal branch, not just the globally nearest face.

Retained three more MIT IPC Toolkit source files at existing pin0985885fecf157ac7c065c5f461028cc0aebe228:distance_type.cpp,edge_edge.cpp,line_line.cpp. Source dispatch separates point/line/line-line metric branches; near-parallel edge classification uses a precision-relative branch. This is a reference to evaluate, not an imported threshold or proof of adequate1e-10 admission accuracy. Source-integrity audit passes1428 records,500 classified identities,10 manifests,66 files; all1428 remain root/README review depth. Evidence: `contact-refresh-precision-2026-10-11/`.


### Query-point closest-feature classification (2026-10-11)

Native per-face probe identifies the rod409 witness on the shared edge of triangles [50886,49374,50890] and [50890,49374,49379]. Both query-point features are edges with identical closest point and metric gap about -6.337811863e-8m. Reclassifying the rounded projected point q incorrectly selected Face on the second triangle, replacing the edge metric with its face-plane distance (-6.362593450e-8m). trajectory_contact_oriented now classifies the original query point p, consistently with static contact. No admission tolerance, density or CCD gate changes. Numeric regression reproduces the actual triangle and trial segment. All381 release physics tests pass,43 ignored; actual native trial overlay now keeps the edge metric within floating-point roundoff. Evidence and immutable source/binary/fixture hashes are in contact-query-feature-2026-10-11. Full469-guide replay of the last accepted no-descent pose is running separately; local branch correctness is not full jump or FPS qualification.

Query-feature replay is TERMINAL exit101 after35.93s, iteration20 no-descent. Worst preceding gap fell from approximately4.008um to13.933nm, but remains above unchanged admission tolerance. Iteration19 retained33 nearby rejected-candidate rows; next direction still fails descent. This is improved geometry and a validated feature fix, not a converged full469-guide replay, full720-frame jump or FPS proof. New rejected-direction and failure replay are retained for the next residual investigation.

Residual frozen-pose diagnostic replay is TERMINAL exit101 in5.21s. All33 currently violated mesh rows have positive predicted outward directional gap; predicted merit slope -6.593900172e-16. Full candidate moves coordinates up to0.385mm in original direction capture and introduces large fresh penetration (direct replay worst rod341/segment19 endpoint approximately435um). Backtracking does not achieve descent; final trial merit increase6.741e-28 comes from rod39/segment17 while dominant residual13.933nm stays unchanged at coordinate roundoff. IPC retained trust_region.cpp explicitly bounds displacements by candidate-safe distances and refreshes candidates after enough restrictions; this source mechanism motivates investigating missing contacts/feature validity along our proposed direction, not copying its numerical cutoffs. Per-trial per-row opt-in trace added to isolate which rows activate at intermediate scales.

Per-row residual replay TERMINAL exit101.381 physics tests pass,43 ignored. At scale1e-6 fresh rod152/segment16 and rod291/segment16 dominate growth; at1e-7 a fresh rod367/segment17 excess94.378pm appears; at1e-8 a fresh rod355/segment17 excess104.842pm appears despite absent original segment contacts. These contributions outweigh the tiny predicted descent. Next isolate the actual moving-triangle witnesses and minimum search for those newly activated rows; evidence does not yet distinguish real feature activation from refresh/discovery error. Full log and extracted new-contact rows retained in contact-residual-rows-2026-10-11. No tolerance relaxation or solver change in this diagnostic pass.


### Endpoint identity and segment-partition merit (2026-10-11)

Native time-aligned probes on rod355/segment17 and rod367/segment17 find the same physical witness before and after tiny trial. Before fraction is exactly1, so mesh_motion publishes segment18/fraction0; after fraction is0.9999999999999998 or0.9999999999999997, so publication remains segment17. The adjacent segment already supplies that endpoint contact, and per-segment maximum merit then counts the same violation twice. The previous apparent new-segment activation is thus not evidence of a newly intersected surface.

trajectory_contact_oriented now uses exactly closest_points' a+(b-a)*time interpolation and recognizes exact endpoint equality before reconstructing an interior fraction. Face interpolation is aligned too. No epsilon snapping or physical admission relaxation. Actual rod355 trial witness now has fraction1 and unchanged physical depth approximately0.205nm. Numeric regression retains original and trial coordinates plus moving triangle. Pre-regression suite381 passes; new test suite and full469-guide replay are running. Replay has progressed through accepted steps beyond its previous no-descent pose, but steps remain very small and no convergence/FPS qualification is established. Evidence: contact-moving-witness-2026-10-11.

Endpoint numeric regression PASSES; complete release physics suite382 passed,0 failed,43 ignored. Full469-guide replay PID96772/session47498 verified live at46s, iteration16, worst gap13.93332nm; only tiny descent. Do not restart or treat as terminal from observation timeout. Next investigate how fresh near-active contacts constrain the full elastic proposal without reintroducing far rejected-probe planes.

Endpoint-identity replay is TERMINAL exit101 at78.75s, iteration31 contraction-budget stop, worst gap13.933256nm. No remaining live process from that run. Endpoint identity fix remains independently qualified; residual convergence is not. Next mechanism retains original-pose-admissible contact planes from the nearest rejected bracket trial before accepting a smaller descent step, then rebuilds the common elastic solve at the same frozen pose. This targets repeatedly rediscovered approaching contacts; plane admission is a necessary locality check, not a global nonconvex clearance proof, and final refreshed geometry/whole-path CCD gates remain authoritative.


### Approaching-contact model refresh before tiny acceptance (2026-10-11)

Common nonlinear backtracking now retains mesh rows from the nearest rejected bracket trial when that trial penetrates but the frozen pose satisfies the same physical plane within the existing1e-10 admission gate. Before publishing the smaller accepted step, any new such rows rebuild the same elastic solve at the frozen pose. Rows are still discarded after actual accepted motion; far full-proposal planes are not automatically imported. This is local constraint generation, not a global nonconvex validity certificate: refreshed geometry and whole-path CCD still own final admission. ContactModel row merging uses its existing exact endpoint Jacobian ownership.

382 release physics tests pass,43 ignored; pinned-source audit passes. Full469-guide residual replay uses immutable binary/fixture/source hashes in contact-approach-model-2026-10-11. Process622/session33599 started after an initial launch delay; four same-pose enrichments added1,6,17,8 rows, then iteration4 accepted a full scale1 step with tangent_complete=true, merit3.2545e-16 ->1.59395e-17 and worst gap13.933nm ->3.365nm. Subsequent geometry remains unresolved, so this is not yet convergence or full-jump/FPS qualification.

Approach-model replay TERMINAL exit101; test runtime85.27s, total launch-to-exit106.83s including observed initial launch delay. Iteration32 no-descent, worst gap-1.09670593287646055e-10m versus unchanged1e-10m tolerance. Full steps reduced initial13.933nm penetration to approximately0.10967nm, leaving approximately9.67pm excess.382 physics tests pass; full469-guide replay still fails and full720-frame/FPS remain unqualified. New rejected direction and nonlinear fixture retained for residual row precision/branch investigation. No active replay from this run.


### Preserve the nearest nonempty counterexample (2026-10-11)

Residual replay of the approach-model failure is terminal exit101. Per-row original-plane evaluation shows8 admissible-origin/penetrating-trial rows at scale1e-6, then zero such rows at1e-7 and every smaller failed trial. The backtracking code replaced its useful stored counterexample with each empty set, and consumed the set only if a descent trial was found. Thus remaining9.67pm excess was not proof that the enriched elastic model itself cannot progress: it had lost approaching constraints before rebuilding.

Backtracking now retains the nearest nonempty set and consumes it before branching on whether descent was found. The same frozen-pose rollback, exact-Jacobian merge, original-plane admission gate and final geometry/CCD admission remain in force. Analysis with all candidate rows and scales: contact-nearest-counterexample-2026-10-11/counterexample-analysis.json. Intermediate382-test suite passed; final-source build and actual469-guide residual replay still required.

Nearest-nonempty final-source physics suite382 passed,0 failed,43 ignored. Actual469-guide residual replay PID6524/session46750 consumes8 formerly lost rows atiteration0 with descent_found=false, then18,11,5,1 additional rows. Iteration5 accepts scale0.1 with zero excess-penetration merit and worst gap33.214pm, inside unchanged100pm tolerance, but tangent_complete=false. The next same-pose elastic solve is still being constrained by further approaching rows; final CCD/convergence remains unproven. This intermediate geometrically admissible pose is not a completed replay.

Nearest-nonempty replay TERMINAL exit101,104.60s. Iteration16 no-descent begins with zero excess merit, while full elastic proposal has merit1.4534e-20. No replay process remains. Strict trial_merit<merit cannot accept any feasible trial when merit=0; this rejects elastic progress on the feasible manifold independently of CCD. Next acceptance branch uses strictly decreasing actual implicit structural/inertial Cosserat energy only when both before and trial excess merit are exactly zero. Contact discovery penalty is excluded from that objective because its changing row set is not a physical structural-energy change. Final complete-step, geometry, strain and CCD gates remain unchanged.

Feasible-elastic branch release suite382 passed,0 failed,43 ignored. Full469-guide residual replay TERMINAL SUCCESS exit0,82.07s test runtime. Iteration30 full scale1, tangent_complete=true, strand_admitted=true, unresolved=false, zero excess merit, worst gap4.3283pm versus100pm admission tolerance. reconcile_positions_mode completion executes whole-path mesh admission; ignored replay also asserts every root unchanged and max relative stretch<=5%. This qualifies the complete nonlinear solve for this saved residual pose only, not original frame4 capture,720-frame jump or FPS. Binary/source/fixture hashes and logs: contact-feasible-elastic-2026-10-11.

Original full469-guide frame4 capture replay TERMINAL SUCCESS exit0,188.99s test runtime. Iteration38 tangent_complete=true, strand_admitted=true, unresolved=false, worst gap51.0224pm versus100pm admission tolerance. Whole-path mesh admission, preserved roots and5% strain test assertions pass. Cumulative fixes thus qualify both saved residual and original frame4 nonlinear substeps. Full720-frame coupled jump and performance are still unproven. App binary test target built successfully but does not contain the library-owned qualification; building the library test target before the full-cycle run.

Current app library test target built successfully (voxy_app-1c1560a10812259f); exact full-cycle test confirmed in --list. Full720-frame authored jump with original skin,469x20 guides and continuous contacts launched with frozen binary/source hashes under artifacts/articulated-jump-2026-10-11/authored-jump-feasible-elastic. PID18436/session4868 verified live at36s, running real nonlinear/swept refinement. Correct VOXY_HAIR_SWEEP_REFINEMENT_TRACE is enabled. CPU stage timings are not rendering FPS. Await authoritative terminal/summary; do not restart from observation timeout.


### Full-cycle CPU sample and exact midpoint scaling (2026-10-11)

Full720-frame qualification remains live PID18436/session4868. A2-second native CPU sample during the first coupled frame identifies equality QR solve, segment closest points, interval polynomial subdivision, continuous line certificate and Bernstein conversion among sampled hot leaves. Counts include threads and are not wall-clock shares/FPS.203 swept-stage timing observations had median615.10ms, initial projection32.84ms, initial admission79.24ms and8 refinements; this is a partial multi-solve profile, not total frame timing. First completed frame50.95s; current CPU path is far from requested real-time throughput. Capture under authored-jump-feasible-elastic/cpu-sample.txt and partial-profile.json.

Specialized positive-singleton interval scaling in polynomial::split replaces general four-product interval multiplication by0.5 with endpoint scaling, preserving the same outward next_down/next_up and whole-interval fallback on nonfinite results.383 physics tests pass,44 ignored; new regression compares exact output bits on signed zeros, subnormals, extreme/nonfinite bounds and20000 seeded intervals. Original paired wall benchmark is heavily preempted; do not use its ratios. A standalone harness includes the production module verbatim and runs4million degree-six subdivisions per child process. Six alternating process pairs measured CPU user+system median speedup1.09454x (individual1.064..1.160x). This is a local primitive improvement, not frame/FPS or GPU-support proof. Evidence under interval-midpoint-performance. Running full-cycle frozen binary predates this arithmetic-equivalent optimization and must not be presented as its performance benchmark.


### Compile-time Bernstein coefficients (2026-10-11)

The sampled hot Bernstein conversion recomputed binomial products/divisions per query despite a fixed maximum degree6. Ratios now use a compile-time7x7x7 f64 table calculated with the same integer binomial recurrence and floating division. Runtime coefficient multiplication/addition order and outward rounding are unchanged.384 physics tests pass,44 ignored; a new regression checks exact output bound bits for7000 seeded polynomial inputs across degrees0..6 against original runtime-binomial conversion. Standalone production-module harness, runtime degree black_box and2million conversions per child, six alternating CPU user+system pairs: median speedup1.14213x, individual1.126..1.157x. No whole-frame/FPS claim. Artifact bernstein-table-performance includes source hash, harness, test and timing logs. Full-cycle frozen app still predates these equivalent arithmetic specializations; PID18436/session4868 verified live at8min,1/720 frames completed. Keep that run intact; final correctness remains unproven.


### Solver-owned GPU RHS scratch and packed batch upload (2026-10-11)

Reviewed retained TressFXSimulation.hlsl resident structured position/history/frame buffers and PPF collision_window.rs caller-owned capacity-preserving allocation commentary. Applied the ownership principle inside the existing checked GpuHairLinearSolver, without importing PPF's manual-arena lifetime model or adding another solver. Shader RHS transport now owns one exact-layout scratch per solver and one immutable packed upload per admitted batch. GPU copies feed each wave; gather and staging snapshot precede the next scratch overwrite. Factor/header/status semantics, per-batch readback wait, fallback ordered-copy transport and readback capacity admission remain unchanged. Workspace creation counter and existing bounded GPU regression assert exact same-layout reuse across calls/batches.

Real Apple M4 Max/Metal qualification passes: bounded staging/zero quarantine and original physical residuals; mixed system shapes/load order; full469-guide x17-load response batch bitwise equality with host-copy route. Focused tests finish0.50s,0.34s,6.18s respectively. Current full-size transport median host-copy574.8775ms versus shader498.8903ms; this compares two current modes, NOT before/after optimization, not full simulation or FPS. Both are far above6.25ms. Artifact gpu-rhs-owned-workspace records test logs and source/binary hashes. GPU route remains the existing explicitly selected qualification/hybrid route.

Full720-frame CPU jump is TERMINAL FAILED exit101 after735.10s test runtime (739.67s launch-to-exit), frame2, completed1/720. Error: joint hair contact position linearization did not converge. Worst final rod447/segment15 gap0.82760385nm versus0.1nm tolerance; published time/skin/guide-frame pose remained at last admitted frame as asserted by failure-state.json. New failed-contact.vhr and detailed trace preserved under authored-jump-feasible-elastic. No remaining live full-cycle test. Passing standalone previous contact captures does not qualify the complete jump. Next nonlinear investigation must use this new frame2 capture.

### Frozen-pose strand counterexamples (2026-10-11)

IPC Toolkit retained trust_region.cpp candidate refresh remains the source mechanism reference: rejected trial geometry must enrich the local solve rather than merely shorten its step. The full469-guide frame2 replay showed new strand intersections dominating merit at intermediate scales while the old outer model retained only mesh planes. ContactModel now retains admissible strand witness rows for the same frozen pose, resets speculative impulses, deduplicates exact witnesses, and clears them after accepted movement. These rows enter the existing common contact active set and swept admission, including its force owner; no second solver or tolerance change.

Validation: physics release library 385 passed, 0 failed, 44 ignored. New original frame2 capture replay TERMINAL exit0 in151.04s, iteration112 full scale1, tangent_complete=true, strand_admitted=true, unresolved=false, worst mesh gap -7.078500071395511e-11m within unchanged1e-10m admission. Binary/fixture/source hashes and terminal evidence: artifacts/articulated-jump-2026-10-11/frame2-strand-counterexamples/. This qualifies one saved pose only. Full720-frame coupled jump remains unqualified; this CPU diagnostic is not render FPS or evidence of160FPS.

### Strand counterexample cross-capture regression and coupled run

Same frozen new physics binary also passes the earlier original frame4 full469-guide fixture: terminal exit0,109.13s, iteration40 full tangent admission, strand_admitted=true, unresolved=false, worst gap -5.113356999614773e-11m. Evidence artifacts/articulated-jump-2026-10-11/frame4-strand-counterexamples/. This preserves the preceding mesh-contact repair on its actual historical capture. App release library rebuilt successfully.

Full720-frame CPU coupled qualification launched with that app build at artifacts/articulated-jump-2026-10-11/authored-jump-strand-counterexamples-captured/. Status must be read from live process/summary/terminal files; no completion claim. The preceding directory without -captured records an intentional owned-process termination before first completed frame to correct a misspelled capture export environment variable, not a simulation failure or timeout. Correct export is VOXY_HAIR_CONTACT_REPLAY_EXPORT.

### Reuse exact batch CCD result during witness discovery

Code inspection found TriangleMesh::trajectory_constraints evaluating sweep_capsule_triangle twice for each exact non-clear capsule/face/options tuple: first in swept_capsule_queries and again in trajectory_contact_oriented. The batch caller now enters a private after-nonclear-sweep witness routine. Standalone queries keep validation and CCD; broad-phase overlap alone never enters the bypass. Clearance admission for every new trial remains unchanged.

Physics release library385 passed,0 failed,44 ignored. Moving-mesh regression retains exact surface-crossing rejection; a valid plane approach compares every witness scalar bit against the standalone route. Full469-guide original frame2 replay with newly frozen physics binary launched at artifacts/articulated-jump-2026-10-11/frame2-sweep-result-reuse/; consult terminal.json/live process for completion. No timing improvement asserted under concurrent machine load. The running full720-frame app qualification predates this optimization and uses its own immutable binary.

### CCD reuse full-model saved-pose verification

frame2-sweep-result-reuse is now TERMINAL PASS exit0. Full469-guide original frame2 fixture reaches iteration112 full scale1 tangent completion, strand_admitted=true, unresolved=false, worst gap -7.078500071395511e-11m. All136 logged nonlinear merit/admission/approach-model events exactly match the pre-optimization frozen replay; comparison.json records the scope. This complements the scalar bitwise witness regression and385 passing release tests. Elapsed149.38s is only run timing, not a valid before/after speed comparison under concurrent load and not FPS.

A snapshot of the ongoing full coupled run (refinement-snapshot.json, log hash included) recorded400 completed contact solve profiles,320155.8ms aggregate profiled time,254224.4ms in refinement plus final checks (~79.4%). Profiles do not cover the entire app frame and do not prove which refinement substage dominates. The full720-frame run remains live and unqualified; consult its process/summary/terminal evidence.

### Refinement substage instrumentation

Added opt-in HAIR SWEPT REFINEMENT PROFILE (same existing VOXY_HAIR_SWEEP_REFINEMENT_TRACE flag): witness activation, repeated projection, and component admission are timed separately. No solver branch, physical tolerance or budget changed. tools/analyze_hair_refinement_trace.py supports both old logs and detailed records, validates finite nonnegative fields, and reports explicit detailed-record count. Parser checks cover legacy input, aggregate values and invalid nan/inf/negative values. Physics release385 passed,0 failed,44 ignored.

Live full469-guide frame2 diagnostic at artifacts/articulated-jump-2026-10-11/frame2-refinement-stage-profile/ has its own frozen binary/fixture/source manifest. Initial snapshot18 completed profiles: witness2002.765ms, projection10597.610ms, admission9031.665ms. These are partial concurrent wall times, not isolated benchmark or whole-frame timings. They show repeated projection is substantial alongside CCD; earlier aggregate refinement timing did not justify attributing the entire cost to collision discovery. FrozenSystems already caches per-rod systems/native load columns within each frozen Newton pose; do not add a duplicate factor owner. The720-frame coupled run continues on its earlier immutable binary.

### Terminal stage profile, exact-key workspace, and new coupled failure

Full saved469-guide stage-profile replay is TERMINAL PASS exit0,236.09s under concurrent load. All136 logged nonlinear events match the preceding saved-pose run.113 detailed contact profiles: witness13660.37ms, projection64351.05ms, admission74762.69ms. These are scoped contact-stage wall times, not frame timing or an isolated comparison.

NativeLoadCache now owns one reusable exact-bit key workspace instead of allocating a key on every lookup. Only insertion clones an owned key; signed-zero/subnormal identity, operator validation,128-column/256KiB stored payload admission remain unchanged. Existing bitwise native-load regression includes reversed loads, signed zero, subnormal, invalid operator and full cache. Physics release385 passed,0 failed,44 ignored. No speed claim yet.

Full720-frame coupled run authored-jump-strand-counterexamples-captured is TERMINAL FAILURE exit101,1013.84s.1/720 frames completed; frame2 failed joint contact position linearization at128-iteration budget. Published time/skin/guide frames preserved. Final accepted penetration merit history reached0; final worst gap -9.630327850721348e-11m within unchanged1e-10m tolerance, but tangent completion remained unproven amid repeated model enrichment. This is a distinct new actual fixture; do not equate it with the earlier passing captured frame2. New frozen replay at frame2-coupled-zero-merit-replay has binary/source/fixture hashes and process/terminal evidence. Budget was not raised.

New coupled-zero-merit replay is TERMINAL FAILURE exit101,38.68s. At iteration19 origin contact merit is0, full candidate merit2.733996945146791e-18; all12 backtracked trials fail combined contact/structural-energy descent. Final retained mesh gap is within1e-10m gate. This narrows the next diagnosis to feasible-contact structural descent/stationarity, not residual penetration reduction or simply increasing128 iterations. Key workspace change passed385 release tests; no claim that it fixes this physical convergence issue.

### Zero-merit energy and original-path counterexample

Diagnostic-only VOXY_HAIR_ELASTIC_DESCENT_TRACE reports actual structural energy slope, global and per-rod energy differences, and independently certifies the original strand trajectory.385 release tests pass. Frozen zero-merit-elastic-direction run TERMINAL FAILURE exit101,40.58s: iteration19 slope +1.4378564508942496e-11J; smallest feasible-endpoint trial raises energy by3.515467946e-15J, corroborated by per-rod delta sum3.515468424e-15J. Global energy cancellation is not the primary cause.

zero-merit-origin-path-audit TERMINAL FAILURE exit101,24.86s: all observed zero-endpoint-merit origin paths, including iteration19, return strand_fraction=Ok(1.0). Therefore the hypothesis that energy increase is necessary to repair an unsafe original strand path is contradicted.

Canonical operator capture zero-merit-origin-constraint-owners: first base island1 row, maximum original bound9.63032785e-11m. First refined island39 rows includes4 original bounds >1e-10m, maximum1.64202096e-6m. These affine cuts exclude zero correction despite the independent safe-origin certificate. Original row bit ownership and immutable VQC inputs are retained, not inferred from reduced/shifted RHS. contact_swept_constraints uses first-approach normal with later collision barycentric/time witness: investigate an origin-admitting local plane for independently certified safe origins, retaining first-side restoration for unsafe origins. Do not unconditionally flip normals on already crossing prescribed-root trajectories. No physical acceptance/tolerance/budget changed by this diagnostic work.

### Certified-origin local swept planes

Implemented an origin-admitting swept normal for independently certified safe frozen trajectories. At each witnessed barycentric feature/time, use the radial separation of that same feature on the frozen origin path when its distance is at least the physical diameter. Otherwise retain the previous first-approach side. The whole-origin CCD certificate is cached lazily once per immutable Newton admission transaction; unsafe prescribed-root paths do not receive this privilege. Full candidate CCD and final geometry gates remain authoritative; no radius/tolerance/budget changed. IPC safe-region locality is the mechanism reference, not copied code or thresholds.

Regression uses exact captured row134 coordinates (rods431/447): prior first-feature normal demands1.642um correction; same origin feature is81.850um apart for80um diameter. New affine row admits zero correction with1.8um margin. Unsafe-origin fallback and existing first-contact-side/root-motion tests remain intact.386 physics release tests pass,0 failed,44 ignored.

Full469-guide actual zero-merit failure replay launched with frozen new binary in zero-merit-origin-normal/. Source/fixture/binary hashes and unit log retained. Partial observations through iteration37 show negative energy directions, accepted decreasing energy and admitted original strand path, with no unresolved endpoint penetration. Earlier iteration19 uphill failure has not repeated, but the replay is still live at this observation and full convergence is unproven. Consult its process/terminal log; do not restart on observation timeout.

### Feasible elastic progress versus zero contact stagnation

zero-merit-origin-normal is TERMINAL FAILURE exit101,541.00s: iteration60 after exactly9 accepted increments, all contact merits0, admitted original strand paths and decreasing structural energy. Final worst gap -6.1614749558202734e-12m. This run verifies the repaired normals avoided the previous iteration19 uphill failure, but exposes the separate work-budget bug: old8-increment contraction condition requires earlier contact merit>0 and therefore rejects any all-zero window, regardless of actual elastic descent.

The work-budget history now records actual post-increment structural energy at zero contact excess. Positive penetration still requires the original10% contraction over8 accepted increments. An all-zero contact window permits continuation only for finite strictly decreasing actual energy; flat, increasing, missing or nonfinite energy does not qualify. The128 outer-iteration maximum, tangent completion and every geometry/CCD gate are unchanged. Regression covers each phase and stagnant/ascending energy.387 release physics tests pass,0 failed,44 ignored. New same-fixture frozen replay zero-merit-elastic-progress is running; read process/terminal evidence before claiming convergence.

### Actual continuation past the false zero-merit budget rejection

Same frozen fixture with new energy-progress budget reaches iteration61 after9 accepted zero-merit increments. The first9 logged nonlinear merit events match the prior terminal failure byte-for-byte, including iteration60 accepted halfstep and worst-gap gate. Old binary stopped immediately afterward; new binary independently certifies origin strand path at iteration61 and continues. budget-continuation-proof.json records the hashed log snapshot and live process. This verifies the corrected budget branch on the actual full469-guide capture without changing those physical steps. Pose convergence remains unproven until terminal success.

### Actual zero-merit coupled fixture finally converges

zero-merit-elastic-progress is TERMINAL PASS exit0,719.74s test time (720.248s launch-to-exit under concurrent host load). Full469-guide actual coupled-frame2 failure fixture completes at iteration93, full scale1, tangent_complete=true, strand_admitted=true, unresolved=false, zero contact excess, worst gap -5.192225424813547e-11m. Root-position and5% relative-strain assertions pass. The128 cap and1e-10m physical gate were not changed; existing whole-path strand/mesh admission still owns acceptance. This validates both certified-origin plane repair and feasible-energy budget transition together on the real captured failure. It does not validate the720-frame cycle or160FPS.

Historical frame2/frame4 cross-capture regressions started sequentially with the same immutable passing physics binary (origin-normal-cross-capture-frame2/-frame4). App release library rebuild started separately for eventual full-cycle replay; neither is claimed complete yet.

### Application build with certified-origin planes and energy progress

App release library rebuild TERMINAL SUCCESS in6m05s; immutable /tmp/voxy-origin-normal-app.bin preserved with binary/source hashes and build log in origin-normal-app-build/. This is compilation only, not a runtime or FPS qualification. Historical original frame2 regression remains live; the sequential frame4 regression has not started yet. Full coupled720-frame run should use the frozen app only after cross-capture evidence permits it.

### Feasible-origin privilege includes body contacts

Cross-capture review found origin_path_admitted certified strand motion but did not check the original body's contact rows. An origin-preserving elastic plane is a feasible-phase privilege: current original physical rows must also have finite gaps >=-1e-10m. Otherwise the existing first-approach restoration is retained. This does not change contact discovery, rows, radii, actual acceptance tolerance,128-iteration maximum or energy objective. The body-row gate is evaluated inside the existing immutable transaction before granting the strand certificate privilege; no second solver or geometry owner.

Regression demonstrates that a fully safe strand path with a2e-10m body violation must stay in restoration, while5e-11m remains within the unchanged gate.388 release physics tests pass,0 failed,44 ignored. A new immutable physics binary drives sequential feasible-phase-frame2, feasible-phase-frame4 and feasible-phase-zero-merit saved full469-guide regressions. Their statuses must be checked live/terminal; no convergence claim yet. The previous broader-normal cross-capture run remains separately identifiable and live at the last observation. Earlier successful app build is retained but marked superseded for the next runtime qualification because physics source subsequently changed.

### Cross-capture restoration phase evidence

Broader-normal historical frame2 comparison TERMINAL FAILURE exit101,1074.49s: joint hair contact position linearization did not converge. Sequential frame4 was not launched because the runner stops on failure. This binary predates the body-feasibility scope guard and is not evidence against the newer guarded binary.

The guarded feasible-phase-frame2 run reproduces the first129 nonlinear/model/admission events of frame2-sweep-result-reuse exactly, including iteration106 reaching zero contact merit and gap -9.142637634112029e-11m. First divergence is iteration107, after entering the fully feasible phase, in added strand pairs. This validates the intended phase boundary on the real capture; it does not prove final convergence. At iteration114 the guarded run accepts a halfstep with zero merit and gap -6.201086415465745e-11m, tangent incomplete. Continue the same live handle rather than restart on observation timeout. Current-source app release build is separately running in feasible-phase-app-build/.

### Preserve already origin-admitting contact planes

Guarded feasible-phase-frame2 TERMINAL FAILURE exit101,508.02s: reached128-iteration limit after contact restoration, with final published accepted gap -7.040126341041123e-11m and zero merit; tangent incomplete. Other queued captures did not start because the runner stops on failure. This is not a successful pose qualification.

Inspection showed origin_normal replaced every certified safe-feature normal by the radial normal, including first-approach planes whose projected original separation already exceeds the physical diameter. Such a valid plane has no origin exclusion to repair. The revised rule preserves its exact normal when dot(separation,fallback)>=diameter; radial replacement is limited to genuinely origin-excluding planes and still requires the existing independent feasible-origin certificate and sufficient radial separation. No limit, tolerance, acceptance or physical radius changed. Added an oblique supporting-plane exact-normal regression beside the actual captured exclusion regression. Release physics suite started; result pending.

Supporting-plane revision passes389 release physics tests,0 failed,44 ignored. Frozen /tmp/voxy-supporting-origin-physics.bin now runs sequential supporting-origin-frame2, supporting-origin-frame4 and supporting-origin-zero-merit captures with source/fixture/binary hashes and failure replay export enabled. Stop queue on any failure. No captured-pose convergence claim yet. The earlier feasible-phase app build completed successfully in5m44s and is frozen separately as /tmp/voxy-feasible-phase-app.bin, explicitly superseded by this later physics revision.

### TressFX rapid-root-motion mechanism audit

Reviewed pinned TressFXSimulation.hlsl CalculateStrandLevelData (around lines680-730), VelocityShockPropagation (740-763) and IntegrationAndGlobalShapeConstraints (610-665), not merely repository descriptions. It derives a rigid root transform from two immovable vertices and raises a propagation coefficient to1 when the unscaled second position difference exceeds a threshold. The following pass blends that rigid transform into both current and previous free-vertex positions. Integration also supports teleport reset and global rest-shape attraction.

This is a visual stabilizer, not equivalent to Voxy's inertial Cosserat solve: applying its history rewrite to physical guides would alter momentum and the frozen trajectory underlying continuous contact admission. Voxy HairRod::predict retains world-space free-guide history and applies the attachment as a boundary condition; swept advance keeps predicted state as an inertial target and admits the coupled root path. Therefore do not copy the VSP history mutation or its pseudo-acceleration threshold into physical guides to hide jump/contact failure. Retain TressFX's resident per-strand root data and staged dispatch ownership as GPU scheduling references; any predictor optimization must preserve the physical objective, immutable old trajectory and final CCD gates. No TressFX solver or assets imported in this review.

### Supporting-plane cross-capture terminal evidence

Supporting-origin-frame2 TERMINAL FAILURE exit101,246.62s under current host load, reaching128 outer iterations without tangent completion. The sequence stops before frame4/zero-merit. Final state is saved in supporting-origin-frame2/failed-contact.vhr. Preserving already-supporting normals is regression-tested but has not restored convergence on this pose. No timing speedup claim is justified by these differently loaded runs. Started supporting-origin-final-pose diagnostic replay from the saved terminal state with actual elastic descent tracing; this distinguishes residual direction/energy failure from insufficient remaining nonlinear work after contact restoration. A diagnostic restart cannot establish original-transaction convergence and no cap/tolerance was changed.

### Admit a certified feasible elastic step before obsolete trial cuts

Actual final-pose trace shows repeated descent_found=true model enrichment at the same pose despite strictly lower structural energy. For example iteration40 has slope -2.182723563562908e-11J and a feasible trial at1/16 with actual energy delta -1.3537141787257732e-12J, then adds4 mesh rows and2 pairs and discards the step. Larger rejected trial counterexamples unconditionally take priority over the already accepted smaller trial. This repeatedly spends outer iterations on old-pose contact discovery.

Changed only the zero-contact feasible elastic branch: before deferring an accepted step to rejected-trial cuts, independently require the unchanged5% strain gate, mesh_motion::admit for the complete published-old-to-trial path, and strand_fraction==1 on that same original full path. A certified descent step proceeds through existing reaction scaling, model invalidation and final tangent/geometry admission. Without this certificate, and throughout positive-merit restoration, prior model enrichment remains unchanged. This does not turn a partial step into tangent completion.128 iteration limit and all physical tolerances remain unchanged. PPF step.rs STEP B29 is the retained mechanism reference for whole-increment continuous line-search ownership; no PPF solver code or thresholds copied. Release physics suite is running.

Supporting-origin-final-pose diagnostic TERMINAL PASS exit0,115.30s, iteration42 fullscale1, tangent_complete=true,strand_admitted=true,unresolved=false,gap -2.5768272688836277e-11m. This proves the saved terminal geometry can converge under the same equations but is not a pass of the original128-iteration transaction. It supports investigating repeated deferral of already feasible elastic descent rather than relaxing physical gates.

Admitted-elastic-trial revision passes389 release physics tests,0 failed,44 ignored,3.07s. Immutable /tmp/voxy-admitted-elastic-trial-physics.bin runs original full469-guide captures sequentially in admitted-elastic-frame2, admitted-elastic-frame4 and admitted-elastic-zero-merit, stopping on failure, with replay export, binary/source/fixture hashes and unit log. Original-transaction convergence remains unproven pending terminal results.

### Distinguish contact-model deferral from accepted nonlinear motion

Extended tools/analyze_hair_refinement_trace.py with explicit accepted nonlinear step records, approach-model deferral records, observed deferrals with trial descent and zero-merit accepted-step counts. This reports observed events only, not independent physical or convergence certificates. Supporting-origin-frame2 has25 accepted steps and97 model deferrals with trial descent; supporting-origin-final-pose has6 accepted steps and25 such deferrals. Each report retains the exact input log hash. Legacy113 detailed refinement profile records remain readable; empty input and rejection of NaN/infinite/negative nonlinear values verified. Original admitted-elastic-frame2 replay remains live; no terminal or speed claim.

### Reuse bounded CCD query workers from Rayon

A3-second native sample of live admitted-elastic-frame2 is retained in live-process-sample.txt. It observes component admission/line certificates, capsule/polynomial CCD, equality QR, thread creation and worker joins. This short all-thread stack sample is not a CPU-time percentage, full-frame profile or FPS benchmark.

Inspection found ordered_query_results creates scoped OS threads on every geometry batch. Replaced only this scheduler with Rayon1.12.0 (exact dependency version, Cargo.lock checksum resolution), using one lazy bounded executor of at most8 threads and synchronous borrowed install. Indexed par_chunks results collect in input order; ordered result concatenation retains first-rejection semantics. Serial small-query path remains and inability to construct a pool falls back to exact serial evaluation. No geometric equations, pair traversal within each chunk, radius, admission or reduction order changed. Dependency primary references: https://docs.rs/rayon/1.12.0/rayon/struct.ThreadPool.html and https://docs.rs/rayon/1.12.0/rayon/iter/trait.IndexedParallelIterator.html. Added repeated-query worker-identity membership regression beside existing canonical-order/first-error tests. No speedup claim before measurement.

Explicit turn interruption removed prior executable/build processes and temporary logs/immutable binaries; process inventory and missing tool handles revalidated this. Original admitted-elastic-frame2 log ends atiteration79 without a terminal result; interruption-audit.json records incomplete status, not numerical failure. Current-source release physics validation restarted with persistent artifacts in resident-query-pool-build/. The earlier original-transaction validation must be rerun with a newly frozen binary after this build passes.

Resident-query-pool current-source release suite TERMINAL PASS390 tests,0 failed,44 ignored,0.39s. Persistent build log, source/lock hashes and frozen physics-tests.bin are retained in resident-query-pool-build/ rather than temporary storage.

Resident-pool-witness TERMINAL PASS on469-guide VJR1 capture: original reference/current serial/current pooled/canonical production witness activation agree bit-for-bit in affine constraints and ordered strand groups over3 repeats. Logged times are different algorithm routes under current host load, not before/after scoped-thread versus persistent-pool speedups and not FPS. Sequential same-binary resident-pool-frame2/frame4/zero-merit qualification started, stop on any failure. Consult each terminal record before claiming convergence.

### Actual before/after scheduler comparison does not yet justify a speed claim

Recovered pre-pool target executable matches the admitted-elastic-frame2 immutable binary SHA-256 exactly. Comparing source manifests confirms only contact_safe_motion.rs differs in physics source. Preserved this executable separately and ran4 alternating before/after processes on the same469-guide witness fixture (12 pooled-route timing samples per variant), each also enforcing canonical reference/serial/parallel signature equality. All8 runs pass.

Under concurrent app compilation and full saved-pose replay, median parallel witness time is18.007604ms before versus26.701792ms after (observed before/after ratio0.6744); ranges12.876-72.679ms versus15.857-71.248ms. This is negative/noisy performance evidence, not an optimization success. Preserve the full reports in resident-pool-before-after/. Repeat with our competing jobs finished before deciding to retain the scheduling change for performance; correctness remains qualified. No FPS claim. Current app build and original saved-pose validation remain separately live.

### Reject measured scheduler regression and shrinking-step acceptance experiment

Resident-pool-frame2 TERMINAL FAILURE exit101,280.32s at128 outer iterations. Contact excess zero fromiteration106, but accepted feasible scales shrink1/32,1/128,1/512,...; tangent completion remains false. Prioritizing every certified tiny elastic step resets the local model repeatedly and did not repair convergence. Removed that acceptance-priority experiment and restored rejected-trial counterexample enrichment before acceptance. Do not describe it as a successful solver repair.

Resident-pool-app-build TERMINAL SUCCESS3m07s, immutable app-tests.bin and hashes retained, now explicitly superseded by subsequent source changes.

With our app build and saved-pose job terminal,8 alternating before/after witness test processes per variant (24 route samples each) again all pass canonical signature checks. Median before14.3501045ms versus after29.760854ms; before/after ratio0.48218, broad outlier ranges retained. Other host activity remains uncontrolled, but both comparisons fail to justify the scheduler as a performance improvement. Removed Rayon dependency and pool/reuse regression, restoring the original ordered scoped-thread scheduler; preserve benchmark artifacts rather than claiming an optimization.

### Minimal angular origin-plane repair

A full radial normal replacement can rotate a valid approach direction more than needed. For a certified feasible frozen origin whose existing normal excludes zero correction, project that normal onto the unit-sphere cap satisfying dot(origin_separation,normal)>=diameter. The radial/tangential decomposition preserves as much first-approach direction as possible;32 machine-epsilon cosine guard addresses roundoff, with radial fallback if the computed plane still excludes the origin or its tangential direction is degenerate. Already supporting planes and uncertified/body-violating origins retain their exact old normal. Radius, physical tolerance, iteration limit, CCD and tangent-completion gates unchanged. Updated actual captured row134 regression requires zero-correction admission, unit normal, and strictly less angular deviation from approach than full radial replacement. Current release physics validation is running; no captured-pose convergence claim yet.

Minimal-origin-normal release suite TERMINAL PASS389 tests,0 failed,44 ignored,1.10s. Rayon dependency/lock changes fully removed. Frozen binary and persistent unit log retained in minimal-origin-normal-build/. Sequential original469-guide captures minimal-origin-frame2/frame4/zero-merit started with failure replay export and actual elastic direction/energy tracing. Stop on failure; pending terminal evidence, not a convergence claim.

### Original historical frame2 converges with minimally rotated origin planes

Minimal-origin-frame2 TERMINAL PASS exit0,163.97s, iteration126, fullscale1, tangent_complete=true, strand_admitted=true, unresolved=false, contact excess0, worst gap -2.4806144695197393e-11m. Root-position and5% strain assertions pass.25 accepted nonlinear steps; unchanged128 total outer attempts and1e-10m physical tolerance. Frozen source/fixture/binary hashes, validation.json and refinement-analysis.json retained. This is one original469-guide saved pose, not the720-frame coupled jump or rendered FPS. The sequential minimal-origin-frame4 case is now live; zero-merit original coupled failure remains queued.

### Historical frame4 also passes the same minimal-normal binary

Minimal-origin-frame4 TERMINAL PASS exit0,83.01s, iteration40 fullscale1, tangent_complete=true,strand_admitted=true,unresolved=false,zero contact excess,gap -5.113356999614773e-11m. Root-position/5% strain assertions pass. Validation and immutable launch/source/fixture/binary metadata retained. Minimal-origin-zero-merit is separately live, with actual negative structural-energy slopes observed, not yet terminal.

Current-source app release build started in minimal-origin-app-build/. Its runner may start minimal-origin-full-coupled-cycle only after all3 saved captures have terminal exit0, physics source hashes agree with the app build manifest, and current build sources remain unchanged. The full cycle uses the exact720-frame coupled test, continuous contacts, persistent failure capture and refinement tracing. Successful saved poses are prerequisites, not substitutes for the full trajectory or rendered FPS.

### All3 original captures pass one immutable minimal-normal binary

Minimal-origin-zero-merit TERMINAL PASS exit0,48.88s, iteration42, fullscale1, tangent_complete=true,strand_admitted=true,unresolved=false,zero contact excess,gap -5.404093218975107e-11m. Root-position/5% strain assertions pass. This is the actual saved coupled-frame2 failure that originally exposed an uphill direction.

All3 original full469-guide captures now have terminal success with exactly the same frozen binary hash, recorded in minimal-origin-normal-build/capture-suite-validation.json. First historical frame2 converges126, historical frame4 converges40, actual coupled zero-merit failure converges42, all below unchanged128 maximum. This establishes bounded saved-pose regression coverage, not a full720-frame or FPS result. Current app build remains live and is eligible for its source-hash-gated full-cycle launch after successful compilation.

### Full coupled trajectory launched after matching source qualification

Minimal-origin-app-build TERMINAL SUCCESS1m12s. Immutable app-tests.bin retained with binary/source hashes. The runner independently verified terminal success of all3 saved poses, matching physics source hashes against the app build manifest, and unchanged build sources before launching the exact ignored720-frame full coupled test.

Minimal-origin-full-coupled-cycle is live (initial PID62585), continuous_contacts=true,469 guides×20 segments, persistent failure replay export and actual swept refinement trace enabled. Atomic summary currently records1/720 completed frames; later status must be checked from the same live handle/terminal record. This is real coupled CPU trajectory qualification, not rendered FPS or proof of the remaining719 frames.

### Correct the meaning of tangent_complete in reports

Source audit: continuous reconcile_positions_mode maps advance_contacts_with_pairs returned minimum admitted fraction to complete=(fraction==1). Safe-motion completion certifies the projected linear increment and physical path; it does not calculate a post-increment nonlinear KKT/stationarity residual. Thus prior natural-language statements calling these passes independently confirmed nonlinear equilibrium were too strong. The saved-pose tests still genuinely pass their original root/strain/contact/CCD and linear-increment gates, but nonlinear equilibrium is unmeasured. Added explicit nonlinear_stationarity_measured=false and tangent_complete semantics to the3 validation records and capture-suite report. Full-cycle CPU correctness also must not be advertised as nonlinear-equilibrium or rendered-FPS qualification without the appropriate independent measurements. No solver acceptance or current running source changed by this evidence correction.

### Full-cycle runtime still fails frame2 despite saved-pose passes

Minimal-origin-full-coupled-cycle TERMINAL FAILURE exit101,214.70s,1/720 frames completed, failure atframe2: joint hair contact position linearization did not converge.128 total outer attempts exhausted with positive contact excess; last accepted gap -4.520801137505063e-10m, beyond unchanged1e-10m gate. Published time/skin/guide-frame state is preserved by failure-state.json. Three saved-pose successes do not establish original full-step convergence.

Final nonlinear contact transaction makes31 accepted pose updates and continues adding fresh approach rows/pairs between them. Its initial worst gap was -0.00018980430138072416m; merit history contracts from7.640656656619829e-8 to1.458817748400449e-19. Full-run refinement snapshot retained with exact log hash. A new same-binary diagnostic replay from the saved terminal state is running in minimal-origin-full-frame2-residual/. It preserves actual prediction/inertia/old trajectory, records elastic descent after feasibility and exports any later failure. Diagnostic restart is not a pass of the original frame. Investigate distinct model-refinement versus accepted-pose work ownership before any budget change; no cap or tolerance changed this turn.

### Contact work accounting qualification

Inspected pinned PPF `driver/step.rs` B0 and B33: its Newton counter is bounded explicitly and advances at the continuing iteration tail after admission. This is a control-flow reference, not evidence for Voxy convergence or a transferable threshold. Voxy now separately bounds accepted pose updates (128) and attempts at one frozen pose (128); generating contact rows no longer consumes an accepted pose update. The previous total 128-attempt bound is increased, with a finite worst-case 16384 attempts. Physical tolerances, whole-path admission, rollback and energy/progress gates remain unchanged.

The captured coupled-frame2 sequence (31 accepted updates and 97 refinements) is covered by a work-accounting regression; all 390 nonignored release physics library tests passed, with 44 ignored. This is not a convergence or performance qualification. The old-budget residual replay of the full coupled frame2 failure terminated with exit101 after128.83s: zero penetration merit still did not complete tangent admission before the cap. Rebuilding the app and qualifying the original full trajectory with the new budget remain required.

### Initial contact profile attribution correction

Source inspection showed initial_projection_ms included initial response discovery, row assembly and NativeNewtonStep construction. Added initial_setup_ms and subtract it from the projection interval; total elapsed and physical solve are unchanged. The analyzer accepts legacy traces (setup not separately measured) and records the number of explicitly separated profiles. Verified equal stage-time conservation between legacy/new fixtures and rejection of NaN, infinity and negative setup measurements. The live contact-work-budget full-cycle binary predates this instrumentation and retains the old combined measurement; its manifest remains authoritative. Partial refinement-stage figures are not whole-frame FPS or isolated benchmarks.

### Live full-trajectory work-budget evidence

The frozen contact-work-budget app admitted one internal frame2 nonlinear transaction at attempt310, with tangent_complete=true, strand_admitted=true, unresolved=false and worst gap -7.560791322830128e-13m. It then continued into another internal transaction; frame2 was not yet published. Evidence: contact-work-budget-full-coupled-cycle/completed-transaction-310.json. This supports separating refinement and accepted-step work budgets, not full-cycle completion, nonlinear stationarity or real-time throughput.

### Borrowed solve-local broad-phase results

Reviewed pinned Newton SolverVBD construction/capture ownership commentary (solver_vbd.py around245-298): persistent contact buffers must be allocated outside graph capture; solver ordering and capacities are explicit. Applied only the ownership principle to existing SweptPairCache, not Newton equations or history. Query now lends its cached or reusable filtered pair slice; the caller uses borrowed/owned transport for cached/uncached paths. Every query still validates fresh bounds, proves envelope containment or rebuilds, and preserves pair ordering and narrow-phase admission. No broader-lifetime geometric certificate was added.

All390 nonignored release physics tests passed (44 ignored). The saved full469-guide VJR1 broad-phase fixture also passed exact original-sweep and eight-scale cached/fresh comparisons; immutable binary, fixture/source hashes and terminal result are in borrowed-candidate-captured. The fixture benchmark intentionally copies output for independent comparison and does not measure this allocation removal in isolation. No throughput or FPS gain claimed. The already-running full coupled cycle retains its earlier frozen app binary.

### Pair-buffer ownership benchmark result

Added an opt-in paired benchmark preserving the previous allocating query as a test-local reference. Both paths use the same full9380-segment VJR1 motion, eight-scale sequence and exact output comparisons, then alternate execution order over64-query batches. All390 library tests passed and the captured benchmark passed. First12 pairs: original median15.485ms, borrowed16.2864ms (5.18% slower). Five repeats/60 pairs: original13.1431ms, borrowed12.7190ms (3.23% faster). The full-cycle job remained concurrent; direction changes across runs mean no stable speedup is established. Retain capacity reuse as an allocation reduction only; no rendered-frame or FPS benefit claimed. Immutable binary/source/fixture manifests and all samples: pair-ownership-benchmark/.

### Original full coupled frame2 admitted

The frozen contact-work-budget full-cycle app published frame2 with continuous contacts and the original469x20 guide topology. Frame2 elapsed1174062.508ms, hair1174059.655ms and skin84.246ms; these diagnostic wall times are not additive and are not rendered FPS. Snapshot and hash: frame2-published-summary.json and frame2-published-validation.json. The process continued to frame3; only2/720 frames are complete. This validates that the separated work budget gets past the original frame2 failure without loosening the contact gates; it does not prove nonlinear stationarity, full-cycle success or production throughput. Separately refreshed voxy_animation library254 tests and the frozen app actual-body seven-clip locomotion profile regression passed; these checks do not establish interactive walking/running integration.

### Work amplification after coupled frame2 admission

Observed per-published-frame trace counts are saved in frame-work-observation.json: frame2 had347 accepted pose updates,1140 outer model refinements,1499 swept contact solves and12989 inner swept refinements; elapsed1174062.508ms. Frame3 also published, with60 accepted updates,78 model refinements,150 contact solves and1705 inner refinements; elapsed113290.716ms. These are CPU diagnostic wall times, not rendering. The full cycle remains incomplete.

Source follow-up: NativeNewtonStep::new in velocity_contacts.rs computes a free increment and owns FrozenSystems; contact_safe_motion::advance_mode creates it afresh for each call. FrozenSystems caches per-rod requests and native load columns across inner swept refinements, but its lifetime ends before the caller retries an unchanged nonlinear pose with newly discovered rows. A future lifetime extension must retain the same physical owner and invalidate on accepted pose/operator changes; introducing a parallel global factor cache or carrying speculative planes across accepted motion would violate existing ownership. No cross-attempt factor reuse is implemented by this audit.

### Work-budget full-cycle terminal result

The contact-work-budget frozen app terminated with exit101 after1435.04s, publishing3/720 frames and failing frame4: joint hair contact direction has no descending geometry step. Final rejection attempt79 had before merit0 and full merit9.294477575716788e-22. Failure-state confirms displayed time/skin/guide frames preserved. The10MB failed-contact.vhr is retained. Started a separately manifested residual replay with actual elastic-descent and contact-part traces under contact-work-budget-frame4-diagnostic; replay success alone would not establish the original full frame. No further budget increase or tolerance change is justified by this terminal result.

### Strict feasible-origin counterexample experiment

Frame4 origin diagnostic reproduced the prior failure with80 retained mesh planes,31 negative origin gaps (minimum -9.97183147934903e-11m), even though none exceeded the geometric admission tolerance. The terminal actual elastic slope was positive5.017632404566579e-13J. This implicates inconsistent zero-step admission of speculative constraints; it does not prove it is the sole failure mechanism.

For zero-merit origins, new mesh/strand counterexamples now must admit the origin without negative slack. The fallback rejected-candidate path applies the same mesh rule. Positive-merit restoration retains prior policy; fresh physical contacts, radii and final CCD/admission tolerances are unchanged. Regression checks a counterexample that excludes the origin only inside the geometric tolerance band versus a genuinely origin-admitting plane.391 release physics library tests pass.

Frozen strict-origin-frame4 residual replay terminated exit101 after42.87s. Retained planes have zero negative origin gaps; it advanced through accepted energy-decreasing corrections but ultimately found no accepted trial at attempt34, with actual slope -1.1784971051591737e-10J and full merit5.6758429269259145e-11. Thus the rule addresses the diagnosed speculative-plane inconsistency but is NOT a frame4 fix or a production qualification. Next investigate joint geometry/energy line-search feasibility; no tolerance, physical gate or budget increase justified. Preserve both diagnostic binaries and traces.

### Geometry-limited backtracking diagnosis

Detailed replay strict-origin-frame4-contactparts reproduced terminal failure at attempt34. No trial reached zero excess; final scale2^-12 still had worst mesh gap -1.8423592933635176e-9m against unchanged1e-10 admission tolerance. Successive halvings reduced this approximately linearly. Therefore the prior no-descent message did not establish absence of a feasible descending prefix; twelve probes ended too early.

Extended the finite line-search work bound to f64::MANTISSA_DIGITS (53 probes) and stop when interpolation rounds to the exact original pose. This is a larger work bound, not a proof of finding every representable prefix. Energy descent and all geometry/CCD tolerances are unchanged.391 release library tests pass. Frozen precision-backtrack-frame4 residual replay is in progress; observed attempts beyond the prior failure find descent but continue model refinement. No original frame4/full-cycle success claimed.

### Exact trajectory-row identity at discovery

Found approximate normal/offset deduplication in mesh_motion::refresh before ContactModel sees rows: normal dot>1-1e-12 and normal offset<=1e-12. Close normals can describe distinct endpoint halfspaces; ContactModel's exact identity cannot recover a row already discarded here. Discovery now requires equal normal, target and physical time metric in addition to source/segment/fraction/trajectory-time identity. No radius or admission tolerance changes.

A real capsule/triangle grazing regression discovers a trajectory row, supplies a nearby old normal inside the prior angular match band, verifies the newly discovered original row survives, then verifies exact repeated discovery is idempotent. Initial fixture crossed an unoriented open surface and was correctly rejected; corrected fixture grazes the radius envelope without crossing the underlying surface.392 release physics tests pass (44 ignored). A separately frozen exact-trajectory-rows-frame4 residual qualification started; the earlier precision-backtrack-frame4 diagnostic remains live on its own binary. No frame4 or performance success established by this row-identity regression.

### Extended-backtracking terminal qualification

precision-backtrack-frame4 terminated exit101 after635.55s at attempt271: before merit0, full merit1.719771361737677e-20, no descending accepted trial. Last tiny trials had positive actual energy delta about4.34e-19J. Extended search admitted intermediate steps but did not resolve the residual capture. This contradicts treating the line-search depth change as a frame4 fix. The later exact-trajectory-rows-frame4 frozen diagnostic is independent and remains live; no further work-bound increases are planned from this evidence.

### Exact identity rotation and terminal residual qualification

math::apply now preserves the admitted quaternion bitwise for zero increments and nonzero increments whose quaternion product rounds to the original. Line-search interpolation uses this same owner. A regression exposes non-idempotent repeated normalization, signed-zero preservation and a real finite rotation.393 release physics tests passed;44 ignored. This fixes a no-op invariant, not proven nonlinear convergence.

Frozen identity-rotation-stagnation replay terminated exit101 after93.06s at attempt31. It used the secondary precision-backtrack-frame4 failed-contact capture, not the original full frame4 initial state. Final tiny trials had equal rounded total energies but mixed-sign per-rod energy differences and nonzero contact merit. Therefore energy accumulation alone is not established as the cause; neither identity preservation nor accepting rounded equal energies establishes feasibility. Full-cycle and FPS qualification remain open.

### Primary IPC solver source audit

Retrieved ipc-sim/IPC commit573d2c7e04104d3f9baf526bdaee7745891a571a, MIT license, Optimizer.cpp and Optimizer.hpp into sources/ipc-sim__IPC. solver-source-manifest.json records immutable URLs and SHA256 hashes. This is the solver repository, distinct from the earlier ipc-toolkit geometry/trust-region audit.

Optimizer.cpp1868-1884 checks a Newton-increment infinity norm against targetGRes, requires a noninitial iteration and completed prescribed motion, then computes the next direction. Around1885-2045 it limits alpha through material and collision feasibility before calling lineSearch;2045-2072 distinguishes tiny feasible alpha from tiny alpha after energy backtracking. These are concrete separation and diagnostic patterns, not a transferable numeric tolerance. IPC uses barrier/contact energies and vertex unknowns; Voxy has constrained Cosserat translations and orientations. Copying its threshold, accepting a small accepted scale, or equating tangent_complete with nonlinear equilibrium would be unjustified. Next implementation needs a physically scaled constrained optimality measure for both translation and rotation, independent of geometry/whole-path admission. No IPC solver replacement or production convergence claim is made by this audit.

### Full-step energy gate experiment rejected by physical regression

Further IPC source inspection: Optimizer.cpp2662-2790 evaluates energy for the initial feasible step, then backtracks against lastEnergyVal + stepSize*c1m; updateTargetGRes2939-2945 scales the stopping threshold with bbox size and dt unless absolute parameters are configured. Neither a numerical threshold nor the barrier formulation transfers directly to constrained Cosserat coordinates.

An attempted Voxy full-step uphill-energy rejection exposed continuous_mode_admits_imported_overlaps_before_starting_its_clock:392 library tests passed,1 failed. Restricting the new gate to continuous elastic projection still failed the same case. The preserved full-step-energy-overlap-diagnostic.log shows a feasible origin at1.3198587506059072e-14J, positive direction slope9.56844403525322e-24J, and full-step increase9.562926880496455e-24J. Smaller trials increase or round to equal energy, so a strict descent-only gate cannot admit this state. This is a direction/optimality issue, not proof that changing an energy summation alone fixes convergence.

The attempted behavior change was removed; physical gates and completion behavior are retained. Added opt-in full-step energy observation and unscaled maximum Euclidean vertex displacement/rotation increment plus their dt-scaled velocity changes to the existing elastic direction trace. These measurements are not a KKT residual or a convergence threshold. They distinguish a small direction from a small accepted line-search scale and provide evidence for a physically scaled constrained optimality design. No nonphysical radius/energy tolerance introduced.

### Exact trajectory-row terminal result

The independently frozen392-test exact-trajectory-rows-frame4 run is now terminal:exit101, runner733.319s, test720.27s. Final attempt271 had slope-2.46064218508588e-12J and full merit1.7197713645525855e-20, then no descending geometry step. Exact discovery identity remains covered by its grazing regression, but does not fix this capture. This supersedes earlier live status; do not restart or describe it as a successful frame4 qualification.

Validation of the retained observation change:393 physics library tests pass (44 ignored); hair integration21 and hair_rebase1 pass. Re-running the overlap test with tracing passes and measures the initially uphill full direction at9.760611334265765e-14m maximum vertex displacement and9.77837648275675e-13rad maximum rotation. These are actual unscaled full-direction magnitudes, not accepted trust scales.

Extended tools/analyze_hair_refinement_trace.py to preserve signed elastic slopes, full-step energy records and optional displacement/rotation/velocity magnitudes. Older directions remain distinguishable by absent magnitudes instead of invented zeros. New metrics must be a complete group, finite and nonnegative; signed slope/delta remain valid. Checked actual new and legacy logs plus NaN, negative and partial metric rejection. Scope remains observed trace only, with no equilibrium/full-cycle/FPS certification.

full-step-direction-stagnation terminated exit101 after99.37s on the secondary residual fixture. Final measured direction: {"line": 255, "iteration": 31, "slope_j": -1.1686804223274552e-10, "full_merit": 2.237452874608742e-10, "max_translation_m": 2.8668758154974667e-05, "max_rotation_rad": 0.004145288847579854, "max_velocity_change_m_s": 0.01376100391438784, "max_angular_velocity_change_rad_s": 1.98973864683833}. The diagnostic preserved the existing rejection, not a convergence fix. Frozen binary/source/fixture hashes and trace-analysis.json are retained. Small backtracking scale is insufficient as a stopping criterion for this finite full direction; next work must reconcile the physical tangent constraints and constrained optimality, preserving exact geometry and whole-path gates.

### Residual-level signed elastic work

Following the IPC source audit of energy-owned line search, isolated a separate floating-point defect: subtracting rounded total potentials can erase a physically signed change even when a feasible prefix is present. The actual squared Cosserat/inertial residual owner now streams N=1 poses for potential or N=2 for the signed difference, without allocating term buffers. It evaluates b²-a² as (b-a)(b+a), with Neumaier accumulation within and across rods. The pose shape, finite result, material/predictions and common timestep remain checked. This refactors the same inertia, stretch/shear and bend/twist energy; it is not a barrier model or an IPC solver transplant. Total potential multiplication grouping changes slightly from the old evaluator, so bitwise equal total-energy traces are not promised.

Shortened feasible trials now require strictly negative residual-level work rather than a decrease in two rounded total energies. No energy tolerance or geometric/contact relaxation was introduced. The outer feasible progress window now sums signed work for its eight transitions with compensation instead of subtracting absolute-energy endpoints. Nonfinite/missing work is rejected; positive-merit contraction policy remains unchanged. Full-step completion remains the existing tangent/geometry gate: the failed full-step strict-energy experiment was not reinstated.

Regression gives one station a large independent inertial term that hides the other station's small energy increase in the rounded total; paired residual work retains its positive sign, exact reverse antisymmetry and zero-motion identity. Ordinary translational/rotational change agrees with direct subtraction; malformed pose shape is rejected. Progress tests cover tiny negative work hidden by an8e-5J baseline, large cancelling signed moves, zero/positive work and missing/nonfinite history. Initial residual evaluator and progress builds each passed394 library tests; final combined validation and a separately frozen residual replay follow.

Final residual-work validation:394 library tests (44 ignored),21 hair and1 hair_rebase integration tests passed. residual-energy-validation.json records log SHA256 and verifies every frozen replay physics source hash still matches the current source. Analyzer checks current/legacy signed-work records and rejects nonfinite work or invalid trial scale.

Frozen residual-energy-stagnation replay is live on its own binary and unchanged secondary precision-backtrack-frame4 fixture. It passed the old terminal attempt31 by admitting scale2.3283064365386963e-10: global total-energy difference0, per-rod total difference-2.755834928781667e-20J, paired residual work-2.7486741686708243e-20J, accepted contact merit0. This establishes a formerly erased strictly descending feasible step, not convergence. Latest observed attempt34 continued physical model refinement; live analysis counted5 trial descents hidden by rounded totals and2 accepted nonlinear moves. The job must be revalidated/polled by its process/session rather than restarted. No original full frame4,720-frame cycle, GPU or160FPS qualification is claimed.

### Finite inertia range after shared residual refactor

Inspection of the new residual owner found a range regression introduced by forming0.5/inverse_mass/dt² before multiplying the squared residual: the coefficient can overflow even when the final physical energy is finite (including exact rest). Replaced the eager coefficient with a small ResidualWeight enum that preserves the original inertia evaluation order0.5*value/inverse/dt², shared by potential and signed work. Elastic material weights retain their prior owner. This is not a general proof of overflow/underflow safety for every representable material/pose.

New regression exercises inverse mass1e-300, permitted dt1e-6, exact rest and a1e-155 displacement. The corresponding inertia coefficient overflows but original-order energy and signed change are finite and agree with independently evaluated scalar inertia.395 library tests (44 ignored),21 hair and1 hair_rebase integration tests pass; diff whitespace check passes. residual-inertia-range/validation-manifest.json retains current-source hashes, frozen binary and test-log hash.

The live residual-energy-stagnation job deliberately remains on its earlier394-test frozen binary; this later inertia evaluation-order repair is not retroactively included in its manifest or attributed to its trajectory. The previously saved residual-energy-validation.json described source equality at its creation time, not equality after this subsequent source edit. Do not restart a live job because current source has advanced. Latest verified process remained live beyond attempt120; full physical convergence and FPS remain unproven.

### Residual-work replay terminal result

The394-test residual-energy-stagnation frozen run terminated exit101 after435.30s (test415.71s), at attempt163. It accepted10 nonlinear moves and observed21 negative residual-work trials hidden by rounded total-energy subtraction. Final full direction still had maximum displacement4.760878574981548e-7m and rotation2.0682795948232037e-5rad, with slope-2.460555830592488e-12J and full contact merit1.7159923457455576e-20. Thus the stable work comparison removes a demonstrated false-zero rejection, but does not establish constrained equilibrium or solve the frame4 capture. trace-analysis.json records the terminal trace. The later395-test inertia-order repair was not in this frozen replay. No longer describe this process as live.

### Grounded authored playback integration

Reuse the already imported CC0 Quaternius library pinned via J-Ponzo/gltf-universal-animation-library revisione24c23cf2a1323488a3faa226ea7ea21f644b73e (assets/animation/quaternius-universal/UPSTREAM.json). No invented walk/run oscillators or new animation asset registry. FemaleRig now has one AuthoredMotion selection owner: authored jump or a grounded clip using the same immutable source/binding machinery. Grounded pose sampling uses clip duration and the shared in-place extraction API; actual body deformation and head attachments consume that same selected pose. Render/shell identities keep separate bindings with one shared source asset, preserved on model replacement. Invalid grounded jump selection is rejected before replacing the admitted rig selection.

Added SceneApp::with_female_locomotion and examples/female_locomotion.rs with idle/walk/jog/sprint selection. This is an in-place skeletal preview on the real character, explicitly without nonlinear shell/hair advance; it does not qualify locomotion contacts, root movement, gait transitions or FPS. Window title identifies the selected gait and preview scope. Existing jump APIs remain separate. New tests cover actual body deformation/head agreement/source identity/invalid selection and preview selection preservation across female-to-male replacement.

Grounded preview validation: actual humanoid profile and actual grounded body/head tests2 passed; grounded rig plus body-replacement integration2 passed; jump rig regression1 passed with1 ignored (overlapping grounded rig test counted once:4 unique tests). Release example compiled and launched. For native UI inspection, packaged a local app bundle with an owned frozen executable (an initial executable symlink prevented reliable UI attachment). CUA confirmed window title Voxy Walk / in-place skeletal preview; initial blank startup capture was followed by two rendered captures with distinct arm/leg positions on the actual body. Preview remains playing. grounded-preview-validation.json records executable/source/test-log hashes. No nonlinear physical gait, foot contact, blending transition or FPS claim.

### Shared grounded transitions on the real character

GroundedPlayback reuses the native transactional Animator and the pinned CC0 Quaternius clips. One source clock publishes admitted cached poses to both render and shell bindings. Walk/Jog/Sprint transitions transfer normalized phase; interrupted fades preserve the displayed source pose using existing frozen-pose semantics. Model replacement retains playback phase and transition state. Phase transfer alone does not synchronize authored foot contacts.

Runtime keys1–4 select Idle/Walk/Jog/Sprint with0.25s blending. Three grounded tests pass, including interrupted-fade continuity, invalid-input clock preservation and body replacement. Continuity uses a1e-6m render tolerance after an exact-equality assertion exposed2.421438694000244e-7m normalization roundoff; no physical tolerance changed. Jump rig regression1 passes,1 ignored. Release example builds. Native UI confirms Walk launch, Sprint rendered pose, and Idle/Jog selection titles. The copied executable in Voxy Gait Transitions.app is frozen; grounded-transitions-validation.json records source, log and executable hashes. No foot locking, nonlinear shell/hair cycle or FPS qualification is claimed.

### Actual authored gait and support-relative IK qualification

Rechecked the pinned ozz foot example at https://raw.githubusercontent.com/guillaumeblanc/ozz-animation/744eb9d99f606eda849acb0b1204f7a3dc20bca1/samples/foot_ik/sample_foot_ik.cc : sample animation, establish actor height, query ankle supports, build ankle targets, correct pelvis, then solve legs and ankle orientation. Voxy already owns exact paired planted-leg reach with bounded shared pelvis displacement, as well as support-local FootPlant pivots. Reuse these existing owners rather than duplicating sample systems or treating its raycast-only terrain adaptation as authored stance detection.

Direct inspection of the imported Quaternius glTF finds no animation extras/extensions on Idle_Loop, Walk_Loop, Jog_Fwd_Loop or Sprint_Loop. This does not prove arbitrary source formats cannot contain contact events; it establishes this imported file does not supply such metadata. A production gait needs explicit stance authority and accepted physical actor movement, not both feet forcibly planted throughout a walking clip.

Added a real-body qualification that samples four authored clips at121 phases each. Explicit full-weight double-stance targets use support-relative pivot anchors, a1e8m common world origin, actor translation up to.03m and yaw up to.04rad. Existing paired IK reserves.005m before knee extension, within.35m pelvis travel. The acceptance independently reconstructs ankle/pivot positions and sole axis from the solved skin matrices, checks bone lengths and unchanged nonroot local translations/scales. It is an explicitly requested stance geometry test; it does not qualify contact acquisition, swing timing, collision dynamics, GPU rendering or runtime FPS.

Final actual-rig qualification passes484 poses with maximum ankle error1.8064793039229698e-7m, including independent pivot and sole-orientation checks. The existing three grounded playback/transition/replacement regressions also pass on the same current test executable. real-gait-plants-validation.json records source, imported asset, executable and test-log hashes plus per-clip metadata observations. This adds acceptance evidence and regression coverage; the running gait preview was not changed to force double stance, and runtime gait contact timing remains open.

### Authored foot roll and native trajectory inspection

The pinned ozz foot sample exposes independent `two_bone_ik_` and `aim_ik_` controls. Voxy's existing contact owner previously always aligned the sole to support; added explicit `FootOrientation::{AlignSupport,Authored}` to the same FootBinding. Default/old-scene behavior remains support alignment. Authored orientation keeps the sampled proper ankle basis and adjusts ankle translation so the captured sole pivot stays on its support. This preserves authored rotation about a fixed caller-selected pivot; full foot geometry collisions and dynamic heel/toe pivot selection remain separate requirements. No new acquisition/clock/physics owner was introduced.

Added `voxy_render/examples/quaternius_gait_contacts.rs`, using ModelAsset and the existing f64 pose sampler, to measure hips/ankles/toes at241 phases for each of four pinned source clips. Raw trajectories and numerical summary are retained under artifacts/articulated-jump-2026-10-11/. Walk and Sprint measured position endpoints match; Jog's source hips differ by.0230365m and left ankle by.0245003m across the loop endpoint. This is a source-trajectory discontinuity requiring investigation/conditioning, not a contact solver failure. Sprint's left toe reaches source-model y=-.0435195m; there is no physical floor in this measurement, so it does not independently prove a collision. These observations rule out treating arbitrary low-foot thresholds and nominal loop names as production contact qualification. No gait-contact sidecar is yet authored or enabled in the running preview.

Final orientation-policy validation:184 editor CPU tests pass,17 GPU-dependent tests ignored by default. Explicit current-binary hardware execution passes all three foot GPU cases (alignment, retargeted alignment, authored roll) on Apple M4 Max/Metal, each with1184 retained bytes and zero after clear, no validation errors. authored-ankle-roll/validation-manifest.json stores source/log hashes and an owned frozen test executable. Native roll presentation, actual gait stance timing, full-foot collision and FPS remain unqualified.

### Explicit Jog loop frame preparation

Inspected the actual pinned glTF keys: Jog hips and leg rotations end at a different pose at22/24s, while Walk/Sprint repeat their initial values. The source also contains16 STEP channels (root constants and tiny scale changes), so the first linear-only preparation attempt correctly refused it. The final native `AnimationClip::append_loop_frame` preserves every channel's interpolation and keys, holds channels that previously ended early to the old duration, then appends the first pose at a caller-selected new duration. LINEAR channels return continuously; STEP channels retain discrete changes. Cubic occupied channels are rejected rather than assigned guessed derivatives. Original clips/caches remain unchanged, and prepared caches are rebuilt by the existing validated constructor.

Primary mechanism reference: UE::Anim::AnimationData::AddLoopingInterpolation, https://dev.epicgames.com/documentation/unreal-engine/API/Runtime/Engine/Animation/UE__Anim__AnimationData__AddLoop-?application_version=5.5, explicitly appends an additional first-matching frame. No Unreal source code or architecture was copied. Voxy's pinned Quaternius recipe uses one24Hz frame, changes Jog duration from.9166666865s to.9583333731s, and preserves all other clip Arc identities. Repeated preparation detects matching endpoint poses and preserves the prepared allocation. GAIT_PREPARATION.json records the explicit asset recipe; raw glTF/bin remain unmodified.

FemaleDemo prepares the shared source once before compiling render/shell bindings. It does not apply runtime smoothing or give consumers different clocks. Original authored samples at their old absolute times and real-character retargeted endpoint poses are tested. STEP channels still step; equal endpoint poses alone do not prove derivative continuity, root-trajectory safety, contact timing or FPS. Normalized phases refer to the prepared, longer duration.

Final closure validation:255 animation core tests pass;5 real humanoid tests,1 replacement test and1 jump test pass,1 jump test ignored. Native CUA confirms Voxy Closed Jog.app launch, key3 selecting Jog and a rendered actual-character running pose. Raw source file hashes still match UPSTREAM.json. Frozen core/app test executables, copied preview executable, source/log hashes and qualified scope are recorded in jog-loop-closure/validation-manifest.json. Screenshots establish native rendering, not seam derivative/contact/FPS qualification.

### Pinned offline contact-curve baker and transfer limits

Inspected thecodebrozilla/FootContactCurves at866b7e05be3de6ee748b0c72c4e9602f9de8bf9f, MIT Copyright2026 Shane Beal. Private/FootContactCurvesModifier.cpp samples component-space foot heights, takes their minimum, then classifies with height and vertical-speed thresholds before writing transactional curve keys. Its first sample uses a forward difference, subsequent samples use backward differences. GitHub API rate limiting was bypassed with an ordinary exact-commit Git fetch; no external code was executed. Original source/license/hash manifest are retained under sources/thecodebrozilla__FootContactCurves/.

Added an independent bounded native periodic-contact baker in voxy_animation. It takes uniform heights, duration and explicit units/settings, uses both periodic neighbors for central vertical speed, and returns candidate gates plus measurements. It rejects malformed/nonfinite/range-overflow data rather than publishing partial gates. It is offline authoring only: no replacement physics, hidden low-foot runtime rule, or second contact clock. Native source sampling/export uses the prepared Jog duration and emits existing FootContactKey schemas for named per-foot mappings.

The actual pinned pack exposes a transfer failure: UE-style6cm/30cm/s thresholds on dense240-per-cycle ankle samples produce fragmented3–8% running candidates. Matching the24Hz authored grid improves Walk coherence but still leaves absent/short running contacts; the toe alternative also fails to produce a complete Sprint stance schedule. Raw/dense outputs and both24Hz candidate assets are retained with physical_contacts_verified=false. They must not be autoactivated or represented as production foot timing. CONTACT_BAKE.md records the settings, reproduction and required per-clip/manual calibration. Existing physics/contact/mixed-fade owners remain unchanged.

Validation: the native animation suite passed 257 tests; the editor suite passed 185 tests with 17 ignored. The real-source candidate binding test checks named clip and bone binding, periodic endpoints, skipped swing handling, and serialization. These checks do not qualify physical stance or frame rate. Evidence: `artifacts/articulated-jump-2026-10-11/contact-bake/validation-manifest.json`.

### Explicit support-relative pivot transfer

The existing `FootPlant` now exposes `with_pivot_preserving_ankle`: an immutable
candidate that transfers a heel/toe pivot while preserving the currently solved
ankle position and rotation. Its support-local point changes by
`R_support^-1 R_ankle (new_pivot - old_pivot)`; subsequent roll stays about the new
point. The caller retains support identity, contact admission and event ownership.
This is not an automatic roll detector: the new point must be admitted against the
support before publication, and can be above or below the surface if the caller
chooses an invalid transfer pose. Editor curve/event integration remains open.

Primary reference inspected: Daniel Holden, [Inverse Kinematics and Foot Locking](https://theorangeduck.com/page/inverse-kinematics-foot-locking),
which preserves the authored toe-to-heel relation when deriving a heel target.
This extension independently derives the rigid pivot-transfer equation; no code
from that article is copied. It reuses the existing contact primitive rather than
introducing another runtime contact owner.

Pivot-transfer validation: 259 animation tests passed. The added regression checks a moving/rotating support, common world origins at 1e12 using local-coordinate comparison, reversible transfer, overflow rejection and preservation of the original candidate. These are geometric checks; physical surface admission and editor event integration remain unimplemented. Evidence: `artifacts/articulated-jump-2026-10-11/contact-bake/pivot-transfer-validation.json`.

### Same-support pivot admission

`FootContactState::prepare_pivot_transfer` uses the existing bounded support probe
and resolves the old anchor before admitting an explicitly requested replacement
point. It requires a valid active stance, acquisition distance/slope admission and
the same support owner in the same scene snapshot. A miss or different owner returns
`None`; errors and rejection leave the original state unchanged. Query charges are
retained. No new contact state machine is introduced. The result must be published
atomically with the caller's matching geometric pivot/pose candidate; the API does
not itself perform geometric transfer, choose gait events or guarantee zero pose
correction when the probed surface differs from the proposed point.

### Combined physical/geometric pivot regression

A gameplay regression combines the actual SupportWorld admission and FootPlant
transfer on a box platform, then moves and yaws the platform while rolling about
the admitted toe. It checks toe coincidence with the resolved physical anchor and
heel clearance for the chosen toe-off direction. This is a rigid contact-site
fixture, not full foot-mesh collision or a complete character gait. A separate
4cm elevated proposal is admissible under the acquisition band but projects 4cm
to the surface: physical admission alone must not be advertised as zero-displacement
geometric transfer. Editor publication must explicitly apply/bound that correction.
The first fixture used the wrong toe-off rotation sign and correctly failed heel
clearance; the sign was corrected, without weakening the clearance assertion.

### Explicit projection-correction budget

Pivot transfer now requires `max_projection_distance` in world units, independently
of the ordinary acquisition band. A proposed point can satisfy acquisition yet
fail the tighter transfer limit. The candidate reports the resolved contact; the
caller must apply `contact.position - input.sole` to the corresponding geometric
pose and publish both together. Zero requires exact coincidence; this API does not
promise smoothing. Invalid limits reject before charging any support queries.
The elevated-point fixture checks a 4cm projection against 0/3.9cm rejection and
4.1cm acceptance, plus NaN/infinity/negative/over-capacity limits. The limit bounds
projection displacement only, not total leg IK correction or joint angles.

### Projection correction is explicit in the transfer result

`prepare_pivot_transfer` now returns the public must-use `FootPivotTransfer`, with
an admitted nonoptional `SupportContact`, proposed `FootContactState`, and exact
world-space `correction = contact.position - input.sole`. Ordinary acquisition
still returns `FootContactCandidate`; its optional-contact semantics are not
changed. The caller can stage both state and pose correction and discard them if
IK fails. The 4cm elevated proposal verifies direction, magnitude and that applying
the reported correction lands exactly on the admitted point. The report does not
itself publish state, run IK or smooth displacement, and cannot force callers to
use the correction. Editor gait-event integration remains open.

### Admitted contact to existing IK target

`FootPivotTransfer::ik_target` converts the physically admitted contact to a
full-strength model-space ankle target using the rigid actor frame, current proper
ankle orientation and explicit ankle-local pivot. It uses the admitted point
directly, including the projection correction; it does not target the uncorrected
proposal. The frame conversion subtracts actor origin before rotation. Existing
`Pose::solve_two_bone` / `AnimatorFrame::with_two_bone_ik` remain the sole IK solver.
The caller must reject `reach_clamped` or failed IK before publishing the proposed
contact state. This bridge does not itself solve/publish a pose or schedule events.
The regression checks a translated/yawed actor, rotated ankle, a 4cm projected
proposal, exact target weight and invalid pivot/orientation/pole preservation.

### Pose/contact staging through native IK

`FootPivotTransfer::solve_pose` now applies the admitted target through the existing
native two-bone solver and returns a staged `(Pose, FootContactState)` only when the
full-strength goal is geometrically reachable. `reach_clamped` yields `None`; errors
and rejection retain the source pose and old published contact. This bridge performs
no scene publication and introduces no independent clock/solver. A three-joint rig
regression checks the solved toe against the physical support point after a 4cm
projection, verifies the staged state, and rejects a distant actor target while
preserving source locals. Full-character event scheduling, collision of the entire
foot mesh and atomic editor publication remain open.

### Renderable frame staging

`FootPivotTransfer::solve_frame` stages the corrected `AnimatorFrame` and replacement
contact state through existing `with_two_bone_ik`. It rebuilds the skin palette and
preserves root-motion joint/displacement and transition weight. It accepts a borrowed
source frame and rejects clamped reach without consuming the previous frame. No
Animator, playback clock or scene publication is owned by this bridge. The combined
fixture compares the palette to the solved pose, checks toe/support coincidence,
metadata preservation and unreachable-target preservation. Calling this operation
from editor gait events and publishing its result remain separate unfinished work.

## 2026-10-13: exhausted swept refinement and source-review coverage

Revalidated the saved corpus with `tools/engine-research/audit.py`: 1428 pinned
README/root records contain 500 classified engine repository identities, including
52 derived engines. Those identity records remain `root_and_readme_only`. The
mechanism manifests contain 72 source records, 69 unique repository/commit/path
files from 11 repositories. These counts do not establish architectural review of
500 engines or full runtime parity. The auditor now rejects conflicting copies of
the same pinned source even when both copies independently match their declared
local digests; a regression covers that contradiction.

The already pinned IPC `TrustRegion::warm_start_time_step` and `update` separate
predicted displacement, safe-region centres/radii, broad-phase candidate inflation
and candidate refresh. Its inflation is derived from displacement and `dhat`;
restricted vertex count can trigger an update. This is a reference for separating
candidate validity from pose publication, not proof that per-vertex truncation can
replace Voxy's coupled rod/continuous-trajectory admission. Voxy must retain its
original force balance, shared component scales, prescribed roots and complete
swept certificate when adapting this scheduling pattern.

The pinned PBD `solve_StretchShearConstraint` uses original rest length and director
orientation to form stretch/shear corrections; the source adds its own numerical
regularizer to the denominator. These equations do not justify importing that
constant into Voxy's physical force/residual tolerance or replacing the native
implicit system with visual positional corrections.

The old Voxy 1024-capacity full-density run stopped at frame 5 after 32 swept
refinements. Its preserved outer state confirms four published frames and no
publication of the failed step. It does not identify whether the terminal limit
was elastic or collision related. The VJR1 geometry-only replay reconstructs rest
lengths from current positions, so it cannot distinguish that elastic failure.
The new companion native rod captures preserve current/previous rest state,
material, orientations and velocities; the final summary records elastic and
angular-admission observations. Future exact terminal captures must determine the
limiting mechanism before changing refinement budgets or adopting trust-region
updates. New captures omit external mesh geometry and are not complete game-state
replays. The dedicated 10-percent-stretched rod regression and 40 safe-motion tests
pass; the actual frame-5 failure is still unresolved.
