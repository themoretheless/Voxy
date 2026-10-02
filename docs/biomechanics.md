# Load-controlled biomechanics in Voxy

Run `cargo run -p voxy_app --example biomechanics --release`. Space pauses;
Esc exits. The existing `tissues` demonstration remains an abstract XPBD example.
This separate example uses continuum finite elements, not tuned edge springs.

The example shows two isolated pressure-bearing penile tunica tubes on the left,
and one bonded concentric internal/external anal sphincter specimen on the right.
These are idealised mechanical specimens, not complete anatomical organ models.
The shaft specimens use a fixed base. Three outer nodes restrain rigid motion of
the sphincter; surrounding pelvic support has not been reconstructed. Display
scale is 20 scene units per metre for the chambers and 35 for the sphincter.

The demonstration cycles through unloading, 8 kPa chamber pressure + IAS
activation 0.3, pressure + IAS 0.3/EAS 0.5, and release. Load changes wait for all
three equilibrium residuals to fall below 1e-4 N. The title explicitly displays
`solving` or `equilibrium` and the maximum residual across bodies. There is no
physiological time model: solver iterations and the two-second viewing hold do
not represent muscle activation kinetics, blood flow or viscoelastic relaxation.

## Continuum formulation

Coordinates are metres, force is newtons, stress and moduli are pascals.
Each linear tetrahedron stores its reference inverse shape matrix and volume.
The deformation gradient is F = Ds Dm^-1. Negative/zero determinants are rejected.
The passive matrix uses isochoric neo-Hookean energy with a volumetric penalty:

```
J = det F
I1 = tr(F^T F)
W_matrix = mu/2 * (J^(-2/3) I1 - 3) + K/2 * (J - 1)^2
```

Each reference fiber a is a unit vector. With l = |F a| and e = max(l^2 - 1, 0):

```
W_fiber = k1/(2 k2) * (exp(k2 e^2) - 1)
W_active = activation * T0 * (l - 1)
```

The fibers resist tension only. The active term produces constant nominal
fiber tension; it has no force-length/force-velocity or smooth-muscle kinetics.
This is an aligned exponential fiber law, NOT the full dispersed HGO law.
Fibers in the ring follow the circumference; tunica templates contain both
circumferential and longitudinal families. Per-layer activation is independent.

Analytic first Piola stresses are differentiated from this energy, then assembled
as nodal energy gradients. Uniform prescribed cavity pressure contributes -p V,
where V is the oriented closed triangular cavity volume. Differentiating that
volume automatically gives follower pressure forces, including end-cap loads.
Cavity construction rejects open or inconsistently oriented boundaries. Inner
rings are sealed by mathematical cap triangles; these are load surfaces, not
additional solid end-cap tissue. The fluid is not simulated.

Equilibrium uses diagonally preconditioned nonlinear conjugate gradients with
Armijo backtracking and restarts. Rejected trials cannot invert elements. The
solver publishes bounded progress with explicit residual, convergence status and
min/max J; exhausting the budget never reports a converged solution. Invalid
loads are rejected before mutation. It is quasistatic FEM, not a dynamics solver.

## Parameter provenance and validation status

Tunica matrix mu = 4.2857 MPa, K = 20 MPa comes from Table 1 of
[Fereidoonnezhad et al., 2023](https://doi.org/10.1016/j.compbiomed.2023.107524).
Those are the paper's literature-derived baseline constants, not a new fit of
healthy human tissue. The exponential reinforcement (k1 = 0.1 MPa, k2 = 10),
geometry, pressure scenario and support conditions are exploratory choices.
They are not the paper's full anatomical model or HGO parameterisation.

IAS/EAS matrix/fiber/active constants are explicitly uncalibrated scenarios.
They are not inferred from penile tissue or presented as human measurements.
Human IAS length-tension experiments are described by
[Glavind et al., 1993](https://pubmed.ncbi.nlm.nih.gov/8238363/), but their abstract
alone does not provide the curves needed to identify this specimen's parameters.
Penile calibration against measured forces and boundary deformation is described
by [Fereidoonnezhad et al., 2024](https://pubmed.ncbi.nlm.nih.gov/38945188/).

`cargo test -p physics --test biomechanics` checks constitutive energy derivatives,
frame indifference, zero passive stress at rest, shear modulus, fiber anisotropy,
nodal force derivatives under pressure, translation/force balance, equilibrium
expansion and contraction, invalid-input preservation, exact affine patch energy,
and convergence of polygonal cavity volume toward the analytical cylinder.
The geometry/patch tests are not organ-level mesh-convergence or physiological
validation. The application test checks loaded FEM geometry and inversion bounds.

## Calibration from measured data

A measured tensile CSV uses this header (stress must be nominal, not Cauchy):

```
stretch,nominal_stress_pa
```

Run:

```sh
cargo run -p physics --example tissue_calibrate -- TRAIN.csv HELD_OUT.csv BULK_PA FIBER_EXPONENT
```

It fits matrix shear and one longitudinal fiber stiffness for passive,
incompressible, traction-free uniaxial tension. Bulk modulus and exponent remain
supplied parameters; uniaxial data cannot identify the bulk modulus. Degenerate
datasets and nonpositive matrix fits fail explicitly. RMSE and maximum stress
error are reported on the separate validation file. The caller must ensure that
these files represent independent measurements; different filenames alone do
not establish independence. Synthetic test data verify the fitting algorithm,
not human tissue properties. The demo does not silently load fitted values;
material and geometry must be assigned for the relevant specimen and experiment.

## Remaining requirements for quantitative anatomical accuracy

The current penile specimens omit cavernosal porous tissue, spongiosum, septum,
skin, fascia, vascular filling and anatomical attachment geometry. The anal
specimen omits mucosa, puborectalis, anal cushions and surrounding pelvic tissues.
There is no contact, friction, lumen wall closure, mutual-body interaction,
viscoelasticity, physiological active force-length law or stress-free prestress
reconstruction. A positive element J does not prevent distant surface overlap.
Near-incompressible linear tetrahedra may exhibit volumetric locking; a mixed
formulation or convergence study is required before trusting quantitative results
at higher K/mu ratios. The current bounded optimizer is not a substitute for
organ-scale numerical convergence and load-step sensitivity analysis.

Completion of quantitative validation needs a selected loading experiment,
reference geometry, layer-specific passive/active data, load/support conditions,
a held-out force/deformation dataset, and an explicit error tolerance. No measured
organ-level accuracy, clinical predictive accuracy or parameter uncertainty has
been established by the current implementation.

## Explicit muscle activation time and layered sphincter specimen

`ActivationKinetics` provides a phenomenological first-order actuator state with explicit rise/fall time constants and tonic activation. For constant excitation u over an elapsed time dt, target activation is `tonic+(1-tonic)*u`; activation advances exactly as `a_new=a+(target-a)*(1-exp(-dt/tau))`. The rise or fall time constant is selected by the sign of target minus current activation. `expm1` retains very small activation increments. State/excitation must lie in [0,1]; times are positive SI seconds. There are no implicit physiological parameter defaults.

First-order excitation/activation models are a common musculoskeletal approximation; [Zajac (1989)](https://pubmed.ncbi.nlm.nih.gov/2676342/) discusses skeletal muscle modelling. The present two-time-constant/tonic law is an explicitly chosen simplified actuator, not an exact implementation of Zajac, a measured internal anal smooth-muscle law, calcium kinetics, motor-unit recruitment or an enteric reflex model. IAS/EAS parameter identification remains required.

`Body::step_muscle_regions` assigns independent kinetic histories to existing material-region identities, stages activation and solid geometry, and commits only after mechanical equilibrium converges. Missing/duplicate regions, invalid activation and equilibrium failure preserve all original element activations, positions and caller kinetic histories. It uses the existing active circumferential fiber stress. Solid mechanics is quasistatic at each activation time; no inertial motion, length/velocity dependence, fatigue, lumen contact or pelvic support has been added. The native `biomechanics` viewer still uses its older prescribed activation schedule; its viewing hold times remain nonphysiological.

`cargo run --release -p physics --example sphincter_kinetics` runs the idealized bonded IAS/EAS tube with synthetic rise/fall times 0.2/0.5 seconds and IAS tonic activation 0.02 (EAS zero). Excitations 0.2/0.1 are held for four 0.1-second steps, then set to zero for four 0.5-second steps. At 0.4 seconds the accepted lumen volume was 4.630228058037e-6 m3; subsequent release increased it toward its tone-bearing state. These are scenario results, not human measurements.

Tests check independent exact exponential rise/fall values, time composition, tonic equilibrium, small-time accuracy and input rejection. A second test uses the actual two-layer tetrahedral specimen: lumen volume decreased from 4.969325665968397e-6 to 4.813705787302594e-6 m3 after a 0.1-second synthetic activation step, at IAS/EAS activations 0.07869386805747332/0.03934693402873666, with converged positive-J mechanics. It verifies per-region element activation, lumen recovery after release and complete rollback on mechanical nonconvergence or an invalid later region. Release suites passed: biomechanics 9, muscle activation 2, tissues 11 (22 total).

## Length-dependent active fiber force

`ActiveFiberLengthLaw` adds an optional explicitly parameterized active force/stretch curve to aligned-fiber elements. With `x=(stretch-optimal_stretch)/half_width`, normalized tension is `(1-x^2)^2` for `abs(x)<1`, otherwise zero. The active potential is the integral of that curve, normalized to zero at reference stretch one: within support its primitive is `half_width*(x-2*x^3/3+x^5/5)`, with constant continuation outside support. Thus active Piola tension is the actual derivative of active potential rather than a length-dependent multiplier inconsistently applied to the old linear energy.

`Material::response_with_active_length` supplies the optional law directly; `Body::set_active_fiber_length_law` assigns it to an aligned-fiber element. Default material response retains the original constant nominal active tension. Cardiac/viscoelastic assignments are rejected by the setter. A configured law is used by element response, assembled nodal gradients and equilibrium. This compact bell is a chosen phenomenological family, not an experimentally fitted human smooth/striated muscle force-length curve; it has no velocity dependence, cross-bridge state or fatigue. Optimal stretch and width must be explicitly provided and calibrated.

`cargo run --release -p physics --example sphincter_kinetics -- --length-dependent` assigns synthetic optimal stretch one and half-width 0.5 to both ring layers. The normal-release example completed activation and release; its accepted 0.4-second lumen volume was 4.632728087090e-6 m3, versus 4.630228058037e-6 m3 for the constant-tension scenario. This small scenario difference is not a physiological prediction.

Tests check peak and inactive support values, active stress against an independent energy finite difference over compressed/extended/sheared states, frame objectivity, and actual layered-ring contraction with consistent assembled nodal energy gradients. Finite-difference checks account for the existing tension-only passive transition at stretch one, where central-difference convergence is first order. Normal-release suites passed: biomechanics 9, muscle activation 4, myocardium 8, organ coupling 6 (27 total).

## Objective active force–velocity correction

`ActiveFiberVelocityLaw` supplies an explicit shortening limit (stretch per second), Hill curvature `c=a/P0`, eccentric tension limit and eccentric rate scale. For shortening speed `s=-lambda_dot/vmax` between zero and one, normalized tension is `(1-s)*c/(c+s)`; beyond the shortening limit it is zero. This branch is the normalized [Hill (1938) hyperbola](https://math.nyu.edu/~peskin/eb_lecture_notes/Hill_AV_1938.pdf). For positive stretch rate the separately chosen phenomenological branch is `1+(eccentric_limit-1)/(1+eccentric_rate/lambda_dot)`. It is not a measured human eccentric or smooth-muscle fit.

`Body::active_velocity_forces` computes `lambda_dot=(F*A).(Fdot*A)/lambda` from current tetrahedral geometry and nodal velocities in m/s. It assembles the difference from the existing held-activation active potential, including optional active length dependence, as nodal forces in N. The returned correction power in W is nonpositive for these branches: reduced shortening tension and increased lengthening resistance dissipate mechanical energy relative to that potential. This is not total chemical energy, heat production or the full active muscle power budget. Cardiac/viscoelastic active elements reject this generic law.

Tests check the independent Hill product identity, shortening cutoff, isometric/eccentric values, nonpositive correction power in the actual layered-ring FEM, zero net force, invariance to added translation and rigid angular velocity, and vanishing correction under rigid motion. This API assembles forces only. It is not yet connected to `InertialBody::step`, the quasistatic sphincter driver or the character skin solver; those conservative algorithms cannot accept the new forces without separate work accounting and time-convergence validation. Cross-bridge kinetics, smooth-muscle calibration and anatomical muscle registration remain open.

## Rate-dependent inertial muscle integration

`InertialBody::step_muscle` now connects the force–velocity correction to actual solid motion through explicit midpoint integration of both nodal positions and velocities. Initial acceleration predicts a half-time geometry and velocity; forces are reevaluated together at that state, then advance the full state. Gravity and stationary plane penalty forces participate through the existing acceleration/potential evaluation. Activation, cavity pressure and material parameters are held fixed over the step. The ordinary conservative Verlet `step` remains available.

`MuscleDynamicStep` reports midpoint correction work `dt*power` and signed work-balance defect `delta(K+U)-correction_work`. A caller-supplied positive absolute tolerance in joules rejects excessive defect. Invalid controls/material laws, intermediate or final constitutive failures and rejected work balance commit neither geometry nor velocities. Negative correction work represents energy lost relative to the held-activation active potential, not metabolic accounting. This is an explicit second-order method, not symplectic or unconditionally stable; a work guard alone is insufficient for long-time trajectory accuracy. Pins and viscoelastic histories remain unsupported by `InertialBody`.

`muscle_dynamics` checks an active, initially shortening single tetrahedron over 0.02 SI seconds at 20/40/80 steps. Consecutive full-state differences had ratio 3.9847128440704256. Absolute cumulative work-balance defects were 1.2608501005523416e-8, 3.174195893623907e-9 and 7.960649703179275e-10 J, respectively. These establish temporal refinement for this synthetic fixture, not agreement with measured muscle trajectories or an independent full-system solver. Linear momentum conservation and atomic rejected-step behavior also pass. Release suites `muscle_dynamics`, `finite_inertia` and `muscle_activation` passed 11 tests. Character muscle registration, coupled time-varying activation work, physiological parameter fitting and organ-level dynamic validation remain required.

## Excitation-driven muscle dynamics and parameter work

`InertialBody::step_driven_muscle` couples `MuscleRegionDrive` histories to the rate-dependent dynamic step. A symmetric split advances activation exactly over half a step at fixed initial geometry, moves the solid with midpoint activation held fixed, then advances activation another half step at fixed final geometry. Excitation remains constant over each supplied step. Caller histories must equal the existing activation of all elements in their regions; mismatches reject explicitly rather than silently replacing an authoritative state. Regions omitted from the drive retain their prior activation.

`DrivenMuscleStep` reports activation parameter work (the potential-energy change at fixed geometry during both activation stages), velocity-correction work and total signed defect `delta(K+U)-activation_work-correction_work`. Parameter work can be negative with the chosen active potential gauge under contraction; it is not ATP consumption or muscle heat. The split stages the entire solid plus all activation histories, committing only after all stages and the final work guard pass. Missing/duplicate regions, history mismatches, constitutive errors and work rejection preserve caller histories, element activations, positions and velocities. This removes the fixed-activation limitation for this new entry point; the ordinary `step_muscle` continues to hold activation fixed.

A rising-excitation synthetic single-tetrahedron test at 20/40/80 steps over 0.02 s obtains trajectory difference ratio 4.022327064164215 and cumulative work defects 2.708279293954204e-8, 6.740503906009672e-9 and 1.6812095052698081e-9 J. The activation at 0.02 s is independently checked against its exact exponential. Constant excitation equal to activation produces exactly the held-activation trajectory and zero activation work; release is checked against the separate fall-time exponential. Atomic failure tests include a missing later region, inconsistent history and tight work rejection. These are numerical and synthetic constitutive checks. The coupled method has not been calibrated to human muscle or connected to anatomically registered character musculature, neural excitation or dynamic pelvic supports.

## Stationary supports and dynamic layered sphincter

`InertialBody::new_with_fixed_supports` retains the original fully fixed nodal constraints and requires exactly zero initial velocity at supported nodes. Both conservative Verlet and rate-dependent midpoint motion leave those positions and velocities unchanged. The original `new` continues to reject pins. `muscle_support_reactions` returns the forces required to balance muscle, elastic, applied, gravity and plane forces at the stationary constraints; unconstrained nodes return zero. Supports do no mechanical work here. Moving support geometry and partial-axis constraints remain unsupported.

The actual bonded two-layer `sphincter_layers` mesh now runs under driven inertial mechanics without removing its three original outer midlength supports (nodes 120, 128, 136). Both layers use optional active length dependence and separate synthetic IAS/EAS excitations 0.2/0.1, density 1000 kg/m3 and rise/fall times 0.002/0.005 s. These deliberately fast numerical scenario constants are not physiological measurements. Over 0.0004 s, 100/200/400 time steps give a full-state difference ratio 3.995185012998636. The finest accepted lumen volume is 4.968357977200628e-6 m3, below the initial 4.969325665968397e-6 m3. Maximum per-step work defects are 1.0101673077668976e-13, 1.259671381913607e-14 and 1.5726596331659598e-15 J. Tests verify exact stationary support state, nonzero support reactions and independent gravity reaction at reference geometry. Release suites `muscle_dynamics` (7) and `finite_inertia` (4) passed.

`examples/sphincter_dynamics.rs` adds a CSV driver for eight 0.0004-second phases (four excitation, four release), with lumen volume, activation, kinetic energy and work ledgers. Its full example run remains unverified: after the above tests passed, normal Cargo compilation encountered a missing unrelated source module `crates/physics/src/liquid/thixotropy.rs`. An attempted direct compilation found only an older cached rlib without the new APIs, so that route did not verify the example either. No unrelated source was altered to bypass the failure. The verified short ring test does not establish later release behavior, anatomical pelvic accuracy, contact closure or human parameter validity.

## Verified dynamic excitation/release driver

The missing unrelated source file is now present and normal Cargo execution of `sphincter_dynamics` succeeds. The driver accepts `--release-phases=N` (1..1000) and `--substeps=N` (1..100000); unknown arguments and out-of-range counts fail explicitly. Four excitation phases are followed by the requested release phases, each lasting 0.0004 s. Default substeps 200 imply dt=2e-6 s; reducing substeps is not a stability guarantee. Output now includes the minimum element J and largest stationary support reaction alongside the existing activation/volume/energy ledgers. The stress API supplies geometric J only here; it does not include the new velocity-dependent stress correction.

`cargo run --release -p physics --example sphincter_dynamics -- --release-phases=40` completed 44 phases through 0.0176 s. Independent CSV inspection verified all fields finite and all recorded lumen volumes and element minima positive. The smallest sampled lumen volume was 4.905716544541e-6 m3 at 0.0088 s, after excitation was switched off at 0.0016 s. Final volume recovered to 4.962647472367e-6 m3 (initial 4.969325665968397e-6 m3). Final IAS/EAS activation was 0.004489313017926/0.002244656508963. Minimum sampled element J across the cycle was 0.9981676272487. Final accumulated work defect was -5.996125853199e-12 J, activation parameter work +4.416528583615e-5 J and rate-correction work -4.396713758630e-5 J. These small errors describe the chosen discrete scenario, not human physiology. Recovery is delayed by residual activation and tissue dynamics; no instantaneous return is imposed after excitation stops.

The synthetic activation time constants remain deliberately fast and the mesh/supports idealized. This run does not establish physiological relaxation time, luminal contact, muscle fatigue, neural reflexes, fluid filling or calibrated pelvic response.

The complete excitation/release regression repeats the cycle at 100/200/400 substeps per phase, checks exact independent exponential rise/release activation values, continued narrowing after excitation removal followed by recovery, stationary support state and final positive element J. Full-state trajectory differences give ratio 3.9932758339505923. Cumulative work-balance defects from the accepted first-phase state through the remaining cycle are 1.7796993512229263e-11, 4.4637020242848155e-12 and 1.11752922843741e-12 J. Release suites `muscle_dynamics` (8) and `finite_inertia` (4) passed, 12 total. This temporal refinement is not an independent experimental or spatial-convergence validation.

## Consistent rate-dependent spatial stress

`Body::muscle_stresses(velocities, law)` now reports current total Cauchy stresses including the active velocity correction, in addition to the existing passive, length-dependent active and pore-pressure stresses. It uses the same element-level correction Piola tensor as nodal velocity-force assembly and transforms it as `sigma_correction=P_correction*F^T/J`. Principal stresses, pressure and von Mises diagnostics are recomputed from the resulting tensor. The method validates nodal velocity count/finite values, parameters, geometry and incompatible active material assignments. It does not advance state.

This closes the prior diagnostic gap: `stresses_at` still intentionally reports the held-activation potential stress without a supplied velocity law, while `muscle_stresses` reports the corresponding rate-dependent total. The sphincter driver now uses the latter and emits `max_total_von_mises_pa` as well as J. Its normal eight-phase run completed; independent CSV inspection verified all columns finite and the stress maxima nonnegative. First/final maximum total von Mises values were 237.7260732857/227.6968343480 Pa for the synthetic scenario. These are constitutive diagnostics, not tissue-injury thresholds or human measurements.

An independent reference-tetrahedron test checks axial total stress `activation*T0*velocity_factor`, vanishing other components, rigid translation/angular-velocity invariance and equality between stress contraction power and negative nodal force power. A deformed two-layer ring test uses `v=rate*x`, for which the current velocity gradient is exactly `rate*I`; summing `current_volume*rate*trace(sigma_correction)` agrees with the negative assembled correction power for both shortening and extension. Release suites finite inertia (4), muscle activation (5) and muscle dynamics (9 at the first full run) passed; the additional deformed-ring power test passed separately, covering 19 distinct tests in total. Physiological fitting, spatial convergence, dispersed fiber recruitment and anatomical muscle registration remain open.
