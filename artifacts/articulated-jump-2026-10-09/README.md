# Articulated jump — 2026-10-09

`jump-preview.gif` contains 120 native rendered frames over six seconds. It shows the real model's rig, crouch, takeoff, airborne knee flexion, arm swing, landing and recovery. This capture uses `female_render --jump-sequence`; it deliberately does not advance secondary physics. It is not evidence of simulated hair motion or real-time FPS.

The shared analytic trajectory has continuous position, velocity and acceleration at phase boundaries and ballistic gravity during flight. Two-bone leg IK holds grounded feet flat and stationary; dual-quaternion skinning preserves rigid bone lengths. The GPU stage uploads a bone palette, then skins the resident mesh before physical displacement and normal transport.

Validation:

- Actual sole vertices checked: 8,280; maximum knee displacement: 0.18173385 m. Grounded foot and bone-length tolerances remain 2 micrometres.
- Actual Apple M4 Max Metal readback, six poses, complete body and trailing rigid hair: maximum CPU/GPU position difference 0.197 micrometres. Invalid scaled palettes are rejected without replacing the previous valid pose.
- Full secondary skin/volume cycle: 696 steps at 120 Hz, two jumps and settling, passed. Hair is excluded from this particular regression. Rendered secondary-region peaks are 39.7–44.0 mm; the separate support peaks are 27.1–27.5 mm.
- Skin suite: 20 passed, including adaptive-step failure atomicity and attachment trajectory preservation.
- Skin retries bisect the original attachment trajectory without changing force convergence tolerances. Exhausted retries discard the entire staged interval.

Remaining qualification: complete articulated-jump hair contact solve, rendered secondary-physics capture, and measured real-time performance above 160 FPS with the complete physical model.

## Hair contact precision follow-up

The articulated-jump native paired run stopped at frame 40: guide 9, segment 14 had -6.0928% strain, exceeding the unchanged 5% gate. Enabling joint position/velocity, swept strand, sampled collider motion and recovered friction together stopped at frame 1 with nonlinear position nonconvergence; this is not a completed 720-frame qualification.

The generic square-root response solver now uses compensated products consistently for original physical gap admission and whitening defect correction. Twice-orthogonalized QR also uses FMA updates, compensated triangular accumulation and compensated reconstruction. No tolerances or contact gates changed. Saved 94-row and 100-row original-load replays now pass, including force-balance admission. The independent `audit_physical_contacts.py` uses Decimal at 100 digits against exported immutable original loads and returned physical responses: all 100 rows pass the original 1e-14 tolerance, maximum active residual 1.8492184589471823e-15, minimum inactive gap 1.782631205918748e-7 m. This fixture proof does not imply complete nonlinear hair stability or GPU contact support.

Hair unit tests: 122 passed, 7 ignored. Binary audit payload: `physical-contact-audit.bin`; machine-readable result: `physical-contact-audit.json`.

## Axial strain localization

The unchanged full-contact native articulated jump repeats failure at frame 40, guide 9, segment 14. The second substep's structural iteration 3 has signed strain -9.631134495480254e-7; the immediately following mesh-contact phase has -0.06117017998375229. Structural iteration 5 then returns to -2.2143554876397076e-6, but final mesh projection produces -0.060928133801394724, retained through finish/friction. The result identifies sequential mesh position projection as the source of excessive compression, rather than the free elastic solve or final friction impulse. The defect remains unresolved.

Opt-in phase diagnostics now include per-segment signed axial strains with no changes to simulation state. Both existing trace-preservation tests pass. `analyze_strain_trace.py` reproduces `strain-phase-analysis.json` from the saved trace (decompress `strain-phase-trace.log.gz` first).

## Interior segment contact recovery

Closed-body collision discovery now retains triangle crossing parameters and examines interval midpoints between crossings. A midpoint with negative signed distance supplies an additional contact with interior penetration depth, rather than a boundary-only fiber-radius correction. This adds constraints; it does not certify clearance between samples, replace swept admission, drop strands, or relax convergence/strain gates. Prescribed follicle permission remains clipped to the first segment's first 15%. Open meshes retain their previous path.

A tetrahedron regression verifies recovery of penetration greater than 0.1 m with both segment endpoints outside, and verifies that constraint discovery leaves positions unchanged. Hair unit tests: 123 passed, 7 ignored; integration tests: 20 passed.

Whole-model result remains FAIL: the previous joint unswept run stopped at frame 1 with nonlinear contact nonconvergence. With interior witnesses, that geometry solve completes but frame 1 violates the unchanged 5% axial strain gate: guide 45, segment 15, +6.0439%. Increasing structural iterations to 32 makes this +8.2361%, so that increase is not adopted as a fix. The ordinary projection mode also now fails at frame 1 (+6.0439%); the previous frame-40 result is historical, not current qualification. Authored-groom recovery and coupled elastic/contact convergence remain necessary. No complete jump, real GPU physical solve, or >160 FPS claim follows from these tests.

## Admission before publication

`HairSystem::step_validated` reuses the existing single staged physics solve and applies caller admission before replacing state. Rejection preserves the complete system, including velocities, contact history, diagnostics and subsequent evolution. Existing `step`/`step_with_solver` retain their original generic acceptance through this shared owner. External accelerator effects cannot be rolled back.

`FemaleHair` stages body-collider refitting and root targets separately, checks the existing 5% strain/finite/root gates inside the physics transaction, then publishes all three states together. A collider clone is required for atomic refitting; this change is not a performance result and does not establish complete `FemaleDemo` transaction ownership. It preserves the prior state on failure; validity of the authored initial groom still requires separate qualification.

The rejected-candidate regression checks complete state equality and equivalence of the next accepted step. A real-model injected accelerator failure checks unchanged guide state, root targets and collider surface queries. Hair tests: 124 passed, 7 ignored; integrations: 20 passed; real-model failure atomicity: 1 passed. The 720-frame full-model command still fails on native frame 1, now before excessive strain is published.

## Initial groom validity audit

The rest-curve export now includes initialized positions, per-guide maximum axial strains and finite-triangle segment intersections with the real collider. Root permission is handled by clipping the first segment to its free 15–100% interval before querying. This audit supplies intersection counterexamples; absence of hits does not certify capsule clearance or swept motion.

The prepared model has 469 guides. Its authored rest curves contain 51 intersecting segments. After the constructor's two preparation steps there are no detected centreline intersections, but guide 18 already has 44.8759572169116% relative axial strain; one guide exceeds the existing 5% gate. Guide 45 starts at 2.6556860933214965% before its later frame-1 failure. Thus preserving the previous initialized state does not imply preserving a valid groom.

The existing `resting_groom_contacts_preserve_lengths` regression on the original body asset also FAILS, even in its isolated no-body/no-self-contact diagnostic case: 7.489484659959023% maximum strain exceeds the unchanged 1% rest gate. That case is diagnosis only; runtime contacts remain enabled. Full original/prepared-model validity is unproven. The next required correction is geometric authoring of admissible rest curves before physical material/rest state creation, without baking preparation strain into new rest lengths or loosening gates. See `groom-initialization.json`, `groom-initialization-summary.json`, and `groom-rest-regression.log`.

## Native geometric groom fitting

`TriangleMesh::fit_authored_guide` now performs bounded geometric fitting before `HairRod::new` establishes material/rest state. It retains the root and number of points, uses closed-surface signed distance, finite-triangle capsule proximity and between-crossing interior witnesses, and returns an error on invalid data or exhausted fitting rather than a partial curve. It changes authored shape and arc length; it never rebases a running rod or bakes simulated strain into rest lengths. First-segment follicle permission remains 0–15%. Strand/strand preparation is separate, and this method is not a swept-motion certificate.

Prepared-model result: all 469 guides and 21 nodes per guide retained. Authored body-intersecting segments fall from 51 to 0; initialized centreline intersections remain 0. Maximum initial axial strain falls from 44.875957% to 2.867114%; no initialized guide exceeds the unchanged 5% gate. Authored polyline length ratios relative to previous curves range from 0.999507 to 1.146016, reflecting routing around the body. `fitted_prepared_groom_has_valid_initial_state` passes the real prepared-model root/finite/strain admission and checks every authored free segment for finite-triangle intersections. Generic hair tests: 125 passed, 7 ignored; integrations: 20 passed; all app examples compile.

The original-body resting regression is improved but still FAILS: isolated free rods now have approximately 1e-6 strain; body-only case reaches 1.090700%, above the unchanged 1% gate. Full prepared-model ordinary projection passes frames 1–9, rejects native frame 10. Joint position/velocity mode passes 1–46, rejects native frame 47 for strain. Joint mode with sampled collider motion passes 1–44, rejects native frame 45 for nonlinear self-contact nonconvergence (guides 381/402, remaining gap about -5.69 micrometres). Thus no complete 720-frame jump or GPU dynamics/FPS qualification is implied. `groom-fitting-result.json` and fitting/model logs record these scopes.

## Nonlinear half-step admission

Opt-in `VOXY_HAIR_FAILURE_TRACE_EXPORT` now exports rejected staged phase/projection/friction observations before the transaction discards them. Export failure cannot change solve admission. The saved frame-45 trace demonstrates a two-cycle in the selected component: worst contact alternates between larger/smaller penetration while decreasing only every two iterations. The last 32-iteration stage starts at -40.12 micrometres and ends at -5.69 micrometres; its observed negative-gap norm alternates as well. `analyze_rejected_projection.py` extracts these selected-component observations; they are not the full-system merit or a motion certificate.

The nonlinear contact owner now evaluates a whole-system penetration merit after refreshing actual mesh and strand geometry. If a full proposal increases that merit, it tests a half-position/half-rotation proposal. It retains the half only if its merit is below both the previous state and the full proposal. Otherwise it restores the full proposal and continues existing bounded reconciliation; there is no claim of unconditional descent. Half proposals scale paired positional reactions consistently and require another reconciliation iteration. Prescribed roots stay fixed, and final fresh-geometry, strain and force-balance gates are unchanged. This adds work and is not a measured performance improvement.

Validation: 125 hair unit tests and 20 integration tests pass. Native paired full-density articulated jump with joint position/velocity and sampled collider motion now passes frames 1–60; the previous frame-45 nonlinear contact failure no longer occurs. Native frame 61 rejects axial/root/finite admission (combined existing error), so the complete 720-frame jump is still FAIL. This native-only mode proves neither actual GPU contact dynamics nor rendered >160 FPS. See `contact-damping-model.log`, unit/integration logs and rejected frame-45 trace.

## Specific admission and remaining nonlinear correction

Application admission now distinguishes nonfinite coordinates, displaced scalp roots and excessive axial strain, preserving the existing thresholds and transaction. Opt-in `VOXY_HAIR_REJECTED_GUIDE_EXPORT` captures the rejected guide's geometry, signed segment strains, contacts and, for actual simulation steps, the reassembled original linear system at the correct substep dt and its native correction. These diagnostics never apply the correction or relax admission.

The six-iteration sampled/joint run repeats failure at native frame 61: guide 18, segment 4, +5.127441579289194% strain against 5%. Roots and coordinates pass; this guide's final contact list is empty. Its remaining native correction at dt=1/240 s reaches 3.7614996820855 mm and 0.31150130620715 rad. `audit_rejected_guide_linear.py` independently evaluates original matrix products with Decimal at 100 digits: maximum scaled residual 1.378117083084468e-16, within the unchanged 1e-8 force-balance admission. Linear solve admission does not establish nonlinear equilibrium; the remaining correction is substantial.

Increasing structural iterations to 12 is NOT adopted: the trajectory instead fails at native frame 32 on nonlinear self-contact nonconvergence, guides 12/1, final penetration about -8.89 micrometres. Thus a global fixed-count increase is not a verified solution.

The stronger fitted-groom native mode (joint position/velocity, sampled collider motion, swept strand positions, recovered friction pressure) also completed with FAIL: frames 1–17 pass, native frame 18 reports `joint square-root active contacts did not converge`, after 309.88 s. Repeated component motion underflow preceded the failure. This is completed native-only evidence, not a pending process, GPU physics result or FPS measurement. All original convergence/strain/contact gates remain unchanged. Next required work is converged nonlinear elastic/contact equilibrium together with continuous contact admission, rather than accepting the present large residual or weakening its guards.

## Actual implicit-energy step acceptance

Native and external structural corrections now share bounded Armijo backtracking against the actual implicit rod energy (inertia, stretch/shear, bending/twist and unilateral mesh penalties), rather than accepting only the quadratic linear model. The energy uses the same world-frame residual arithmetic as force assembly. A stiffness- and mass-scaled floating-point evaluation bound prevents rejecting resting configurations whose energy is near machine roundoff; original strain, contact and force-balance gates are unchanged. Backtracking is bounded to 24 attempts, preserves fixed roots and rejects an unresolved proposal. This acceptance check does not establish nonlinear convergence or certify subsequent contact projections.

Initial versions incorrectly rejected rest/contact fixtures and prepared-groom initialization; those versions are not the final validation. Current checks: 126 hair unit tests pass (7 ignored), all 20 hair integrations pass, and diff whitespace checks pass. The full-density native paired articulated-jump run with joint position/velocity and sampled collider motion passes frames 1–63, then FAILS at frame 64 on joint contact position linearization nonconvergence. Its diagnostic selects guide 432, segment 9, mesh contact with approximately -1.929 mm gap. The previous frame-61 strain rejection is avoided in this selected mode, but the complete 720-frame jump remains unqualified. This is native-only evidence, neither a GPU dynamics qualification nor a >160 FPS measurement. See energy-descent-{units,integrations,model}.log.

## Bounded nonlinear geometry backtracking

The single half-step trial is replaced with up to 12 decreasing common position/rotation scales. Every trial refreshes actual mesh and strand geometry; a trial is retained only when its penetration merit is below both the preceding state and the full proposal. Paired positional reaction history is scaled by the accepted common fraction. Unaccepted trials restore the full proposal and its refreshed geometry, preserving the existing bounded reconciliation behavior; this is not unconditional descent. No density or admission threshold changes.

Validation: 126 hair unit tests pass (7 ignored), 20 integration tests pass, diff whitespace checks pass. The full-density native paired articulated-jump run again passes frames 1–63 and FAILS at frame 64. Its selected mesh witness now has -1.401 mm gap (guide 395, segment 8); it differs from the prior selected witness, so this is not proof of a system-wide accuracy improvement. Backtracking alone does not resolve the contact equilibrium. The next investigation must distinguish lack of a descent direction from exhausted nonlinear iteration count using per-iteration actual geometry diagnostics. GPU dynamics and >160 rendered FPS remain unqualified. Logs: contact-backtrack-{units,integrations,model}.log.

## Frame-64 actual-geometry merit diagnosis

Opt-in VOXY_HAIR_NONLINEAR_MERIT_TRACE records each nonlinear iteration's before/full/accepted penetration merit and common accepted scale without changing acceptance. A completed repeat again FAILS at native frame 64. The last 32-iteration loop reduces merit from 0.003641188068620127 to 9.570393279471111e-6 square metres, but accepted merit increases at iterations 7, 16, 21 and 24 when all tested fractions fail to improve and the full proposal is restored. Tangent completion is false for 31 of 32 iterations. This shows the current fallback does not guarantee descent; it does not prove that raising the iteration cap resolves the failure. Next work must address contact direction/feature transitions and fallback rather than assume monotone convergence. Data: contact-merit-model.log and contact-merit-frame64.json.

## Reject known worsening contact directions

When all 12 smaller refreshed-geometry trials fail to improve penetration merit, the nonlinear owner now restores its pre-increment positions/rotations, refreshes mesh constraints and returns an explicit no-descent error before recording proposed reactions. The enclosing step transaction still preserves public state and history on failure. It no longer knowingly accepts the worse full proposal. This correction establishes rejection behavior, not the existence of a feasible direction or a completed jump.

126 hair unit tests pass (7 ignored); 20 integrations, including complete step rollback and future-dynamics regressions, pass. Diff whitespace checks pass. The full-density native paired run passes frames 1–63, then FAILS at frame 64 with the explicit no-descending-step error, instead of continuing four known-worsening increments. This failure must be resolved by a feasible contact direction/temporal continuation; early refusal is not the requested final physical result. Logs: contact-descent-{units,integrations,model}.log.

## Temporal refinement qualification in progress

A qualification-only VOXY_HAIR_QUALIFICATION_SUBSTEPS setting accepts 2..32 and applies identically to the paired systems after groom initialization. Runtime defaults remain unchanged. The four-substep full-density articulated-jump run uses the existing sampled mesh trajectory and interpolated root poses, preserving physical/contact admission. At this observation it is still running (exec session 29957, log /tmp/voxy-hair-temporal4-model.log); no terminal pass/failure or GPU/FPS claim follows. Diff whitespace checks pass. Re-poll this handle rather than restart on observation timeout.

Source inspection also identifies one global angular trust fraction in velocity_contacts::reconcile_contact_positions_with_solver: the largest rotation across all rods caps every rod's positional correction. This preserves paired reactions but can couple disconnected contact components unnecessarily. Its effect is not yet measured or corrected; component-local trust scales must preserve every connected response and be tested before adoption.

## Completed four-substep diagnosis and component-local trust

The preceding four-substep run completed with FAIL at native frame 42 after 244.45 s: joint nonlinear position linearization exhausted its cap; the selected strand witness (guides 347/376) has -1.2655690864795528e-10 m gap, beyond the unchanged -1e-10 admission. Smaller temporal steps alone are not a qualified fix and runtime defaults remain unchanged. See temporal4-model.log.

Positional contact reconciliation now reuses safe_motion::trust_components: each connected strand-contact component receives its minimum angular trust fraction; disconnected rods are not reduced by another component's largest rotation. Position/orientation increments and paired multipliers use the same component fraction. Mesh constraints belong to their rod, while all response pairs conservatively connect their rods. Existing fresh nonlinear geometry gates still detect any newly introduced pair. 126 unit tests (7 ignored) and 20 integrations pass, including angular component locality and native/worker/accelerator equivalence scopes. The two-substep full-model rerun is still pending (exec session 26280, /tmp/voxy-component-trust-model.log); this is not completed jump, GPU or FPS qualification.

The component-local two-substep run has now completed: frames 1–63 pass, native frame 64 still FAILS with no descending geometry step (120.89 s). The component-local change removes unnecessary coupling but does not resolve this selected failure; no simulation-wide improvement claim follows. Both qualification handles 29957 and 26280 are terminal, with logs saved in this directory. Next required investigation is the original response direction and changing closest features of the rejected frame-64 contact component, not another blind increase of temporal or nonlinear counts.

## Rejected contact-direction audit

Opt-in VOXY_HAIR_REJECTED_CONTACT_DIRECTION_EXPORT captures pre-increment/proposed positions and initial mesh planes at the actual no-descent rejection. Export failure does not change admission. An independent Python audit computes the frozen mesh-plane directional derivative and piecewise unilateral merit along all 13 tested scales. The full native run again FAILS at frame 64 (121.44 s), iteration 7.

There are 388 initial mesh constraints. Their frozen initial merit is 1.1964161946502298e-5 m² and directional derivative -1.8612753106690255e-5 m²; the full correction reduces frozen mesh merit to 2.34882332523087e-6 m². All tested smaller scales also reduce this frozen merit. Yet actual refreshed all-contact merit rises from 1.196505778013976e-5 to 1.819068461980613e-5 m². Thus the original direction is descent for initial mesh planes while refreshed geometry rejects it. This does not isolate new mesh features from strand-contact changes, and is not a complete constraint-gradient certificate. The next evidence needed is a refreshed mesh/strand decomposition and feature-identity continuity along the rejected line. Files: rejected-contact-direction.json, analyze_contact_direction.py, contact-direction-analysis.json and contact-direction-model.log. JSON parsing and diff whitespace checks pass. No completed jump/GPU/FPS claim.

## Refreshed mesh/strand decomposition

Opt-in VOXY_HAIR_CONTACT_PARTS_TRACE records separately summed actual refreshed mesh and strand negative-gap squares, plus the worst mesh witness. It leaves the admission merit arithmetic unchanged. The repeat FAILS at native frame 64, iteration 7, after 127.27 s. Initially mesh merit is 1.19641619465023e-5 m² and strand merit 8.958336374647603e-10 m². Full proposal: mesh 1.819068142343137e-5, strand 3.196374765754053e-12. At scale 1/4096: mesh 1.1964288592800266e-5, strand 8.869581149656126e-10. Thus mesh geometry supplies the increase, including at the smallest tested scale; strand contribution improves. The worst interior witness's fraction shifts from 0.7929718509327244 to 0.7929799726309412 at the smallest scale, with the same reported normal. This supports investigating moving interval-midpoint witnesses, rather than attributing this rejection to new strand contacts. It does not prove a unique cause or fix.

Current interior witness uses the midpoint between surface crossings; its material fraction changes with endpoint motion. A fixed-fraction linear normal omits this derivative. Next correction to test: locate deepest signed-distance penetration within each interior interval, so smooth interior extrema eliminate the tangential witness-motion term; nonsmooth/multiple minima still require conservative geometry handling. Existing root permission, full density and admission thresholds must remain. Files: contact-parts-frame64.json, contact-parts-model.log. No completed physical jump/GPU/FPS claim.

## Deeper interior witnesses and metric recovery normals

Interior crossing intervals now retain coarse signed-distance samples (including the old midpoint) and refine sampled local-minimum brackets with bounded golden-section search. The deepest observed sample determines the constraint fraction. This approximates deepest penetration; it is not a global extremum/clearance certificate for arbitrary nonconvex geometry. Follicle permission and all existing physical gates remain unchanged.

The interior witness normal now uses the closest-point metric distance gradient, while the feature pseudonormal still determines distance sign. This corrects the previous use of a sign pseudonormal as a distance derivative at concave edges. The tetrahedron regression checks analytic depth in three geometric scales; the concave-edge regression checks the interior witness's normal against the known outward metric gradient. 127 hair unit tests pass (7 ignored), 20 integrations pass, and diff whitespace checks pass. The first overly strict exact-equality assertion was replaced with a 1e-14 comparison for interpolation roundoff; physical admission gates were not changed.

The deepest-witness-only full model (before the metric-normal correction) completed FAIL at native frame 64, no descending direction, 147.42 s. The current deepest+metric run is still live: exec session 30952, /tmp/voxy-deep-metric-model.log, at least 9 frames admitted at the last observation. Do not restart solely on an observation timeout; poll this exact handle. No completed jump, actual GPU dynamics or >160 rendered FPS claim follows yet.

## Completed metric-witness run; segment-envelope step merit

The deepest+metric-witness full native run completed FAIL at frame 64, no descending geometry step (135.62 s). The previously pending handle 30952 is terminal; deep-metric-model.log records this result. Metric recovery is corrected, but it does not solve the full-model equilibrium.

Mesh step merit now sums squared maximum negative gap per canonical segment, rather than summing every mesh witness. Newly discovered duplicate/dependent witnesses must not increase the metric solely through their count. All original constraints still participate in solving, and final fresh-geometry admission still checks every mesh/strand gap at the unchanged thresholds. Strand pair merit is unchanged. This changes globalization weighting, not physical feasibility gates, and is not a certificate that the deepest observed witness represents arbitrary nonconvex segment geometry. The raw diagnostic mesh sum is now labelled mesh_raw to distinguish it from the admission metric.

The regression checks redundant same-depth witness invariance and increased merit for a deeper witness. 128 hair unit tests pass (7 ignored), 20 integrations pass, and diff whitespace checks pass. Full native articulated-jump qualification is still live (exec session 54618, /tmp/voxy-envelope-merit-model.log), not yet a terminal pass/failure. Poll this same handle; no completed jump, GPU or FPS claim.

The segment-envelope full-model run is now terminal: frames 1–63 pass, native frame 64 FAILS with no descending geometry step after 143.55 s. The previous live handle 54618 completed; no qualification is pending from that run. New weighting is invariant to redundant witnesses but did not resolve the observed contact direction. See envelope-merit-model.log.

Next required infrastructure: exact rejected-component nonlinear replay with current sampled collider geometry, original rest/material state, pre-increment predicted/current poses and original constraints. The existing contact_position_fixture_tests nonlinear surface replay is historical and requires a VHC1 collider capture; it does not replay current frame 64. Do not use it as evidence for this failure. The already saved rejected-contact-direction.json supplies positions and mesh planes but lacks rest/material, predicted frames and the actual sampled mesh, so it is insufficient for a faithful nonlinear replay.

## Verified rejected-state nonlinear replay

Opt-in VOXY_HAIR_CONTACT_REPLAY_EXPORT writes VHR1 at a no-descent rejection after restoring the pre-increment pose. The capture includes all 469 rods' material/rest/current/old/predicted positions and frames, velocities, and the actual sampled collider's current/previous triangle vertices and vertex velocities. Spatial caches and BVH are rebuilt during loading; this is not a byte-identical whole-system checkpoint, full trajectory replay, or accelerator-state rollback. The reader is an ignored diagnostic test, not a public asset-import API.

The full model again FAILS at native frame 64 (123.15 s), producing rejected-contact-replay.vhr (~9.8 MiB). The independent native replay verifies captured and rebuilt initial segment-envelope merit are exactly the same printed f64 value, 2.902079228447023e-5 m² (checked at unchanged 1e-12 relative diagnostic comparison), and reproduces the same no-descending-step error. It completes in 0.29 s. 128 existing hair unit tests pass; 8 diagnostics are ignored by default. Compilation's initial mutable-reference error in the new diagnostic loader was corrected before validation. Diff whitespace checks pass. This proves faithful initial geometry merit and matching selected failure, not every hidden cache/state bit or a successful physical step.

Replay command:

```sh
VOXY_HAIR_CONTACT_REPLAY_FILE=/Users/themoretheless/Documents/ChatGPT/Voxy/artifacts/articulated-jump-2026-10-09/rejected-contact-replay.vhr CARGO_TARGET_DIR=/tmp/voxy-release-120fps-20261007 cargo test -p physics --release --lib replay_rejected_contact_geometry -- --ignored --nocapture
```

Use this captured failure to develop and audit geometry/contact directions before rerunning the full jump. Keep baseline rejection reproduction separate from future success qualification. Logs: capture-replay-model.log, contact-replay-validation.log, replay-capture-units.log. Full jump/GPU physics/>160 FPS remain unqualified.

## Fully interior segment discovery correction

The captured-state diagnostic shows refreshed raw mesh merit remains discontinuous at very small line fractions. Source inspection identifies omitted interior depth for segments wholly inside the solid: the previous discovery evaluated interior minima only when the segment had boundary crossings. Endpoint contacts alone can miss deeper penetration. Discovery now also evaluates the free interval when either free endpoint has negative closed-surface signed distance; first-segment follicle permission is retained. The tetrahedron regression places the complete free segment inside the volume and checks the known 0.2 depth, with unchanged positions during discovery. 129 unit tests (8 ignored at that run) and 20 integrations pass.

The historical replay strict-merit check now intentionally exposes changed discovery: captured old merit 2.902079228447023e-5 versus corrected discovery 5.175412518189963e-5 m². It FAILS that historical equality assertion; this is not a current passing replay. The separately ignored captured_contact_reaches_feasible_geometry qualification rebuilds current geometry, requires actual contact convergence and retains the original 5% strain/root gates. It FAILS after 4.96 s with shared contact projection constraints did not converge. Thus missing depth is corrected, but the enlarged tangent contact system is not solved and a physical step is not qualified. The current default suite has an additional ignored success-qualification test; historical failure reproduction and future feasible-state admission are kept distinct. Next work: inspect/export the original rejected projection component and distinguish incompatible direction geometry from linear solver nonconvergence. No complete jump, GPU or FPS claim.

## 304-row feasible tangent system and dense fallback admission

The corrected capture exports a 304-row VQP1 original projection system. Its stalled physical residual is about 69.9 micrometres versus the unchanged 1e-11 m tolerance. Independent HiGHS tangent feasibility and nonnegative least-squares dual checks use SciPy in /tmp/voxy-contact-audit-env; audit_projection_feasibility.py re-evaluates original products at 100 Decimal digits. The unconstrained tangent feasibility example uses large coordinates (~0.439 m), so it is not a physical motion solution. The independent elastic dual solution has 152 active rows and original complementarity/gap error far below admission; see inside-projection-feasibility.json. Symmetrization is used only to construct the independent dual candidate; all reported final residuals are evaluated against the original exported matrix.

The native dense active-set fallback previously returned without solving n>256. Its bounded supported size is now 512 (the full Gram matrix is at most 2 MiB; additional working storage is separate). This admits the actual 304-row component, not a change in physical tolerances or iteration count. The original native exported-system inequality/complementarity fixture now passes all existing gates. 129 regular hair units (9 ignored) and 20 integrations pass; diff whitespace checks pass.

The nonlinear captured-state success qualification still FAILS (4.99 s): the previous linear projection error is gone, but 32 refreshed-geometry iterations do not converge. Last observed worst mesh gap is about -2.045 mm on guide 334, segment 10. The merit trace reduces through its final iterations but remains roughly 2.36116e-5 m². This is not a completed physical step/full jump, GPU solve or FPS qualification. Current logs: projection304-native.log, projection512-replay.log, projection512-merit-replay.log, projection512-{units,integrations}.log. Next work must resolve nonlinear contact geometry/globalization rather than relabel this tangent solve as physical success.

## Current full-path run and repeated endpoint query removal

A new 720-frame full-density native paired trajectory starts with both fully-interior discovery and the 512-row dense fallback. It is still live (exec session 58612, /tmp/voxy-current-path-model.log), with at least 17 frames admitted at the last observation. This matters because the historical capture was already deeply penetrating when created with earlier discovery. Do not restart this run on observation timeout. It is not GPU/FPS evidence.

While that binary is running, endpoint inside classification now reuses an already exact signed node query only at an identical position, or a still-valid positive clearance certificate. Moved nodes are queried again; clipped first-root interior points receive fresh queries. This removes redundant nearest-surface work without dropping geometry. The running full-model binary predates this query-reuse edit; current query-reuse validation consists of 129 unit tests (9 ignored), 20 integrations including stale-clearance regression, and selected-capture initial-merit/outcome comparison. The corrected capture retains exactly printed initial merit 5.175412518189963e-5 and the same nonlinear-linearization failure. Current replay time is 4.10 s versus earlier 4.99 s, but concurrent load and single samples prevent claiming a reliable performance improvement, much less rendered FPS. All physical gates remain unchanged. Logs: endpoint-query-{units,integrations,replay}.log. Diff whitespace checks pass.

## Current-path medial witness failure

The fully-interior-discovery/512-row full-density trajectory completed FAIL at native frame 64 (221.61 s). Handle 58612 is terminal. The current-path capture (~9.8 MiB) is distinct from the historical pre-discovery capture. Strict current replay checks exact printed initial merit 4.846947083589153e-6 m² and reproduces the same no-descent rejection in 0.29 s. This validates initial rebuilt geometry and selected error, not full hidden-cache identity.

Brute-force vectorized nearest-triangle queries on the captured sampled mesh independently inspect the worst interior point (guide 432, segment 7, fraction 0.7572541925918105). At fraction offsets ±1e-6, nearest features switch between triangles 72898 and 72971. Signed metric-normal projections along the segment are -0.01483422036 and +0.01353005295 m respectively. Thus this deepest observed point is a nonsmooth minimum of signed distance along the segment, not a differentiable stationary point whose single selected normal is orthogonal to the segment. The fixed-fraction single-normal tangent omits moving-witness/branch effects.

A convex combination with left weight 0.47701038568564313 yields a stationary generalized gradient with segment dot product ~1.19e-18 m and norm 0.11168019678386332. This is diagnostic evidence for a generalized envelope derivative; it is not yet an implemented force/contact law or a complete proof for arbitrary nonconvex shapes. Normalizing that vector without consistently scaling the linear residual would change the Jacobian, so the next correction must keep force/length units and contact admission consistent. Files: current-path-model.log, current-path-replay.log, current-path-contact-replay.vhr, current-path-direction.json, audit_interior_witness.py, current-witness-audit.json. No completed physical jump/GPU/FPS claim.

## Two-sided interior envelope derivative

Interior medial witness linearization now probes both sides of the deepest observed material fraction. When the metric-normal segment slopes straddle zero, a convex combination makes the combined segment derivative stationary. The unit force direction and original gradient magnitude remain separate. Interior plane target displacement is divided by that magnitude, RodContact::metric_scale carries it, structural penalty/energy use the original physical gradient and residual, and actual merit/fresh contact admission use physical residuals. Normal surface speed is combined consistently from the two metric normals and sampled surface velocities. Ordinary contacts retain scale 1.

This is a bounded two-sided observed derivative, not a complete Clarke feature enumeration/global nonconvex certificate. Near-zero gradients, endpoint extrema and unbracketed features retain the preceding metric-normal rule; nonlocal medial escapes remain unresolved. The tetrahedron regression checks analytic envelope derivative and finite differences; the existing energy-gradient regression now covers a scaled contact. Historical plane fixtures explicitly retain scale 1. Initial compile errors in fixture initializers/imports were corrected before validation. 130 units (9 ignored), 20 integrations and diff whitespace checks pass.

On the current-path historical capture, initial physical merit remains approximately 4.8469470835892e-6 m²; the previous immediate no-descent rejection becomes 24 accepted decreasing iterations, reaching 4.235547626715524e-6, then FAILS again with no descending step (1.93 s). This shows selected progress, not contact equilibrium or a successful physical step. The new full 720-frame native paired run begins with this derivative correction, and is still live; exec session 66728, log /tmp/voxy-envelope-gradient-model.log, export envelope-gradient-path-replay.vhr on no-descent. No GPU dynamics or >160 rendered FPS claim.

## Physical contact diagnostics after envelope scaling

HairContactDiagnostic now exposes metric_scale and reports physical gap_m via RodContact::physical_gap, rather than labelling the normalized virtual plane residual as physical depth. Normal, target and surface velocity are documented as linearization quantities; a medial virtual target is not asserted to be an actual mesh point. The focused read-only diagnostic regression passes for ordinary contacts and scale 0.25. Contact-direction JSON also includes metric_scale. Diff whitespace checks pass. This diagnostic edit does not change the active full-model binary's dynamics.

The full envelope-gradient trajectory remains live at observation (exec session 66728, /tmp/voxy-envelope-gradient-model.log), with at least 19 admitted frames. There is no terminal full-jump result yet; re-poll the same handle rather than restart. No GPU dynamics/FPS claim.

## Envelope-gradient trajectory failure and contraction-gated work budget

The full envelope-gradient run completed FAIL at native frame 56 after 129.75 s. Its last selected witness is strand/strand (guides 292/291), gap -3.2967176618636125e-6 m after 32 increments, versus the unchanged 1e-10 geometry threshold. This is an earlier failure than the prior frame-64 body case; the change is not a completed trajectory improvement. Handle 66728 is terminal.

Nonlinear reconciliation now allows up to 128 increments, but after the initial 32 it continues only if actual freshly queried merit contracted at least 10% over the last eight increments. Stagnant geometry still rejects, and all force/contact/strain gates are unchanged. The 10%/eight-step criterion is a computational continuation heuristic, not a proof of convergence or a relaxed physical tolerance. It needs full-path validation. Cap/stagnation failures can now export their staged state through the same VHR1 diagnostic path, and opt-in merit history is retained in the log.

130 unit tests (9 ignored), 20 integrations and diff whitespace checks pass. The new native full-path run is still live (exec session 74538, /tmp/voxy-contracting-budget-model.log, possible export contracting-budget-path-replay.vhr). Re-poll this handle rather than restart on timeout. No successful complete jump, GPU dynamics or >160 rendered FPS claim.

## Contraction-budget terminal result and jump verification

Session 74538 completed with FAIL at native frame 58 (129.32 s), after admitting frame 56 using 33 contact increments. The repeat with rejected-guide export also FAILS at native frame 58. Guide 86 has maximum signed axial strain 0.07099598825664444, beyond the unchanged 5% gate. Original staged guide, contacts and reassembled linear system are saved in contracting-budget-rejected-guide.json. This is a rejected candidate, not published simulation state. Logs: contracting-budget-model.log and contracting-budget-strain-capture.log. No successful complete hair jump or rendered FPS result.

The actual-model leg IK test and analytic jump continuity/ballistic test were rerun and pass. Actual sole vertices: 8280; maximum knee displacement: 0.18173385 m. Diff whitespace validation passes. The existing jump-preview.gif remains rig-only and does not demonstrate secondary physics.

## Complete rejected strain state and coupled correction

The endpoint admission capture reproduces native frame 58 rejection (135.57 s) and stores all 469 rods, prediction/inertia state, orientations, material data, and endpoint collider geometry in strain-admission-replay.vhr. Export is opt-in and occurs only after a completed staged step rejected by the caller; public state remains rolled back. Export failures do not alter admission.

The ignored captured_elastic_contact_preserves_strain_admission test verifies initial reconstructed contact merit within relative 1e-12, confirms initial strain rejection, and alternates actual elastic energy-admitted corrections with full nonlinear contact admission. It PASSES after ten additional cycles: maximum strain 0.04396673621142333 (unchanged limit 0.05), roots exact. Intermediate strain rises to 0.12218799185786869; this does not prove monotone strain descent, physical equilibrium, or a complete jump. Log: elastic-contact-strain-verified.log.

Experimental terminal_contact_iterations now rechecks nonlinear contact geometry after each elastic correction when joint positions are enabled. This removes the prior possibility of leaving new penetration after the final elastic correction. Defaults are unchanged pending full-path qualification. Full 720-frame native run with ten additional cycles per substep is in progress (exec session 99279, /tmp/voxy-coupled-terminal-jump.log); poll the same live handle. No GPU/FPS claim.

Post-change regular validation: 130 hair unit tests pass (10 ignored), 20 hair integration tests pass, diff whitespace check passes. Full native run 99279 is confirmed running and has admitted frames 1–3; this is not a terminal qualification result.

## CPU strand refresh profile and exact pre-sort culling

Five-second macOS sample of the confirmed live native full-path process (PID 3232, session 99279) identifies strand gathering and integer pair sorting as major CPU stacks. This is a sampled CPU profile, not rendered FPS or a GPU benchmark.

For immutable strand refresh only, the existing current capsule-AABB rejection now executes before candidate allocation/sorting. Surviving candidates retain lexicographic order. Sequential position projection retains the original candidate list and live bounds check because earlier projections can create later contacts; its behavior is unchanged. Contact tolerances, density, root exclusions and narrow-phase geometry are unchanged.

On the actual 469-guide rejected-state capture, paired reference/candidate verification finds exactly the same 27 pair geometries, order, normals, impulses and rod contact planes, with unchanged positions. Final twelve-repeat alternating measurement: reference median 2.240667 ms, filtered median 1.801500 ms, about 19.6% faster for this operation. Timing is fixture-specific and subject to concurrent full-run load, not an end-to-end FPS result. The separate coupled strain replay still passes (1.78 s). Regular hair units: 130 pass, 11 ignored; integrations: 20 pass; whitespace check passes.

Full jump session 99279 continues on the already-running coupled-terminal binary, which predates this pre-sort optimization; its eventual result qualifies coupled iterations only. It has admitted frame 49 at this checkpoint. Preserve/poll the same process rather than restart.

Confirmed live poll of session 99279 now admits native frames 58 and 59: the previous frame-58 strain failure is passed in the full path with coupled terminal iterations. The 720-frame qualification is still incomplete; no GPU/FPS claim.

## Full coupled-terminal path result

Session 99279 is terminal FAIL after 462.37 s: frames 1–63 admitted; frame 64 rejects joint contact direction without any descending geometry step. Frame-58 strain rejection is passed, but the complete jump is not qualified. The capture repeat on the current pre-sort optimized source is now running (session 82722, /tmp/voxy-coupled64-capture.log) with nonlinear VHR export coupled64-contact-replay.vhr. The attempted direction environment name in this repeat is not the supported one; once VHR is available, obtain direction JSON by localized replay with VOXY_HAIR_REJECTED_CONTACT_DIRECTION_EXPORT. Preserve this confirmed live repeat rather than restart it.

## Real Metal structural batch and default factor reuse

Original f64 structural systems are reassembled from all 469 captured rods at h=1/240 s after refreshing actual endpoint contacts, and independently native-force-admitted before export (strain-admission-structural-batch.json). This is a rejected frame-58 endpoint fixture, not a complete trajectory.

Apple M4 Max Metal runs the actual 469 heterogeneous systems with one GPU-solved residual correction and original f64 force residual validation. No native factorization fallback is used. Eight repeated solves pass: maximum native/GPU position correction difference 6.378851033538371e-13 m, angle correction difference 3.641044063473764e-11 rad; checked hybrid median 11.529375 ms. This includes CPU packing/residual products and blocking GPU readback; it is not a GPU kernel-only time or rendered FPS.

Alternating fresh/reused-factor runs on the same full fixture all pass original force gates and 1 micrometre/5e-5 rad comparison budgets. Seven measured repeats per mode: fresh median 12.072709 ms, reuse median 11.370625 ms (about 5.8% lower on this fixture under concurrent native-run load). Immutable matrix/layout checks remain before RHS uploads. Same-call refinement factor reuse is now the constructor default; explicit fresh-factor overrides remain available. Refinement count is unchanged, and zero-refinement behavior is unchanged.

Independent actual Metal regression covers 469 compliance systems, 12 changing-RHS batches and refinement counts 1–3. Cached/fresh corrections are bitwise identical, 24 reused dispatches; fresh total 270.568084 ms, reused total 241.076377 ms. This qualifies factor reuse across these cases only, not nonlinear contact convergence, CUDA/other vendors or >160 rendered FPS.

## Captured frame-64 endpoint witness defect

Session 82722 is terminal FAIL at frame 64 after 393.11 s, capturing coupled64-contact-replay.vhr. Exact local failure reproduction passes in 0.36 s: captured/rebuilt merit both 1.7522810769070946e-5 m². This PASS reproduces rejection; it does not qualify simulation success.

The rejected direction export now includes freshly queried contacts at the smallest tested scale (1/4096). Canonical mesh merit rises from 1.7520651259290344e-5 to 1.9883921590530353e-5 m². Almost all increase is rod 432 segment 9, whose recorded gap changes from -40 micrometres at t=0.860395 to -1.5382875 mm at t=0.991092. Maximum segment-guide movement at this scale is about 0.791 micrometres. Direct signed-distance probes and independent oriented ray classifications agree the segment interior is inside the collider; classification was not changed. Files: coupled64-trial-feature-analysis.json, coupled64-interval-sign.log.

Coarse endpoint minima previously did not bracket/refine the last sampling cell, allowing a deeper medial peak between the endpoint and preceding sample to remain undiscovered. An analytic tetrahedron regression at t=0.99 and reversed t=0.01 across scales 1e-3, 1 and 1e3 FAILS the previous algorithm. The fix refines deepest endpoint sampling cells while retaining endpoint candidates; the regression now passes. This remains bounded approximate constraint discovery, not a global nonconvex clearance certificate.

Post-fix regular hair units: 131 pass, 12 ignored; integrations: 20 pass; actual prepared full-density groom admission test passes (1.22 s). On the historical captured state, the prior no-descent direction failure is gone, but success qualification still FAILS after 1.67 s by insufficient merit contraction after 32 increments; worst residual gap about -1.450596 mm, rod 432 segment 9, very small envelope gradient. No completed full trajectory claim.

The revised full-path run starts from initialization rather than the already-penetrated historical state, with unchanged 10 coupled terminal iterations and all original physical gates. Session 25428 is confirmed live, log /tmp/voxy-endpoint-path-jump.log, admitted frames 1–14 at this checkpoint. All three actual failure exports are enabled with correct names: endpoint-path-contact-replay.vhr, endpoint-path-contact-direction.json, endpoint-path-admission-replay.vhr plus rejected-guide JSON. Poll this exact process. No >160 rendered FPS/CUDA/full-goal completion claim.

## Endpoint-refined full-path result and GPU transport

Session 25428 is terminal FAIL at native frame 64 after 362.20 s: nonlinear geometry linearization does not converge; final worst residual gap -1.5992235899016936 mm on rod 334 segment 10. The earlier no-descent failure is absent, but full jump admission remains incomplete. Current captured state is endpoint-path-contact-replay.vhr.

Structural residual RHS updates now use one coalesced upload buffer with ordered command-encoder copies, instead of 469 individual queue writes. Original matrix/layout equality is checked first, coefficient/factor storage is unchanged, and upload lifetime covers submission/readback. Residual input reuses the exact previously admitted coefficient words/scales via with_rhs rather than re-equilibrating/repacking the unchanged matrix. No new physics/precision switches or reduced density are introduced.

Actual Metal full batch qualifies all original f64 force gates and comparison budgets. Older rejected-frame fixture: fresh-factor median 10.192167 ms, reuse median 9.077125 ms, maximum position difference 6.378851e-13 m and angle difference 3.641044e-11 rad. Newly exported endpoint-path failure fixture: fresh median 11.007125 ms, reuse median 9.950750 ms, maximum position difference 2.0200952e-11 m and angle difference 1.4325536e-9 rad. These are checked hybrid solve costs on two captured states under concurrent native workload, not GPU-only timing or rendered FPS.

Independent 469-system, 12 changing-RHS batch regression covers refinement counts 1–3: cached/fresh results are bitwise identical, original force gates preserved, 24 cached dispatches; fresh total 269.350167 ms and cached total 195.371167 ms. The captured single-guide GPU force test also passes with the default cached/coalesced path. Diff whitespace check passes.

Qualification call accounting now uses the actual configured substep count rather than fixed two, and expects zero bridge calls for native-only runs. This removes an unrelated false failure at the end of successful native/temporally-refined qualifications; physical admission and CPU/GPU difference budgets remain unchanged.

A controlled half-timestep full-density native qualification is now running: correct setting VOXY_HAIR_QUALIFICATION_SUBSTEPS=4 confirmed in the runtime log, h=1/480 s, unchanged force/strain/contact gates, 10 coupled terminal iterations. Session 97589 is confirmed live with frames 1–19 admitted, /tmp/voxy-half-step-path-jump.log. The preceding mistaken TEMPORAL_SUBSTEPS invocation (session 6249) was explicitly interrupted with exit 130 and does not count as a four-substep result. Capture exports are half-step-path-{contact,admission}-replay.vhr, direction/rejected-guide JSON. Poll session 97589, do not restart on timeout. No complete jump, >160 rendered FPS, CUDA or full-goal completion claim.

## Terminal update: compact GPU transport and witness precision

The four-substep full trajectory is terminal: frames 1–64 admitted, frame 65 rejected (no descending contact geometry), 991.92 seconds. Earlier live-status notes are superseded. Full coupled jump and >160 rendered FPS remain unqualified.

Structural GPU readback now gathers RHS/status only: 474,644 bytes instead of 4,729,412. The full 469-system Metal test compares full and compact outputs bitwise and preserves original f64 force admission; final ordinary-capability medians were 9.856084 ms fresh and 8.902000 ms reused factors. A Metal device requested with one storage binding also passes, using ordered buffer copies; medians 14.196833/14.869584 ms. These are blocking hybrid bridge timings, not rendered FPS or other-vendor evidence. Factor-reuse regression with 469 systems and changing RHS passed.

Interior witness refinement now resolves physical brackets to 1e-12 m (maximum 80 iterations), below the 1e-10 m contact tolerance. An analytic multi-scale endpoint-peak regression failed with the previous 32 iterations and passes with the correction. Hair unit tests: 131 passed, 12 ignored; integration tests: 20 passed. Localized frame-65 success qualification still fails at iteration 1.

An experimental 24-backtrack search reached the contraction cap: merit 4.60451033844569e-6 to 4.604352402939551e-6, about 0.0034 percent reduction, penetration still approximately 1.55 mm. It was reverted to 12 attempts; physical gates were never relaxed. Further work must address the stalled nonlinear contact direction rather than merely increasing iteration budgets. No live process remains from these runs.

## Continuous moving-face query and captured Jacobian audit

On the actual frame-65 capture, the worst weak-gradient mesh witness is rod 51, segment 7, penetration 1.550181646 mm, gradient magnitude approximately 0.00154. A new ignored fixture audit independently finite-differences refreshed geometry for common translation and each endpoint separately, three axes and three displacement scales. At 1e-8 m perturbations, absolute gradient error stays below 3e-6 (gate 5e-6), confirming interpolation weights and the small local gradient for this witness. This does not prove every captured constraint or global convergence.

New public TriangleMotion/sweep_capsule_triangle query advances the entire capsule centreline against linearly moving/deforming triangle vertices. Relative endpoint/vertex velocities provide its speed bound; it shares the existing conservative advancement kernel with capsule/capsule queries. Seven regressions cover passing through a surface with both endpoint states clear, moving collider, common translation cancellation, deformation with sampled safe prefix, collapsed triangle edges, non-clear initial/iteration-limit outcomes, and invalid/precision-inadmissible inputs. Hair units: 138 passed, 13 ignored; integration: 20 passed; captured endpoint-Jacobian audit: passed.

The new query is not yet integrated into HairSystem structural motion or moving-mesh broadphase. It does not classify closed-volume containment and does not itself recover initially penetrating geometry. Full coupled jump remains unqualified at frame 65; no new rendered FPS or GPU dynamics claim. Next work is mesh-wide continuous admission and contact activation during structural motion, preserving prescribed roots and strain/force gates.

## Mesh-wide temporal broadphase and full captured groom query

TriangleMesh::swept_capsule_contacts now constructs temporal face and node bounds from previous/current vertices once per capsule batch, reusing the immutable static BVH topology. It validates all capsule inputs before culling, preserves every non-clear query including iteration-limit outcomes, and sorts output by capsule index and face vertex ids. No mesh, rod, rest geometry, or existing spatial caches are mutated. Bounds cover linear interpolation only; closed-volume containment remains a separate responsibility.

Two new regressions verify exact agreement with exhaustive moving-face queries through a multi-level BVH, including a moving face missed by current-only bounds, and invalid inputs outside all candidate bounds. No allocation is made per BVH node when accumulating temporal bounds.

On the historical frame-65 model, all 20 segments of guide 51 match exhaustive queries against all 121,728 faces exactly: 19 Approach outcomes, no initial contacts or iteration limits. All 469 guides (9,380 segments), retaining the existing 15-percent prescribed follicle exclusion, were also queried against the full collider. Results: 31 InitialContact, 434 Approach, 4 IterationLimit. Observed CPU query times were about 24.5 ms per captured snapshot, not a steady-state benchmark or rendered FPS result. Four unresolved fractions remain conservative, never clear: guide 334/segment7 gap 22.06 nm, guide395/segment5 gap252.55 nm and1.00696 um on two faces, guide401/segment7 gap0.497 nm. Their 1024-iteration bound remains unchanged. These outcomes cannot be used to authorize a full step.

Final focused hair units pass (140 passed, 15 ignored); integration passes (20); full-model selective exhaustive and full-groom diagnostic tests pass. Passing the full-groom diagnostic means successful query classification, not feasible hair dynamics. Continuous mesh admission is still not connected to the structural integrator; initial contacts need response ownership and moving collider/attachment time must advance consistently. Full articulated-jump dynamics, rendered >160 FPS, and GPU/CUDA implementation of these new queries remain unqualified. All tool sessions from this update are terminal.

## Time-aligned continuous admission connected to HairSystem

Collider sampling previously retained the entire frame's previous vertices on every substep. New immutable TriangleMesh::motion_interval restricts previous/current vertices and duration to the actual subinterval, preserves prescribed material velocities, validates interval endpoints, and treats untimed refits as stationary. HairSystem now uses these aligned intervals when sampling collider motion. Adjacent interval boundaries, invalid-source preservation, static-motion reset, and velocity/duration consistency are covered by regressions.

An opt-in HairSystem::continuous_mesh_admission flag checks each solved substep trajectory before finish/publication. It preserves the prescribed first-15-percent follicle exclusion, rejects unknown or entering motion transactionally, includes query cost in mesh profiling, and survives groom rebase. Qualification flag is VOXY_HAIR_CONTINUOUS_MESH_ADMISSION; default remains false until response and full-model qualification succeed. A moving closed slab that passes completely through an unchanged rod between clear endpoints is accepted by the prior discrete step and rejected atomically by the new mode; poses, history, caches, and diagnostics roll back, and a subsequent clear step remains valid. A clear trajectory preserves rod physics bitwise.

Touching/sliding trajectories are admitted only through sufficient continuous separation certificates. A fixed closest-feature axis bounds every capsule endpoint against every finite-triangle vertex at both endpoint states; affine relative projections then bound the entire linear path. For moving face planes, outward-rounded signed cubic, squared-clearance degree-six, and strict nonzero-normal bounds prove whole-path separation. Their radius-minus-1e-10 physical threshold matches existing admission; no physical tolerance was loosened. Interval arithmetic and polynomial subdivision are shared with prior swept-strand line certificates rather than duplicated. Unknown, overflowing, degenerate-normal, budget-exhausted or entering paths remain non-clear. Tests cover finite vertex sliding, tolerance violations, and rotation with clear/touching endpoints but intermediate penetration.

Full 720-frame CPU-control qualification with continuous admission remains FAILED at frame 1: guide53/segment18, face [15708,15711,15712]. The final export run ended after 4.20 seconds of test execution; no long-running process remains. This is not GPU trajectory evidence despite the historic test name. Both separation certificates correctly fail this path. Initial-mesh-contact-motion.json captures exact capsule/face endpoints; independent NumPy/SciPy diagnostic audit samples 1001 times and optimizes capsule points. It finds initial gap approximately zero, final gap 0.7129375 mm, and intermediate penetration about 2.322904 um at normalized time0.153. This is sampled evidence, not a global certificate. A native regression using the exact captured endpoints also detects more than2 um penetration at that time and prevents admission.

Final hair units: 148 passed, 15 ignored. Integration:20 passed. Captured guide51 exhaustive-vs-temporal-BVH test still passes against121,728 faces. The new whole-groom certificates did not resolve the historical four iteration limits. Full continuous jump requires initial-contact reaction/activation in the structural solve, not a bypass or endpoint-only acceptance. Dense grooming, root/strain/force/contact gates remain unchanged. GPU/CUDA continuous dynamics and >160 rendered FPS are still unqualified; full thread objective remains unfinished.

## Space-time witness Jacobian and local constitutive response

New native contact_trajectory.rs discovers a penetrating capsule/finite-triangle witness over normalized substep time. It retains sixteen sampling cells and bounded golden-section refinement to 1e-12 m relative-motion width; this is approximate discovery, never a continuous-clearance certificate. Entire initial overlap, zero-distance oriented-surface crossing, overflow and invalid queries remain explicit errors. A previously certified clear trajectory produces no constraint.

For a witness at time t, physical residual is t times the virtual endpoint-plane residual; endpoint load columns are t*(1-s)*n and t*s*n. The virtual target removes the fixed old pose before dividing by t. Thus contact enters the existing f64 compliance/inequality solver through its physical Jacobian rather than pretending a past contact is an endpoint contact. The trajectory helper is currently connected only to opt-in rejection diagnostics, NOT to the full HairSystem force loop or velocity/friction ownership. Full continuous admission still rejects the first jump frame until that integration is complete.

A regression uses the actual guide53/segment18 capsule/face capture. Its optimized minimum is t=0.15325123552152253, s=1, gap=-2.32356470299 um. All six endpoint derivatives agree with independently re-optimized geometry to absolute1e-6; frozen-row finite differences also agree. A LOCAL two-segment native Cosserat compliance fixture, with a synthetic prescribed root carried by the capsule's common translation, gives an admitted linear contact correction without root movement. Stretch is1.26931798e-6; re-optimizing actual space-time geometry after the reaction leaves5.22419554 nm penetration, versus2.3235647 um initially. This still exceeds the unchanged0.1 nm contact tolerance. It is a local force/Jacobian qualification, not an original full-guide or full-groom trajectory proof; the synthetic root choice is not evidence for the actual articulated attachment. An initial frozen synthetic root introduced unrelated18-percent strain and was corrected to the specified translating fixture.

Final units149 passed/15 ignored; integration20 passed; git diff --check passed. No new full-jump or GPU/FPS qualification was performed in this update. Next is reuse of the common nonlinear contact loop with refreshed space-time rows, preserving separate endpoint velocity/friction semantics and the final continuous certificate. The thread's full engine/hardware/material/animation objective remains active and incomplete. All sessions from this update are terminal.

## Shared nonlinear loop now owns refreshed space-time constraints

The captured local two-segment constitutive fixture reaches certified trajectory admission after two native contact reactions, preserving its prescribed translating root and <=5-percent strain. Its first refreshed penetration5.224 nm requires one additional reaction. The final result passes the continuous finite-triangle query, not only a frozen tangent plane or sampled minimum. This remains a local fixture, not the original articulated full guide.

HairSystem's existing nonlinear contact loop now has a continuous mode. With joint positions and continuous_mesh_admission enabled, every mesh refresh—including full corrections, backtracking trials, and restored poses—adds freshly discovered space-time constraints through the same native/external contact solver, trust policy, physical merit, and rejection transaction. Successful tangent/geometry convergence must also pass continuous mesh admission. No separate nonlinear solver was introduced. Initial authored swept-strand setup retains discrete initialization. If joint positions are off, the continuous flag continues to provide admission only.

Private RodContact and public HairContactDiagnostic carry optional trajectory_time. Past constraints remain distinct from endpoint contacts, share canonical endpoint ownership, and are skipped by endpoint normal-velocity projection; implicit positional reaction still changes finish velocity. A regression confirms past constraints do not impose endpoint surface velocity while otherwise identical endpoint constraints do. Temporal candidate queries internally retain face indices so response discovery avoids scanning all faces for each stable face-id lookup. Public query order remains canonical by capsule and vertex ids.

Replay VHR2 stores continuous-mode ownership; VHR1 remains supported. Continuous replay qualification reconstructs temporal constraints and uses the same mode in the common contact loop. Legacy diagnostics explicitly reject VHR2 instead of silently treating it as a discrete capture. A roundtrip regression verifies mode and authoritative previous/current vertices.

Final units151 passed/15 ignored; integration20 passed; diff check passed. Full 720-frame native-only CPU-control qualification remains FAILED on frame1 after3.52 seconds: trajectory surface crossing needs oriented contact activation. This run preceded the final endpoint canonicalization adjustment; no current full-trajectory success is claimed. Exact zero-distance face crossings lack a unique unsigned-distance gradient and are rejected rather than assigned an arbitrary normal. Next work must use the closed collider's orientation/old-side ownership to activate that branch, then rerun the full coupled trajectory. Dense groom, original force/strain/contact tolerances, and preserved public-state rejection remain intact. No GPU/CUDA or rendered >160 FPS proof. All sessions from this update are terminal.


### Moving separation axes for initial finite-feature contacts

Closed oriented face crossings now activate a winding-derived face reaction; zero-distance edge/vertex crossings still reject rather than inventing a normal. Continuous proofs now apply to any non-Clear conservative-advance result, including endpoint Approach. Adaptive intervals can use an affine moving separation axis: outward-rounded polynomial inequalities cover every capsule endpoint against every triangle vertex, positive axis norm and radius-minus-original-tolerance throughout each interval. Endpoint interpolation padding tightens the local tolerance. Failed/budget-exhausted certificates remain unknown.

The captured frame-2 rod463/segment17 contact is continuously certified at the unchanged 1e-10 m tolerance. Its independent 1001-time numerical audit found a minimum gap around -2.69e-11 m; that audit is diagnostic, not the admission proof. The regression also enlarges the radius by 1e-8 m and verifies rejection. Hair units155 passed/15 ignored; integration20 passed; whitespace check passed.

Full-density native CPU qualification is running in terminal session2759, log /tmp/voxy-moving-axis-full-jump.log. Frames1-3 have passed at this checkpoint. This is not rendered FPS, GPU trajectory qualification, or complete jump proof. Preserve the running process and inspect its terminal outcome.

The moving-axis qualification session2759 is terminal FAIL at frame16 after165.55seconds; frames1-15 admitted. Failure is shared contact projection nonconvergence,319constraints,residual0.00016m. No primitive-sweep rejection was exported. A capture repeat session89843 uses VOXY_HAIR_PROJECTION_FAILURE_EXPORT to save frame16-projection.json (despite its extension, actual format is binaryVQP1). It runs the pre-metric-correction binary.

Inspection found position_constraints lost RodContact.metric_scale: it used virtual unit-plane residual/gradient while geometry merit used physical_gap. Corrected both bound and load gradient by the same physical metric, including trajectory time. No tolerance changed. New regression checks the physical row bound and Jacobian. Corrected units156pass/15ignored, integration20pass; app test binary builds. Full corrected qualification still pending the terminal capture repeat.

Capture session89843 is terminal FAIL atframe16 after171.66seconds, saving the VQP1 tangent system. Independent linear-program audit reports infeasible; rows68and74 are EXACT opposite Jacobians on rod86point19 with positive sum bounds7.999999999994058e-5m. This is a tangent inconsistency, not a solver-iteration shortage; no nonlinear feasibility conclusion follows.

Corrected physical-metric session40491 is terminal FAIL atframe11 after119.40seconds, primitive sweep iterationlimit on rod414segment19 face[49411,51218,50889], fraction0.0005631668,gap2.1008038e-10m. Original motion captured in physical-metric-rejected-motion.json. Independent refined approximate minimum is -9.99752814e-11m at normalizedtime0.0204781849, only2.47e-14m above the original -1e-10 tolerance. That is diagnostic, not a proof.

Added quadratic moving-axis certificates with outward bounds derived directly from ORIGINAL relative endpoint motion restricted to each interval. This removes resampling padding from that certificate while retaining padding for the local fixed-axis path. Whole convex primitive projections, positive axis norm and squared physical clearance are proved with degree<=6 outward Bernstein bounds. Captured near-tolerance slide now certifies; radius increase1e-12m rejects. Units157pass/15ignored. Error paths now optionally save full VHR2 nonlinear replay plus mesh contact rows before propagating positional-projection failure.

Full corrected qualification session68008 is running, /tmp/voxy-original-motion-full-jump.log. Full jump/rendered>160FPS/GPU continuous dynamics still not qualified.

Original-motion qualification session68008 is terminal FAIL atframe43 after527.15seconds; frames1-42 admitted. Failure is no descending geometry step. Full VHR2 state saved in original-motion-contact.vhr. Local replay reproduces the captured merit EXACTLY and rejection in1.31seconds, producing frame43-direction.json.

This capture exposes contact-discovery activation discontinuity: original merit's full negative gap squared adds approximately tolerance squared when a previously admissible contact first appears just below -1e-10m. Small trials improved existing contacts but activated a new rod198segment4 row around -1.01003544e-10m, causing artificial merit jumps. Merit now measures EXCESS physical violation below the unchanged -1e-10m admission boundary for both mesh and strand contacts. Final physical gap, strain, root and continuous admission gates are unchanged; target load inequalities still aim at zero gap. A regression verifies continuity at the activation boundary and positive penalty immediately below tolerance.

Captured frame43 now converges in TWO native contact increments: first accepted scale1/128, secondfullscale; final continuous admission passes, rootsbitwiseexact, strain<=5%. Local replay log voxy-frame43-continuous-merit-replay.log. This is actual captured full-density geometry, not a substitute model. Units158pass/15ignored, whitespacecheckpass. Old captures' stored raw merit predates this definition; use corrected-feasibility replay rather than old-definition identity assertion.

Full corrected qualification session72269 is nowrunning, /tmp/voxy-continuous-merit-full-jump.log, with all projection/direction/mesh rejection captures enabled. Do not restart this process on timeout. Full articulatedjump, rendered>160FPS, GPU/CUDA dynamics and broad engine parity remain unqualified.

Checkpoint: session72269 remains live and has admitted frames1-3 on the corrected continuous-merit source. Final integration20passed; diffcheckpassed. Preserve/poll this exact process. All other sessions mentioned above are terminal. The broad goal remains active. Rig-only jump-preview.gif is displayable evidence of leg articulation, not secondary dynamics or FPS.

Live macOS CPU sample: PID43698 (session72269 binary), five-second10ms sample saved in voxy-continuous-jump-cpu-sample.txt. Dominant compute stacks are mesh contact refresh, nearest finite-feature queries and outward polynomial certificates; this CPU-control run does not profile rendered GPU FPS.

Experimental earlier fixed-axis/support-plane admission plus cached initial closest pair preserved captured-frame43 final x/q BITWISE (cmp certificate-order-before.state vs certificate-order-after.state). Three warm cargo replay timings: baseline0.8026/0.7117/0.7039seconds; trial0.8041/0.7205/0.7073seconds. No material speedup; production sweep ordering REVERTED to the prior qualified implementation. Optional test-only VOXY_HAIR_REPLAY_FINAL_STATE_EXPORT retained for exact state comparisons. No performance improvement claimed. Full session72269 remains live atframe37 and must be polled, not restarted.

## Coupled elastic contact follow-up

The previous full continuous run completed with a frame-44 strain rejection (564.39 s). The fresh `strain44-capture/qualification.log` independently reproduced it after 651.95 s, with guide 221, segment 19 at +10.03259817% strain. Original physical gates remain unchanged. The completed-step diagnostic now exports mesh motion over the actual last substep; `admission-last-substep.vhr` is an explicitly normalized diagnostic copy of the earlier capture, not a resumable complete simulation checkpoint.

The zero-increment contact projection API retains its pure correction contract. A separate entrypoint couples the free elastic Newton increment and unilateral reactions using the same implicit operator and existing trust/component ownership. Runtime use is confined to the opt-in continuous mesh mode: enabling it in the established discrete mode failed the worker/accelerator integration fixture, so that mode retains its validated original path. The line search uses the frozen physical contact-merit directional derivative and safeguarded quadratic interpolation, still at most 12 trials with actual refreshed geometry acceptance.

Library validation: 301 passed, 34 ignored. Final ordinary hair integration: 20 passed. The captured strain-44 replay now passes the former early no-descent point but still fails nonlinear convergence after 26.06 s; it does not qualify the jump or stretch admission. No full trajectory or >160 rendered FPS claim follows from these tests.

The geometry-owned immutable temporal BVH cache passed its mutation/invalidation regression and preserved the earlier captured final state bitwise. Seven interleaved full-density query pairs had medians 34.4483 ms uncached and 33.5901 ms cached, with identical query results. This selected CPU batch improvement is not rendered FPS. The engine corpus audit separately verifies 500 accepted identities, with all 1428 candidate reviews still classified as root-and-README-only.

Final-state recheck: 301 library tests passed (34 ignored), 20 hair integration tests passed, and the actual-model planted-foot/knee/bone-length jump regression passed.

## Full trajectory follow-up: contact model and shared physical clock

A solve-local mesh contact model now retains physical rows discovered on rejected full candidates, rebuilds the same implicit solve at the uncommitted pose, and keeps those rows out of geometry admission/publication. The contact-merit line search also handles violations below its former 1e-20 early-exit floor. An exploratory mesh-plus-endpoint-strand model brought the captured frame-44 strain to 0.6534659% and passed mesh CCD in 2.45 s. An independent WHOLE STRAND PATH audit then returned only a 2.3460320795e-7 safe prefix. That endpoint/mesh result does not qualify the articulated hair trajectory; the exploratory separate strand model was removed.

The continuous joint path instead reuses the existing swept structural/contact owner. Authored self-overlaps are admitted before the simulation clock begins, with pinned roots and unchanged stress-free lengths. The same owner now handles prediction, every structural candidate, and nonlinear contact correction. An independent complete staged strand-path check runs before publication alongside mesh CCD. No strain, force, gap, density or material gate was relaxed.

Swept affine rows retain their original physical metre residual and time Jacobian, rather than dividing the row by a near-zero witness time. Prescribed root motion stays in the affine bound. Collision-limited steps now activate the blocking witness even when capped strain is small; the prior strain-only trigger could repeatedly stop at the activation band without incorporating its reaction. Continuous structural Newton iterations retain the original substep clock: independently safe iteration segments cannot certify their overall physical interval.

New regressions exercise an unmodeled time-scaled mesh plane, imported touching guide tips with a moving collider, an unseen pair through the production joint driver, a low-strain collision limit requiring a reaction before publication, physical time gradients at 1e-8 through 1, and safe individual iteration segments whose overall chord crosses another guide. Final validation: 307 library tests passed (34 ignored), 20 hair integration tests passed, and diff/runner shell checks passed.

Stricter full-density runs exposed earlier gaps: `unified-contact-capture` rejected imported self-penetration on frame 1; `continuous-init-capture` exposed and retained a subsequently repaired invalid zero-duration sampling attempt; `continuous-initial-state-capture` and `complete-swept-pipeline-capture` failed frame 1 under the stronger continuous/force pipeline. `blocking-witness-capture` completed frame 1 with both native copies identical, then was explicitly stopped while frame 2 remained live because its binary predates the final original-clock fix. The stop is not a mathematical failure or complete trajectory result.

The runner now enables swept structural motion and force-balanced friction recovery and captures root-motion failures in addition to the original diagnostic exports. `original-clock-capture` is the current full-density native qualification of the final clock change; its outcome must be read from its live process/log, not inferred from the unit suite. Complete jump, rendered secondary-physics capture, GPU/CUDA qualification and >160 rendered FPS remain unproved.

### Immutable active-set QR prefix reuse

Live original-clock CPU sampling identified equality QR factorization as the largest sampled runnable stack. The unilateral solver now retains the unchanged sorted active prefix within one immutable operator, rebuilding from the first changed column. Bounds, triangular solves, residual refinement and original physical admission remain unchanged. A before-change reference verifies bitwise state and reaction equality through insertion, removal and changed bounds with nearly parallel columns. Release library checks: 308 passed, 34 ignored. This is not a measured whole-frame speedup. The already-running original-clock binary predates this optimization; its results cannot qualify the new binary. Full jump secondary-physics capture and >160 rendered FPS remain unproved.

### Captured physical contact comparison and rerun

VQC1 input /tmp/voxy-consistent-contact-next.bin contains 100 rows and 882 whitened coordinates. Seven alternating-order original/cached comparisons were bitwise identical for coordinates and multipliers. Median QR times: original 1.647083 ms, cached 1.613375 ms (2.047% reduction). This selected task is not whole-frame performance. Original physical load/force admission replay also passes. Final checks: 308 library tests passed (35 ignored), 20 hair integrations passed. The old live original-clock run was explicitly stopped because its solver was superseded, not because an observation timed out. New full-density 720-frame CPU paired qualification is live under session 49931, qr-prefix-capture/qualification.log; inspect its live process before continuing. Complete jump, GPU physics and >160 rendered FPS remain unproved.

### Exact capsule candidate hierarchy

Live CPU profile of the QR-prefix binary now ranks swept_capsule_pairs above QR solve. Replaced one-axis sweep enumeration with a balanced geometry-only hierarchy over unchanged padded bounds. Includes touching bounds, keeps every overlapping leaf pair, and sorts original identities. Before-change sweep reference matches dense, sparse and degenerate bounds. Stable full-groom VJR1 snapshot capsule-bvh-motion.vjr: 469 guides, 9380 segments, 13312 exact identical candidate pairs in all seven alternating comparisons. Median original enumeration 4.873 ms, hierarchy 2.387458 ms (51.00% reduction); excludes bounds construction and narrow-phase CCD, and is not frame/FPS evidence. 309 library checks pass (36 ignored), 20 hair integrations pass. Old live session 49931 was stopped explicitly because the binary was superseded; full native trajectory rerun uses a fresh capsule-bvh-capture directory.

### Scale-space admission counterexample

The stable capsule-bvh-motion.vjr proposed increment does not converge under the 64-step component-clock procedure (captured_joint_root_motion_admission FAIL). Explicit full original-clock queries admit zero free increment and positive uniform scale 1e-6 and 1e-5; reject 1e-3 and larger selected scales. Thus a positive checked path exists for this captured proposal; the failure is not proof of infeasible prescribed roots. The component procedure ends with pair (286,344), safe-time fraction 0.9999324226546666, still unresolved. Diagnostic-only VOXY_HAIR_SWEEP_COMPONENT_TRACE records each component reduction and fixture scale probes. A restricted-group zero/bracket experiment failed to resolve this input and was removed. Retain full continuous admission and investigate newly connected neighbors when proposing component scales; do not interpret temporal CCD fractions as affine free-increment fractions when staged roots move. Existing live session 4824 still runs unchanged physical behavior; diagnostic-only edits do not require restarting it. 309 library tests pass (36 ignored); full jump and >160 FPS still unproved.

### Checked scale-space correction with new-neighbor merging

The former positive-motion counterexample capsule-bvh-motion.vjr now passes captured_joint_root_motion_admission (0.11 s selected replay; old replay failed in 0.33 s). The controller tries checked free-increment backtracking before its existing conservative fallback; roots/staged poses and physical time remain fixed. Every candidate is admitted by unchanged complete strand_limits. A newly colliding neighbor joins the restricted graph and the same scale is requeried before shrinking it. A four-guide regression proves dynamic neighbor inclusion, exact roots, and an unchanged independent-guide scale of 1. No zero scale, unchecked endpoint or tolerance slack is published. Budget exhaustion does not admit a candidate. 310 library checks pass (36 ignored), 20 hair integrations pass. Old live session 4824 was explicitly superseded, and the fresh scale-space-capture directory runs the full 720-frame native qualification. A positive safe prefix is not complete nonlinear force equilibrium, full jump, GPU physics or >160 FPS.

### Exact zero-coordinate QR reduction

Live scale-space profile again ranks EqualityQr::solve first among runnable sampled stacks; the existing rod-island partition is already present. Original captured VQC1 has 100 rows, width 882, only 639 coordinates with any nonzero load. The native inequality path now solves exactly that nonzero coordinate subspace and restores the full coordinates before original force/load and physical residual admission. No magnitude threshold; tiny 1e-10 directions remain. All-zero load feasibility/rejection is explicitly tested. Seven alternating comparisons against the pre-prefix uncached original produce bitwise identical state and reactions; selected medians 1.542292 ms original and 1.154042 ms compact+prefix, approximately 25.17% reduction, with noisy outliers retained in the log. This is selected QR-task timing, not total frame time or FPS. Original physical load replay passes, 311 library tests pass (36 ignored), 20 hair integrations pass. The live full trajectory session 90348 completed paired frame 1 with zero coordinate/quaternion difference and remains running frame 2. It uses the earlier scale-space binary, not this new exact-subspace optimization; retain that useful trajectory run and qualify the compact binary separately after its terminal outcome. Full 720-frame jump, complete rendered physics, CUDA and >160 FPS remain unproved.

### Verified frame-two wait

Session 90348 and child 60907 were both revalidated live after several minutes of CPU work; the last completed paired frame remains 1. A new five-second sampling profile, scale-space-frame2-cpu-sample.txt, again places EqualityQr::solve first among runnable stacks (200 samples), not the broad-phase query. No terminal error or exported failure checkpoint exists yet. Preserve this trajectory process and re-poll the same handle. Observation timeout is not failure; exact-subspace binary qualification is still pending independently.

### Immutable coordinate map across numerical refinements

NonzeroCoordinates now owns one compact matrix, or borrows full-width columns when no reduction exists. Joint native load solve constructs it once before the eight bounded numerical defect refinements; only bounds change between trials. Reconstruction and original physical load/force/residual gates remain in their original owner. Changed-bound reuse matches the pre-optimization uncached unilateral reference, including tiny directions and released rows. 312 library checks pass (36 ignored), 20 integrations pass, captured original physical-load replay passes. No new timing claim follows. Session 90348 was re-polled live; paired frame 1 remains its latest completed frame. It still uses the earlier scale-space binary. Full trajectory, new compact-map binary trajectory qualification, GPU/CUDA physics and >160 FPS remain unproved.

### Current compact-map full-trajectory run and measured tasks

Started fresh immutable-coordinates-capture full-density 720-frame native paired run, session 69803, while retaining the verified-live older scale-space session 90348 (last completed frame 1). Opt-in VOXY_HAIR_QR_PROFILE records calls >=10 ms, their system/row/full/compact widths and refinement index. These are whitened coordinate admission timings, not original physical residual success, frame times or FPS. Observed latest-binary groups include 14 rods/68 rows, full1764 compact324,16.799625ms; 17 rods/47 rows, full2142 compact987,12.770417ms. Subsequent calls include 22-26ms. Even individual calls exceed the 6.25ms total-frame budget for 160FPS. 312 library checks pass (36 ignored), shell syntax and diff checks pass. New run has no completed paired frame at this observation and remains live. Both trajectories and real GPU/CUDA/rendered performance remain pending.

### Current-jump slow operator preserved and replayed

Opt-in one-shot VOXY_HAIR_QR_PROFILE_INPUT_EXPORT saves the first >=10ms operator without replacing an existing file; VQC1 contains original and effective bounds, all original physical matrices/loads and full whitened columns. Live capture slow-contact-capture/slow-contact-input.vqc has 17 systems, 46 rows, 2142 full coordinates, 987 nonzero coordinates. Captured original physical load/force/gap replay PASS. Seven original/optimized comparisons are bitwise identical for coordinates and reactions. Timing spans 29-119ms original and 11-166ms optimized under concurrent full-trajectory CPU workloads: noisy task measurements, not a stable FPS or speedup claim. Capture-purpose session33610 was explicitly stopped after capture+verification, not on observation timeout. Full trajectory sessions90348 and69803 remain independently live. 312 unit checks pass (36 ignored), diff and runner syntax checks pass. No complete jump, CUDA/GPU physics or >160FPS claim.

### Measured active-factor rebuild work

Opt-in VOXY_HAIR_QR_ITERATION_TRACE reports accepted unilateral iteration count, final active cardinality and total basis columns recomputed. The actual slow-contact VQC1 task (46 rows, 987 compact coordinates) deterministically needs 32 iterations, ends at 21 active constraints, and recomputes 259 QR basis columns in all seven runs. Original/optimized results remain bitwise identical. This identifies sorted-active suffix rebuild work as a concrete optimization target; it does not prove an alternate ordering is physically admissible or faster. Current active ordering is unchanged. Full trajectory sessions 90348 and 69803 remain pending.

### Append-active ordering experiment (test only)

Production sorted ordering is unchanged. Test-only appended_active_experiment uses the same QR, 512-iteration limit and original complementarity/nonnegative-reaction tolerance, appending new identities rather than resorting existing basis columns. Actual slow-contact VQC1: 32 iterations, 21 final active, 78 rebuilt basis columns instead of259; all seven comparisons pass unchanged whitened AND original per-system load/force/gap admission. Maximum coordinate difference 2.4649982254080225e-22. Selected medians sorted10.886792ms/appended5.5735ms, about48.8% lower task time; not frame or FPS evidence. Second archived 100-row VQC1 was captured at whitening refinement4 (original/effective bounds differ by up to2.625026931935004e-13, tolerance1e-14). The new one-shot physical harness fails on the original sorted result before testing appended physics, original row2 gap -2.9436045614894546e-13. This is not causal evidence of an append-order defect; it demonstrates the harness must replay the canonical bounded original physical defect-correction owner before promotion. Preserve failure log and do not loosen its gate. 312 ordinary unit checks pass (37 ignored). Full trajectory runs remain pending; appended ordering is not promoted.

### Full original physical refinement ownership audit

The native response implementation now has one shared private physical owner parameterized by coordinate solve; its public default still invokes sorted QR. Test-only appended QR passes through the same whitening, eight original-bounds defect refinements, force/load balance and nonnegative complementarity admission. Slow current-jump 46-row input passes both variants through this full owner. Archived 100-row/refinement4 input passes sorted owner but rejects append=true with joint square-root active contacts did not converge. The labelled log makes this causal distinction explicit, replacing the earlier one-shot harness limitation. Thus changing production ordering is not qualified by the first input speedup; keep the experiment test-only, preserve rejection and do not loosen tolerance. 312 ordinary unit checks pass (37 ignored); 20 hair integrations pass. Alternate QR conditioning/active-set handling or a fully validated original-owner fallback is needed before promotion. Full trajectory/GPU/CUDA/>160FPS requirements remain open.

### Guarded append ordering in the native physical owner

Native joint-load solving now first attempts append-active ordering through the entire unchanged physical owner. Any rejected coordinate/force/bound/refinement attempt restarts sorted ordering from immutable requests and ORIGINAL bounds; no failed candidate is published and no gate is relaxed. Both orderings share a single QR active-set implementation (ordering flag) and a single physical owner, removing the test-only duplicate driver. Full 100-row/refinement4 replay visibly triggers sorted retry and passes original physical loads/gaps; full current-jump 46-row replay passes too. 312 library tests pass (37 ignored),20 hair integrations pass. Existing timing reports measure the raw QR variants, not guarded fallback cost or entire frames. Verified-live compact-map child68765/session69803 was explicitly superseded after this material strategy change. Fresh guarded-order-capture runs full720-frame native trajectory; older session90348 was not stopped. Full trajectory, real GPU/CUDA physics and>160FPS remain unproved.

### Prepared-operator reuse for sorted retry

Native guarded ordering now prepares factors, full whitened columns and the exact nonzero map once per immutable request set. Both attempts share the same physical refinement owner and read-only factors/columns; each starts fresh effective bounds from ORIGINAL input bounds. Rejected append responses/duals are not reused. Current-jump46-row full owner and100-row numerical-refinement fallback replays pass. 312 unit checks pass (37 ignored),20 integrations pass. No new measured timing claim. Existing guarded trajectory session12057 is revalidated live, completed paired frames=1, logged sorted retries=0; this binary predates prepared-retry reuse and must not prove its timing. Retain it for trajectory outcome; old scale-space session90348 also remains unmodified. Full720-frame jump, rendered physics, GPU/CUDA and>160FPS remain pending.

### Skip counterexample samples only after continuous Clear proof

Latest guarded-frame2-cpu-sample shows capsule sweep/distance work together exceeds QR in sampled runnable stacks. Swept witness activation previously sampled 32 times even when every permitted region had CapsuleSweep::Clear. It now skips those certified-clear pairs; unresolved/initial-contact/iteration-limit regions retain original discovery samples and continuous admission. Both root-corner trim regions must be Clear before skipping a root pair. A before-change reference on full469-guide/9380-segment VJR1 gives bitwise identical original constraint bounds/Jacobian entries AND complete response-group identities, normals, barycentric coordinates and impulses. Selected three alternating measurements: original10.703584,10.006917,10.415542ms; optimized5.62325,5.495666,5.38225ms. This is roughly47% selected witness-task reduction, not frame/FPS proof. 312 library checks pass (38 ignored),20 hair integrations pass. Current session12057 remains live on the prior binary and is retained for its full trajectory outcome. Updated-binary full trajectory, rendered physics, GPU/CUDA and>160FPS remain unproved.

### Verified continued trajectory and CPU parallelism feasibility

Both session12057/child85321 and session90348/child60907 were revalidated live. Older scale-space trajectory completed paired frame2 with zero position/quaternion discrepancy; newer guarded trajectory latest completed frame remains1. Neither is a completed720-frame jump. Existing native square-root projection partitions rods into independent islands but solves these islands serially; existing structural/skin owners already use scoped threads, and physics has no Rayon dependency. Parallel admission would need immutable free-pose inputs, ordered reaction publication and serialized failure exports, not independent writes to public rod state. No parallel-solver change or performance claim is made at this observation. Preserve running processes rather than restart on observation timeout.

### Parallel immutable native contact islands

Large native square-root projections now solve independent rod islands using available CPU workers and an atomic task index. Workers read one immutable free pose/operator; results/errors are reordered by original island identity, and poses/reactions accumulate in the original deterministic order only after successful batch solve. Small work and single islands remain serial. Diagnostic VQC/VQI writers serialize file writes. Ownership now tests exact nonzero gradients rather than squared norms, so a nonzero1e-200 Jacobian cannot underflow into an independent owner. The existing300-island fixture compares full linear/angular coordinates and multipliers BITWISE for1,2,4 workers; exact roots and original residual gates pass. A valid-island plus513-row rejected-island regression preserves all pre-existing reaction sentinels on error. 314 library checks pass(38 ignored),20 hair integrations pass. This proves scheduling parity on these cases, not a frame speedup or>160FPS. The prior live guarded process was explicitly superseded. Fresh island-workers-capture runs the full720-frame trajectory; older control90348 is retained. Complete jump, render/GPU/CUDA and real-time targets remain pending.

### Observed workers and balanced pair-tree traversal

Session45514/child99395 is confirmed live. Its five-second profile shows scoped island-worker stacks executing; scalar geometry broad-phase/sweep work now dominates sampled runnable tops. Added balanced dual-tree splitting by subtree size, avoiding unconditional descent of the first node to leaves. Original padded bounds and final sorted IDs remain unchanged. Full9380-segment VJR1 yields exactly the original13312 pairs; seven alternating comparisons against the previous one-sided hierarchy have medians2.379417ms versus2.297708ms (about3.4% selected query reduction, with noise and two slower pairs). This is modest query timing, not whole-frame/FPS evidence. 314 unit checks pass(38 ignored),20 integrations pass. Preserve existing island-workers trajectory; it predates this traversal-only optimization. A larger remaining opportunity is local candidate reuse between checked component reductions with exact bound-containment validation; no such cache is implemented or claimed here. Full jump/GPU/CUDA/>160FPS remain pending.

### Candidate cache experiment

The captured eight-scale query sequence rebuilt the envelope on all eight queries. Exact candidate lists matched fresh queries, but timings were noisy and no speedup was established. Removed cache use from the production contact path; kept the cache experiment under cfg(test). After removal: 315 unit tests passed, 38 ignored. Hair integration suite: 20 passed. Full jump qualification remains running; neither complete secondary-physics trajectory nor >160 rendered FPS is established by these checks.

### Seeded conservative candidate envelope

Subsequent experiment seeds the local envelope with both staged (zero free correction) and initial trusted correction endpoints. Every actual query validates exact current padded bounds, checks containment, filters with original current AABB overlap, and rebuilds on enclosure failure. No narrow-phase or final independent whole-trajectory admission changed. Captured 9380-segment eight-scale queries: fresh/cached seconds (0.018299917,0.003499333), (0.017374792,0.003413208), (0.018031958,0.003403792); exact pair lists matched and cache built once each. Enabled in component scale search only. 316 unit tests passed / 38 ignored; 20 hair integration tests passed. Full running qualifier predates this cache; it has reached paired frame 3 with zero position/quaternion difference, but complete 720-frame trajectory and rendered FPS remain unverified.

### Seeded cache physical-clock qualification

Captured VJR1 full groom replay compared complete cached vs fresh narrow-phase limits at eight scales (1, 0.5, 0.25, 0.125, 0.0625, 0.03125, 0.00001, 0), including identical rejection outcomes. The test also runs component scale search and independently admits the resulting complete trajectory with a fresh query and checks pinned roots. Passed in 0.21 s. New full 720-frame native paired qualifier launched in seeded-envelope-capture; live session 30653. Full completion, GPU/CUDA execution, and rendered FPS remain unproven. Previous goal turn was progress: production conservative cache plus exact regression and benchmark evidence.

The current full qualifier reached runtime (cargo 13754 / test 14417 confirmed live). Earlier island-workers (99395) and scale-space (60907) runs were explicitly terminated as superseded by the current production changes, not because of observation timeout or physical failure. Their logs remain preserved; neither was complete. New full qualifier remains running under session 30653.

### Immutable QR column validity

Three-second live CPU sample of test PID 14417 retained in seeded-envelope-live-cpu-sample.txt. Dominant computational stacks include continuous sweep, segment_pair, EqualityQr::solve, and triangular solves; sampling is not frame timing. EqualityQr now caches per-column shape/finiteness validity once for its immutably borrowed operator. Active IDs retain lazy rejection (invalid inactive columns are not rejected prematurely); bounds, tolerance, active shape and IDs still checked on every solve. Arithmetic, QR factors, reactions and physical gates unchanged. 317 unit tests passed / 38 ignored, including invalid inactive/selected/missing identity regression and existing bitwise cached/reference tests. Both original physical owner captured fixtures passed, including mandatory sorted retry case. No isolated timing gain claimed for this metadata change. Full seeded-envelope trajectory session 30653 confirmed live; runtime predates this QR metadata-only change and has completed paired frame 1 with zero differences. Complete trajectory and rendered FPS remain unverified.

### Ordered parallel continuous strand queries

Independent immutable candidate-pair narrow-phase queries now run in scoped native threads for >=1024 candidates, capped at available parallelism / 8. Smaller tasks remain serial; tracing remains serial by default. Results are joined in canonical input order, preserving first-error selection and every physical fraction; no narrowed clearance, root exclusion, density, sweep budget, or terminal-state semantics. Normal ordering/first-rejection regression passed; 318 unit tests passed / 38 ignored; hair integration 20 passed. Captured 469-guide VJR1 test compares full query results with 1/2/4/8 workers across 8 scales, then independently admits chosen whole trajectory and checks roots: passed. Measurements retain separate fresh serial, cached serial, cached parallel timings; these are selected query timings, not frame FPS. New full qualifier started under session 43834 in parallel-sweep-capture. Prior session 30653 remains live pending new runtime readiness; neither trajectory completed.

### Read-only witness stage and parallel experiment

Generalized the ordered query executor for immutable query results. Witness extraction now collects every successful geometry result before canonical cut publication and original constraint rebuild; failures do not publish partial cuts. Captured regression compares full constraint bounds/Jacobian and response-group bits against the original pre-Clear-skip reference, current serial and current parallel: exact match. Original/current-serial/current-parallel seconds: (0.011784666,0.00573125,0.005470292), (0.011664458,0.00534075,0.006038417), (0.01025275,0.005666333,0.003645792). No stable isolated parallel witness speedup established, so production witness stage stays serial; explicit parallel counts are used for qualification only. Continuous strand-limit queries retain their independently measured parallel production path. 318 units passed / 38 ignored and 20 integrations passed before the final serial-default change; final check logged separately. Current full qualifier session 43834 / test 18979 confirmed live with frame 1 complete, zero differences. Seeded-only session 30653 / test 14417 explicitly terminated as superseded by parallel continuous strand-limit queries; its evidence preserved. No full trajectory or rendered FPS qualification yet.

Final serial-default unit check passed: 318 passed / 38 ignored / 0 failed.

### Per-frame solve wall-time visibility

Full model qualification now logs HYBRID HAIR FRAME TIMING for each completed native and external solve, with native_only/cpu_control labels. Timing excludes rendering and subsequent verification; it must not be presented as rendered FPS. Existing aggregate totals are unchanged. cargo check -p voxy_app --tests passed (existing warnings retained). Current running binary session 43834 / PID 18979 predates only witness staging and this test logging; no restart was made for instrumentation. Confirmed live and paired frames 1 and 2 completed with zero position/quaternion differences; frame 3 incomplete at observation. Complete trajectory, GPU contact end-to-end, CUDA, and 160+ rendered FPS remain unverified. Previous turn was progress: atomic read-only witness staging, exact serial/parallel/reference evidence, final 318 unit tests.

### Third-frame live profile

Session 43834 and PID 18979 confirmed live. Five-second CPU sample retained in parallel-sweep-third-frame-cpu-sample.txt. The highest computational collapsed stack count is EqualityQr::solve (1591), followed by continuous advance closure (971), segment_pair (960), and unilateral gap construction (731). Idle system/graphics and join waits are not evidence of query load imbalance. No dynamic scheduling change made without evidence. Paired frames 1 and 2 remain complete; third incomplete, with new QR profile entries continuing. Next investigation: existing immutable NonzeroCoordinates currently starts a new unilateral EqualityQr for each changed-bound defect solve; determine whether cached factors can survive those restarts while keeping fresh active state, original arithmetic order and full physical admission. This is an investigation, not an implemented optimization or trajectory/FPS proof.

### Defect-refinement reuse audit

Authoritative current profile audit retained in qr-refinement-profile-audit.json: 7938 calls selected by >=10 ms profiler, every recorded call at refinement 0; no selected refinement >0. Therefore cross-defect QR reuse is not supported as an acceleration for the currently measured slow calls. Summed call durations overlap across threads and are not elapsed trajectory time or FPS. Source also resets active state for every changed-bound solve; retaining only the final prefix would preserve at most a short common prefix on restart. No speculative factor-cache change introduced. Current full qualifier remains session 43834; retain same run rather than restarting on observation timeout. Next optimization investigation should target active-set QR and repeated gap evaluation within one solve, with exact current physical admission and captured reference comparison.

### Exact structural sparse gap evaluation

Unilateral active-set gap evaluation prepares strictly nonzero column supports once per immutable solve. No magnitude cutoff; 1e-200 coefficients remain. Original compensated/FMA product arithmetic and retained product order unchanged. Explicit finite-state gate prevents structural zeros from hiding 0*infinity rejection. QR basis construction and physical force/load admission remain unchanged. Test-only dense gap path shares the same solver; seven alternating captured compact operator runs matched coordinates/reactions bitwise, median dense 5.438625 ms vs sparse 3.969792 ms (~27%). This is a selected QR task, not frame speed. 319 unit tests passed / 38 ignored; both captured physical-owner cases passed, including guarded sorted fallback. Current full qualifier session 43834 remains running and predates this gap optimization; no full trajectory or rendered FPS claim.

Sparse-gap final hair integration: 20 passed / 0 failed.

### Sparse gap arithmetic and full runtime

Added structural-zero arithmetic regression: catastrophic cancellation, subnormal products, signed zeros, overflow/NaN rejection and 1024 deterministic finite-input bit-pattern arrays compare original dense and supported compensated dot bitwise (NaN class for rejected overflow). 320 units passed / 38 ignored. New full 720-frame qualifier session 47344 / cargo 30672 / test 31251 confirmed live with sparse gaps, serial atomic witness staging, parallel continuous queries and frame timing. Old session 43834 / test 18979 explicitly terminated as superseded by the measured sparse-gap change; not a timeout/failure, log preserved. Earlier run reached 3 paired frames with zero difference and was incomplete. Full current trajectory and 160+ rendered FPS remain unverified.

### First measured full hair frame cost

Current qualifier logged paired frame 1 at native 15568.834541 ms and paired native external 13852.147 ms, with zero position/quaternion differences. This is CPU-only hair physics with diagnostic profiling enabled, excludes rendering/verification and may include concurrent test/build contention. It nevertheless does not establish anything near the 6.25 ms total frame budget for 160 FPS. 1311 selected slow QR calls observed by snapshot (not all confined to first frame). Machine-readable observation retained in sparse-gap-capture/timing-observation.json. Existing swept refinement trace enabled in runner for future full launches to distinguish repeated cut activation / projection from inner QR work. Current session 47344 stays running; no restart for diagnostic flag change. Original full engine/physics/hardware goal remains incomplete.

### GPU boundary and growing contact cost

Current CPU paired frame 2 logged native 55472.587833 ms / external 54947.82675 ms, zero position/quaternion difference. Cost grows after frame 1, so microbenchmark improvements do not qualify real-time dynamics. Source contact_square_root_projection.rs::solve_island_increment invokes HairResponseSystem::solve_joint_load_inequalities_native; GPU response dispatch exists in gpu_hair_response_dispatch.rs but the coupled active-set inequalities remain native. Current real-GPU captured contact response test launched under session 97756 using four rod121-contact-matrices.json cases. Its scope is linear response shape/load validation and native-relative error, not coupled full trajectory or FPS. Full CPU qualifier session 47344 remains live.

Real GPU captured contact-response test completed successfully; adapter, response count and max relative error retained in current-gpu-contact-responses.json. This does not validate native coupled active-set replacement or complete animation/FPS.

### Outer contact-solve cost diagnostic

With existing VOXY_HAIR_SWEEP_REFINEMENT_TRACE, complete strand advance now logs total wall time, initial projection time, initial component admission time, refinement count, cuts, constraints, groups and minimum admitted scale before pose publication. Disabled by default; physical values, budgets and admission unchanged. cargo check -p physics passed; focused safe_motion suite with trace enabled: 26 passed / 2 ignored, actual profile output observed. This small suite does not qualify full groom cost or GPU/FPS. Full qualifier session 47344 stays live with its original binary; it predates the new diagnostic and is not restarted for logging.

### Verified third-frame wait

Session 47344 / PID 31251 confirmed live and polled without restart. Frame 3 completed with native_ms=107056.352375 / external_ms=101412.26437500001 and zero paired position/quaternion differences. Cost grows across first three frames; selected QR/candidate improvements do not prove real-time full dynamics. Current run advances beyond frame 3 and has no terminal result. This turn yields trajectory/timing evidence and a verified wait; no goal completion/blocking claim.

### Complete captured GPU response coverage

Strengthened gpu_captured_body_contact_responses to assert system counts, per-system load counts and per-load correction widths before zipped numerical comparison. Prevents missing/truncated GPU outputs from silently passing qualification. Rebuilt and reran on real Metal: passed, 4 systems / 8 responses, original validate_load_correction gates and <1e-8 relative-error gates unchanged. Log gpu-contact-complete-coverage.log retained. This is linear response coverage, not coupled active-set GPU or full trajectory/FPS parity. Full CPU qualifier session 47344 confirmed live during test; no restart.

Verified same-session wait: frame 4 completed, native 120380.138209 ms / paired external 119804.372708 ms, zero position/quaternion differences. Session 47344 remains live advancing frame 5; no restart. Complete 720-frame trajectory and real-time hardware performance remain unproven.

### Correction: coupled backend route distinction

Current velocity_contacts.rs::constrained_newton_increment explicitly branches on solver.is_none(). Native mode first uses square-root QR (with original-free legacy recovery if rejected); external linear solver mode runs response_batches::prepare_with_free through supplied backend then CPU solve_projection (Gram/PGS budget 4096). Earlier broad wording that coupled inequalities simply always use native QR was incomplete. Metal captured linear-response validation does not prove either coupled path equivalence or a GPU coupled active-set solve. Exact source-route audit retained in coupled-backend-path-audit.json. Native full qualifier session 47344 continues; no path/density/tolerance changed.

### Remove eager unused native Gram responses

Source audit found constrained_newton_increment always prepared free + all Gram response loads before canonical native QR rebuilt its independent square-root columns. Native successful QR path now prepares only elastic free loads (all rods, including independent guides); legacy Gram responses/compliances are prepared only after rejected QR before original-free legacy projection. External solver path unchanged. Strengthened closing-contact test compares original eager preparation + QR against new path and exact multiplier bits, checks independent elastic guide/root invariants, and asserts no unused responses were prepared on successful QR. 320 units passed / 38 ignored; 20 hair integrations passed. No measured whole-frame speedup yet. Current full qualifier session 47344 predates this optimization and remains running; frame 5 completed zero differences, native_ms=145816.849375 external_ms=145962.99687499998, so real-time performance still unachieved.

### Lazy native compliance full runtime

New full qualifier session 56003 / cargo 45795 / test 46354 confirmed live. Old session 47344 / test 31251 explicitly terminated as superseded by removal of eager Gram compliance preparation; five completed paired frames retained, not a complete trajectory. Current outer solve profiles show 6-8 cut refinements and roughly 217-311 constraints in selected transactions, ~257-392 ms total vs ~12.4-12.5 ms initial projection and ~19.6-24.5 ms initial admission. These selected transaction observations show substantial refinement work after the first solve; not frame timing or a whole-frame improvement claim. Same constraints, physical tolerances, density and budgets retained. Current observation stores real profile rows. Full 720 frames and 160 FPS remain unproven.

### First lazy-compliance frame cost audit

First complete paired frame passed with zero differences: native 12249.602958000001 ms / external native 12085.081041000001 ms. Not a controlled before/after benchmark (different profiling and concurrent compilation/run conditions). First-frame cost audit, restricted to log before frame-1 timing marker, records 76 completed contact transactions across both CPU copies, 556 cut refinements, maximum 311 constraints; summed transaction wall time 23051.345626 ms, initial projection 956.68242 ms and initial admission 1741.479251 ms. This identifies repeated cut/projection work as a major measured cost; does not establish causes in outer call ownership yet. Machine-readable first-frame-cost-audit.json retained. Current session 56003 stays live; no 720-frame or 160-FPS proof.

### Outer call ownership investigation

Current mod.rs structural loop invokes advance_swept_strands then refreshes mesh geometry and invokes reconcile_positions_mode. In continuous_mesh && self_collision mode that nonlinear reconciliation loops up to 128 times and itself invokes advance_swept_strands, including another full elastic free Newton computation and witness-cut refinement. This explains why the measured first paired frame has more transactions than nominal structural iterations; it does not establish any of these calls as safely removable. Pose-dependent operators, mesh recontact and original whole-interval admission remain necessary. Audit retained in outer-contact-call-ownership-audit.json. Next design work requires separate structural-step/geometry-correction ownership and behavioral qualification rather than reducing budgets, density or tolerance. No speculative physical path change made; session 56003 remains running.

Current lazy native compliance qualifier reached frame 3: native 89597.422917 ms / paired external 95946.018541 ms, zero position/quaternion differences. Same session 56003 confirmed live and continues frame 4; no restart. Three-frame prefix is not full jump qualification or rendered FPS.

Same-process verified wait completed lazy-compliance frame 4: native_ms=129861.74075 external_ms=136083.49312499998, zero position/quaternion differences. Timing is worse than earlier sparse-only selected frame-4 observation; conditions were not controlled, so no general full-frame performance gain is established. Current run starts frame 5; latest refinement trace observed global cuts=529 / constraints=1132 at refinement 4, not per-island capacity. Full qualifier session 56003 still live, no restart.

### Checkpoint scope audit

Inspected contact_replay.rs and current HairRod/HairSystem fields. VHR1/VHR2 save positions, orientations, velocities, material and mesh geometry, but are diagnostic snapshots from a nonlinear correction and use a test-only reader. They omit system settings/initialized admission state, frame/time/rig state, contacts and normal/surface velocity accumulators, spatial warm-start caches that require tie-behavior qualification. They are not qualified full simulation resume checkpoints. Audit checkpoint-format-audit.json retained. Current full session 56003 remains live; no restart/checkpoint claim.

Verified same-process wait reached lazy-compliance frame 5: native_ms=189461.518 external_ms=178761.386042, zero position/quaternion differences. Current session 56003 remains live advancing frame 6. Cost growth persists; no whole-frame speedup/FPS qualification established.

### Finite segment feature certificates

The lazy-native-compliance full qualifier failed at native frame 7 after six
identical paired CPU frames. The captured geometry replay reproduced the
failure with zero admitted penetration merit: a full Newton step remained
blocked by continuous strand admission. The blocking pair was rod 20 segment
19 / rod 27 segment 17. Its captured small-increment path exhausted conservative
advancement after 1024 iterations at time 0.09170332360632288; sampling was used
only to locate the feature, never to admit motion.

The continuous certificate now combines outward-rounded supporting-line and
finite-endpoint separating-feature polynomials over a partition of the complete
time interval. Every accepted interval must satisfy one complete sufficient
certificate, including the endpoint inward-side condition. Unknown, nonfinite,
and exhausted certificates remain rejection. The original line-only certificate
is tried first. Original radii and admission tolerance are unchanged.

The captured pair now admits its whole interval through the sufficient proof.
The captured full 469-rod geometry replay converged in 14.58 seconds with pinned
roots and the original 5-percent strain gate. This is a geometry replay, not a
resumable full animation checkpoint or rendered FPS qualification.

Final physics units: 322 passed / 38 ignored. Evidence is under
`lazy-native-compliance-capture/feature-certificate-final-units.log` and
`frame7-feature-certificate-replay.log`. A new 720-frame paired native CPU run
was launched in `finite-feature-certificate-capture`; its completion is pending.

### Exact sparse QR residual evaluation

The immutable QR operator now retains exact nonzero coordinate indices for
original-column residual refinement. It skips only coefficients equal to zero,
keeps original product order and compensated/FMA accumulation, and explicitly
rejects a nonfinite state before sparse products. Q orthogonalization, active
ordering, tolerances, force admission, and original bounds are unchanged.
Unused copies of all existing constraint bounds were also removed from swept
cut assembly; no caller consumed the former change flag.

Validation of the latest physics source: 322 unit tests passed / 38 ignored,
20 hair integration tests passed, and both captured 46/100-row QR fixtures
matched original coordinates/reactions bitwise and passed original physical
admission. Logs are in `finite-feature-certificate-capture/qr-residual-*`.
Isolated residual timings (100 evaluations, seven alternating paired repeats):
46 rows median 5.949 ms dense / 0.46825 ms sparse; 100 rows 8.118708 ms /
2.344458 ms. Support construction is excluded. These are residual evaluation
measurements, not complete QR, full physics frame, GPU, or rendered FPS proof.

The live 720-frame run in `finite-feature-certificate-capture` was compiled
before sparse residual/cut-copy changes. Its results qualify the finite feature
certificate trajectory only; they cannot measure the latest QR optimization.
Three completed paired frames matched exactly at the time of this entry.

### Frozen-pose native free Newton step

`NativeNewtonStep` borrows one immutable rod pose and stores its timestep and
original free increment for a single swept cut-refinement transaction. Each
native contact projection starts from a clone of that original free increment;
contact rows and reactions are still solved afresh. The context is dropped
before publishing a pose. External backend preparation is unchanged. The
legacy recovery still starts from original free and prepares its own compliances.

Latest validation: 322 physics units / 38 ignored and 20 hair integration tests
passed. The strengthened closing-contact fixture compares coordinate and reaction
bits against fresh preparation under three different row bounds, retains pinned
roots, and checks the unconstrained guide. The historical 469-rod frame-7 geometry
replay converged in 9.50 seconds; this single observation is not a controlled
whole-frame performance comparison. Logs: `frozen-native-free-final-units.log`,
`frozen-native-free-hair-integration.log`, `frozen-native-free-frame7-replay.log`
in `finite-feature-certificate-capture`.

The running full-animation binary predates this optimization. Six paired native
CPU frames had completed with identical coordinates at the time of this entry;
frame 7 and the remaining 720-frame qualification are pending.

## Contiguous QR coordinate reconstruction (2026-10-10)
Production EqualityQr now reconstructs Q*z by walking columns contiguously, with per-coordinate Neumaier/FMA accumulators. Each coordinate retains identical product and correction order; no sparsity threshold or tolerance change. The captured-layout experiment includes accumulator allocations and passed bitwise comparisons on 46x2142 and 100x882 inputs (isolated 100-evaluation timings only). Full QR fixtures also match uncached reference coordinates/reactions bitwise; original physical admission tests pass on both. Physics units: 322 passed, 39 ignored. Hair integration session9867 completed successfully: 20 passed. Full animation session66222 remains on the earlier frozen-native-step executable; its timings do not measure this production change. No rendered FPS qualification. Logs: frozen-native-step-capture/column-layout-*.log.

Full qualification now runs the contiguous-coordinate production source in session38935, PID39009, contiguous-qr-capture/qualification.log, 720 frames with unchanged full density/contact/CCD flags. New process confirmed computing swept refinements before old PID29392 was terminated. Old session66222 terminal exit101 by intentional SIGTERM, not a solver failure; four paired frames and replacement reason retained in frozen-native-step-capture/superseded-run.json. Poll38935, do not restart on observation timeout. Full completion/GPU/FPS remain unproved.

## Shared immutable QR support map
Removed duplicate nonzero-coordinate map construction from unilateral_supported: gap evaluation now reads EqualityQr.supports, already used for original-column residuals. Same exact !=0 rule, coefficient order, compensated/FMA products, finite-state rejection and bounds. 323 units passed, 39 ignored; both captured full QR reference comparisons passed. Current live full session38935/PID39009 still uses the preceding contiguous-coordinate executable (frames1-2 paired differences zero); its measurements do not include this support-map allocation removal. No new full-run restart for this isolated allocation edit at this checkpoint.

## Lazy frozen-pose structural matrix ownership
NativeNewtonStep now owns FrozenSystems borrowing its exact immutable rod slice and timestep. OnceLock lazily assembles each touched rod's original response system once per cut-refinement transaction. Every solve clones the system and constructs fresh contact loads; original matrix scaling, factors, QR, physical/reaction/CCD admission and fresh native fallback are unchanged. This caches assembly only, not factorization, contacts or subsequent mutable poses. Existing coupled closing-contact test checks bitwise coordinates/reactions against fresh preparation over three changed bounds and asserts exactly two touched matrices assembled once (free third guide remains unaffected). 323 unit tests and 20 hair integrations pass. Historical 469-rod frame7 geometry replay passes roots/5% strain admission in 9.37s; one observation under concurrent workload, not a controlled acceleration or animation checkpoint. Logs: contiguous-qr-capture/frozen-systems-*.log. Live full session38935/PID39009 remains the prior contiguous-QR executable, through three paired frames with zero differences; full verification of shared-support and FrozenSystems changes is still pending. No GPU or rendered FPS qualification.

Full production qualification now session58922/PID46034, frozen-systems-capture/qualification.log. Includes contiguous-coordinate QR, shared exact support, lazy frozen-pose structural matrices; unchanged720frames/full469x20guides/contact/CCD gates. New process confirmed computing before old39009 stopped with SIGTERM; old38935 terminalexit101, three completed paired frames retained in contiguous-qr-capture/superseded-run.json. This replacement follows actual validated production changes, not an observation timeout. Poll58922; do not poll38935. Full animation/GPU/rendered FPS not yet qualified.

## Exact sparse QR basis products
Production unilateral QR now stores exact nonzero support of each normalized Q basis column; prefix invalidation truncates matching supports. Orthogonalization products keep original coordinate order, Neumaier/FMA arithmetic and both MGS passes; full-coordinate FMA updates remain unchanged. Fresh finite-state rejection after each update prevents hiding zero*infinity. Direct standalone equality path remains dense. Captured full unilateral QR dense/sparse experiments: 7 alternating pairs, median speedups 1.35786x (46 rows) and 1.05258x (100 rows), coordinates/reactions bitwise equal. Support construction and finite checks included. Measurements do not cover full physics/GPU/rendered FPS. 323 units passed39ignored, strengthened cached-prefix test covers both basis modes; physical admission fixtures46/100 passed;20hair integrations passed. Logs/measurements: frozen-systems-capture/sparse-basis-*. Live full session58922/PID46034 still uses the preceding frozen-matrix executable; two paired frames zero differences; verification of latest sparse basis on full animation is pending.

Sparse-basis production source also passed historical469rod frame7 geometry replay, preserving pinned roots and5%strain, in7.38s (one diagnostic observation, not controlled whole-animation/GPU/FPS proof). Full720 qualification now session90191/PID52315, sparse-basis-capture/qualification.log, unchangeddensity/CCD/contactflags. New process verified computing refinements before old46034 terminated; old58922 terminalexit101 by intentionalSIGTERM, three paired frames retained in frozen-systems-capture/superseded-run.json. Replacement follows validated production sparse-Q optimization, not timeout. Poll90191; all preceding full handles terminal. Full720/rendered160FPS/hardware coverage remain unproved.
