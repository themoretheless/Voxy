# Hair mechanics

The `female` and `female_animation` examples use `physics::hair`, a geometry-independent
CPU Cosserat rod solver. The previous `physics::strand` showcase remains available
for grass and simple strands. Hair uses segment material frames, not an attraction
of every particle toward a prescribed world-space hairstyle.

Each guide has centerline positions, linear velocities, segment quaternions and
angular velocities. The root position follows its actual animated scalp vertex;
the root material frame follows the head joint. The initial groom is sampled from
the scalp, resampled by arc length, and moved outside the anatomical surface before
its intrinsic curvature is recorded. Two unforced initialization steps resolve
initial contact overlap before presentation. The authoring ellipsoid is not used
for runtime contact.

## Constitutive model and solve

`HairMaterial` uses metres, kilograms, seconds and pascals. Circular-section area is
`A = pi r^2`, bending rigidity is `EI = E pi r^4 / 4`, and twisting rigidity is
`GJ = EI / (1 + nu)`. Mass comes from density, area and segment length. The default
80 micrometre diameter, 1300 kg/m^3 density and 4 GPa Young modulus are configurable
example parameters, not measurements of the source mannequin's hair.

The stretch/shear energy of an edge is based on `x[i+1] - x[i] - l[i] d3[i]`,
with longitudinal stiffness `EA/l` and transverse stiffness `GA/l`. Bend/twist
energy uses the vector part of consecutive relative material-frame quaternions,
relative to their rest rotation. Its stiffnesses are `EI/l` and `GJ/l`.
The first position and first segment frame are clamped to the follicle.

The solver predicts inertial positions and rotations, then minimizes the implicit
elastic energy with a coupled Gauss-Newton solve. Its symmetric banded matrix is
solved with Cholesky factorization over the free coordinates. The six fixed root
coordinates and the final station's three unused frame coordinates remain
uncoupled identity rows and are excluded from factorization and substitution.
Quaternion corrections use exponential-map
increments, with bounded rotation updates. This is an implicit rod solve, not
XPBD: a local XPBD prototype failed the cantilever stiffness check and was replaced.
Transverse frame inertia is regularized; the system is not calibrated for measured
high-frequency axial torsional waves. Damping is exponential in elapsed time, and
linear aerodynamic drag uses air velocity relative to the guide.

The current anatomical groom has 469 guides of 20 segments, six nonlinear iterations and at
least two substeps. Input steps up to 50 ms automatically subdivide to at most
1/240 s; the full character regression validates both 30 Hz and 20 Hz inputs.
Independent rod/body-contact passes can run on several CPU workers. Capsule
self-contact runs in a deterministic shared pass after those workers join.
Workers perform all independent iterations up to the next shared contact pass
in one batch (four iterations, or the final partial batch). With self-contact
disabled, the entire iteration range is independent. Serial and parallel
positions and orientations agree exactly across full and partial batches.

## Contacts and limits

The actual animated body triangles are cached in a refitted BVH. Closest-surface
queries resolve particle penetration; segment/triangle queries detect contact
between particles. Capsule/capsule contacts include both different guides and
non-adjacent segments of one guide. A spatial hash provides the broad phase.
Near pinned follicles, the embedded boundary is excluded from segment projection;
contact corrections are bounded to avoid a vanishing lever arm generating an
arbitrarily large displacement. Active contact planes also participate in the
coupled elastic solve. A tangential velocity limiter provides friction-like
contact damping; it is not a calibrated moving-surface Coulomb friction solver.

Contact is discrete. Swept query bounds do not provide continuous time-of-impact
collision detection. Collider pose is sampled per input step, not continuously
interpolated through each internal substep. Fast body motion and incompatible
attachments can still exceed the contact/strain tolerance. This is guide-level
simulation, not a solve of all the individual fibers in a dense human hairstyle.
The groom has 60 render followers per guide. The renderer can generate the full
fibre geometry on the GPU without geometry readback; guide physics is a separate
owner. This does not establish a real-time GPU dynamics system or measured hair
scattering. CPU throughput is reported by the groom test separately from displayed
frame rates.

The formulation follows the material-frame rod model in
[Position and Orientation Based Cosserat Rods](https://diglib.eg.org/handle/10.2312/sca20161234).
The coupled approach addresses the stiffness convergence problem discussed in
[Direct Position-Based Solver for Stiff Rods](https://www.animation.rwth-aachen.de/media/papers/2018-CGF-Rods.pdf),
but does not reproduce that paper's XPBD solver.

## Validation and inspection

```sh
cargo test --release -p physics --test hair -- --nocapture
cargo test --release -p voxy_app --lib female_hair -- --nocapture
cargo run --release -p voxy_app --example female_animation
cargo run --release -p voxy_app --example hair_render
```

Mechanical tests cover rest equilibrium, density-derived mass, material-dependent
sag, root inertia, axial twist, contact between nodes, crossed guides, timestep
refinement, invalid inputs and serial/parallel equivalence. The small-deflection
cantilever test compares sag against `rho A g L^4 / (8 EI)`, accounting for the
clamped first segment. Asset tests check scalp bindings, finite state, guide strain
and fixed render topology on the actual procedural rig.

`hair_render` creates `/tmp/voxy-cosserat-hair.png`: a close-up at initialization and
after one simulated second. Both images come from the native scene renderer and
the same physical solver as the interactive examples.

On the 2026-10-01 local validation run, all eleven mechanical tests, three BVH query
tests and three anatomical-groom tests passed. The complete six-second rig loop
had maximum relative guide strain 0.003093 (0.31%). The beam test measured 0.1464 mm
sag versus a 0.1345 mm analytical reference (8.9% difference). Halving the internal
timestep changed the free-end position by 0.947 mm in the gravity-response test.
The groom tests reported 23.09 ms per 1/120 s input step and 47.83 ms per 1/60 s
step on this machine with parallel workers. These are solver/test CPU timings,
including surface refit and contacts, not displayed frame rates.

After correcting fringe arc lengths and batching independent iterations, the
six-second anatomical-groom regression measured maximum relative strain 0.002139
(0.214%) and 22.83 ms per 1/60 s input step. This later measurement includes surface
refit and contacts and does not establish a displayed frame rate or a controlled
speedup ratio against the earlier run.

`HairRod::rest_positions()` exposes the immutable stress-free authored curve.
`HairRod::rebased(points)` reconstructs segment frames, lengths, mass and inertia
with the existing material. It preserves node count and resets dynamic/contact
state. `HairSystem::rebased(curves)` stages every guide before returning a new
system and preserves iteration, worker, substep and contact settings. Any invalid
curve leaves the original system unchanged. This is a rebuild, not transfer of
velocity or stored twist. The body demo now reconstructs the canonical groom under the selected body
morph before creating rods; this rebuild recalculates physical lengths and mass.
Its collider/root offsets use the edited surface, and rendered fibres are not
morphed twice. Head poses use a conjugated local affine frame; root positions and
collider refits use the full nonlinear body morph. This is an approximation to
follicle orientations under anisotropic scaling, not full skeletal rebasing.
One 240 Hz step with height 182 cm/head size 1.2 and a Metal snapshot passed.
Long-run extreme-shape stability and total hairstyle mass remain unvalidated.

## Experimental accelerator qualification (2026-10-09)

Physics owns the original double-precision banded operator, fixed root degrees of
freedom, contact geometry, load columns and admission checks. The accelerator
implements `HairLinearSolver`; the renderer owns GPU transport and dispatch. A
`HairResponseSystem` holds one physical matrix and several loads, so contact
responses reuse one factorization without introducing another constitutive model.
GPU batching, compact readback and bit-preserving GPU response transport remain
opt-in. Transport equivalence is distinct from trajectory equivalence.

The joint position and velocity projections use the same rod compliance
`H = M + dt^2 K`. The experimental `recover_friction_pressure` option projects the
current free Newton increment under this operator and reads its absolute normal
reactions. The virtual increment is never applied to the physical pose. These
reactions replace accumulated numerical position corrections as the pressure
input to strand friction. The option requires joint position projection and is
preserved by groom rebasing. It is disabled by default.

The analytical paired-rod check matches a known inertial load, discards stale
correction history, releases separating contacts and retains tangential friction
at zero closing normal speed. It also checks unchanged poses and paired momentum.
These are mechanical fixture checks, not calibration against measured human hair.

On Apple M4 Max / Metal, the complete 469-guide candidate with follicle-preserving
groom, sampled collider motion, joint contact projections and force-balanced
pressure passed 60 input steps. Maximum position disagreement with native physics
was 4.6859518e-10 m. The 120-step run failed: first position disagreement above the
1e-6 m gate occurred at step 111, guide 121, node 13; maximum disagreement was
9.7932202e-5 m. Quaternion disagreement also exceeded its 5e-5 component gate.
Roots and stretch passed their separate checks. The full candidate is therefore
not admitted for default runtime use. A 120-step run covers one simulated second;
it does not establish six-second settling or arbitrary rig motions.

Read-only phase tracing is disabled by default. When enabled, it captures one
selected guide and the friction pair inputs. Structural snapshots from CPU worker
threads are joined in deterministic guide order; observation on/off produces
bit-identical positions, rotations and velocities in serial and parallel checks.
Phase comparisons identify amplification stages without changing solver gates.

The current default demo was recorded for six simulated seconds from front and
side, with full GPU-rendered hair geometry. This recording uses the existing
native dynamics, not the rejected accelerator candidate. Offline sample cadence
is not runtime FPS. Neither this recording nor the qualification proves >160 FPS,
CUDA, or hardware support beyond the tested Metal adapter. Reproducible results
and logs are in `artifacts/hair-contact-divergence-diagnostics-2026-10-09/`.

### Captured contact amplification and continuous queries

A read-only snapshot of the selected guide's entire contact-connected component
now records transient constraints before each nonlinear positional projection.
On the failing step 111, guides 71, 93, 106, 121 and 122 form a five-guide component
with 13 canonical constraints. Replaying each captured input on the native solver
reproduces its full-model output within 2e-13 m, including the input previously
solved on the GPU. The 1.72e-7 m difference is therefore reproduced by perturbed
geometry under the same native solver; it is not explained by GPU linear-solve
error in these captured equations. The fixed body-only tangent and nonlinear
surface replays remain stable. An independent 80-digit capsule geometry audit
also agrees with the existing closest-point calculation.

Between the last admitted substep and the next structural contact query, the
critical capsules move from about 0.4 micrometres of clearance to nearly
79.6 micrometres of penetration, against the required 80 micrometre centreline separation.
Such deep discrete interpenetration makes the contact problem sensitive to small
input changes. The portable `rod121-contact-component.bin` fixture reproduces the
shared tangent solve without full-model warm-up; its accompanying metadata states
the scope and excluded independent components.

`sweep_capsules` adds a geometry-only conservative advancement query for linearly
moving capsule endpoints. Relative endpoint velocities bound the change in
minimum centreline distance. Common rigid translation cancels from that bound.
The result distinguishes clear motion, initial contact, conservative approach and
an unresolved iteration limit. A limit never claims absence of collision. Queries
below the numerical precision of their coordinates are rejected for recentering.
This is a floating-point query with a tolerance, not an interval-arithmetic proof
or an exact time-of-impact solver.

Analytical crossing, common-translation, separating, sphere/segment, initial
penetration and iteration-limit cases pass. The three actual captured capsule
motions are limited before deep overlap, at fractions approximately 0.00461,
0.00499 and 0.02493. This query is not yet integrated into normal dynamics.
Contact-safe coupled Newton updates, initially touching constraints and prescribed
roots must be handled together before runtime admission. Simply clipping every
step at an initial contact would freeze legitimate motion; dropping constraints
would allow tunnelling. No production stability or FPS claim follows from this
primitive.

### Free Newton response coverage (2026-10-09)

The free-increment response batch now includes every rod, even with an empty contact graph. Contact-only requests still include only participating rods. This prevents a future coupled structural/contact step from silently freezing unconstrained guides. A regression compares both unconstrained guide increments against independent canonical native solves, checks exact fixed root increments, and verifies that response preparation leaves poses unchanged. All 81 hair unit tests pass (3 ignored), and the release application check passes. This is response preparation only: contact-safe prediction, swept broadphase, nonlinear admission and runtime integration remain unfinished. It does not qualify the GPU trajectory or establish rendering FPS.

The candidate free increment and contact reactions now share `constrained_newton_increment`, also used by the experimental force-pressure recovery. Regressions prove closing-contact suppression, separating-contact release with zero attraction, unchanged free-guide motion and exact root increments. The helper does not publish poses: its contact planes are linearized at the current state, so swept and nonlinear admission remain required. 83 hair unit tests passed (3 ignored), hair/hair_rebase integration checks and the release application check passed. Evidence: `artifacts/hair-contact-divergence-diagnostics-2026-10-09/coupled-newton-result.json`.

### Swept candidate ownership (2026-10-09)

`swept_capsule_pairs` selects pairs from endpoint swept AABBs expanded by radius, tolerance and numerical margin. Sweep-and-prune uses the largest scene extent and returns sorted unique index pairs. It avoids the existing spatial grid span cutoff: large movement is included rather than silently skipped. Invalid geometry or insufficient coordinate precision returns an error. Anatomical follicle permissions and rod adjacency exclusions are deliberately caller-owned. `swept_capsule_contacts` queries those candidates and retains every non-Clear outcome, including InitialContact and IterationLimit. The three captured guide121/122 transitions are selected and produce the same conservative query outcomes as direct queries. Randomized 64-motion exhaustive continuous comparisons and 17 intermediate time samples pass. All 86 hair unit tests pass (3 ignored); release application check passes. This does not yet connect swept admission to HairSystem or qualify render performance; worst-case candidate count remains quadratic. See `swept-broadphase-result.json` in the divergence diagnostic artifacts.

### Experimental swept structural runtime (2026-10-09)

`HairSystem::swept_strand_positions` is default-off and requires joint self-contact positions and zero terminal structural bypass iterations. It retains free prediction as an inertial target, admits prescribed-root movement, solves free structural and contact response loads jointly, and checks swept capsules before applying structural increments. Rebase resets initial admission. First use stages a contact repair of authored geometry at the collider motion start, with a 0.4 nm radius margin; clearance and 5% strain admission must pass before continuous evolution. A failure rolls back the whole step. Follicle permissions exclude only the root/root parameter corner through two trimmed-strip queries. Near-contact motion uses a fixed separating-plane support certificate over every endpoint pair, allowing certified opening/sliding motion. CCD stops within the static contact activation band. Mesh/contact reconciliation still uses discrete nonlinear admission; this is not whole-step moving-mesh CCD.

92 hair unit tests and 21 hair/hair_rebase integration tests pass. The full 469-guide Metal test compiles but FAILS frame 1 strain admission: rod390 segment0 compression is 7.4641004%, above 5%. 47 paired native/GPU phase snapshots differ in position by at most 1.533e-12 m; both reproduce the compression. Its root moves 222.945 um while the first free node moves only 0.056681 um. At that checkpoint the global sweep fraction stalled independent structural motion. The next contact-component experiment below replaces it and rechecks cross-component sweeps, without relaxing strain/clearance gates. The authored rest groom separately has 26 penetrating pairs (worst 76.8985 um), confirmed by an 80-digit reference. Initial repair advances the experiment beyond that initial penetration failure. No production activation, complete GPU trajectory, or rendered FPS is proven. Evidence: `swept-runtime-result.json`, `groom-initial-contact-audit.json`, `swept-runtime-rod390-comparison.json`, and compressed trace in the divergence diagnostic artifact directory.

### Contact-component structural admission (2026-10-09)

The experimental swept structural step now groups existing strand contacts, applies angular trust limits per group, and merges groups when continuous queries find a new restricted pair. After every merge or reduction it re-queries the complete proposed motion: a formerly clear neighbor can collide when another group slows. Old CCD fractions are discarded when unequal group clocks merge. A bounded 64-pass admission loop commits only fully checked trajectories; failure preserves the original HairSystem transaction. Normal reactions use the same group scale and canonical alias multiplicity. Unit regressions verify a fully moving independent guide, collision created by neighbor reduction, and component-local angular trust. 95 hair unit tests and 21 hair/hair_rebase integration tests pass.

Full native/control model: frame 1 passes all existing root, finite-state and 5% strain gates, resolving the old rod390 compression. At this checkpoint frame 2 failed because prescribed roots were moved and checked before their coupled free-node response. The joint-root experiment below replaces that intermediate admission. Real Metal GPU model: frame 1 FAILS with `banded solve: Residual { system: 0 }` on response call 17. An opt-in test-only `VOXY_HAIR_RESPONSE_FAILURE_EXPORT` captures the exact original f64 matrix/RHS/load batch on error (469 requests, 480 loads). Ignored `gpu_captured_rejected_response_batch`, using `VOXY_HAIR_RESPONSE_FAILURE_FIXTURE`, reproduces the same rejection in 0.13 s, independent of full model warmup. Native response solves of that captured batch pass before GPU replay is attempted. These failures remain failures; no residual or strain gate was relaxed, and no CPU fallback was introduced. Default activation remains off, render performance is unqualified. Evidence: `component-motion-result.json`, original response batch, native/control trace and rejection replay logs in the divergence artifact directory.

### Joint prescribed-root candidate path (2026-10-09)

The first structural iteration in each substep now uses the previously admitted positions as the continuous-query start and the candidate pose with exact new roots as its endpoint. The new roots remain boundary conditions for force assembly; their root-only intermediate pose is not separately published or admitted. Group reductions change only free increments, never prescribed root endpoints. Regressions cover a common rigid translation that would collide if only roots moved, and eight runtime moving-root steps with exact roots and <5% strain. 97 hair unit tests pass (4 ignored); 21 hair/hair_rebase integration tests pass.

The full native/control model still FAILS frame 2: component admission reaches its 64-pass bound. Opt-in `VOXY_HAIR_ROOT_MOTION_FAILURE_EXPORT` records an exact VJR1 binary query fixture (469 guides, 936184 bytes). Ignored `captured_joint_root_motion_admission`, via `VOXY_HAIR_ROOT_MOTION_FAILURE_FIXTURE`, reconstructs only the geometry/candidate/contact-connectivity data used by admission and reproduces the failure in 0.64 s. It does not reproduce constitutive assembly or the full animation. A separate 33-time candidate audit finds 85 pairs with negative distance samples, including root-adjacent segments136/1 and113/0 at t=0.34375, with -62.536 um gap, independently checked using the 80-digit segment reference. The minimum sampled gap is -79.896 um. Sampling supplies counterexamples only, not a certificate when no penetration is found. Reducing only free motion is insufficient for these prescribed-root paths: newly swept contacts must enter the coupled elastic solve with the known kinematic contribution. No admission gate was relaxed; production mode and real-time/GPU qualification remain unfinished. Evidence: `joint-root-motion-result.json`, VJR1 fixture, sampled audit and replay logs in the divergence artifact directory.

### Swept elastic contact cuts and interval clearance (2026-10-09)

The experimental coupled solve now includes transient contact planes discovered along prescribed-root candidate paths. Their bounds include known root motion and witness time; fixed roots retain zero solve increments. Each segment pair replaces its previous transient plane as the contact feature changes. Collision witness barycentric weights are retained, while the normal follows the first approach side. Refinement is bounded to 32 attempts and never publishes a failed candidate. The final continuous guard remains required: sampled witnesses alone cannot certify clearance.

Near-contact rotating pairs additionally use adaptive time intervals with separating-plane support over all endpoint pairs. An interval is accepted only when its plane covers both interval endpoints; affine endpoint motion then covers intermediate times and segment parameters. Exhausted queries remain restricted. Floating-point margins are included, but this is not a formal interval-arithmetic proof. All 100 hair unit tests pass (4 ignored). The latest saved candidate has no negative gaps in 33 samples and its full continuous admission replay passes in 0.04 s. This is a geometry replay, not a complete constitutive animation test.

The full 469-guide native/control test still FAILS at frame 2, now with `swept strand motion starts in penetration`; frame 1 passes. This identifies an outstanding intermediate-position admission problem after the improved candidate query. Real Metal response precision rejection remains unresolved. Default activation stays off; neither complete jump stability nor rendered FPS is qualified. Evidence: `swept-cuts-interval-result.json` and associated logs in the divergence diagnostic artifacts.

### One clearance budget per strand pair (2026-10-09)

The frame-2 initial penetration was -1.4475977446642193e-10 m for segments447/18 and462/19. Nonlinear reconciliation checked each strand's split target against a 1e-10 m threshold, effectively permitting a pair to spend two clearance budgets. It now refreshes pair witnesses and checks their actual combined geometric gap before accepting the pose. The existing threshold remains unchanged. A regression explicitly constructs passing individual target gaps with a failing combined gap, exercises reconciliation and verifies exact fixed roots. All 101 hair unit tests and 21 hair/hair_rebase integration tests pass; diff whitespace checks pass.

The full native/control model now completes frame 2 dynamics but FAILS the existing 5% strain qualification: rod288 segment0 compression is 22.5193072345%. The paired runs are native/control, not a real GPU trajectory qualification. A repeat with per-phase observation shows compression already present after prediction (12.368331% at substep0 and 22.519521% at substep1). All six structural iterations barely move the free node; discrete contact projection, friction and finish phases do not create this compression. Safe component reduction is therefore still suppressing the elastic response to prescribed-root movement. Successful geometric admission needs further coupled elastic/contact refinement when its reduced endpoint remains overstrained, not a relaxed strain threshold. Default activation stays off and rendering FPS remains unmeasured. Evidence: `pair-clearance-result.json`, phase summaries and before/after logs in the divergence diagnostic directory.

### Elastic refinement of restricted motion (2026-10-09)

Swept contact refinement now also runs when a geometrically admitted reduced component leaves any segment above the existing 5% strain qualification. Both prescribed-root and subsequent structural iterations use this path. Unrestricted Newton steps still proceed normally; the change does not replace constitutive dynamics with length projection. A regression checks prescribed-root compression under a nearly stalled free increment, sufficient free motion and exact zero root increments. All 102 hair unit tests and 21 integration tests pass. Full native/control still FAILS frame2, now during the refined contact solve: 103 constraints fail the original 1e-14 m projection tolerance. The reported pre-FISTA residual is 1.8765017058e-9 m. No gate was relaxed.

Opt-in `VOXY_HAIR_PROJECTION_FAILURE_EXPORT` saves a VQP1 diagnostic: original unnormalised Gram response operator, effective bounds reconstructed from the final floating-point state/reaction, multipliers, shape and sparse contact Jacobian. File-write failure leaves the physical failure intact. `audit_projection_failure.py` uses independent SciPy/NumPy algorithms, never a runtime fallback. The captured 103-row Jacobian has numerical rank103; primal linear feasibility gives maximum violation1.7686e-18 m. A nonnegative nullspace Farkas-witness search is infeasible. The normalized Gram minimum eigenvalue is4.7533e-17, with asymmetry1.6653e-15. These are numerical findings, not a formal exact feasibility proof. Independent L-BFGS-B reaches7.7903e-12 m KKT residual, still failing the runtime tolerance. The evidence points toward an ill-conditioned compliance solve requiring a more stable formulation; it does not justify dropping constraints, raising tolerance or declaring the trajectory qualified. Default activation remains off. See `elastic-contact-refinement-result.json`, the rejected projection fixture, audit and logs in the divergence diagnostic directory.

### Pivoted dual active solve (2026-10-09)

For at most256 constraints, a bounded dense active-set solve follows unsuccessful projected Gauss-Seidel/PCG and precedes the existing projected-gradient path. It uses diagonal scaling, partial-pivot elimination and up to512 active changes. A negative trial multiplier releases an equality at the first nonnegative boundary; every original inequality remains eligible for re-entry. Singular/nonfinite or unadmitted candidates return failure to the existing solver path. Acceptance requires all original complementarity residuals and a fresh check of the actual updated response state at the unchanged tolerance. Larger islands retain the existing matrix-free path. No diagonal regularization, tolerance increase or constraint deletion is used.

The captured103-constraint dual now passes the original1e-14 m gate in Rust; an independent dense active-set audit converges in eight iterations with2.0329e-19 m maximum residual. Regressions cover release of nearly dependent active planes and re-entry of a released plane. 104 ordinary hair unit tests and21 integration tests pass (five unit tests ignored), plus the explicitly run103-row capture replay. The full native/control model advances beyond that failed solve but still FAILS frame2 on a subsequent106-row solve. Its capture replay also fails. Independent primal linear feasibility finds4.3029e-19 m violation; the floating-point Gram minimum eigenvalue is-4.9533e-16. A dense audit cycles on row74: it is violated by about1.8354e-12 m, enters, and immediately releases at zero fraction; the computed conditional curvature is zero. Forming the compliance Gram has lost the small independent direction. A stable formulation from the original constraint Jacobian and rod factor remains necessary; the full animation, real GPU response and rendered FPS remain unqualified. See `dense-active-result.json`, both fixture logs and the106-row operator/cycle audits.

### Square-root compliance and equality QR (2026-10-09)

The canonical band response now exposes its unchanged forward and backward triangular halves. `HairResponseSystem::whiten_loads_native` retains columns W=L^-1*load for the original H=L*L^T, so later contact QR need not form W^T*W. `solve_load_equalities_native` computes a minimum-energy displacement for independent load equalities: twice-orthogonalized modified Gram-Schmidt gives W=Q*R; solve R^T*z=b, form y=Q*z, then solve L^T*x=y. Admission rechecks each original load dot displacement against the caller's positive finite absolute tolerance. Invalid shapes, nonfinite data, incompatible/unresolved bases and failed residuals return errors. Both APIs and existing native responses share factor preparation. No new diagonal shift or factor policy was introduced.

Regressions verify native compliance-energy agreement, exact fixed DOFs and agreement with a known physical minimum-energy response. An identity-system fixture has loads [1,0] and [1,1e-10]: its rounded Gram determinant is zero, while square-root coordinates preserve the1e-10 direction and QR satisfies original equalities at1e-14 tolerance. Incompatible duplicate equalities and invalid tolerances fail. The reference-response comparison is normwise; an initial component-relative test incorrectly demanded sub-roundoff accuracy on tiny components (observed2.16e-18 difference) and was corrected without changing equality admission. All107 hair unit tests and21 integration tests pass (five unit tests ignored).

This is a native equality primitive, not yet the full unilateral active-set solve across multiple rods. It is not used as a GPU fallback and has not qualified the rejected106-contact animation path. Global square-root column assembly, active-contact integration, real GPU support and rendered performance remain unfinished. The most recent full-model evidence is still the previous frame2 failure. See `square-root-qr-result.json` and associated unit/integration logs.

### Native unilateral square-root primitive (2026-10-09)

The QR factor now also solves R*lambda=z for equality reactions. A bounded512-change unilateral solve operates on square-root columns directly: add a violated inequality, solve the active equalities, and release the first reaction that reaches zero along a nonnegative step when a trial reaction is negative. Released rows remain in the full inequality query and can re-enter. Every candidate is checked against original column projections; unresolved/rank-deficient/nonfinite bases or exhausted iteration budgets fail, with no Gram regularization or dropped admission rows.

`HairResponseSystem::solve_load_inequalities_native` returns the native minimum-energy displacement and nonnegative reactions for one response system. It validates original load inequalities/complementarity at the caller's absolute tolerance and force balance with the existing canonical `validate_load_correction`. Tests cover zero reaction for opening contacts, coupled closing-contact activation, admission of near-parallel inactive rows, native reference displacement and positive reaction agreement. A two-active-row fixture uses columns[1,0] and[-1,1e-10]: rounded Gram loses the independent direction, while QR satisfies both original bounds at1e-14. This is an algebra stress fixture with large finite reactions, not an anatomical material calibration. All110 hair unit tests and21 integration tests pass (five unit tests ignored).

The cross-rod HairSystem contact path does not yet use these square-root columns, and no GPU square-root path or new full-animation qualification was run. The last full-model evidence remains the106-row frame2 failure. Global column assembly and publication through the existing transaction owner remain required before this constitutes a runtime fix. See `square-root-active-result.json` and associated logs.

### Joint square-root response owner (2026-10-09)

`HairResponseSystem::solve_joint_load_inequalities_native` now projects global contact inequalities across multiple independent local compliance systems. Each local system supplies one load per global row, with zero loads for uninvolved rows. The owner concatenates square-root coordinates, solves one unilateral QR problem, converts each coordinate block through its own original triangular factor, validates local force balance and then rechecks the original summed global load inequality/complementarity. Fixed DOFs remain local exact zeros. The single-system inequality API delegates to this same owner, removing a duplicated projection/validation implementation.

The joint regression checks equal and opposite reactions/displacements of two systems under one contact, exact fixed roots, zero response to an opening constraint and rejection of mismatched global load counts. All111 hair unit tests and21 integration tests pass (five unit tests ignored). The joint owner is still a native API; HairSystem's constraint-to-request wiring and GPU square-root support are not yet connected. No CPU fallback was inserted into the GPU backend, and no new full-model or rendered-FPS qualification was run. The previous106-row frame2 failure remains the latest full-animation evidence. See `joint-square-root-result.json` and associated logs.

### Native Newton square-root recovery (2026-10-09)

The native constrained Newton path now retains its original free candidate for at most256 constraints. If existing contact projection fails, it rebuilds original per-rod contact loads from constraint gradients using the same canonical request assembly, solves global square-root inequalities for bound minus J*free, adds the response to the original free linear/angular increment, replaces reactions and rechecks every original constraint against the same scaled tolerance. Uninvolved rods retain their full free increment; fixed root corrections stay zero. Physical pose publication and nonlinear/swept admission remain owned by the existing HairSystem transaction. Explicit external solver calls never enter native recovery, so this does not introduce a GPU-to-CPU fallback.

A wiring regression checks closing-contact removal, opening-contact release without attraction, pair reaction conservation, exact roots and unchanged independent guide motion. All112 hair unit tests and21 integration tests pass (five unit tests ignored). The ignored full-model test now supports explicit `VOXY_HAIR_NATIVE_ONLY=1` only with `VOXY_HAIR_CPU_CONTROL=1`; both paired trajectories use native dynamics and its log labels that mode. This supplies native full-model evidence without implying GPU response qualification.

The469-guide paired native-only run passes frames1 and2, including the existing root/finite/5% strain gates and identical paired trajectories. Six earlier106-row projection failures are recovered by square-root solves. Frame3 still FAILS with `swept strand components have no admissible progress`, after29.93 s of qualification execution. Whole-jump stability, real GPU square-root support and rendered FPS remain unqualified; elapsed qualification time is not a rendering measurement. Swept mode remains default-off. See `square-root-newton-result.json` and associated logs.

### Between-sample swept witnesses (2026-10-09)

Motion failure export now also records component underflow/no-progress, and opt-in `VOXY_HAIR_SWEEP_LIMIT_TRACE` exposes the exact restricted capsule paths. The previous frame3 failure repeats in32.18 s; its938696-byte VJR1 geometry fixture repeats admission failure in0.47 s. Its initial6171 candidate pairs have no negative gaps in33 coarse samples. Independent localized time searches nevertheless find real between-sample intersections, checked with the80-digit distance reference: pair149/9-202/19 reaches-1.36148774156e-7 m at time0.00544294. Subsequent reduction to approximately0.309362663505 creates a root-adjacent collision385/0-393/1 with endpoint gap-8.46954072505e-6 m. Repeated reduction reaches1.744e-321 and then zero. The earlier zero coarse negatives were never a clearance proof. These checkpoints are stored in `native-square-root-frame3-result.json` and audits/replay logs.

Transient swept contact discovery now augments coarse times with a bounded64-iteration golden-section minimum locator for non-Clear capsule-query regions, including both follicle permission strips. It supplies counterexamples only: closest-feature changes can make multiple minima, so it never certifies clearance or replaces final continuous admission. The captured149/202 regression verifies all33 coarse times miss the penetration while the locator finds it. All113 hair unit tests and21 integration tests pass (five unit tests ignored).

The refined full native-only model still FAILS, now at frame2 with261 constraints: the current256-row native QR recovery capacity is exceeded. The original projection reports7.25452373666e-11 m residual against unchanged1e-14 tolerance. This is not a qualified trajectory improvement; more true contact witnesses expose the next solver capacity problem. Independent contact-island assembly is needed rather than omitting those witnesses. Default swept activation remains off, GPU support and rendered FPS unqualified. See `localized-contact-result.json` and associated logs.

### Rod-compliance contact islands (2026-10-09)

Native square-root Newton recovery now partitions the complete constraint set by rod connectivity. Nonzero row gradients connect rods; all stations of one rod share an owner because the constitutive compliance couples them. Canonical minimum-index components retain original row order and independent local DOFs. Each island assembles only its own original load columns and solves independently; untouched rods retain free motion. The old256-row global recovery cutoff is removed. A bounded512-row capacity applies per island, with explicit failure above it rather than omitted constraints. This is computational capacity, not a change in geometry/force tolerances. All island reactions are staged and committed only after the full original candidate passes admission. External solvers still never enter this native recovery path.

A regression solves300 independent original constraints with exact roots and every original residual, exceeding the previous global cutoff. Another verifies transitive connectivity across different stations of one rod: particle-level partitioning would incorrectly separate them. All115 hair unit tests and21 integration tests pass (five unit tests ignored). The full469-guide paired native-only model now passes frame2 with261-row projection recovery and unchanged root/finite/5% strain checks. Frame3 reaches443-444-row projections and still FAILS with `swept strand components have no admissible progress` after51.62 s. No complete jump, real GPU QR or rendered-FPS qualification follows. Default swept activation stays off. See `contact-island-result.json` and associated logs.

### Exact moving-line reference for frame3 (2026-10-09)

The repeated469-guide native qualification passes frames1 and2 and fails frame3 after50.85 s. The new captured geometry replay reproduces the admission failure. Thirty-three sampled times across6179 initial candidate pairs contain no negative gaps; these samples do not certify continuous clearance.

An independent exact-rational reference constructs the degree-six polynomial `(w dot (u cross v))^2 - threshold^2 * |u cross v|^2` from the traced binary endpoint values. Exact Bernstein subdivision proves nonnegative numerator clearance and strictly positive line-direction denominator throughout the interval. Infinite-line separation lower-bounds segment separation. With the existing threshold `2*40e-6 - 1e-10`, all nine nominal full-step trace restrictions certify clear. Three subsequent reduced-clock trace pairs remain uncertified; independent pointwise checks find root-adjacent penetrations of approximately40.61,11.02 and27.57 micrometers. Those three rows are reduced trials, not original full-step restrictions.

This identifies conservative fixed-plane rejection as a cause of artificial reduction and subsequent root collisions. The bounded reference returns uncertified at depth64 or budget exhaustion; it never substitutes for runtime admission. Production native/GPU certificate integration remains unfinished, with no complete jump or rendered-FPS qualification. See `contact-islands-continuous-reference-result.json`, the exact certificate audit and current capture/replay logs.

### Native moving-line continuous certificate (2026-10-09)

The native motion owner now tries a bounded moving-line certificate before fixed separating-plane subdivision. `contact_line_certificate.rs` encloses every polynomial operation, power-to-Bernstein conversion and midpoint subdivision by outward-rounded f64 intervals. Nonnegative lower bounds of all degree-six clearance controls and strictly positive denominator controls certify the whole interval; nonfinite arithmetic, parallel directions, depth48 or512-node exhaustion return unknown. Unknown retains the previous admission path. The geometric tolerance is unchanged; no samples or optimizer minima authorize motion. This is native geometry admission, not a GPU physics implementation or a GPU-to-CPU solve fallback.

The captured nine nominal frame3 restrictions now certify in Rust, while all three reduced collision paths remain rejected. The original full captured frame3 geometry admits its complete checked path in0.03 s. Tests also reject an interior tunnel, unknown parallel lines, exhausted queries and numeric overflow. All118 hair unit tests pass (five ignored);20 hair integration tests pass.

The new469-guide native-only qualification still FAILS at frame3 after243.19 s, with altered candidates and up to540 original constraints. Its new geometry replay fails too. Independent80-digit pointwise checks find seven real penetrating nominal initial-trust paths, including137/0-201/16 with gap about-80 micrometers at time0.957798309 and145/0-200/17 at time0.835569476; both supporting segments approach an actual intersection. Two reduced paths add approximately39 and41 micrometer penetrations. This differs from the previous nine safe restrictions: the admission guard must reject these new candidates. The qualification binary precedes the final nonfinite-arithmetic early-rejection guard; all finite-input arithmetic is unchanged afterward and the final unit run includes that guard. Complete jump stability, GPU qualification and rendered FPS remain unproven. Next work concerns nonlinear contact-cut consistency, not relaxing clearance. See `line-certificate-runtime-result.json`, the new VJR1 fixture and reference/replay logs.

### Retained swept collision witnesses (2026-10-09)

Swept refinement previously replaced an existing plane solely by segment-pair identity. The replacement could constrain another barycentric point while reopening a prior witnessed crossing. Refinement now retains distinct point/time/normal witnesses for the lifetime of one `advance` admission transaction. Exact duplicate witnesses do not change the set. Each changed set rebuilds the original base constraints plus every retained witness using the unchanged canonical coalescing rule; transient witnesses do not enter friction history or persist across Newton transactions. No admission row is discarded to meet solver capacity. The final continuous geometry guard remains mandatory.

A regression constructs a candidate admitted by the new witness alone while its earlier point crosses the original side; retaining the original witness rejects that candidate and keeps pinned-root gradients zero. The first-contact-side regression now also preserves old planes after feature changes. All119 hair unit tests pass (five ignored) and20 integration tests pass. The complete469-guide native-only run passes frames1 and2 but still FAILS at frame3 after163.07 s. Its final captured geometry has two nominal initial-trust restrictions instead of the previous seven;80-digit pointwise checks confirm145/0-200/16 penetrates14.239 micrometers at the endpoint and145/0-200/17 penetrates4.887 micrometers near time0.455037343. Reduced trials create further17.871 and41.819 micrometer penetrations. The changed candidate remains invalid; smaller traced restriction counts do not qualify the complete jump. Increased witness sets expose further nonlinear refinement/scalability work. Native test duration is not rendered FPS; GPU behavior and full animation remain unqualified. See `retained-cuts-result.json`, captured VJR1 geometry and reference/replay logs.

### Shared angular-trust paths and original-load residual capture (2026-10-09)

Native constrained Newton now tries the existing original-column square-root solve before exhausting iterative Gram projection. Its legacy native projection remains available from the original free candidate when QR fails; explicit external solvers keep their previous path. The primary-only paired native qualification still fails frame3, but takes16.79 s instead of163.07 s in the preceding retained-cut run. This is test execution with potentially different floating-point trajectories, not rendered FPS or a universal speedup benchmark.

That failure stops adding witnesses after eight refinements. The saved candidate's common angular trust scale is0.945919777320238: admission checks scaled motion while witness discovery previously checked full motion. `trust_components` now owns the shared grouping, shape/finite validation and angular trust calculation. Witness motions use those same scales; affine load gradients scale each rod's free contribution while prescribed roots remain exact. Every retained point/time/normal feature is relinearized on the current common group scales after connectivity changes; obsolete clock paths do not accumulate as extra inequalities. A test shows clear rigid transport becoming a true collision under a common angular reduction, verifies discovery of that collision and compares each affine row with its actual scaled geometric path including moving roots. Refinement exhaustion explicitly rejects before pose publication. All120 hair unit tests pass (five ignored), and20 integration tests pass.

The updated469-guide native qualification still fails frame3 after32.33 s, now at1041 constraints with `joint square-root inequality residual failed`, following21 completed refinements. An opt-in `VOXY_HAIR_QR_FAILURE_EXPORT` records VQI1 original matrices/loads/bounds, whitened columns, coordinates, reactions, fixed ranges and returned responses without rounding. The repeated capture fails after31.49 s; the failed island has seven systems,88 inequalities,882 coordinates and1328076 bytes. A100-digit pointwise audit finds row2 original-load residual1.71512535520351e-14 against1e-14 tolerance, while its whitened residual is1.5898157955114373e-15. Fixed responses are exactly zero. The maximum unscaled response component is3287.334167371116, indicating severe scaling during angular-trust iteration; it is not an admitted physical pose.

Independent original-load linear feasibility returns a point whose100-digit worst gap is-3.7601458306143805e-20, within the original tolerance. That point has very large unconstrained parameters and proves neither minimum energy, realistic motion nor continuous clearance. Next work concerns original-space numerical defect correction and nonlinear trust convergence, with original gates preserved. Whole jump, real GPU QR and rendered FPS remain unqualified. See `current-trust-result.json`, exact failed-island capture/audit and qualification logs.

### Original-load numerical defect correction (2026-10-09)

Joint native inequality response now performs up to eight bounded defect refinements using the same canonical local factors and whitened columns. After back-transformation it measures each original load projection versus the corresponding whitened projection and compensates that numerical mapping defect in the next solve's surrogate bounds. Each trial resolves the complete unilateral system; final acceptance still uses the original immutable bounds, nonnegative reactions, original complementarity and original per-system force balance. No diagonal regularization, omitted row or relaxed physical tolerance is introduced. Unresolved refinement remains an error before publication.

An ignored VQI1 regression reconstructs the actual seven-system,88-row failure, first confirms the captured original result violates admission, then checks every original inequality, reaction and exact fixed DOF of the refined result. The replay passes in0.01 s: the formerly failing native accumulation changes from1.548414174656898e-14 to9.232198339148567e-15 against the unchanged1e-14 tolerance. Independent100-digit original-load evaluation of the exported VQR1 result gives maximum active residual8.584505250198136e-15, minimum inactive gap1.7849081825681185e-7 and zero fixed responses. All120 hair unit tests pass (six ignored), and20 integration tests pass.

The full469-guide paired native-only qualification still fails frame3 after94.50 s, now with `native square-root Newton admission failed` at1041 constraints following21 completed refinements. The local joint load gate succeeds; adding island responses to the original free candidate and validating the complete Newton rows exposes the next error. A trial allocating one eighth of the tolerance to local solves fails the captured QR convergence gate and was reverted; the verified defect-correction implementation retains the original tolerance. This evidence does not qualify a complete jump, realistic parameter calibration, GPU response execution or rendered FPS. Next work must inspect the complete original Newton candidate rather than accepting the local response result as global proof. See `original-defect-result.json`, regression/reference artifacts and qualification logs.

### Original Newton response-addition refinement (2026-10-09)

The opt-in VJN1 capture preserves the original free increment, rounded corrected increment and all original constraint entries/bounds/reactions before publication. The repeated native frame3 failure takes31.83 s; its1109984-byte capture contains469 rods and1041 constraints. Row164 alone fails: native gap1.1005194151814113e-14 and100-digit gap1.019156555946466e-14 against1e-14 tolerance. The original angular increment maximum is0.060078526395315496, while the rejected candidate reaches4118.14342192446; none of that candidate is a qualified physical pose.

Native island response assembly now checks the actual rounded `original_free + response` against the original Newton rows before committing the island correction. Up to eight refinements compensate only the numerical addition/projection defect in the next local response bounds. Every trial still resolves the entire unilateral island and validates original local force balance. The original global Newton gate, nonnegative staged reactions and final continuous-motion guard remain mandatory. No tolerance or physical row is relaxed. A small identity-compliance rounding fixture first demonstrates local-response admission followed by failed free-addition admission, then verifies the refined candidate against the unchanged original row and exact root. It is an arithmetic stress fixture, not anatomical calibration. All121 hair unit tests pass (six ignored) and20 integration tests pass.

The full469-guide paired native-only run passes frames1 and2, gets through the former1041-row addition failure and completes refinement22, then FAILS at frame3 on1044 constraints with `joint square-root active contacts did not converge`, after31.38 s. Repeated capture takes32.61 s. New VQC1 input capture records an unsolved seven-system,91-row,882-coordinate joint problem in1356348 bytes, with zero numerical bound shift at its first QR trial. It includes canonical matrices/loads and original/working bounds without inventing a returned solution.

Independent original-load linear feasibility produces a100-digit minimum gap-6.94863010559121e-20, within the unchanged1e-14 tolerance. Its very large unrestricted parameters do not prove minimum energy, force balance, realistic motion or continuous clearance. The whitened LP reports success but independently violates its inequalities by2.323942189552357e-4, so that status is not accepted as a feasibility certificate. Next work concerns active-set QR convergence and scaling using the exact failed input. Full jump, actual GPU execution and rendered FPS remain unqualified. See `newton-addition-result.json`, VJN1/VQC1 captures and independent audits.

### Active equality coordinate refinement (2026-10-09)

The captured VQC1 input with 91 rows exits the unrefined unilateral QR after ten active-set iterations: no inactive violated row can enter, while the active equality residual fails admission. Bounded residual correction now uses the same Q/R factors to correct coordinates and reactions against the original columns. No row or tolerance is relaxed. The captured native replay passes all original whitened inequalities and complementarity at 1e-14; 121 hair unit tests pass.

Full 469-guide native-only jump still fails at frame 3 after 63.18 seconds; this is not GPU or rendering performance evidence. The separate rendered soft-region test passes: breast vertical displacement peaks 12.09/12.10 mm and buttock peaks 9.40/9.59 mm, with settling. That test disables hair and does not qualify abdominal or general skin motion. Logs are in artifacts/hair-contact-divergence-diagnostics-2026-10-09/qr-equality-*.log and body-rendered-region-motion.log.
