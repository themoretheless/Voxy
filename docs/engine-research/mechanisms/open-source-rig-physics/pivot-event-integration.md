# Pivot-event integration: verified boundaries

Inspection date: 2026-10-11. This is an integration contract, not implemented gait support.

## Sources inspected

- Current `docs/adr/0002-general-engine-architecture.md`: authoritative simulation,
  authoring and presentation are separate; fixed ticks own publication.
- Current `crates/voxy_editor/src/animation_runtime.rs`: `accept_physics` already
  calls `FootRuntime::correct_at_phase` with the physically accepted actor matrix,
  shared query budget, target/source intervals and frozen-source tick.
- Current `foot_placement/contact_events.rs`: mixed swing is evaluated in common
  fixed-tick coordinates, including skipped windows and bounded event work.
- RAG wiki `voxy-voxy-adr-0001-8f968a85`, page id
  `1a54f62b-5e07-41c9-8d5c-1a796143b287`: historical single-writer contract.
  Its older V1 platform exclusions and 60fps target do not supersede the current goal.
- RAG raw `4d76ad78-004c-4144-859d-a8be2724fb82` and
  `591525b0-eab7-4200-bc32-d14381a8875e`: MakeHuman ankle/foot/toe rig layouts;
  retrieved snippets do not establish ready locomotion event or locking implementations.
- [Godot SkeletonModifier3D](https://docs.godotengine.org/en/stable/classes/class_skeletonmodifier3d.html):
  modifiers run after AnimationMixer; influence is applied by the skeleton, so a
  modifier should not apply it a second time. Voxy retains its own transactional ownership.

## Integration requirements

1. Author pivot identity/position separately from stance weights. Preserve legacy
   fixed `sole_offset` behavior for assets without pivot metadata.
2. Sample requested pivot events using the existing target/source/frozen playback
   intervals. Define interrupted-fade behavior explicitly; no second phase clock.
3. Stage changes inside the accepted-physics correction path. Resolve the old
   anchor and probe the replacement against the same immutable support snapshot.
4. Require same support identity, explicit projection limit and successful IK.
   Retain the previous pivot, contact and frame on rejected transfer. Do not silently
   publish the contact or advance a pending event when pose preparation fails.
5. Persist the accepted local pivot in the staged per-foot runtime so subsequent
   frames keep using it. Do not overwrite shared binding data or retarget source data.
6. Publish pose, palette, pivot and contact together with existing owner playback.
   Keep root-motion/transition metadata and the single accepted tick authoritative.
7. Test skipped events, interrupted fades, budget failure/retry, support removal,
   asset replacement, unreachable targets and source-to-target rig pivot mapping.
8. Qualify actual Walk/Jog/Sprint contact labels and toe/heel collision on the model
   before enabling preview gait locks. Threshold-sweep candidates remain unapproved.

## Retrieval limitations

Index-first navigation was followed. Wiki lexical hits for the contact query were
unrelated CAD results; the raw hybrid query exceeded its embedding budget. The raw
lexical fallback returned rig definitions rather than contact scheduling. This is
not evidence that no useful document exists in the corpus. Saved tool responses:
`artifacts/articulated-jump-2026-10-11/contact-bake/rag-integration-retrieval.json`.

## Runtime progress

The editor BoundFoot now retains its accepted local pivot separately from shared
FootBinding data. `request_pivot_transfer` validates and stages an explicit request.
The existing accepted-physics correction path probes it, requires full contact
weight, and updates pivot/contact only after reachable IK and palette preparation.
A miss or clamped transfer retains the accepted pivot and pending request. Subsequent
frames use the accepted pivot for acquisition and correction. Outer owner/frame
staging remains responsible for rollback of whole ticks and multi-foot failures.

The request API is not yet called by production clip events. It currently provides
the runtime integration point; named phase metadata, fade arbitration, event retry
identity/cancellation and source-to-target pivot mapping remain unfinished. An
explicit pending request is not automatically canceled by swing or asset events;
those policies must be implemented before scheduling authored gait events.

## Stance lifetime cancellation

Explicit requests now require an existing anchored contact. A request is canceled
when mixed swing is crossed, the actor is airborne, planting is disabled, contact
weight is zero or physical preparation no longer resolves a contact. Swing/airborne/
disabled stance resets the accepted pivot to the authored default for the next
landing. A rejected same-stance surface proposal stays pending while the old contact
remains valid. This prevents a prior stance request from leaking into a new landing.
The regression checks unanchored request rejection, airborne cancellation and the
next landing. Budget errors still propagate through outer staged-owner rollback.
The earlier note that requests survive swing is superseded by this implementation.
Clip scheduling and fade arbitration of competing pivot requests remain open.

## Atomic per-owner foot correction

`FootRuntime::correct_at_phase` now stages the bounded per-foot state on a clone
(shared settings remain Arc-owned), executes correction, and replaces the runtime
only on success. Failed event scans, queries or IK cannot partially consume pending
requests, reset accepted contacts or publish pivot changes in earlier feet. Shared
query charges intentionally remain spent on failure. The regression crosses a
zero-contact window, spends its five-key event budget, fails the subsequent support
probe, verifies old contact/pivot/request preservation, and retries successfully
with sufficient budget. This complements the existing outer accepted-physics owner
transaction; it does not introduce a second playback clock. Copy cost has not been
profiled and this change does not qualify the frame-rate goal.

## Named pivot metadata

FootBinding now contains optional `clip_pivot_keys` maps of public `FootPivotKey`:
normalized phase, target-ankle-local pivot and projection correction limit. Keys
are intended as discrete step requests, not interpolated contact weights. The schema
requires phase 0/1 endpoints with matching pivot and limit, strict increasing phases,
finite bounded coordinates/limits, at most 64 named maps and 4096 aggregate keys.
The runtime binder validates that each named clip resolves uniquely alongside stance
maps. Missing metadata deserializes to an empty map for legacy fixed-pivot behavior.
The schema is currently for closed cycles; noncyclic endpoint semantics would need
an explicit extension. Metadata is not yet sampled or scheduled by production ticks.

## Automatic target-phase selection

The accepted-physics foot correction now samples named pivot keys with a step-valued
binary lookup on the existing normalized clip phase. Missing phase/clip mappings
reject the staged correction. Sampling charges the shared work budget. In ordinary
stance the selected point seeds new contact acquisition or replaces the pending
same-support request; rejected requests retry only while their key remains selected.
A key matching the accepted pivot clears an obsolete request. Multi-key travel
selects the current phase state rather than replaying obsolete intermediate locks.

During live or frozen source fades, pivot changes are deferred and pending requests
are cleared; the accepted pivot is held unless swing/airborne cancellation applies.
When the source disappears, the current target key regains ownership. This is a
hold-through-fade policy, not blending heel/toe contact identities. Full-strength
IK, same-support admission, projection limits and atomic staging still apply.
A native closed-cycle fixture checks initial heel, a live-fade hold, toe selection
after fade completion and phase-one heel closure. Real asset pivot labels, retarget
mapping and full-model motion/collision/FPS qualification remain unfinished.

## Actual Quaternius pivot coordinates

`quaternius_gait_contacts --inspect-rig-pivots` samples the imported native f64
skeleton at 241 phases in each of four raw clips, on both sides (1928 records).
The toe joints are direct children of their ankles. Their origins are approximately
`[0, 0.17330084, 0]` in ankle-local meters, with maximum local drift of 1.50e-7m
(left) and 2.31e-7m (right), and reconstruction error below 2.8e-16m. The source's
local bone Y points along this segment; it must not be assumed to be world up or
the conditional fixture's forward Z. This inspection measures bone origins, not
heel/toe sole-surface points. Target-body retargeting must map the site's position
and foot sizing explicitly and validate actual sole geometry before enabling the
pivot metadata. Native evidence: `source-rig-pivots.json` in the contact-bake folder.

## Reference-local point retargeting

The existing ModelRotationRetarget now provides authoring-only
`map_reference_local_point(source_joint, point, size_ratio)`. It resolves the mapped
target joint, carries the source reference bone-local vector into model coordinates,
applies the explicit model-axis conversion and positive size ratio, then converts
through the inverse target reference affine frame. Bone origins are corresponding
anchors; source model translation is deliberately not copied to the target bone.
Full affine vector transforms retain scale magnitudes and reflection rather than
assuming local Y/Z axes or using only a rotation quaternion. Translation-channel
scaling is not reused as an implicit foot sizing rule. Target reference pose follows
the canonical binding's reference calibration.

The operator rejects unmapped joints, nonfinite points, invalid sizing and singular
or overflowing frames. It does not infer heel/toe surface locations or preserve
animated end-effector positions. It is not called in the per-frame path. Regression
uses different bone/model axes, unequal nonuniform scales, a reflected target and
an explicit 1.4 size ratio, with an independent model-space vector equality check.
Actual target-model sole geometry and its selected sizing remain unqualified.

## Actual target sole geometry inspection

A focused native app test loads the actual prepared OBJ and its existing FemaleRig
skin weights. Selecting vertices with at least 0.5 foot-joint influence gives 4441
vertices per foot. It converts actual vertex coordinates into ankle-local space.
Left bounds are approximately X[-0.00668,0.09938], Y[-0.08004,0.01230],
Z[-0.08909,0.14600] meters; right bounds mirror X with small mesh asymmetry.
The lowest 5mm band contains 422/429 vertices and spans roughly Z[0.030,0.138].

Consequently the lower-band rear extremum logged as `heel_candidate` is NOT an
anatomically verified heel: it excludes the rear mesh extent near Z=-0.089. The
mesh has a curved sole and lateral offset from the authored ankle, so a single
lowest-band or bone-origin heuristic must not be promoted to a heel/toe profile.
Need inspect rear/forefoot surface regions separately and verify collision/roll
against the actual skinned geometry. No physical pivots are enabled by this test.
Evidence: `target-foot-surface-validation.json` and its frozen source/mesh hashes.

## Reference foot support envelope

The actual-mesh diagnostic now sweeps 25 pitch angles per foot from -0.6 to +0.6rad
and selects the point minimizing rotated Y over the dominant-foot vertex set.
Translating that point onto a plane keeps all selected rigid reference vertices
above the plane (verified to 1e-12m numerical tolerance). Keeping the angle-zero
pivot instead permits up to 0.0773033m left / 0.0768809m right penetration. Selected
local Z moves between approximately -0.081 and +0.141m, covering rear and forefoot
regions that the previous lowest-5mm band missed.

This is a support-envelope measurement of a rigid reference proxy, not a collision
certificate for animated weighted skin, all foot triangles, tissue deformation or
continuous motion between samples. No runtime pivots are activated. It establishes
that a fixed bone-origin/flat-contact point is inadequate and provides actual mesh
candidates for subsequent skinned-geometry/physical qualification. Evidence:
`target-foot-roll-envelope.json` and `target-foot-roll-envelope-validation.json`.

## Actual skinned gait envelope

A focused native test now loads the actual body and prepared Quaternius source
(including Jog loop closure), uses the existing calibrated humanoid retarget and
publishes 25 native in-place poses in each of Idle/Walk/Jog/Sprint. It runs the actual
FemaleRig deformation and compares the 4441 dominant-foot vertices per side to the
reference geometry transformed by each current ankle matrix. This is 100 actual
poses / 200 foot records, not substitute procedural clips.

Maximum absolute lower-envelope disagreement is 1.49e-7m Idle, 2.00e-7m Walk,
2.06e-7m Jog and 2.50e-7m Sprint. Local vertex deformation reaches 0.02073m near
mixed skin influences, yet does not change the selected subset's lower envelope
materially in these samples. The test checks finite results and that nonrigid
motion is exercised; numeric maxima are measurements, not universal certified
bounds. This supports using the reference proxy for further qualification of this
particular mesh. It does not cover all foot triangles, tissue simulation, arbitrary
poses, changing scale, support slopes, continuous collision or stance labels.
Evidence: `gait-foot-skin.json` and `gait-foot-skin-validation.json`.

## Ready OSS convex proxy preparation

The isolated offline Rust tool `tools/engine-research/foot-hull` pins
`parry3d-f64 = 0.28.0` and its own Cargo.lock, using
[Parry try_convex_hull](https://docs.rs/parry3d/0.28.0/parry3d/transformation/fn.try_convex_hull.html).
It is not a new gameplay solver/dependency. Native mesh diagnostics export exact
ankle-local source clouds into the captured log; the tool refuses absent/invalid
clouds, construction failures, non-source hull vertices and failed containment.

4441 points reduce to 629 left / 650 right hull vertices. Measured support errors
are zero over 1200 3D directions per foot. Independent face-plane scans place all
original points inside within 1.39e-17m floating error (admission tolerance 1e-9m),
and every output vertex equals an input point. This checks a reference convex proxy;
it is not an exact-arithmetic enclosure certificate or proof of animated/full-mesh
continuous collision. Extreme support points are retained rather than approximated
by mesh simplification. No runtime profile is enabled. Evidence: `foot-hulls.json`,
`foot-hull-validation.json` and the isolated dependency lock.

## Native affine support query on the OSS hull

`voxy_animation::FootSupportPoints` now selects the minimum reference point along a surface normal through the full linear bone/actor transform, including nonuniform scale and reflection. It owns no contact state and makes no physics query. Translation is excluded to retain local precision; exact ties retain source order. Invalid clouds, singular frames and nonfinite projections reject.

The standalone Parry 0.28.0 auditor calls this native query on both actual foot hulls (629 and 650 vertices). Across 1,200 directions per foot, maximum disagreement against all 4,441 source vertices is 5.551115123125783e-17 m. The core animation suite passes 262 tests, including affine/reflected support and invalid input regressions. Receipt: `artifacts/articulated-jump-2026-10-11/contact-bake/native-foot-support-validation.json`. Formatting was applied after validation without semantic changes.

This verifies a reference-geometry query. Runtime character contact admission, deforming skin collision, continuous collision and performance qualification remain open.

## Actual mesh affine support audit

The Parry audit now checks the native support query against independently transformed full source clouds (4,441 vertices per foot), rather than only synthetic affine fixtures. Six transforms per foot combine pitch -0.6/0/0.6 rad, nonuniform scale (2, 0.5, 3), reflection and shear. Each transform uses 1,200 surface normals: 14,400 total comparisons. Both the returned projection and transformed selected point are compared with the full-cloud minimum. Maximum measured error is 2.220446049250313e-16 m; failure threshold is 1e-9 m.

Evidence: `artifacts/articulated-jump-2026-10-11/contact-bake/affine-foot-support-validation.json` and `affine-foot-support.json`. This is reference-geometry validation, not deforming skin, continuous collision, runtime character contact or FPS qualification.

## Geometry selection request in FootRuntime

`request_geometry_pivot` now resolves the existing support anchor against the supplied accepted physics snapshot, composes the current ankle and actor linear frame from the sampled pose, selects a reference point with `FootSupportPoints`, and queues it through the existing pivot-transfer admission path. Missing support, invalid frame or failed query do not replace the pending request. Geometry selection alone cannot publish a contact.

The existing three-frame physical pivot persistence test now queues through geometry selection and also checks singular-frame rollback. Full editor suite: 190 passed, 0 failed, 17 ignored. Receipt: `artifacts/articulated-jump-2026-10-11/contact-bake/geometry-pivot-editor-validation.json`. This explicit entry point is not yet called automatically by the FemaleRig preview; real hull asset binding and automatic invocation remain open.

## Geometry selection work accounting

Geometry requests reserve one work unit per composed joint plus one per reference-point projection from the existing shared support query budget before either loop. Anchor resolution charges separately; spent physics work is retained on rejection. `FootSupportPoints::point_count` exposes the exact projection count. Invalid projection limits reject before physics queries.

The physical persistence regression now verifies exact work charges and a failed reservation with 128 geometry points and a 100-unit budget: existing support state and pending pivot remain unchanged. Full editor suite: 190 passed, 0 failed, 17 ignored. Evidence: `artifacts/articulated-jump-2026-10-11/contact-bake/geometry-pivot-budget-validation.json`. Work units are bounded operation accounting, not measured time or an FPS guarantee. Automatic character integration remains open.

## Automatic selection for installed geometry

`FootRuntime::set_support_geometry` installs validated immutable ankle-local reference points and a physical projection limit. It accepts only Authored orientation without authored pivot keys, keeping one owner of automatic pivot selection. On an established contact, full stance weight and no active source fade, correction automatically selects against the accepted support normal, charges projection work and proposes through existing same-support admission and IK. Explicit pending requests retain priority. Newly acquired contacts are eligible on the following frame. Rejected automatic requests are discarded and resampled from the next pose.

Regression exercises automatic selection without an explicit transfer call, three-frame contact/palette consistency, singular-work-budget rollback, physical rejection without stale pending state, incompatible orientation rejection and airborne release. Full editor suite: 191 passed, 0 failed, 17 ignored. Receipt: `artifacts/articulated-jump-2026-10-11/contact-bake/automatic-geometry-validation.json`.

The actual character hulls are not yet installed by the model asset/preview path. Asset persistence, full deforming-foot collision and runtime performance remain unqualified.

## Persisted support geometry binding

`FootBinding.support_geometry` now optionally stores target-ankle-local f64 points and the explicit physical projection limit through public `FootSupportGeometry`. Legacy assets omit this field and retain prior behavior. Validation rejects empty/oversized/nonfinite geometry, invalid limits, incompatible orientation and authored pivot-key conflicts. Runtime construction compiles the validated points once into shared immutable `FootSupportPoints`; no per-frame deserialization or point-cloud cloning is introduced.

The automatic physical pivot regression now round-trips model settings through JSON and creates the runtime from the loaded settings. Schema regressions cover legacy omission, invalid clouds, oversized clouds, invalid limits, incompatible orientation and unknown fields. Full editor suite: 192 passed, 0 failed, 17 ignored. Receipt: `artifacts/articulated-jump-2026-10-11/contact-bake/geometry-asset-validation.json`.

Actual FemaleRig hull data still needs binding to its runtime model and physical contact configuration. These tests do not qualify the full character, gait stance labels, deforming-foot collision or FPS.

## Actual FemaleRig reference hull asset binding

`assets/characters/blender-female/prepared/foot-support-reference.json` stores the offline Parry hull points under exact left_foot/right_foot identities (629/650 vertices). `FemaleRig::foot_support_geometry` loads the asset, resolves native bones, validates bounded geometry and explicit projection limits, and checks each hull vertex is an exact current dominant-weight source mesh point in the current bind ankle frame. It returns the joint identities and public persisted editor geometry settings. Missing/mismatched source vertices, wrong identity, invalid inputs or invalid limits reject. This membership check is not a full-mesh fingerprint or containment audit for arbitrary changed meshes.

The initial exact-membership test detected offline JSON-decoder coordinate rounding. Exported hull coordinates were canonicalized to their corresponding native source points, changing at most 2.7755575615628914e-17 m per component; the exact membership gate was retained. Canonicalization receipt and focused native test are in contact-bake/foot-asset-canonicalization.json and actual-foot-asset-binding-canonical-tests.log. Focused real-body test: 1 passed, 0 failed, 345 filtered out; shifted-body and invalid-input rejection are included. Binding receipt: `artifacts/articulated-jump-2026-10-11/contact-bake/actual-foot-asset-binding-validation.json`.

The loader is not yet invoked by the preview physical contact owner; no automatic full-character support, deforming collision or FPS qualification is claimed.

## Full reference geometry fingerprint

The hull tool and FemaleRig loader now both use BLAKE3 over `voxy-foot-reference-v1\0`, exact bone-name bytes plus NUL, point count as LE u64, and ordered ankle-local f64 coordinate bits as LE u64. All 4,441 dominant-weight source points per foot participate, including points outside the hull vertex list. Changing an interior source point, point order, bind coordinate or selected membership invalidates the asset. This hashes the selected reference geometry, not animation skin weights or the complete body.

The offline tool now enables serde_json float_roundtrip and regenerates hulls directly from exact native points, avoiding manual coordinate canonicalization. Both hulls retain 629/650 vertices; repeated 14,400 affine comparisons have maximum error 2.220446049250313e-16 m. Native focused real-body regression: 1 passed, 0 failed, 345 filtered out, including an interior-point change that previous hull-vertex membership alone could miss. Receipt: `artifacts/articulated-jump-2026-10-11/contact-bake/foot-fingerprint-validation.json`.

Strict bit fingerprints intentionally require rebaking when reference arithmetic or source ordering changes. Runtime preview physical-owner integration and full character/FPS qualification remain open.

## Real rig physical contact through skin deformation

Inspection confirms the current grounded FemaleDemo is animation_only and has no accepted CharacterTickPreview/SupportWorld owner. It must not fabricate contact authority from clip phase. A real-body integration fixture now uses the actual hull asset, actual FemaleRig ankle frames and native chains, accepted SupportWorld box contact 1 cm above the reference sole, native two-bone IK and palette publication. `FemaleRig::deform_pose` accepts the corrected pose directly so physical IK is not discarded by animation resampling; existing `deform` delegates with its sampled pose.

Both feet pass the actual integration fixture without reach clamping. Pivot errors are about 4.125e-8 m; measured dominant-foot skin minimum clearance is -3.725290298461914e-8 m (f32 deformation), maximum actual skin movement about 17.2-17.3 mm. The fixture tests one static support pose per foot, not gait contacts or continuous collision. Focused test: 1 passed, 346 filtered out. Skinning regressions: 4 passed, 0 failed. Receipt: `artifacts/articulated-jump-2026-10-11/contact-bake/real-foot-physical-skin-validation.json`.

The animation-only preview still needs a genuine accepted character controller/support owner and qualified stance intent before enabling physical foot placement there.

## Accepted character tick with actual feet

The actual FemaleRig physical fixture now uses the existing CharacterPhysics fixed_step_with_preparation transaction at 1/120 s, a physical CharacterBody under gravity and an actual box floor. Grounded state and actor displacement come from AcceptedCharacterPose; reference-foot support queries and native IK run in that tick's preparation callback. The accepted actor displacement is retained when evaluating the published skin. No separate simulation clock or fabricated grounded flag is used.

Both actual feet pass: pivot error approximately 3.484e-8 m, measured dominant-foot skin minimum clearance 0 m in this static fixture. A following tick requests 10 cm X movement, prepares contact and deliberately rejects pose preparation. The test verifies both published scene matrix and CharacterPhysics state are unchanged. Focused regression: 1 passed, 0 failed, 346 filtered out. Receipt: `artifacts/articulated-jump-2026-10-11/contact-bake/real-foot-character-tick-validation.json`.

This qualifies the integration transaction on an actual rig fixture. FemaleDemo UI still does not own this character transaction; full gait stance ownership, two-foot runtime integration and FPS remain open.

## Shared native skeleton entry point

Public FootRuntime::from_skeleton binds native rigs by canonical Skeleton names without creating a dummy ModelAsset. Imported model construction retains ModelAsset::resolve_joint_name because imported display aliases differ from canonical skeleton joint names. Both constructors share validation, chain admission and cached immutable geometry. Runtime retains a cheap shared Skeleton clone.

Public correct_fixed_stance stages the same existing contact/geometry/IK correction implementation using the retained skeleton. It rejects clip-driven contact/pivot bindings; editor phase-aware playback remains responsible for those. Callers must stage the owner during CharacterPhysics preparation and publish it with the returned corrected frame after transaction success. No duplicate contact or IK system is introduced.

The existing automatic physical geometry regression now exercises canonical native skeleton binding and this public correction path. Full editor suite: 192 passed, 0 failed, 17 ignored. Receipt: `artifacts/articulated-jump-2026-10-11/contact-bake/native-foot-runtime-validation.json`. Actual FemaleDemo adoption and full gait contact ownership remain open.

## Shared two-foot owner on actual FemaleRig

FemaleRig::fixed_stance_foot_runtime now compiles the verified hull asset, actual parent-derived leg chains, explicit caller stance flags and contact settings into the shared FootRuntime. Initial sole pivots come from actual reference geometry in bind frames; no manually named or guessed leg indices are used. Gait stance intent is not inferred.

A real-body integration fixture stages this owner inside CharacterPhysics::fixed_step_with_preparation and publishes the owner with its corrected frame only after success. Both feet are evaluated together for four 1/120-second ticks, including acquisition and automatic geometry selection, preserving frame root metadata and using deform_pose. All eight dominant-foot skin clearance measurements are -2.9802322387695312e-8 m against the physical floor, within the 3e-6 m regression limit. Focused test: 1 passed, 0 failed, 347 filtered out. Receipt: `artifacts/articulated-jump-2026-10-11/contact-bake/actual-two-foot-owner-validation.json`.

This qualifies fixed stance in the shared native path, not gait, jump, deforming collision or frame rate. FemaleDemo window adoption remains open.

## Dynamic actual-foot roll regression and lock-relative admission fix

The real two-foot fixture now spans 121 fixed steps, pelvis lowering up to 5 cm and opposing foot pitch up to +/-0.25 rad. It originally failed at tick 17 with dominant skin clearance -25.779 micrometers. Pivot admission incorrectly tested the new point from the unconstrained animation ankle, so animated pelvis displacement consumed the replacement projection limit despite the existing accepted foot lock.

Replacement requests now add the current admitted contact correction before the physical support probe: proposed = animated_new_pivot + (accepted_old_contact - animated_old_pivot). This tests the replacement relative to the current accepted ankle lock; same-support admission, projection limits, IK reach checks and rollback remain unchanged. The 3 micrometer skin regression gate and 2 cm projection limit were retained.

All 242 actual skin measurements pass, maximum absolute clearance 2.086162567138672e-7 m. Full editor suite: 192 passed, 0 failed, 17 ignored. Original failing log and successful logs are retained; receipt: `artifacts/articulated-jump-2026-10-11/contact-bake/pivot-lock-relative-validation.json`. This is a synthetic dynamic stance sweep on the actual rig, not authored gait/jump, full deforming collision or FPS qualification; demo window adoption remains open.

## Actual moving-platform support qualification

FootRuntime::support_lock exposes only the accepted SupportAnchor and accepted ankle-local pivot, excluding pending proposals. Physics retains ownership of world-space anchor resolution. The actual two-foot rig fixture now resolves both locks in each accepted CharacterTickPreview, checks support identity and compares corrected pose pivots against these current physical contacts.

Two 121-tick sweeps share the fixture: static floor and platform translation +/-3 cm along X plus yaw +/-0.2 rad, both with 5 cm pelvis compression and opposing +/-0.25 rad foot roll. Each also checks actual dominant-foot skin clearance against the physical plane. Both tests pass (484 foot measurements total, 347 other tests filtered out). Moving sweep maximum pivot error 9.063843345505482e-08 m; maximum absolute skin clearance 2.086162567138672e-07 m. Regression gates remain 3e-6 m. Receipt: `artifacts/articulated-jump-2026-10-11/contact-bake/actual-moving-platform-validation.json`.

Vertical platform motion, authored gait/jump, full deforming collision, demo-window adoption and FPS remain unqualified.

## First-landing geometry and vertical drop reacquisition

The actual rig fixture now includes a platform snapshot drop of 5 cm at tick 30 and restore at tick 80. CharacterPhysics determines airborne state under gravity; both foot locks must be absent in every airborne tick and the uncorrected sampled pose must be retained. This exercises 11 airborne ticks and physical landing at tick 41. It does not claim continuous platform dynamics or airborne mesh collision.

The first version exposed 8.724 mm dominant-foot penetration on landing: acquisition used the neutral pivot and deferred geometry selection until the following frame. FootRuntime now selects against the probed physical normal during initial acquisition, reprobes the current geometric point on the same support and publishes its pivot only after IK succeeds. Failed replacement admission does not publish the known obsolete geometric landing. Shared budget and staged rollback apply.

All three actual rig scenarios pass (static, horizontal/yaw platform, vertical snapshot drop). 704 grounded foot skin measurements have maximum absolute clearance 2.086162567138672e-7 m under the unchanged 3e-6 m gate. Full editor suite: 192 passed, 0 failed, 17 ignored. Original failing log retained. Receipt: `artifacts/articulated-jump-2026-10-11/contact-bake/first-landing-geometry-validation.json`. First-frame acquisition now supersedes earlier documentation that intentionally deferred selection to the next frame.

Authored gait/jump, continuous platform dynamics, airborne full-skin contact, demo-window adoption and FPS remain open.

## Running native physical stance example

`female_physical_stance` is now a standalone SceneApp example using `with_female_physical_stance`. The PhysicalStance owner composes the existing CharacterPhysics accepted preparation transaction and shared FootRuntime for both actual rig feet. FemaleDemo integrates it in bounded 1/120-second steps, publishes corrected poses to deform_pose and keeps hair/head attachment transforms aligned with the accepted head palette. A dedicated title identifies the physical stance diagnostic. The existing grounded gait mode remains separate.

Focused full-model render-vertex regression: 1 passed, 0 failed, 350 filtered out. It advances 12 character steps, verifies visible mesh motion and measures -3.725290298461914e-8 m floor clearance on the 60,866-vertex body. Release example build succeeded and the app bundle was launched; native UI observation confirmed the titled playing window and visible whole character. Receipt: `artifacts/articulated-jump-2026-10-11/contact-bake/physical-stance-window-validation.json`. Launch bundle: `artifacts/articulated-jump-2026-10-11/Voxy Physical Stance.app`.

This diagnostic advances character/foot physics, not nonlinear tissue or hair dynamics. It uses CPU skinning and rejects the existing incompatible VOXY_GPU_SECONDARY preview path rather than silently replacing its palette. It is not full gait/jump or a >160 FPS qualification.

## Native CPU baseline and accepted GPU-facing palette

The launched Physical Stance app was restarted with existing VOXY_FRAME_PROFILE instrumentation, raised in the native UI and sampled over 60 unpaused presented frames (10-69). Mean present interval 380.392 ms, 2.6289 FPS; p95 396.087 ms. Mean animation preparation 0.40285 ms versus CPU mesh stage 362.7431 ms. This is an observed host baseline with logging and other processes, not an isolated benchmark. Snapshot and calculations: `artifacts/articulated-jump-2026-10-11/contact-bake/physical-stance-profile-snapshot.log` and `physical-stance-cpu-profile.json`. The >160 FPS goal is unfulfilled.

PhysicalStance now retains the skin palette already prepared from accepted physical IK. FemaleDemo's GPU-facing palette accessor returns this palette with accepted actor displacement rather than sampling the unrelated jump/neutral animation. The head attachment also uses the same retained palette. Focused full-model regression verifies exact palette equality against corrected Pose::skin_matrices and exact head-matrix equality, while retaining the rendered floor-clearance check: 1 passed, 0 failed, 350 filtered out. Receipt: `artifacts/articulated-jump-2026-10-11/contact-bake/physical-stance-palette-validation.json`.

This prepares correct pose ownership for GPU upload; GPU skinning/hair geometry are not yet connected to this diagnostic. The visible app remains on the previously built CPU render path.

## Actual GPU physical stance skinning qualification

The existing resident rigid-skinning compute path now has an explicit hardware regression for accepted physical stance poses, sharing the original jump test harness. On Apple M4 Max / Metal both tests passed. Each compares all 60,866 body vertices over six poses against CPU deformation; physical stance maximum position error is 2.4507767e-7 m, original jump 1.9687948e-7 m, under the unchanged 2e-6 m gate. Trailing head attachments and UV/color preservation remain checked. Tests are hardware-dependent and ignored by default; this run explicitly executed them with --ignored.

Receipt: `artifacts/articulated-jump-2026-10-11/contact-bake/physical-stance-gpu-compute-validation.json`. This validates compute deformation only. The visible Physical Stance renderer remains on its CPU path; no FPS improvement or complete hair/tissue simulation is claimed.

## Full-resolution stage profile and pinned OSS reference

An explicit ignored diagnostic constructs the complete physical stance mesh over four accepted steps, including full hair and face features, and rejects nonfinite positions. It passed with 2,535,255 vertices per sample. Warm sample stage means are recorded in `artifacts/articulated-jump-2026-10-11/contact-bake/physical-stance-full-mesh-profile.json`: surface-light diffusion approximately 101 ms, hair/shapes approximately 29 ms, face features approximately 25 ms and mesh validation approximately 42 ms. These are CPU mesh-construction times, not presented FPS. GPU body skinning alone cannot remove the principal bottleneck.

Downloaded and inspected ozz-animation skinning sample at commit `744eb9d99f606eda849acb0b1204f7a3dc20bca1`, with license and SHA256 receipt in `contact-bake/ozz-skinning-source`. It separates sampling, local-to-model conversion and remapped inverse-bind skin palettes before rendering. This confirms the accepted pose/palette boundary; it does not supply our nonlinear skin solver, dynamic face shading or GPU hair physics. No C++ source has been copied into runtime.

## GPU diffusion on accepted runtime poses

The full-mesh diagnostic can export each actual CPU surface-light equation and converged reference with `VOXY_STANCE_DIFFUSION_FIXTURES` (use an absolute output directory). The exporter compacts the same active triangle domain: 60,814 vertices, excluding 52 unreferenced body vertices whose source values remain unchanged. The existing native GPU diffusion example now preserves input scope in its report rather than mislabeling runtime input as a static imported body.

Four accepted runtime poses pass the existing gates (maximum irradiance error <1e-3, squared relative equation residual <1e-6, nonnegative irradiance), each at 512 cold-start Chebyshev iterations. Maximum error across poses is 8.525626834954281e-5. Only one qualified-solve GPU timestamp was available: 27.916084 ms. Qualified wall times are 55.25-72.92 ms including encoding, waiting, readback and CPU equation audit. This establishes accuracy but fails the 6.25 ms whole-frame budget for >160 FPS; native renderer integration and a faster admitted solver remain open. Warm-start performance across changing geometry is not established. Receipt: `artifacts/articulated-jump-2026-10-11/contact-bake/physical-stance-diffusion-gpu-validation.json`.

## Resident preconditioned CG surface light solver

Added `SurfaceLightPcg`, an original WGSL Jacobi-preconditioned CG implementation for the unchanged `(M + lambda L)x = M source` RGB equations. The operator shares the existing symmetric CSR admission with SurfaceLightDiffusion. Reductions remain GPU-resident, dispatches are ordered in one compute pass, there are no floating-point atomics or CPU round trips inside the solve. The initialized incident buffer is reused as a contiguous search-direction vector after all initial neighbor reads complete; the weighted degree is retained with residual state, avoiding a repeated adjacency walk for diagonal preconditioning. Eight storage bindings are explicitly required and checked. Iteration counts alone do not establish convergence.

OSS mechanism reference inspected and pinned: Ginkgo CG kernels at `d5a8402121b9e4f4b3b82d30b54978d9192007d8`, source/license/hash receipt in `contact-bake/ginkgo-cg-source`. Ginkgo supplies the recurrence reference, not a linked Metal implementation; no C++ source was copied into runtime.

The analytic hardware gate checks unchanged channels, invalid source/operator rejection and split-call continuation. An independent four-vertex fixture checks an exactly zero color channel, a zero-radius channel and an isolated zero-mass vertex. Original Chebyshev analytic and full-body regression also pass.

Six full-resolution runtime fixtures span accepted physical stance times 0.0167, 0.2667, 0.5167, 0.7667, 1.0167 and 1.2667 seconds, retaining 2,535,255 vertices in CPU mesh construction and compacting only the original surface equation domain (60,814 active vertices). All six GPU PCG solves pass the unchanged irradiance/parity/residual gates at 40 iterations. Maximum irradiance error 8.195741376330057e-4; maximum squared relative equation residual 1.2657465977914982e-12. Actual GPU timestamps span 4.600542-4.6175 ms, compared with the earlier recorded 27.916084 ms qualified Chebyshev solve. These are kernel observations on the current host, not an isolated interleaved benchmark. Earlier unbatched/packed and near-initial-pose measurements are retained as historical evidence.

Receipt: `artifacts/articulated-jump-2026-10-11/contact-bake/physical-stance-pcg-cycle-validation.json`. Reproduce fixture export with `VOXY_STANCE_DIFFUSION_FIXTURES` set to an absolute directory and explicitly run the ignored `physical_stance_diffusion_cycle_fixtures` test; invoke `surface_light_diffusion INPUT REPORT pcg` for each fixture. Native renderer integration, GPU coefficient assembly for changing geometry, runtime convergence admission, broader poses/materials and the >160 FPS whole-frame target remain open. A fixed 40-iteration budget is not approved for arbitrary inputs.

## Resident dynamic surface operator assembly

Added `SurfaceLightGeometry`, compiling immutable triangle/vertex/edge incidence once and gathering current coefficients from a resident position stream on GPU. It supports the existing nine-f32 SceneVertex stride and other strides >=3. Triangle areas contribute area/3 to barycentric vertex mass; edge weights retain the existing positive `2*area/(3*length_squared)` equation rather than substituting a cotangent operator. Each directed edge gathers the same source-ordered contributions, retaining bitwise symmetry without floating-point atomics. Static topology, storage usage, buffer bounds and device limits are checked before GPU allocation. Geometry status explicitly signals nonfinite/degenerate triangles and sum overflow, and is reset per assembly.

`SurfaceLightPcg::from_geometry` shares the assembler-owned mass/edge buffers; its existing uploaded-operator constructor retains shared static validation. The frame owner must encode assembly before a fresh solve and admit status/residual before presentation. No coefficient readback or CPU solve occurs between assembly and PCG. This primitive does not itself publish frames or preserve the previously presented model on failure.

OSS barycentric mass reference inspected at libigl commit `7100764c2a2833284a8bdfa9b948d2b3bf124624`; source/MPL license/hash receipt in `contact-bake/libigl-mass-source`. This is a mathematical mechanism reference, with no C++ source copied into runtime.

An explicit GPU example reuses one operator and solver across six accepted physical stance surfaces (60,814 active vertices, 121,388 triangles, 364,408 directed edges). All pass coefficient parity (<1e-4 relative), bitwise symmetry, geometry status and unchanged light/residual gates. Maximum relative mass error 2.632698824266881e-7, edge error 6.270619840923496e-7; maximum irradiance error 8.196337422777811e-4. Invalid collapsed triangles and NaN positions report status 1; restored geometry reports 0. An independent exact right-triangle solve also covers an unreferenced vertex, exactly zero channel and zero-radius channel. Original PCG/Chebyshev analytic and full-body regression pass.

Measured GPU assembly 0.145542-0.1595 ms and shared-buffer solve 5.762667-5.80825 ms. The uploaded-operator PCG regression measured 4.597708 ms; the resident path overhead remains unexplained and should be profiled rather than assumed free. These are observed kernel times, excluding setup, uploads, tail readback/audit and all other frame work. Reproduce with `surface_light_geometry REPORT INPUT...`; runtime fixtures are exported by the explicit ignored physical stance cycle test. Receipt: `artifacts/articulated-jump-2026-10-11/contact-bake/physical-stance-resident-light-validation.json`.

GPU incident-light generation, runtime residual/publication admission, native material integration and whole-frame >160 FPS remain open. No visual/FPS improvement is claimed for the currently running CPU example.

## GPU numerical admission and retained illumination

Added `SurfaceLightAdmission`, independently recomputing the actual surface equation residual instead of accepting the CG recursive residual or an iteration count. It checks finite/nonnegative candidate and incident RGBA, geometry status, positive-mass domain contracts, squared relative equation residual and a max-error estimate under unchanged thresholds (1e-6 residual ratio, 1e-3 absolute error). Consumers must check has_accepted; a rejected first attempt does not create a valid publication. GPU-only conditional publication preserves the entire last accepted RGBA stream and generation on rejection. This is illumination retention, not whole-model rollback.

Explicit-residual stopping mechanism reference: [Ginkgo stopping criteria](https://ginkgo-project.github.io/ginkgo-generated-documentation/doc/develop/group__stop.html). An initial direct `max |r_i|/mass_i` estimate proved too conservative on the third physical pose: even a CPU error below 1e-6 could not pass. The original failing log and red-channel CPU diagnostics remain retained. No threshold was loosened. Four damped Jacobi steps (omega=2/3) now form a signed error correction z for r=A*x-b. In exact arithmetic the positive-weight M-matrix gives `||error||inf <= ||z||inf + max_i |r_i-(A*z)_i|/mass_i`; separate global maxima are reduced and added, rather than incorrectly combining per-row maxima. A floating-point roundoff allowance is included. This remains a model-qualified numerical estimate, not a formal interval certificate. The correction changes the admission estimate, not the candidate lighting.

`SurfaceLightPcg::reset_from_resident_source` supports an ordered GPU incident-light copy without CPU inspection; its dimensions/copy usage are checked before encoding and publication admission is mandatory for unvalidated values. The diagnostic uploads one immutable incident stream; assembly, solve, audit and conditional publish execute before a single tail readback. Scratch uses 64 bytes per vertex and eight storage bindings.

All six accepted runtime poses pass at 64 CG iterations; twelve 1/40-iteration attempts are rejected while preserving accepted light. Maximum accepted CPU irradiance error 4.647714170191719e-6; maximum GPU estimate 3.880141593981534e-4. Each accepted estimate explicitly covers the independent CPU error. Collapsed geometry, NaN geometry, NaN incident light, negative RGB and negative alpha all reject without changing accepted RGBA bits or generation; valid recovery advances generation 6 to 7 and exactly reproduces the prior deterministic result. The independent right-triangle/unreferenced-vertex analytic case, original analytic solver checks and resident geometry regression pass.

Admission GPU times in the final cycle run span 0.150375-0.444042 ms; these are current-host kernel observations, with no FPS inference. Receipt: `artifacts/articulated-jump-2026-10-11/contact-bake/physical-stance-gpu-light-publication-validation.json`. Reproduce via `surface_light_admission REPORT INPUT...`. Native material/frame integration, GPU incident generation, arbitrary-model/pose qualification, rigorous roundoff certification, full-model rollback and >160 FPS remain open. The currently running CPU example has not adopted this path.

### Paired incident and diffuse light publication

The GPU admission accepted buffer now stores n diffuse RGBA values followed by n incident RGBA values from the same accepted attempt. Conditional publication updates both streams together; rejected attempts preserve both bitwise. This prevents a future draw transfer from mixing stale diffuse light with current rejected incident light. The existing header and numerical criteria remain unchanged. The Metal audit passed all six physical poses, the analytic zero-radius/unreferenced-vertex fixture, corrupt-input rejection and recovery. Receipt: `artifacts/articulated-jump-2026-10-11/contact-bake/physical-stance-paired-light-validation.json`. This is not native renderer integration or an FPS qualification.

### Accepted light transfer into scene draw storage

`SurfaceLightTransfer` reuses the existing resident scene-stream scatter approach (`FiberSurfaceTransfer`). Immutable material bindings map compact light indices to unique draw vertices. A GPU dispatch consumes the paired accepted diffuse/incident snapshot and writes only SceneVertex color words 5..8 with the existing CPU material formula. Position, UV, separate normals and unbound vertices remain untouched. No CPU light readback or material shader ABI change is required. A missing accepted snapshot performs no writes; a rejected current attempt continues using the complete previous accepted pair. Constructor validation rejects duplicate/out-of-range mappings, invalid material values, incompatible storage and device identity/limits.

The focused actual-Metal regression uses separate diffuse and incident values, sparse nonidentity mapping, initial absence of an accepted frame, valid publication, then corrupted incident light. It checks CPU color parity, geometry/normal/UV preservation and bitwise retention after rejection. This draw-storage bridge is not yet wired into the native full-character frame loop. Full-model pixels, GPU timings, all-backend compatibility and >160 FPS remain unqualified.

### Real body surface material pixel qualification

The `surface_light_material` example now runs GPU operator assembly, 64-step solve, admission, resident color transfer and drawing for all six actual accepted physical-stance body-surface fixtures (60,814 active vertices, 121,388 triangles). It renders the unchanged `female_material.wgsl` with 4x MSAA and compares each image to independent f64 CPU-reference light shaded through the same material. Each view contains 83,774 to 86,991 occupied pixels. Maximum RGB channel error is 0 for the first pose and 1/255 for the other five. PNGs were visually inspected.

Initial neighbouring viewport comparison failed at 13/255. Rendering both references in identical viewport coordinates in successive passes removes viewport-dependent raster/interpolation footprint variation in the procedural material. No shader or tolerance was weakened; the failed log is retained. GPU output and CPU reference images are composed side by side only after readback. Receipt: `artifacts/articulated-jump-2026-10-11/contact-bake/physical-stance-body-material-pixel-validation.json`. This qualifies real body-surface material pixels, not the native full-character frame loop, full groom, dynamic tissues, all backends or >160 FPS.

### Resident normal lighting and native character frame integration

`SurfaceLightFrame` now owns one ordered path from the draw geometry's resident normals through directional incident lighting, current-pose operator assembly, a fresh GPU PCG solve, independent admission and accepted-pair transfer back into the same draw vertex colors. Immutable prefix topology/settings bind once; each call follows pose/normal updates. Zero, nonfinite or overflowing normal lengths emit invalid incident values and reject publication. Invalid iteration budgets reject before encoding work. Settings validate finite nonnegative light weights and normalized nonzero directions. This remains the existing artistic two-direction surface illumination equation, not volumetric scattering or shadow transport.

`VOXY_GPU_SURFACE_LIGHT=1` wires this path into `SceneApp` before native drawing, after any GPU deformation. Ordinary, shared and separate hair geometry setup requests storage usage when this mode is enabled. Diagnostic skin/strain views keep their CPU colors; changing the surface-light fraction away from the fixed configured value also uses the CPU reference. `VOXY_GPU_SURFACE_LIGHT_AUDIT=1` adds an optional asynchronous 32-byte publication-header tail audit. The ordinary path has no light readback or audit polling. The qualification mode deliberately retains CPU mesh construction and CPU reference illumination; it is not yet an accelerated replacement for the entire mesh/face pipeline.

Live native qualification on Apple M4 Max/Metal launched a separate `Voxy GPU Surface Light.app`, preserving the already open old preview. The full character, eyes, groom and skin material were visually observed. A stable profile snapshot contains 68 successful sampled GPU admissions through generation 125, and 120 warm present intervals averaging approximately 484 ms (2.066 FPS). Another CPU preview remained open and compilation also occurred on this host; this is a reported observation, not a controlled speed comparison. The new preview was left paused using Space. No resolution, MSAA or groom simplification was introduced by this change. Receipt/profile files are under `artifacts/articulated-jump-2026-10-11/contact-bake/native-gpu-surface-light-*`.

Focused Metal tests verify nonunit normal normalization, pose/normal changes, invalid zero-normal rejection, recovery and retained accepted colors after a new CPU mesh upload. Full-model CPU dependency removal, end-to-end pixel parity under dynamic face/groom updates, GPU stage profiling, all-backend qualification and >160 FPS remain open.

### Authored light-normal fidelity and CPU diffusion removal

A new actual full-character source-parity regression found that the first native resident-light integration used final geometric draw normals instead of the authored normals transported by the CPU reference. Its maximum incident-light discrepancy was 0.6157110184431076. Independent solver admission did not detect this because it verifies the supplied equation, not the upstream artistic normal contract. The failed log is retained as `full-model-gpu-incident-draw-normal-failure.log`; the preceding native admission receipt proves execution/numerical admission only and must not be read as CPU appearance parity.

`SurfaceLightFrame::new_with_normal_source` now accepts a separate resident lighting-normal buffer. The native transitional route uploads the already computed authored lighting normals and performs direct illumination, assembly, solve and publication on the GPU. This preserves the CPU reference without misidentifying final draw normals. The original default constructor still binds draw normals for callers whose lighting contract uses them. The full-model test on 60,866 body vertices reports maximum incident error 1.7881393432617188e-7 and accepted diffuse error 4.618137003919998e-6 against the existing CPU equation. Thresholds remain 3e-6 and 1e-3 respectively. GPU normal transport for this separate stream remains open.

Outer lip vertices inherit body colors through clipping and subdivision. `FaceFeatures` optionally emits exact source-triangle barycentric color bindings while executing the existing geometry algorithm; it does not duplicate the clipping implementation. The actual model emits 12,901 bindings with maximum CPU metadata reconstruction error 1.1920929e-7. Inner lip/oral constant material colors remain independent. `SurfaceColorTransfer` consumes body colors after accepted-light transfer, and updates only target RGBA words. Read-only source prefix and unique destinations outside that prefix exclude dispatch dependencies/write races. Its actual-Metal test covers changing RGBA inputs, unchanged positions/UV/normals/neighbours, invalid weights, prefix violations and duplicate targets. Ordinary frame construction does not allocate the optional polygon-weight metadata.

The native GPU mode now omits CPU diffusion during eligible frames. The one-time initial mesh retains CPU illumination as the reference; `VOXY_GPU_SURFACE_LIGHT_CPU_REFERENCE=1` retains CPU diffusion for qualification comparisons. Diagnostic skin/strain, film and non-default facial parameters use the existing CPU path. Async-worker/GPU-secondary combinations and experimental additive transmission are explicitly rejected until their lighting-normal/pose ownership and additive-color contracts are qualified. No supported rendering mode silently drops those contributions.

The actual full-character regression additionally builds the 2,535,255-vertex mesh in resident mode with empty CPU light caches and proves they remain empty. It verifies identical index topology, positions, UVs, complete authored normals and procedural material coordinates, plus all independent feature/hair colors outside the body/inherited-lip domain. The test passes, as do eight existing lip-filtered regressions. Logs: `full-model-resident-light-cpu-bypass-validation.log`, `inherited-surface-color-gpu-test.log`, `resident-light-lip-regressions.log`.

A separate `Voxy Resident Surface Light.app` was built/launched, preserving historical bundle binaries and receipts. Live visual inspection confirms the current full character with eyes/groom/materials; GPU headers publish accepted frames, and warm CPU diffusion stage traces are approximately 0.001–0.002 ms. This removes the CPU surface solve from these frames but leaves CPU deformation, normal transport, facial geometry, full mesh reconstruction/validation/upload and GPU solve costs. Native present profiling is observational under concurrent host load; >160 FPS, all hardware/CUDA and full production qualification remain open.

Final native snapshot: 351 warm present intervals, mean cadence equivalent to 3.8205695350986475 FPS, 197 sampled successful GPU publications. Warm CPU diffusion stage maximum is 0.024 ms; typical traces are 0.001–0.002 ms. The new preview was left paused. Final receipt: `artifacts/articulated-jump-2026-10-11/contact-bake/resident-surface-light-cpu-bypass-validation.json`. Both retained-light regressions passed after the authored-normal source change.

## Full-density resident groom, native integration (2026-10-11)

The synchronous native path now supports `VOXY_RESIDENT_HAIR=1` together with resident surface lighting. It reuses the existing `FiberSurfaceInput`, fibre extraction shader, and scatter into the exact drawing vertex/normal buffers. Only 469 × 21 solved guide frames are refreshed; all 60 followers and four radial vertices per follower remain. The main CPU mesh is built without hair. This is geometry extraction, not a replacement for Cosserat/contact simulation.

Source review: the retained AMD TressFX and TressFX-Godot shaders separate skinned guide integration, local/length constraints, follower expansion and rendering. Their algorithms are reference mechanisms; this change does not claim a port of their simulation. The ozz skinning sample (retained source/commit/license receipt in `contact-bake/ozz-skinning-source`) separates SamplingJob, LocalToModelJob and remapped inverse-bind palettes. That supports the existing accepted-pose boundary; it does not justify freezing the face or dropping tissue deformation.

The existing GPU groom frame producer omitted the head transform used by the CPU append path in rig-only/unsimulated mode. The adapter now applies the same head transform to positions and frame axes before admission. The actual groom regression compares 2,363,760 vertices at ticks 1 and 31: maximum position error 1.20137e-7 m, maximum normal error 2.5463115e-7; UV/colors exact, nonhair geometry unchanged. The first attempt failed because the readback pool was configured after ComputeProgram creation; its failed log is retained. Configuration was moved before program creation without relaxing any parity threshold. Passing log: `actual-posed-groom-gpu-parity-retry.log`.

`Voxy Resident Groom.app` was observed rendering the complete character and groom, then left paused. The frozen native profile contains 1,159 uninterrupted active presentation intervals, 7.9414 FPS, median 123.760 ms. All 1,417 observed surface-light audits accepted. Last-300 CPU stage medians: deformation 31.2975 ms, authored normals 8.099 ms, face features 40.515 ms, hair/shapes 0.858 ms, validation 1.868 ms. Host jobs were concurrent; this is not a controlled speed ratio. CPU-normal upload, CPU body/face construction and their uploads remain. The richer existing dedicated fibre shader is used; legacy combined-mesh hair shading is not pixel-identical by design.

Native evidence/limits and hashes: `resident-groom-native-validation.json`, `native-resident-groom-profile.json`, `native-resident-groom-profile-snapshot.log`. Full jump, realistic tissue dynamics, 720-frame hair/contact qualification, all hardware/CUDA and >160 FPS remain open. The next performance work must address the measured body/face costs while retaining dynamic facial geometry, materials and secondary motion.

### Native groom pixel gate remains open; final pose moved to GPU

An actual full-groom render test now runs compute extraction, scatter and the existing dedicated fibre material in one ordered submission, comparing against CPU vertices with the same camera and 4x MSAA. Initial GPU hair vertices are deliberately poisoned, and the second reference pose must change the image. This is separate from matching the legacy combined-mesh hair material.

The first image gate failed at tick 1: 59,971 occupied pixels, 26 differing channels, maximum 5/255 against the unchanged <=1/255 requirement. Applying the head transform before follower/radial expansion reordered the CPU arithmetic. `FiberSurfaceInput` now retains an affine pose block; the shader expands local fibres first and applies the pose to final positions and normals. Synchronous and background frame owners upload the matching pose with solved guide frames. Transform updates reject nonfinite, non-affine and singular matrices before changing CPU input bytes. This also avoids submitting nonorthogonal transformed guide bases to frame admission.

Geometry comparison after this change passed both actual poses (2,363,760 vertices each): position error <=1.20137e-7 m, normal error <=1.6924466e-7, UV/colors unchanged. However, the image gate still failed: 24 differing channels, maximum 5/255 at tick 1. The two failed image logs are retained (`actual-groom-material-pixel-validation.log`, `actual-groom-material-pixel-pose-order-validation.log`). No pixel-equivalence claim is made and the threshold is not relaxed. The image test now records both poses before applying that same final gate and prints coordinates/colors of significant differences for diagnosis.

A full-density pre-existing rotated-frame regression caught unwanted normalization of untouched guide normals. The shader now preserves the authored normal when the pose is identity; transformed normals follow the CPU normalization path. Its passing retry is `resident-groom-existing-fibre-regression-retry.log`. Native bundle/profile above belong to the preceding head-frame implementation; the new pose-block implementation has not yet been qualified in the native window. GPU extraction, GPU simulation, and a complete physical jump remain distinct requirements.

Final pose-block revision evidence: actual geometry passed again on both poses (`actual-groom-final-pose-geometry.log`), transform/input admission passed, full-density existing frame regression passed, and all three background-worker publication regressions passed. The final two-pose image diagnostic still rejects: tick 1 has 24 differing channels / max 5/255; tick 31 has 46 differing channels / max 4/255. Significant pixel coordinates are recorded in `actual-groom-two-pose-pixel-diagnostics.log`. Receipt `resident-groom-final-pose-validation.json` records current source/log hashes and marks the strict image gate FAILED. No new native FPS or full-physics claim is attached to this revision.

## Facial collider refit and full native pose-block run (2026-10-11)

`FaceFeatures` now owns a derived `RefCell<Option<TriangleMesh>>` for the lash skin collider. Each append refits current posed points through the existing physics API rather than sorting/building the BVH again. Untimed refit clears motion history; failed refit replaces the cache from current points and never queries stale geometry. Disabled-contact diagnostics do not use the retained cache. This spatial cache is not accepted simulation state, a new collision system or a second physics owner. Brow/lash/fuzz control names are resolved once per append; density, length, thickness, contact correction and all tessellation remain unchanged. Optional existing mesh tracing now separates contact preparation, strands, rims, lips and oral anatomy.

The actual body-asset test compares the complete generated feature mesh against a fresh collider on seven poses, including closed/intermediate/open blinks, jaw/smile/brow changes, head rotation/translation, lash acceleration and changed length/density/thickness. Every sample retains 92,892 vertices and 393,558 indices, exactly matching both vertex attributes and topology. Cached append totals 96.485 ms versus 110.293 ms for forced fresh builds in that one observational test; both paths use hoisted controls. Contact preparation is roughly 0.13 ms refit versus 3.4 ms rebuild in its later samples. Refitted trees can cost more during query traversal, so only whole-append timing is relevant. Existing feature regressions: 24 passed, 1 ignored. Passing logs: `face-contact-refit-parity-retry.log`, `face-contact-refit-feature-regressions.log`.

The shared target directory disappeared during the first native build, producing confirmed terminal ENOENT output failures. A new build succeeded with `CARGO_TARGET_DIR=/tmp/voxy-face-refit-20261011`; no other jobs or checkouts were removed/reset. `Voxy Face Contact Refit.app` contains that binary and combines the final GPU hair pose block, full-density groom, resident surface light and facial collider refit. It presented through frame 237 with 234/234 successful observed light audits. After warmup, 218 intervals gave 12.42156 FPS, median 77.84975 ms. Median CPU costs: deformation 20.189 ms; authored normals 5.696 ms; face features 19.7075 ms (contact 0.201, strands 10.204, lips 6.6265, oral anatomy 2.589); hair/shapes 0.613 ms. Concurrent host work and a different run prevent attributing a speed ratio against earlier previews.

The AX window was observed playing, but the app exited before screenshot inspection. No application error or matching diagnostic crash report was found; the exit cause is unestablished. Final model appearance was not visually qualified. Stable profile, receipt and hashes: `native-face-contact-refit-profile.json`, `native-face-contact-refit-profile-snapshot.log`, `face-contact-refit-validation.json`. The strict hair image gate remains failed; >160 FPS, full jump and dynamic contact qualification remain open.

### Next ready upstream mechanism: explicit fibre tangents

The retained TressFX-Godot source at `ad3d1faad2338a386e4baf4e0b00a560aab4dd0d` provides solved guide tangents to strand vertices (`upstream/src/Shaders/TressFXStrands.hlsl`, GetExpandedTressFXVert) and normalizes that explicit tangent for lighting (`TressFXLighting.hlsl`, AccumulateHairLight). Current Voxy fibre material instead derives tangent from screen derivatives of position and arc. Explicit solved-strand tangents are a source-backed candidate for stable directional lighting; no port is claimed yet. A diagnostic explicit-matrix multiplication variant did not fix the strict CPU/GPU image gate (5/255 and 4/255 still observed) and was kept only as an artifact, not adopted as the runtime shader.

Current legacy `gpu_secondary_binding` also lacks the actual shell embedding displacement path: its controls cover support and region offsets. Reusing it wholesale would retain a different end state. Body transfer work must include accepted shell-minus-posed-reference displacement and live face features rather than freeze their initial geometry/materials.


### Explicit solved fibre director stream, 2026-10-11

Adapted the explicit tangent data flow observed in retained TressFX-Godot GetExpandedTressFXVert (commit ad3d1faad2338a386e4baf4e0b00a560aab4dd0d). Voxy uses the solved Cosserat frame w director; this is not a claim of identical discrete guide tangents or a TressFX simulation port. SceneGeometry now owns an optional independent xyz/handedness stream (vertex slot 4, location 6), included in device memory accounting and shared by LOD index variants. Ordinary geometry has no extra allocation. The resident groom scatter writes transformed normalized directors on GPU; CPU reference and initialization supply the same independently calculated frame directions. Existing non-resident material paths retain their previous shader. Initial hidden hair still allocates the complete resident surface so visibility can be enabled safely.

Qualification: two GPU scatter tests passed, including two nonuniform affine poses and unchanged neighbour tangents; five ordinary LOD tests, one explicit GPU LOD test, and one partition regression passed. Actual 2,363,760-vertex groom rendering on Metal/MSAA4 ran on two poses with poisoned initial GPU positions and tangents. The unchanged maximum 1/255 image gate FAILED: tick 1 maximum 8/255, tick 31 maximum 3/255, eight differing channels each. Explicit pose multiplication diagnostic reproduced the same errors and was not adopted. Cause remains unestablished. No native visual/FPS result for this revision. Receipt and complete logs: artifacts/articulated-jump-2026-10-11/contact-bake/explicit-director-validation.json.


### Groom image mismatch isolation, 2026-10-11

The previous turn was progress: optional resident director transfer and focused GPU regressions. This turn adds a test-only explicit material override, preserving the default production material and max1/255 gate. Four independent fragment outputs isolate constant-white coverage, base colour, normals and director; all share unchanged full geometry and camera. A fifth variant marks vertex position invariant. The same actual two poses and complete density remain in use.

Results (maximum RGB8 error tick1/tick31): coverage63/0, base colour4/1, normals60/46, director42/3, invariant production material8/3. White pixel(376,493) is CPU[128,128,128] versusGPU[191,191,191], consistent with one MSAA4 sample changing coverage. Thus at least this mismatch exists before lighting; changing the hair highlights alone cannot fix it. Other normal/director differences cannot yet be attributed uniquely to buffer arithmetic versus visible surface/interpolation. Invariant output did not fix the gate and was not adopted. Independently rerun actual full groom geometry comparison passed at both poses, max position1.20137e-7m and normal1.6924466e-7. Small metric errors do not establish per-pixel image parity.

Next investigation: geometry arithmetic contraction/rounding and per-sample visibility. Apple MTLCompileOptions documentation identifies configurable floating-point math behaviour; inspected vendored Metal WGSL path sets preserve invariance but does not explicitly set math mode. No backend-wide setting is changed on this evidence. Production image qualification, visual native qualification and target FPS remain open. Receipt: artifacts/articulated-jump-2026-10-11/contact-bake/groom-isolation-validation.json.


### Metal numerical compute policy and passing full-groom image gate, 2026-10-11

A/B mechanism isolation used the actual full-density groom and the unchanged max1/255 image gate: default Metal math failed8/3; safe math on all stages passed1/1; compute-only safe math passed1/1; render-only safe math failed8/3. This localizes the effective fix to compute compilation. It does not identify every contracted/reassociated operation. No geometry simplification, per-pixel tolerance increase or camera change was used.

Adopted explicit vendored wgpu-hal feature `metal-safe-compute`, selected by voxy_render only for Apple targets. The WGSL/Naga Metal compute path uses `MTLCompileOptions.setFastMathEnabled(false)` for compatibility with older deployment targets. Render stages retain existing settings; non-Apple backends are unchanged. Temporary diagnostic environment control was removed from runtime source. The feature is recorded in both vendored Cargo manifests. This is a platform compute compilation policy, not an application-level second simulation owner.

Final binary without diagnostic overrides passed the actual2,363,760-vertex two-pose MSAA4 image gate: max1/255 at both ticks1/31. Geometry gate passed: max position5.9606464e-8m, normal1.6924466e-7. Two fibre-scatter and two resident-light transfer/admission regressions passed. Only these tested poses and pipelines are qualified; this is not full720-frame or all-hardware qualification.

Actual groom extraction GPU pass timestamps, with3 warmup batches and9 measured batches of16 dispatches, showed medians0.3403046875ms and0.396737ms for ticks1/31. The first profiler attempt rejected nonpositive/unwritten intervals. Resolving counters after completion of the sampled submission, following the existing Metal QR profiler, produced valid positive samples. These numbers exclude scatter, rendering, CPU simulation/deformation, upload and readback. Safe-versus-fast overhead and native FPS are unmeasured; concurrent hardware load was not established as exclusive. Full logs, initial failed timestamp attempt and source/binary hashes: artifacts/articulated-jump-2026-10-11/contact-bake/metal-compute-policy-validation.json.


### Native full-frame scheduling, 2026-10-11

Built and launched a new native Metal-compute-policy bundle. Computer use reported Mac locked and manual unlock required; presentation-cadence samples were absent, so no visible FPS or screenshot result is claimed. The CPU-stage trace is retained as observational data only. The agent stopped its own diagnostic process with SIGTERM after verifying executable identity; this was an intentional stop, not a crash.

Source inspection found an unconditional16ms redraw delay after every synchronous full female-model frame, independently of measured work. Removed that successful-frame throttle; retained16ms retry only for SkippedOccluded/SkippedTimeout. Existing winit event dispatch and frame-loop scheduling remain responsible for input and simulation. This removes an artificial obstacle to >160FPS without reducing geometry or physics. Native example rebuild passed. Saved the updated independent bundle `artifacts/articulated-jump-2026-10-11/Voxy Uncapped Full Groom.app`; it has not been visually qualified or benchmarked with an unlocked display. Receipt: contact-bake/native-uncapped-validation.json. Target FPS remains open.


### Exact expression support rejection, 2026-10-11

Inspection of the full CPU deformation path found facial-expression evaluation across all body vertices even when the final front mask was exactly zero. The new branch rejects only this exact zero support after the separate eye rotation path; forehead tension remains a later pass. No approximate influence cutoff, density reduction, or physical solver change is used. A test-only unculled call to the retained expression algorithm establishes independent before/after results.

Production prepared body (body-forehead-refined.obj)60866 vertices plus eyes1092: all61958 positions, UVs and RGBA matched bitwise on7 poses with/without lid contour,14 cases.24442 body vertices avoid inactive expression calculations. A single observational run took50.773418ms unculled versus32.290956ms culled summed across14 expression evaluations. This is not a native FPS result or controlled general speedup. The original unrefined source body also passed, but its42342 vertices are not substituted for the prepared runtime mesh qualification. Eleven existing facial regressions passed; four ignored diagnostics were not run. An initial test compile failure from an unavailable bytemuck dependency was fixed with direct f32::to_bits comparisons and retained in the logs.

Added optional FACE DEFORMATION stage tracing under VOXY_FACE_MESH_TRACE to distinguish clone/wrinkles,complexion,expression,rig,support in future full-frame profiles. No new timings from these substages are claimed yet. Retained ozz skinning source confirms remapped inverse-bind palette flow; existing GPU secondary path cannot be adopted as full parity because it lacks the finger-local layer/grasp correction, full face/morph inputs and shell-minus-reference embedding. Those remain integration requirements; the existing dual-quaternion volume behaviour must be preserved.

Native example build passed; saved Voxy Face Exact Support.app. CUA recheck still reported locked Mac, so the new bundle was not launched for visual/FPS claims. Receipt and logs: contact-bake/face-exact-support-validation.json. Full rig GPU transfer, unlocked native qualification and >160FPS remain open.


### Physical shell sparse GPU transfer component, 2026-10-11

Added readonly SkinEmbedding shell topology access, keeping physics independent of rendering. The app adapter compiles the existing validated barycentric bindings into CSR rows for SurfaceDeformation: three influences per body vertex, empty rows for trailing eyes. Dynamic controls are636 shell-minus-posed-reference displacement vectors. Subtraction occurs in the physical solver's f64 precision before conversion to GPU f32; sending absolute f32 shell/reference poses would risk cancellation. This component needs10176 dynamic bytes per pose, not a dense body displacement upload. Physics state remains owned by Skin; no additional solver or state owner is introduced.

The actual runtime mesh (60866 body vertices,78278 total including eyes) passed a GPU/CPU comparison on three shell states, each encoded twice without resetting the source. Maximum position error5.9604645e-8m under unchanged3e-7m geometry tolerance. UV/RGBA and trailing eyes were unchanged; zero control reset restored exact base positions, and rig_pose_only generated zero controls. The CPU reference uses existing SkinEmbedding deformation after the same independently posed base, so skeletal motion is not added twice. Two existing physics embedding regressions passed.

Fixture scope: the first random per-node deformation around skeletal targets was rejected by Skin.set_state (skin energy overflow). No guard was bypassed. The retained successful fixture applies smooth2/3mm spatial fields to an already accepted physical shell; its third state aligns the reference with the accepted shell for exact zero-reset validation. These cases qualify the transfer mechanism and actual topology, not a physical full-jump trajectory or coupled material behaviour.

This component is not activated in the native full-model frame path. Activation must wait for faithful resident rig/face/morph inputs, finger-local/grasp preservation, correctly ordered authored normal transport and current-pose facial attachment consumers. No FPS or native visual claim for this component. Receipt: contact-bake/full-body-gpu-shell-validation.json.


### Composed GPU skeletal pose and shell displacement; tied-weight anchor fix, 2026-10-11

Extracted canonical pre_skin_surface_vertices from the existing CPU flow, preserving wrinkle, complexion and expression ordering. Both CPU pose application and GPU preparation use this input rather than defining another face pipeline. The actual78278-vertex mesh passed GPU dual-quaternion skinning followed by sparse physical shell displacement against the full CPU posed/finished reference on default physical-stance ticks1/31: max2.395799e-7m and2.4038948e-7m under unchanged3e-7m tolerance, exact UV/RGBA. Per-pose immutable GPU preparation is used in this test; persistent input publication is not yet integrated.

Found and reproduced a real CPU/GPU blend discrepancy: CPU Iterator::max_by chooses the last equally weighted maximum, whereas WGSL chose the first. A three-joint0.4/0.4/0.2 mixture with0/200/100degree rotations differed by0.117975675m on a0.1m test point. Updated the shader comparator to >= with an explanatory comment; the same counterexample now passes3e-7m. This fixes CPU/GPU hemisphere-anchor parity, not arbitrary animation continuity.

After the shader fix, composed rig/shell position comparison still passed. Sparse shell nonaccumulation/zero-reset regression passed; actual full groom image comparison stayed within1/255 at both poses. Authored normals, active finger-local poses/grasp corrections, nondefault morphs, persistent accepted-pose publication and native body activation remain open. No new native FPS or screenshot claim. Receipt: contact-bake/full-rig-shell-composition-validation.json.
