# Soft tissues

`physics::tissue` is a dependency-free CPU XPBD model in local metre/kilogram/second coordinates. It extends the existing physics crate and renders through `SceneApp`.

Run the native abstract specimen demonstration:

```sh
cargo run -p voxy_app --example tissues
cargo run -p voxy_app --example tissues -- --smoke
cargo test -p physics --test tissues
```

Left to right: skin, buttock tissue, breast tissue, lip tissue, sphincter ring, penis shaft. Space pauses; Esc exits. Skeleton attachments oscillate, and muscle activation cycles automatically. These are abstract material specimens, not anatomical meshes. The render mesh uses colored edge ribbons to expose the constraints.

The blue shaft is an abstract square-section tetrahedral volume with its base attached to a moving support. Gravity and lateral acceleration produce bending and secondary motion; passive edge and volume constraints restore its shape. It does not model erection, blood pressure or contact between organs. The green ring shows isolated sphincter contraction.

## Model

Each particle has position x, velocity v, and inverse mass w. Zero inverse mass is a skeleton attachment, moved explicitly with `move_pin`. Free tissue remains behind when an attachment moves, producing secondary motion. The integrator decays stored velocity by exp(-damping * dt), adds acceleration * dt, and predicts positions. External acceleration is not attenuated by the velocity damping factor; this preserves static load balance. Corrected velocities are reconstructed from position differences.

For each scalar constraint C and compliance alpha, the solver uses:

```
a = alpha / dt²
Delta_lambda = (-C - a * lambda) / (sum(w_i * |grad_i C|²) + a)
x_i += w_i * grad_i C * Delta_lambda
```

Multipliers are zeroed at the beginning of each timestep and accumulated across iterations. Distance constraints use C = |x_a - x_b| - L. Tetrahedral constraints use C = signed_volume - rest_volume, with exact analytic gradients. Signed rest volumes support either input winding. Small volume compliance models approximately incompressible bulk tissue.

Muscle edges use L = L_rest * (1 - 0.35 * activation), with activation in [0,1]. Returning activation to zero restores the passive rest length. The ring combines neighboring and second-neighbor muscle links; it has an actual open lumen, not a filled volume. This is a circumference actuator, not a pressure-controlled physiological sphincter model. Lip samples activate surface edges of an elongated volume.

The skin specimen is a cross-braced grid with longer-range links approximating resistance to bending. Bulk specimens are eight tetrahedra filling an octahedron, with two skeleton attachments. `Tissue::new` also accepts application-supplied topology, inverse masses and muscle-edge assignments. Presets differ in stretch compliance; their constants are illustrative and have not been fitted to measured human tissue. Distance compliance has units m/N; volume compliance has units m^6/J. Resolution changes require material calibration.

Use a fixed timestep, typically 1/240 s, and 24 iterations. `step` rejects steps larger than 1/30 s. Invalid inputs and numerical overflow preserve the prior solver state. Constructor validation covers indices, finite values, nonnegative material parameters and degenerate edges/cells.

## Contacts and limits

Contacts project free particles out of externally supplied spheres, including tissue particle radius, after each constraint sweep. Pinned particles take priority over contact and may overlap colliders. Constraints and contact can conflict, so use sufficiently small timesteps and adequate iterations. The demo currently drives attachment and muscle motion without external colliders; contact is covered by the solver regression tests.

There is no triangle contact, friction, continuous collision detection, tissue self-contact, mutual collision between separate specimens, mesh inversion barrier, tearing, anisotropic fiber constitutive law, anatomical rig, or calibrated nonlinear hyperelasticity. Sphere contacts are discrete and can tunnel. Strong deformation can invert tetrahedra. The simplified skin links do not reproduce a shell's dihedral bending law. This implementation is suitable for prototyping secondary motion, not clinical biomechanics.

Algorithm reference: Macklin, Müller and Chentanez, [XPBD: Position-Based Simulation of Compliant Constrained Dynamics](https://mmacklin.com/xpbd.pdf), 2016.

## Continuum skin and anatomical model

For the newer layered nonlinear skin shell, viscoelasticity, triangle contact and real female anatomical surface demonstration, see [skin.md](skin.md). The illustrative `TissueKind::Skin` remains the older edge-link sample.

For the separate continuum FEM pressure/active-stress specimens and their
verification, calibration and anatomical limitations, see [biomechanics](biomechanics.md).
Run `cargo run -p voxy_app --example biomechanics --release`.

## Rounded volumetric mesh

`physics::tissue::ellipsoid(center, radii, density, refinement, pinned_axes, material)`
constructs a conforming tetrahedral ellipsoid approximation for the same `Tissue`
solver. Density is in kg/m³ and radii are in metres. Refinement levels 0 through 3
produce 8, 32, 128 and 512 cells. Boundary edges are shared across faces, bisected
on a unit sphere and then scaled by the supplied radii. Radial tetrahedra fill
the resulting inscribed convex polyhedron without overlapping cells.

Node 0 remains the center. Nodes 1 through 6 remain the positive and negative
X, Y and Z axis endpoints. Only those six axis endpoints can be selected as
pins through this constructor. Every node receives one quarter of each incident
cell's mass, using the supplied density and actual discrete cell volume. A pin
has zero inverse mass in the existing solver; its physical inertia is not
integrated. With no pins, summed nodal mass equals density times mesh volume.
The polyhedral volume converges from below to the analytic ellipsoid volume;
the constructor does not rescale mass to a larger smooth analytic volume.

A render surface can use the actual boundary and bind with `EmbeddedSurface`.
This avoids requiring a shrunken interior shell to visualize the volume.
Material compliance is still a constraint parameter and is not automatically
resolution independent. No anatomical calibration is implied. Existing `sample` presets are unchanged. The body-motion demo now uses level-one
ellipsoids (32 tetrahedra per region), density 1000 kg/m³, and the existing
illustrative material presets. Its render mesh linearly subdivides the actual
physical boundary instead of shrinking a Loop-subdivided shell. Bone attachment
identities remain 3 and 6. The walk/jump test checks less than 5 percent volume
drift, dynamic center motion and settling. This is qualification of the selected
demo configuration, not a resolution-independent constitutive calibration.
The snapshot CSV additionally records each region's actual volume in m³.

## Physical bulk modulus

`Tissue::set_bulk_modulus(Some(K))` configures bulk stiffness in Pa on the same
volume constraints. For rest cell volume V0, cell compliance is |V0|/K and
stored volumetric energy is K(V-V0)²/(2|V0|). This makes the volumetric energy
scale with material volume rather than assigning equal compliance to differently
sized cells. `None` restores the earlier uniform `Material::volume_compliance`.
Invalid modulus or unrepresentable compliance preserves the complete old state.
A body without tetrahedra rejects a physical bulk-modulus configuration.

The pressure fixture pins a triangular base and loads the free apex with the
consistent generalized force -p*dV/dz. It checks volumetric strain p/K at two
geometric scales and two time steps. This qualifies the bulk term alone; edge
constraints are absent in the fixture. It does not establish a full isotropic
continuum material, mesh-independent shear, physiological tissue parameters,
or production deformation guarantees. The mannequin uses an illustrative bulk
modulus of 1 MPa; its edge presets remain uncalibrated.

The static edge-load test now checks its analytic extension with a 0.1 mm bound
at both 120 Hz and 240 Hz. Earlier acceleration damping produced an O(dt) load
bias that was admitted by a 7 mm bound; the corrected momentum update removes
that particular bias.

## Shared rounded geometry and continuum shear

`biomechanics::TetraMesh::ellipsoid(center, radii, refinement)` is the single
rounded-mesh generator. It returns only points, positively oriented cells and
an outward boundary. It validates topology through the existing mesh checker.
`tissue::ellipsoid` delegates geometry to it and adds density-derived inverse
masses, pins and compliant constraints. The same mesh can feed the existing
FEM owner with `mesh.into_body(pins, &material)` and its inertial owner with
per-cell densities. This does not create a second rounded geometry algorithm.

`cargo test -p physics --test tissue_continuum_geometry` checks the neo-Hookean
FEM on the rounded meshes at three resolutions. Homogeneous simple shear has
J=1 and energy W=mu*gamma²/2. Tests compare total FEM energy against the analytic
density times volume, with volume evaluated independently from the outward
surface integral. The summed nodal gradient's virtual work matches mu*gamma*V,
and its resultant is zero. Other checks compare lumped masses across owners and
confirm the boundary is accepted by the existing conforming refinement method.

This qualifies affine energy and force response on shared geometry. The moving
mannequin still uses compliant tissue dynamics. The FEM inertial owner now accepts prescribed support targets with explicit
actuator work and an energy-defect guard. That path is not yet connected to the
moving mannequin; selecting stable FEM substeps and preserving atomic frame
publication still require integration.

## Prescribed FEM supports

`InertialBody::step_with_support_targets(targets, dt, tolerance)` accepts one
next position for every pinned node, rejects duplicates or free-node targets,
and shares the existing Verlet integration path with stationary/free-body steps.
Free nodes receive their two force kicks and preserve inertia. Pinned nodes
follow linear segments at velocity (x_next-x_previous)/dt.

The actuator work is independently evaluated from endpoint reaction forces:

```
W_reaction = sum_pins [0.5*(g_before+g_after) - m*a] dot dx
W_pin_kinetic = sum_pins 0.5*m*(|v_next|²-|v_previous|²)
W_support = W_reaction + W_pin_kinetic
energy_defect = delta(K+U) - W_support
```

Here g is the potential gradient, including the existing elastic, contact,
cavity and dead-load contributions; uniform acceleration uses its separate
-m*a potential gradient. The pin velocity change is an explicit actuator
impulse. Thus stopping a moving support books negative kinetic work instead of
silently retaining its previous velocity or deleting kinetic energy.

The energy tolerance bounds endpoint-work quadrature and Verlet error. It is
not an actuator energy budget, a continuous-support acceleration law, or an
unconditional stability guarantee. Inversion, gap crossing, invalid targets
and excessive work defect roll back the full owner. The stationary step and
muscle integration/reaction APIs reject nonzero prescribed pin velocities; the
caller must explicitly stop pins through the prescribed-support path before
returning to those APIs. Moving-support and rate-dependent muscle integration
are not yet combined into a single work-qualified step.

The support-step report exposes reaction work separately from pin kinetic work.
For its defect calculation, pin kinetic changes cancel analytically against the
same actuator impulse work. Free-node kinetic change is evaluated separately,
so large prescribed pin kinetic energy cannot erase an elastic/Verlet defect.
If adding the work components loses a component larger than the declared
absolute energy tolerance, the step rejects with full rollback. A smaller
component remains available in its separate report field, although the derived
total may not resolve it. This avoids rejecting roundoff-scale rest-strain work
during ordinary rigid translation. A regression
uses fast prescribed compression to exercise this receipt-representability gate.

The shared Verlet drift also checks the full tetrahedral volume path. Signed
determinants along a linear nodal segment are cubic polynomials. Their endpoints
and all interior stationary points are evaluated with orientation normalization
and a floating-point margin. Thus two positive endpoint volumes cannot hide an
intermediate singular or inverted volume. Tests include quadratic collapse,
a cubic path with a negative interior minimum, and an admissible quarter turn.
This path check applies to free and prescribed-support Verlet steps. It does not
extend the separate rate-dependent muscle midpoint integrator's path contract.

### Maxwell relaxation with inertial supports

`InertialBody::new_viscoelastic_with_supports` accepts committed Ogden-Maxwell
histories; `step_viscoelastic` splits exact held-deformation relaxation around
frozen-history velocity Verlet. The returned receipt separates support work,
released Maxwell heat, and the mechanical/relaxation energy defects. Heat is
returned to the caller; this API does not update a temperature owner.

Each relaxation uses the actual committed memory change, so an increment lost
to rounding cannot create nominal heat. The complete pose, velocity and material
history is staged and published only after independent defect checks pass.
Classical inertial steps reject history-bearing material. Experimental HGO
stress memory is excluded from this passive-energy integration path.

`viscoelastic_inertia` checks analytic simple-shear relaxation, held-strain
subdivision, objectivity, driven work/heat balance, free-node inertia and rollback.
The neutral skeleton-driven display now owns Ogden-Maxwell continuum bodies.
Its abstract material specimens retain XPBD, with each specimen holding one solver
owner. Each 240 Hz pose interval starts with four subdivisions, scaled by a per-body
refinement predictor. The predictor starts one level coarser than the previous
interval's finest accepted step. Every trial retains the same per-time energy
budget and path checks, with at most eight total refinement levels.
Accepted/rejected trials and peak depth are committed with the energy ledger. A complete display frame publishes
pose, velocity, material histories, energy receipts and clock together.

The render surface is embedded in the solved tetrahedral boundary. The snapshot
CSV records mechanical energy change, support work, released heat and numerical
defect independently. The neutral demo enables cell-local sensible heat with illustrative specific
heat 3500 J/(kg K) and initial temperature 310.15 K. Released heat is deposited
in the same mechanical owner; temperatures are derived from energy and capacity. Material parameters and mannequin dimensions are illustrative;
they are not a calibrated anatomical model.

The existing `body_motion_snapshot --cpu-bench [TRACE.csv]` entrypoint measures
the full twelve-second skeletal physics cycle without GPU work. The optional
trace records offsets and volumes at 20 Hz for trajectory comparisons.

At held-pose relaxation only Maxwell histories change. The independent defect
check therefore sums cell-volume-weighted changes of stored Maxwell branch energy
and compares that change with released heat. Equilibrium elasticity, gravity,
contact and other position-dependent potentials stay fixed and cancel exactly.
This avoids assembling full-body force gradients four times per split step and
avoids subtracting large unchanged global potentials to recover small viscous
energy losses. The subsequent mechanical stage still validates complete forces,
potentials, support work and the positive-volume path.

`enable_maxwell_thermal` optionally gives the inertial owner a reference-temperature
thermal inventory per cell. Capacity uses the same reference mass as nodal inertia.
Energy increments are stored relative to the immutable initial temperature, with
compensated accumulation; temperature is a read-only derived value. Reinitializing
an existing inventory is rejected. A step publishes mechanical pose/velocity,
Maxwell memory and sensible heat together, including a separate thermal deposit
defect in its energy admission. Failures preserve all of these fields.

When thermal storage is enabled, `viscous_heat_j` is a receipt of energy already
deposited internally and must not be deposited a second time by the caller.
Without thermal storage the caller remains responsible for that energy. Spatial
conduction, exchange with liquids/air, and temperature-dependent constitutive
parameters are still separate, incomplete work.

`conduct_maxwell_heat` exchanges sensible energy across explicit cell links with
conductance in W/K. It uses the same exact finite-capacity pair transfer as the
liquid film. Forward half-steps followed by reverse half-steps give a symmetric
thermal network update. Relative sensible energy may become negative during
cooling; derived Kelvin temperature must stay positive. The entire exchange is
staged, including overflow and energy-defect admission. Duplicate/self links and
invalid controls are rejected before mutation.

The caller must supply conductance from geometry, conductivity and contact
resistance. This API does not generate face conductances from an arbitrary FEM
mesh and is not yet called automatically by the neutral demonstration. Solid and liquid thermal ownership can be connected with the exchange API below.

`exchange_maxwell_film_heat` now connects owned solid-cell sensible energy with
`ThermalFilmMixture` in one closed transaction. Contacts specify (solid cell,
film cell, conductance W/K). Forward/reverse half-steps use the shared exact pair
law; dry film cells insulate. Capacity comes from the film's canonical component
mass calculation, and the solid books the opposite of the film's actual energy
increment. Requested/actual transfer error and compensated solid deposit error
consume a shared absolute tolerance. Both owners remain unchanged after a late
failure. Contact topology and conductance are still caller-supplied; the neutral
mannequin does not automatically generate or execute these contacts.


`SolidFilmBinding` binds the complete exterior triangle boundary to its owning
FEM cells, using the existing tetrahedral topology validation. It checks reference
mesh compatibility rather than unique runtime object identity. Film triangles
retain boundary order; stale deformed geometry is rejected. Conductance is the
current triangle area divided by caller-supplied areal thermal resistance in
m² K/W. Explicit `ThermalFilmMixture::update_geometry` preserves canonical mass
and sensible heat, but provides no wet-surface mechanical work law. The neutral
mannequin does not yet execute this liquid coupling automatically.

`SolidFilmBinding::internal_heat_contacts` derives isotropic cell-centred
conductance across shared tetrahedral faces from current area and normal centroid
distances. Two half-cell resistances add in series. Degenerate or same-side cell
geometry and nonpositive conductivity are rejected. This monotone two-point
network preserves the owned energy exchange contract, but lacks non-orthogonal
consistency correction and is not a qualified arbitrary-mesh continuum solver.

The neutral FEM mannequin now executes internal cell conduction using illustrative
conductivity 0.5 W/(m K). Each 240 Hz skeleton interval has a thermal half-step
before and after its mechanical subdivisions. Conductances are recomputed at
each endpoint geometry. Both thermal half-steps belong to the existing staged
frame, so a later mechanics/geometry/thermal failure preserves the whole frame.
Conduction numerical defects are accumulated separately from mechanical work
and Maxwell heat receipts. Conductivity does not feed back into the illustrative
constitutive law. This is symmetric operator splitting, not a qualification of
continuum spatial convergence on the non-orthogonal tissue mesh. Liquid coupling
is still not executed automatically.

The mannequin now retains one immutable `Arc<SolidFilmBinding>` per tissue
region. Boundary ownership and interior shared-face adjacency are audited once
at construction and reused by staged frame copies. Contact evaluation checks
reference-mesh compatibility and recomputes current geometric conductance without
reconstructing adjacency maps. This removes repeated topology construction,
without changing the numerical conduction law. No throughput claim is made.


## Prescribed translating plane contact

`InertialBody::step_with_moving_plane(targets, next_offset_m, dt, tolerance)` advances the existing installed frictionless penalty plane at fixed normal and stiffness. Supports may be prescribed together with the plane, or remain stationary. `step_viscoelastic_with_moving_plane` uses the same contact path inside the atomic Maxwell/history/thermal step.

`DrivenSupportStep::plane_work_j` is independent actuator work, derived from the endpoint contact-potential derivatives with respect to plane offset. It is separate from pin work and viscous heat. Mechanical energy closure subtracts both pin and plane work; the viscoelastic receipt additionally accounts for released heat. Accepted node positions, velocities and plane offset publish together; any rejected trial retains the previous plane, histories and thermal storage.

This API prescribes an external obstacle: it does not represent a finite-mass obstacle or update its momentum. Contact is a nodal boundary penalty and allows stiffness-dependent penetration. Rotation, triangle-mesh contact, friction and character-surface coupling are not implemented by this API. Contact activation can invalidate a coarse trapezoidal-work trial; callers must refine the time segment rather than replace actuator work with observed energy change.

Qualification: `cargo test --release -p physics --test moving_plane_contact`. The tests cover analytic stationary-node work, co-moving pins/plane, Galilean-equivalent impact, independent work/momentum closure, time refinement and atomic history/heat/plane rollback.


## Prescribed plane rotation

`step_with_plane_motion(targets, next_plane, dt, tolerance)` and its `step_viscoelastic_with_plane_motion` counterpart extend the same contact step to offset changes and the shortest rotation of the plane normal. Stiffness must stay unchanged. Antipodal normals are rejected because the rotation axis is not determined by those endpoints; supply intermediate planes.

Receipts retain `plane_translation_work_j` and `plane_rotation_work_j` separately, with their sum in `plane_work_j`. Rotation work integrates the angular contact-potential gradient `sum(k * gap * (normal cross position))` against the prescribed angular increment. It is not inferred from the observed energy change. Unrepresentable lost work components and energy-defect failures preserve the original owner.

For boundary nodes with two nonnegative endpoint gaps, the step checks a conservative gap-curvature/monotonicity bound along spherical normal motion and linear nodal/offset motion. Unresolved potential intermediate contact rejects and needs temporal subdivision. This is an admission guard for the plane primitive, not triangle-mesh CCD or a complete character collision system. The existing penalty/contact and temporal convergence limits still apply.


## Prescribed finite triangle surfaces

`PrescribedTriangleSurface` owns immutable vertex poses and stable shared face identity. `with_positions` creates a validated next pose while retaining topology and barrier controls. It uses the same triangle-minimum barrier and closest-feature/conservative-advancement kernels as internal tissue contact, with a positive separation floor and activation distance. The per-pair coefficient remains a discrete uncalibrated law, not a mesh-independent tissue material.

`InertialBody::set_prescribed_surface` installs/removes the obstacle and returns the parameter-induced potential change. `step_with_surface_motion` and `step_viscoelastic_with_surface_motion` move its vertices linearly together with optional support targets. They integrate opposite feature gradients on body and obstacle, retain independent `surface_work_j`, check simultaneous swept separation, and publish body state and obstacle pose together. Any rejected path/work/history/heat trial retains the previous admitted state. A new asset/topology owner needs explicit installation; motion cannot silently replace it.

The obstacle is kinematic: its work is external actuator work, and no finite obstacle momentum is advanced. The law is two-sided and frictionless. Current evaluation scans cross-surface pairs with geometric lower-bound culling; scalable broad-phase indexing, contact masks, calibrated weighting and imported-character binding remain outstanding. This solver extension does not by itself qualify native rendering or realtime character collisions.

The tissue demonstration can now bind a finite prescribed contact surface atomically
across all continuum regions and advance a combined bone-palette/surface sample.
Adaptive retries interpolate both support targets and obstacle vertices; frame
rollback includes the obstacle pose and clock. Mesh actuator work is accumulated
separately and included in the external-work energy receipt. Qualification is in
`artifacts/adaptive-tissue-surface-motion-2026-10-05/`. The imported character
snapshot has not yet supplied its collision geometry to this interface.

The existing snapshot example now has `--cesium --contact`, sampling the imported
world-space mesh on the same clock as the bone palette. The full 3273-vertex,
4672-triangle character mesh fails initial contact admission against the current
pad placement (`closed surface contact gap`). A successful contact visualization
still requires explicit attachment-region exclusions or replacement of the
intersecting surface regions. Probe evidence is in
`artifacts/imported-character-contact-admission-2026-10-05/`.

`PrescribedTriangleSurface::with_contact_faces` defines an immutable contact
mask in original triangle order without renumbering geometry or gradients.
Changing this authored domain creates a new contact owner; `with_positions`
retains it during animation. Both static forces and swept admission use the
same mask, while geometry validation still covers excluded triangles. Current
CesiumMan attachment domains remain unauthored.

For distinct attachment domains, `TissueDemo::bind_contact_surfaces` installs
one immutable contact owner per continuum region. The matching
`advance_with_palette_and_surfaces` samples regional obstacle poses together
with the skeleton palette. Counts must match exactly; owner swaps reject and
roll back every region and the frame clock. The single-surface API shares this
implementation. Qualification: `artifacts/regional-tissue-contact-2026-10-05/`.

The CesiumMan example now authors separate reference-space attachment boxes,
excluding 32/115/132/157 original triangles per region. Initial binding succeeds,
but the actual contact animation rejects frame 52 (0.216666667 s) on the work
balance guard. Five partial frames are diagnostic output; the full clip remains
unqualified. Evidence: `artifacts/imported-regional-contact-2026-10-05/`.

Frame-52 contact diagnostics show approximately quadratic local work-defect
convergence, but raising the refinement limit to 14 still rejects the same
frame later within its staged interval. The limit remains 12; increasing it is
not treated as the fix. Measured receipts and experiment logs are in
`artifacts/imported-contact-refinement-2026-10-05/`. Optional
`VOXY_CONTACT_REJECTION_TRACE` probes isolated clones after normal rejection;
its relaxed diagnostic trials are never published.

`nearest_active_contact` exposes read-only source/body face identities, closest
barycentric features and distance/gap for diagnostics. Frame-52 measurements
identify character triangle 3062 and tissue face [3,10,15], with about 11 micrometres
of residual gap and 5.55% pinned closest-feature weight. The identities match at
the rejected trial's endpoints; this alone does not prove a constant feature
throughout the interval. Evidence and limitations are in
`artifacts/imported-contact-features-2026-10-05/`.

`PrescribedTriangleSurface::normal_stencils` exposes active positive normal
barrier blocks with original face IDs and barycentric feature weights. Their
`apply` operation includes equal/opposite body and obstacle action and a joint
translation nullspace. These are frozen-feature PSD preconditioner blocks,
not a full geometric Hessian. Curvature is shared with the existing internal
primitive law. Qualification against independent force differences covers gaps
of 14 mm, 1 mm and 11 micrometres. The implicit dynamic solve remains to be
implemented; current imported contact admission is unchanged.

`InertialBody::step_implicit_with_surface_motion` now provides a bounded implicit
midpoint solve for time-independent material with prescribed supports and mesh
motion. Contact-normal/inertia preconditioning and guarded backtracking retain
continuous separation and tetrahedral path admission. Work is computed from
midpoint reactions and obstacle feature gradients, then independently compared
with endpoint energy; failures publish nothing. Constant-acceleration, contact
impulse, work/rollback and 100-step Galilean fixtures pass. Maxwell/thermal
integration and imported frame-52 qualification remain outstanding. Evidence:
`artifacts/implicit-prescribed-contact-2026-10-05/`.

`step_viscoelastic_implicit_with_surface_motion` now runs the midpoint mechanical
solve inside the existing Maxwell/thermal transaction with unchanged split
budgets. The tissue demo uses it for prescribed mesh motion and shares all
regional retry/frame rollback logic. History/heat rollback and held-pose state
equivalence pass. The actual imported clip still rejects frame 52, now on the
implicit work guard; replacing the integrator alone has not qualified it.
Evidence: `artifacts/imported-implicit-contact-2026-10-05/`.

Midpoint velocity/drift reconstruction now uses the impulse equation and mean
velocity, avoiding tiny-step division of world-coordinate subtraction noise.
A dt=1e-10 s analytic regression retains a representable impulse when drift is
below coordinate resolution. Loose diagnostic energy budgets no longer bypass
nonlinear force equilibrium. 27 physics and 18 tissue tests passed. Corrected
frame-52 measurements match the remaining mechanical defect (3.1392e-10 J) to
independent contact midpoint quadrature error (3.1395e-10 J), while trajectory
Simpson integration reduces that contact error to 4.44e-15 J. Consistently
path-averaged contact forces/work are the next step; the full clip still rejects.
Evidence: `artifacts/imported-implicit-impulse-2026-10-05/`.

The implicit contact solve now uses three-point Gauss path-averaged gradients
consistently for forces, support reactions and obstacle work. Its search
objective has the same averaged body gradient, independently checked by finite
differences. Immutable quadrature obstacle poses are prepared once per solve;
32 trajectories match fresh evaluation exactly. 28 physics, 1 prepared-path
and 18 tissue tests passed. The actual imported run still rejects frame 52;
its third region uses fewer admitted prefix subdivisions, but the full clip
is not qualified. Evidence: `artifacts/imported-path-averaged-contact-2026-10-05/`.

Path contact now uses five-point Gauss integration. A submicrometre regression
covers gap reduction from 0.435 to 0.297 micrometres and path-work error below
1e-11 J. The previous three-point actual rejected trial had quadrature error
7.42e-10 J; eight-interval Simpson was also insufficient in that regime.
29 physics and 18 tissue tests passed. The imported full run still rejects
frame 52, now on nonlinear nonconvergence. Next qualify path-weighted normal
preconditioning rather than relying only on midpoint curvature. Evidence:
`artifacts/imported-five-point-contact-2026-10-05/`.

The implicit unknown now uses displacement from the initial pose, eliminating
world-coordinate subtraction from inertial residuals. A tightened 1e-18 J
tiny-step regression requires force refinement below world-position resolution
and passes. 29 contact/viscoelastic and 18 tissue tests pass. A fresh two-second
non-contact skeletal/FEM-pad run renders 41 frames on Metal. The actual
imported surface-contact regression remains red at step 52 (line search);
diagnostic nearest gap is 4.0866e-8 m and rejected state remains unchanged.
Evidence: `artifacts/local-displacement-tissue-2026-10-05/`.

Contact search now retains coupled frozen-feature normal blocks using bounded
preconditioned conjugate gradients on inertia plus the PSD normal operator.
An analytic rank-one inverse test passes with unequal masses, rotated normal
and pinned DOFs. 29 physics and 18 tissue tests pass. At the imported step 52,
nonlinear search now reaches a solution, but the run remains rejected on
quadrature nonconvergence: initial gap 42 nm, 16-panel defect 1.2394e-6 J
versus 3.0518e-10 J budget. Adaptive quadrature is still needed; full contact
animation is not qualified. Evidence:
`artifacts/coupled-contact-direction-2026-10-05/`.

Implicit contact quadrature now bisects the path interval with the largest
independently estimated work error, freezing that partition during each solve.
Endpoint energy is only an error estimator; actuator work remains force-based.
An endpoint test resolves gap reduction 42 nm to 42 pm to below 1e-10 J work
error. 76 library tests pass (2 manual benchmarks ignored), 29 contact/Maxwell
and 18 tissue tests pass. The prior expected-success imported 52-step
regression now passes. A full Metal render commits 64 steps, then rejects step
65 at 0.270833333 s on line search. Six prefix frames are not a complete clip.
Evidence: `artifacts/adaptive-imported-contact-2026-10-05/`.

Triangle conservative advancement now uses a convex relative-velocity bound
instead of summed absolute speeds. Analytic common-translation/narrow-gap
and swept-crossing tests pass; 78 library, 29 contact/viscoelastic and 18 tissue
tests pass (2 manual benchmarks ignored). Imported step 65 still rejects.
Optional guard/CCD traces show all 48 trials pass internal gap and volume but
fail external contact: face [10,7,11] versus source 1308, computed gap
-6.0715e-18 m at path time 0.9999999995951657. This is an endpoint-resolution
failure rather than iteration-budget exhaustion. Next refine quadrature at
the CCD failure location before equilibrium admission. Evidence:
`artifacts/relative-motion-contact-2026-10-05/`.

CCD rejection times now guide quadrature refinement during blocked nonlinear
searches, sharing the original admission kernel. An immutable prepared motion
owns the exact start/end pair and swept index, reused across search trials.
Crossing-time/local-refinement and stale-source false-negative regressions
pass; 80 library, 29 contact/viscoelastic and 19 tissue tests pass (2 manual
benchmarks ignored). Optional failed diagnostic subdivisions are isolated so
they preserve the original error and state. Full imported contact remains
rejected at step 65: final 32-panel defect -2.9943e-6 J versus 3.0518e-10 J.
Next compare solved and impulse-reconstructed endpoints; extra quadrature
alone is not demonstrated sufficient. Evidence:
`artifacts/prepared-ccd-guided-contact-2026-10-05/`.

A rejected-step comparison found that impulse-reconstructed endpoints differed
from solved path endpoints by up to 2.54e-14 m, changing potential by about
3e-6 J against a 3.05e-10 J work budget. The solver now preserves its solved
endpoint, keeps impulse-based velocity and additionally controls residual
impulse work. No admission budget was increased. 80 library, 31 contact/Maxwell
and 19 tissue tests pass (2 manual benchmarks ignored), including strong moving
contact at a 1 micrometre gap and 1e-9 J energy admission. Full imported contact
still rejects step 65, now on nonlinear nonconvergence. Evidence:
`artifacts/canonical-midpoint-endpoint-2026-10-05/` and
`artifacts/implicit-endpoint-consistency-2026-10-05/`.

Closest-feature displacement now uses compensated relative affine coordinates.
Two analytic large-translation tests and a public translated contact energy/force
regression pass. 82 library (2 manual benchmarks ignored), 32 prescribed-contact/
viscoelastic, 52 related geometry/contact and 19 tissue tests pass. Full imported
contact still rejects step 65 on nonlinear nonconvergence. A fresh two-second,
41-frame neutral skeletal/FEM close-up completes; --cesium now honors --close-up.
These are separate diagnostic regions, not integrated anatomical skin. Evidence:
`artifacts/affine-contact-precision-2026-10-05/`.

The shared-trajectory full imported run still rejected step 65. The implicit
solver now preserves sub-ULP displacement in a local contact evaluation frame;
material/world endpoints and independent CCD/energy admission remain mandatory.
A regression shows 2e-18 m contact sensitivity retained locally while the same
world-coordinate addition rounds away. 85 library (2 manual benchmarks ignored),
32 prescribed-contact/viscoelastic, 52 related contact/geometry and 19 tissue
tests pass. Full imported runtime qualification is pending. Evidence:
`artifacts/local-contact-frame-2026-10-05/`.

The local-frame full run rejected step 65 on line search; its completed trace
identifies source face 1308 and near-endpoint CCD failures. CCD now evaluates
relative linear pair trajectories in a moving first-vertex frame, independently
guarding original world endpoints and preserving first rejection locations.
A binary analytic 2^-40 m open-gap test under 2^20 m common translation passes.
88 library (3 manual benchmarks ignored), 32 prescribed-contact/viscoelastic,
52 related contact/geometry and 19 tissue tests pass. The full imported render
still rejects step 65, now with closed surface contact gap. Evidence:
`artifacts/moving-pair-frame-ccd-2026-10-05/`.

Stage-specific optional diagnostics identify local contact quadrature as the
step-65 closed-gap failure, before final world contact admission. A co-moving
body/obstacle regression reproduces an infeasible stationary initial guess.
The solver now tries its current-velocity midpoint predictor only after that
specific initial failure, requiring finite positions, internal gap/volume paths,
external CCD and valid local quadrature before using it. Failed guesses preserve
the original error and full state. The expected-success regression fails before
and passes after the fix; the infeasible predictor rollback regression passes.
88 library (3 manual benchmarks ignored), 26 prescribed-contact, 8 viscoelastic
and 19 tissue tests pass. Full imported runtime qualification remains pending.
Evidence: `artifacts/contact-stage-diagnosis-2026-10-05/` and
`artifacts/feasible-contact-predictor-2026-10-05/`.

The closed-gap velocity fallback passed imported steps 65 and 66, then rejected
step 67 on nonlinear nonconvergence. The solver now also uses the same feasible
inertial predictor when the stationary first evaluation is valid. Failed probe
geometry/evaluation preserves the original result; initial-guess changes do not
loosen equilibrium or final work admission. 88 library (3 manual benchmarks
ignored), 26 public contact, 8 viscoelastic and all 19 tissue tests pass. The full
imported clip is still under runtime qualification. Evidence:
`artifacts/inertial-contact-initial-guess-2026-10-05/`.

Unconditionally using any feasible velocity guess regressed the imported
example to a step-66 closed-gap rejection (the closed-gap-only fallback reached
step 67). Initial selection now requires a strictly lower finite solver
objective for a valid stationary evaluation; the same objective-value function
is shared with line search and reuses cached evaluations. The closed-gap
recovery case and all geometric/equilibrium/work gates remain. 88 library
(3 manual benchmarks ignored), 26 contact, 8 viscoelastic and 19 tissue tests
pass. Full contact runtime qualification is pending. Evidence:
`artifacts/objective-selected-contact-guess-2026-10-05/`.

Objective-selected full import still rejects step 66. Predictor observations
show internal gap/volume open but external CCD rejecting; its endpoint lies
62.9736 nm below the 100 micrometre separation floor (31.4860 nm at half dt).
The original error and full state survive traced probes. A feasible initial
contact restoration beyond current velocity is required. The existing example's
--contact-motion-bench also measures staircase sampling: at t=.275 s, estimated
maximum vertex speed changes from about 1.46 m/s on 1e-3..1e-5 s intervals to
3.79 m/s at 1e-7 s, then all 3273 vertices are unchanged at 1e-8 s. Phase/local
time and rendered skinning currently round to f32 before conversion to physical
f64 coordinates. This is a measured precision limitation, not proof of sole
failure causality. Evidence: `artifacts/infeasible-predictor-diagnosis-2026-10-05/`.

Closed initial quadrature now has a bounded mass-weighted normal restoration
probe after unsuccessful velocity initialization. It moves only free nodes,
recomputes closest features and never commits its pose directly. Its numerical
clearance is not a physical law change. Whole-path internal gap/volume and
external CCD must pass, followed by the unchanged nonlinear equilibrium and
independent world energy/work admission. Geometric pin/source preservation and
fully pinned rejection tests pass. A dynamic fixture with both prior guesses
closed now solves at fixed dt with 1e-10 J closure and an open swept path.
27 public contact, 88 existing library plus 2 new restoration, 8 viscoelastic
and 19 tissue tests pass. Full imported runtime qualification remains pending;
f32 motion sampling remains an identified limitation. Evidence:
`artifacts/restored-contact-guess-2026-10-05/`.

The restored-guess full imported run passed through step 68 and rejected
step 69 / .2875 s on line search. The expected-success imported regression
now covers 68 steps instead of 52, including prior initialization failures
at 65 and 66; its CPU run is pending. This is regression coverage for measured
progress, not a replacement for the unqualified full 480-step gate. Evidence:
`artifacts/restored-contact-guess-2026-10-05/render.log`.


## Double precision authored skeleton and contact provider (2026-10-05)

`Pose64` samples the existing immutable authored tracks and evaluates FK/skin
palettes without narrowing phase or intermediate transforms to f32.
`ModelSurface64` retains original imported vertex/index ordering and UNORM16
skin weights. The generic tissue solver accepts DMat4 supports internally;
legacy Mat4 entry points convert only at the boundary. A sub-render-scale test
checks exact prescribed pin motion, and analytic hierarchy/cubic tests qualify
the new interpolation.

The existing snapshot example exposes `--cesium --contact --wide-contact` to
select wide support and obstacle sampling together. Default `--contact` retains
legacy sampling. Neither mode establishes a full 480-step contact qualification.
The wide mode passed eight initial steps and rejects step 65 with nonlinear
nonconvergence in the explicit 68-step qualification gate. That diagnostic test
is marked ignored with the failure reason and remains runnable with `--ignored`.
This is not evidence that precision alone resolves contact convergence.

The benchmark detects nonzero motion in all 3273 imported vertices at a 1 ns
time increment in the f64 provider, while the f32 provider freezes at this
increment. This is a precision measurement, not a throughput measurement.
Evidence and exact test outcomes: `artifacts/wide-skeletal-contact-2026-10-05/`.


## Shared L-BFGS curvature in midpoint dynamics (2026-10-05)

The existing static L-BFGS two-loop recursion and positive-curvature admission
are now reused privately by implicit midpoint dynamics. At most 12 accepted
free-node displacement/residual pairs approximate missing material/geometric
curvature around the existing coupled inertia/contact-normal inverse.
No force law, 96-iteration limit, residual/impulse-work tolerance, CCD or final
energy admission was relaxed. Nonfinite or non-descent directions discard
history and retain the original positive search metric.

Verification: independent dense inverse-BFGS algebra, strict supported 100 MPa
tetrahedron work (1e-10 J), 92 unit tests, 8 static equilibrium, 28 contact,
11 tissue and 8 viscoelastic tests; 20 TissueDemo integration tests passed.
`imported_contact_sample64` builds both boundaries from one Pose64 and is shared
by runtime/tests. Its reference/phase/topology errors reject atomically, and
its outputs match separate sampling bitwise at the tested phases. The common
provider passed its 68-step regression (152.82 s); it is enabled by default.

The full rendered trial advanced through step 130 and rejected step 131
(t=0.545833333 s) with nonlinear nonconvergence. The 11 saved frames cover only
t=0..0.5 s. The full 480-step clip is still unqualified, and the matching
explicit long CPU gate remains ignored pending completion. A retry-after-line-
search-failure experiment regressed to step 65 and was removed.
Evidence: `artifacts/secant-dynamic-contact-2026-10-05/`.

### Full imported contact qualification (2026-10-05)

The explicit `wide_imported_contact_completes_full_clip` CPU regression passed all 480 steps at 240 Hz (the complete 2-second CesiumMan clip), including the final cumulative energy receipt check. The actual release test took 1221.58 seconds on this host while a separate GPU render was running. This is offline correctness evidence for this fixture, not real-time throughput qualification. Initial complete-trajectory admission is active; the previously failing step 131 now passes. No CCD, energy, residual or subdivision limit was relaxed.

Evidence: `artifacts/contact-frame-diagnosis-2026-10-05/full-clip-pass.log`, the qualified source snapshot and source hash. The separate Metal render remains live and is not yet full-clip visual evidence. Tissue specimens remain separate from character skin; anatomical calibration and production performance remain outstanding. The historical trace field `maximum_drift_roundoff_m` measures the full kinematic equality residual, not pure floating-point roundoff; current tracing names it `maximum_kinematic_residual_m`.

### Contact normal metric pruning (2026-10-05)

The normal-stencil builder now reuses the conservative separation bound already used by contact-force response. Guaranteed inactive barriers avoid the exact closest-feature calculation. Active stencil values/order match the unpruned oracle bitwise on oblique layered, large-translation and activation-boundary cases; closed-gap errors match. 139 focused physics checks and the 68-step imported contact regression passed. Five synthetic 500-call batches measured a median 21.75x speed ratio for this operation only; no whole-engine speed claim follows. The optimized 480-step CPU gate is live (session 52848), and the earlier full Metal render continues with the prior qualified binary (session 12276). Evidence: `artifacts/contact-normal-pruning-2026-10-05/`.

### Instantaneous quadrature pose staging (2026-10-05)

Quadrature samples now avoid constructing a swept BVH that their instantaneous force/normal queries never use. Public surface updates retain the cached swept index; temporary samples preserve validated coordinates, owner identity and instantaneous geometry/index, and explicitly clear stale motion cache. Force/normal results and fallback CCD decisions match the public-stage oracle, including errors. 140 focused physics checks and the combined 68-step imported regression passed. A synthetic 4096-triangle pose preparation benchmark measured about 1.29x for this operation only; full combined runtime qualification is pending. Evidence: `artifacts/contact-instantaneous-poses-2026-10-05/`.

The full baseline native renderer also completed on Apple M4 Max / Metal: all 41 frames from t=0..2 seconds, 1357.988445542 seconds simulation+render, exit 0. Full GIF and finite secondary-motion receipts: `artifacts/contact-frame-diagnosis-2026-10-05/`. This precedes the normal-pruning and transient-pose optimizations and does not qualify their performance.

### Deterministic native contact-region parallelism (2026-10-06)

Independent regions use a shared kernel with bounded scoped workers, while non-contact/WASM paths stay sequential. All workers join, errors are selected in authoring order, and publication remains atomic for the complete frame and clock. 21 integration checks, the additional parallel second-frame rollback/recovery case, imported eight-step complete-state parity, and the 68-step contact regression passed. Three paired initial eight-step region calculations measured median ~3.34x; full-clip/end-to-end speedup remains unproven. The normal-pruning-only 480-step gate also passed (1174.83 s); the transient-pose sequential gate and the new parallel full gate are distinct ongoing validations. Evidence: `artifacts/contact-parallel-regions-2026-10-06/`.

The sequential normal-pruning + instantaneous-pose 480-step gate subsequently passed with final energy receipts (1051.94 seconds). Both performance changes therefore have full-fixture sequential correctness evidence. The later parallel-region gate remains pending and must not inherit that qualification.

The full parallel-region 480-step gate passed with final energy receipts (766.94 seconds). This qualifies the imported fixture with bounded parallel execution; timings do not form a controlled full-engine comparison. Diagnostic work decomposition was added afterwards and passed 141 focused physics checks. An independent uniformly dilated tetrahedron oracle identifies a 0.169632432725650872 J midpoint potential-work defect with zero prescribed-surface/free-kinematic contributions and full rollback. Its contribution to the imported clip remains unmeasured. Evidence: `artifacts/contact-work-components-2026-10-06/`.

### Candidate path-averaged material force (2026-10-06)

The implicit prescribed-surface step now averages material, internal-contact and
fixed-plane gradients over the same frozen quadrature nodes as external contact.
Midpoint kinematics, support ownership, Maxwell staging, CCD, nonlinear limits
and independent endpoint energy/work admission remain in place. The material
search potential is the weighted sum of `(U(x(t))-U(x(0)))/(2*t)`; its derivative
with respect to a free midpoint displacement is the averaged gradient. This is
an [average-vector-field approach](https://arxiv.org/abs/1202.4555), evaluated
with finite quadrature, not an unconditional exact-conservation guarantee.

The independent dilation oracle still measures the original midpoint defect
above 0.1 J, while the new step admits the same motion at the unchanged 1e-10 J
tolerance and its reaction work matches independently evaluated endpoint energy.
A prestressed tetrahedron with moving support also passes free-coordinate
finite-difference checks of the search gradient. 142 focused physics checks
passed (five manual checks ignored). The full 480-step release fixture has been
launched but has not completed; the earlier full-clip qualification must not be
attributed to this changed force scheme. Source snapshots, baseline, logs and
live job handles are in `artifacts/material-path-average-2026-10-06/`.

That first averaged-force full gate subsequently rejected step 126 with
`implicit contact nonlinear nonconvergence` (453.05 s); it is not a qualified
replacement for the previously completed midpoint fixture.

### Material work quadrature refinement (2026-10-06)

Rejected endpoint energy/work admission now also estimates the integration error
of non-surface potentials. Material and contact reuse one frozen partition,
worst-panel refinement policy, 128-panel cap and 32-retry bound. Endpoint energy
is an error estimator; sampled gradient work remains the force/reaction source.
Nothing is published until the complete geometry and work checks pass.

Independent uniaxial neo-Hookean scalar oracles expose coarse five-point work
defects of approximately -6.55e-7 J at 80% extension and 5.95e-6 J at 50%
compression. Both motions now pass in the original 0.02 s step at 1e-10 J without
temporal subdivision. 143 focused physics checks passed (five manual checks
ignored), as did exact serial/parallel complete-state parity over eight imported
steps and the 68-step release contact/energy gate (17.27 s). These timings are
not a controlled comparison or real-time qualification. A distinct 480-step
traced gate is running for this refinement; the failed preceding source and all
earlier full-gate results do not qualify it. Evidence:
`artifacts/material-work-refinement-2026-10-06/`.

The refinement candidate subsequently also rejected imported step 126 with
nonlinear nonconvergence (367.68 s). It is archived, not enabled in the default
solver: the pre-experiment midpoint implementation and its original work-defect
regression were restored without changing other workspace work. The trace
localizes the difficult contact to free body face `[10, 7, 11]` and obstacle
triangle 1308 at its vertex, with zero pinned barycentric weight. The observed
open gap before the failed refinement is about 4.98e-8 m. Internal gap and volume
checks usually pass rejected line-search trials; external CCD rejects them.
This identifies a geometry/constrained-solve case for the next experiment, not a
reason to weaken energy or swept-contact admission. Experimental scalar and
short-clip passes remain evidence for their own narrower scopes only.

### Captured vertex-contact geometry regression (2026-10-06)

The archived averaged-force candidate was replayed in an isolated source copy;
it reproduced the step-126 failure (212.62 s) and captured its terminal leaf at
depth 12, dt approximately 2.543e-7 s. This does not change the default solver.
The test-only application observer preserves body/surface coordinates, velocities,
support targets, topology, contact masks and actual contact parameters. Its
constitutive Debug string is evidence, not a reloadable mechanics checkpoint.
JSON round-trip checks preserve coordinates bitwise at origins 0 and 1e6 and
preserve external CCD decisions without mutating the original body.

The portable geometry fixture is
`assets/animation/contact-fixtures/vertex-1308-predictor.json`. The existing
`body_motion_snapshot` example accepts `--replay-contact-json FIXTURE.json` to
inspect it without GPU, rig sampling or integration. Direct replay admits the
static start and stationary-body trajectory but rejects the velocity predictor;
the latter ends about 4.23e-8 m inside the required separation floor at body face
`[10, 7, 11]` and obstacle triangle 1308's vertex. The captured regression passes
in 0.18 s. Discrete gap samples are diagnostic only, not continuous admission
proof. The remaining issue is constrained nonlinear convergence, not demonstrated
false-positive CCD. Evidence: `artifacts/contact-leaf-fixture-2026-10-06/`.

### Last-iterate contact diagnostics (2026-10-06)

The isolated rejected candidate again failed at step 126 with nonlinear
nonconvergence (200.16 s). Read-only last-iterate logging captures body endpoints;
21 parsed poses are preserved in
`artifacts/contact-leaf-fixture-2026-10-06/terminal-body-poses.json`.
One observed open gap is approximately 1.61e-18 m, below coordinate ULP sizes
for that pose. Coordinate ULP comparison is not a distance-error bound and does
not establish the cause of nonconvergence. The isolated observer now also records
the exact obstacle triangle, behind a terminal-only diagnostic environment flag.
The default solver and its admission criteria are unchanged.

The diagnostic-only isolated 68-step clip passed in 11.15 s and preserved 2
exact obstacle-triangle last iterates in `terminal-observer/exact-triangle-poses.json`.
This short clip does not qualify the failing full-clip candidate.

### Adjacent-coordinate contact sensitivity (2026-10-06)

The portable `assets/animation/contact-fixtures/last-iterate-3062.json` contains
both captured body-edge/obstacle-edge terminal poses from the short isolated clip.
The native `body_motion_snapshot --contact-ulp-json FIXTURE` diagnostic evaluates
36 individual adjacent-f64 coordinate perturbations per pose. Positive-gap
classification changes in 2 and 4 trials respectively; ranges are approximately
[-7.60e-17, 1.38e-16] m and [-9.86e-17, 1.07e-16] m. Translating both triangles to
obstacle vertex 0 leaves the baseline native gaps bitwise unchanged.

`python3 tools/contact_precision_oracle.py FIXTURE` independently solves the
selected interior edge-edge closest-point equations with 90-digit Decimal
arithmetic on exact binary64 inputs. It confirms positive gaps approximately
3.1222638107997636e-17 and 4.234388982451942e-18 m; the native baseline distances
agree within 1e-20 m. This rules out a closed selected feature misreported as open
in these two poses, but does not prove all-feature exact CCD or explain the full
nonlinear failure. The regression checks both oracle agreement and sensitivity.
Both captured contact regressions passed. No solver force, convergence threshold,
contact floor, or energy admission policy changed.

### Isolated geometric contact search metric (2026-10-06)

A new isolated experiment replaces only the contact search metric. For a fixed
obstacle, the body barrier Hessian includes both barrier normal curvature and
closest-feature motion. Closest parameters are eliminated using the Schur
complement of the squared-distance stationary equations. A symmetric Jacobi
solve projects each body block to PSD before inertia is added. Force evaluation,
energy admission, continuous collision checking, step limits and residual
thresholds are unchanged. Unsupported or ill-conditioned feature metrics reject
explicitly; there is no silent change to the physical law.

Finite-difference force derivatives agree for stable interior edge-edge,
vertex-face and face-vertex configurations. A separate eigensolve check retains
positive modes and removes negative ones. The first edge test had a nonunique
closest feature; its failure is retained in the experiment archive, and the
replacement uses an unambiguous edge-edge configuration at the same tolerance.
The isolated 68-step imported clip passes in 15.77 s. A full 480-step clip has been
started; the short result does not qualify this metric or its underlying rejected
averaged-force candidate. The default solver is unchanged. Sources and logs are
in `artifacts/geometric-contact-metric-2026-10-06/`.

### Endpoint quadrature precision diagnosis (2026-10-06)

A separate ignored diagnostic exercises the native constant-normal barrier path
with endpoint gaps approximately 4e-12, 4e-15 and 4e-18 m. The test only checks
finite diagnostic results; a passing test does not claim work admission. The
4e-12 m case meets the 1e-10 J work tolerance with 20 panels. The two smaller
cases fail it even after 128 evaluated panels: final defects are approximately
4.77e-10 and -8.45e-8 J respectively. Best observed absolute defects are
4.11e-10 and 8.37e-9 J. The initial 32-refinement log reports the next unevaluated
partition's panel count; the extended observer corrects this to evaluated panels.

A separate scalar constant-normal calculation evaluates force directly from the
gap rather than reconstructing triangle coordinates. It admits all three cases
within the same tolerance, using 20, 40 and 60--61 panels respectively. This is
not a mesh simulation or proof of the full nonlinear failure's cause, but narrows
the next precision investigation to geometry/trajectory evaluation rather than
simply raising the refinement cap. Logs and the diagnostic source are preserved
under `artifacts/geometric-contact-metric-2026-10-06/quadrature-diagnosis/`.
The running full geometric-metric clip has reached step 108; it remains unqualified.

### Geometric-metric full-clip rejection and compensated-gap trial (2026-10-06)

The isolated geometric metric failed the full imported clip at step 127 with
nonlinear nonconvergence after 509.81 s. It is not enabled in the default solver;
passing derivative and short-clip checks did not qualify it.

The next isolated trial retains affine closest-feature residuals as compensated
pairs, including subtraction and product residuals. It evaluates the barrier gap
as (squared distance - squared minimum)/(distance + minimum), avoiding rounded
norm subtraction. The two captured edge gaps agree with the independent 90-digit
Decimal selected-feature oracle within 1e-28 m; the exact contact-floor test keeps
zero gap zero. This only qualifies those geometry checks. The compensated gap is
now wired into the isolated prescribed barrier behind
`VOXY_COMPENSATED_CONTACT_GAP`; the physical law, CCD and admission tolerances
remain unchanged. It has not yet qualified a full clip. Sources and evidence:
`artifacts/compensated-contact-gap-2026-10-06/`.

With compensated-gap evaluation enabled, 55 focused biomechanics tests pass
(4 manual checks ignored). The isolated 68-step imported clip passes in 12.06 s;
8-step serial/parallel complete-state equality passes in 1.88 s. A full 480-step
clip is now running with terminal geometry diagnostics and optional leaf capture.
These focused checks do not qualify the underlying averaged-force candidate or
justify changing the default solver.

### Compensated contact trajectory sampling (2026-10-06)

The next isolated numerical trial retains the residuals of body/obstacle linear
interpolation, then computes the affine closest-feature gap from those pairs.
Rounded poses still supply the broad phase and closest-feature parameters; CCD
and thresholds are unchanged. Force evaluation, refinement boundary energies and
normal metric curvature consistently use the retained coordinates. The trial
explicitly rejects combination with the unqualified geometric metric, whose
matrix currently assumes ordinary sample coordinates.

The same three native constant-normal endpoint cases now meet 1e-10 J work
admission using 20, 40 and 61 panels. This qualifies only this integration case:
the solver still has its original outer refinement limit. A deterministic
81-panel force-work regression compares identical sample nodes with and without
retained interpolation residuals. Against the independent scalar endpoint-law
oracle, errors are approximately 7.85e-12 versus -8.15e-9 J. Force integration
remains the source of work; endpoint energy is a test oracle only.

56 biomechanics tests pass with the trial enabled (4 manual checks ignored).
An imported short clip is being checked. The previously started full clip tests
gap compensation only and cannot qualify these newer trajectory changes. Sources
and results: `artifacts/compensated-contact-trajectory-2026-10-06/`.

The gap-only full imported clip terminated at step 123 with nonlinear
nonconvergence after 359.29 s. It remains rejected; its compensated-gap unit and
short-clip results are insufficient to promote it. The new trajectory trial is a
distinct compiled version and needs its own full qualification.

The compensated-trajectory trial passes its 68-step imported clip (48.89 s) and
8-step serial/parallel complete-state comparison (7.17 s). Timings are individual
runs, not a controlled performance comparison. Its distinct full 480-step imported
clip has now started with both compensation flags enabled and geometric metric
disabled. The default solver remains unchanged.

### Moving-obstacle work regression (2026-10-06)

A separate compensated-trajectory check holds the body fixed and moves the
obstacle toward an approximately 4e-18 m endpoint gap. Force-integrated obstacle
work agrees with the independent scalar barrier endpoint oracle within
7.85e-12 J. Each sampled body's and obstacle's vertical resultant also balances
within relative 1e-12. This covers externally driven surface motion rather than
only moving body vertices; it does not qualify arbitrary deforming rig surfaces.
The running full imported clip has reached step 84 and remains unqualified.
Additional library/contact rollback gates are being run for the same mechanics.

The same compensated-trajectory mechanics passed the library/contact gates: 103 passed (6 ignored), 31 passed (0 ignored).
The full imported clip remains a separate required gate.

### Bounded quarter-panel work refinement (2026-10-06)

The isolated next trial divides the worst work-defect panel into four intervals
instead of two. It uses the original 32 outer retries and 128-panel ceiling; with
less than three spare slots it retains a midpoint split. Inserted knots must be
strictly interior and ordered. Stored physical energy, force law and work/error
thresholds remain unchanged. The shared work-defect helper also serves material
quadrature; both branches retain their independent work estimators.

The three compensated native endpoint cases now explicitly assert admission
within 32 evaluations and use 22, 52 and 85 panels, respectively. The partition
regression checks capacity, strict order, preservation of existing knots and
unchanged source surface. 104 library tests pass (6 manual checks ignored), plus
31 prescribed-contact integration tests. A 68-step imported clip is being checked
with this separate version. The earlier running full clip does not enable quarter
refinement and cannot qualify this strategy. Sources and logs:
`artifacts/quarter-contact-refinement-2026-10-06/`.

The quarter-refinement version passes its 68-step imported clip in 22.38 s.
Separately, the older compensated-trajectory full clip without quarter refinement
has committed step 132, beyond the earlier candidate rejections at 123--127.
This is progress on the original failing sequence, not a full-clip qualification;
it remains running and must still finish all 480 steps and its cumulative checks.

### Oblique-motion objective and parallel qualification (2026-10-06)

The quarter-refinement version passes the 8-step serial/parallel complete-state
comparison bitwise (2.03 s). A separate finite-difference check perturbs all nine
body endpoint coordinates against a moving tilted obstacle at world origins
0 and 1e6 m. The objective endpoint derivative agrees with half the independently
evaluated averaged body force, as required by endpoint = 2*midpoint-start. The
check uses the actual representable perturbation interval and the stated
2e-5*(1+abs(expected)) bound. This validates objective/force consistency in those
stable-feature configurations, not arbitrary feature switches or a full rig.
The earlier full compensated-trajectory clip is still running and unqualified.

The compensated-trajectory full clip without quarter refinement failed at step
134 with nonlinear nonconvergence after 653.73 s. Passing the former 123--127
failure region was insufficient for promotion. Its full log is retained, and the
default solver remains unchanged. The separately checked quarter-refinement
version now has its own full 480-step gate running; its exact compiled source
snapshot is under `quarter-contact-refinement-2026-10-06/full-clip-source/`.

### Actual solver frame observation (2026-10-06)

The 90-digit selected-feature oracle first compared 21 published failed world
poses with rounded translations by the old body origin. No signs changed; the
largest absolute gap shift is approximately 3.30e-18 m, and one near-zero gap
changes by approximately a factor of five. These reconstructed translations are
not the solver's actual displacement-preserving local endpoints and do not prove
a nonlinear failure cause.

A read-only observer then captures the actual local and published world endpoints
for solved contact leaves. The 68-step clip still passes (41.88 s). All 326
recorded selected pairs are checked independently after adding vertex-vertex and
vertex-edge oracle branches alongside edge-edge and vertex-face. Maximum actual
local/world selected-pair energy difference in this successful fragment is about
3.99e-11 J. This measures frame sensitivity for these pairs, not the total model
or the failed deep leaf; no origin or admission policy has changed.

The quarter-refinement full clip has passed step 180 and is still running.
The running binary predates the read-only frame observer and cannot inherit its
new diagnostic outputs. Code, exact poses and logs are preserved in
`artifacts/contact-frame-precision-2026-10-06/`. The default solver is unchanged.

### Clean contact solver candidate (2026-10-06)

A separate `/tmp/voxy-contact-production-20261006` source copy removes the rejected
geometric metric, its optional stencil matrix and coupled-matrix experiment. It
also removes runtime algorithm switches: compensated gap/trajectory evaluation
and bounded quarter refinement are always applied there. Read-only diagnostic
switches remain separate from physical behavior. No main-worktree solver changed.

The clean candidate passes 102 library tests (6 manual tests ignored) and 31
prescribed-contact integration tests. Three retired geometric-metric tests are
not part of this candidate; their source and failed full gate remain archived.
The existing serial/parallel fixture now optionally exports the complete Debug
state after eight steps, allowing comparison across the experimental and cleaned
source copies. That comparison is in progress, not assumed from source similarity.
The predecessor full clip is still running and has reached step 216; it cannot
alone qualify the cleaned source. Snapshot and gate evidence:
`artifacts/production-contact-candidate-2026-10-06/`.

The experimental and clean copies exported identical complete Debug state after
the eight-step serial/parallel fixture: both files are 16,352,547 bytes, SHA-256
`8afbdfc78855f9d8772f248d08126deffcee858065f3f19471ce9dccb879bddf`.
This is an exact comparison for that fixture, not proof of full-clip equivalence.
The clean candidate now has its own independent 480-step full gate running,
without algorithm environment switches. Both copies remain isolated from the
main-worktree default solver pending qualification.

### Clean-candidate material integration gates (2026-10-06)

The clean candidate also passes 8 L-BFGS integration checks and 8 viscoelastic
inertia checks. These are additional to its 102 library and 31 prescribed-contact
checks (149 completed checks total, with 6 separate manual library tests ignored).
They cover solver and constitutive integration paths affected by material-force
averaging; they do not establish anatomical calibration or all-platform support.
The full experimental predecessor has committed step 288; the clean candidate
has committed step 96. Both full-clip processes remain running and unqualified.
The main-worktree default is unchanged.

### Experimental full-clip success (2026-10-06)

The isolated compensated-gap, compensated-trajectory and quarter-refinement
predecessor completes all 480 imported steps and the final independent body
energy-receipt assertions after 1317.68 s. This is the first successful full gate
for the corrected material-force averaging sequence in these experiments. It
qualifies this specific imported two-second clip, not all rigs or real-time
performance; approximately 22 minutes of wall time is a material performance
limitation. Its exact source snapshot and complete log are retained under
`artifacts/quarter-contact-refinement-2026-10-06/`.

The cleaned implementation has a distinct full gate still running (last observed
step 120). Eight-step state equality and predecessor success are insufficient to
claim the cleaned source's full result. No main-worktree default was replaced.

### Clean full-clip success and stronger energy audit (2026-10-06)

The cleaned solver completes all 480 imported steps in 750.55 s and passes the
original energy-ledger closure assertion. That assertion subtracts the reported
accumulated defect; it must not be described as proving zero physical energy drift.
Per-leaf admission separately enforces the requested time-scaled energy budget.
This run qualifies the cleaned numerical kernel for this clip, not real-time use.

A stronger test now independently evaluates each nominal frame's stored-energy
change minus actuator work plus dissipated heat, without subtracting reported
defect. It checks each body against 1e-5 J per 1/240 s frame (plus an explicit
floating-point summation allowance), and bounds the sum of absolute frame errors.
The audited 68-step clip passes. Absolute error sums for its four bodies are
approximately [6.64e-8, 7.85e-7, 8.64e-6, 6.16e-8] J, below the requested
6.8e-4 J per-body budget. This does not bound cancellation within individual
substeps. The completed full run predates this stronger test-only audit and does
not establish its 480-step aggregate result. Source, raw receipts and audit JSON
are preserved in `artifacts/production-contact-candidate-2026-10-06/`.

### Corrected kernel enabled in primary worktree (2026-10-06)

The cleaned, full-clip-qualified AVF/material/contact kernel is now active in the
primary worktree. Previous source files and SHA-256 records were preserved under
`artifacts/production-contact-candidate-2026-10-06/main-transfer/`. Other dirty
application work was retained. Primary physics gates pass 149 tests (six manual
tests ignored); primary example contact gates pass eight tests, including the
68-step strengthened energy audit, captured contact fixtures and eight-step
serial/parallel complete-state equality. Two manual example tests are ignored.
The strengthened energy audit was transferred without replacing other example
CLI and fixture functionality. A separate strengthened 480-step audit and a fresh
primary Metal render are still running; neither is reported as completed here.

### Strengthened full-clip energy audit completed (2026-10-06)

The cleaned candidate passes all 480 steps with the strengthened independent
nominal-frame energy audit in 1282.25 s (uncontrolled concurrent execution).
Absolute frame-error sums for the four bodies are approximately
[2.99e-5, 2.69e-6, 5.68e-5, 3.12e-7] J, all below the 4.8e-3 J per-body
budget. Reported numerical defects are not subtracted. Individual frame budgets
and the original ledger closure also pass. This does not bound cancellation
inside substeps or qualify real-time simulation. Raw results are preserved in
`production-contact-candidate-2026-10-06/independent-energy-audit-480.{json,log}`.
The primary Metal render remains pending at this point.

### Primary Metal rendering and delivery checks completed (2026-10-06)

The primary example completes its fresh Metal render: 41 distinct frames over
two simulated seconds, plus volume/energy/motion CSV receipts and a four-pose
strip. Simulation plus rendering took 1416.077 s in uncontrolled concurrent
execution. This is not real-time performance. The interactive preview is
`artifacts/corrected-animation-2026-10-06/preview.html`; deformable volumes are
still displayed separately from the character skin. Delivery checks across
physics, animation, scene, render, editor and application pass 2295 tests;
65 explicitly ignored tests are not claimed as executed. Workspace formatting
also passes.

### Imported render-skin displacement binding (2026-10-06)

`EmbeddedSurface::deform_relative_into` composes FEM displacement relative to
the current skeletal reference with an already posed render surface. The
application binds reference-space vertices only when they lie inside a valid
tetrahedral region; exterior vertices retain ordinary skeletal motion.
Overlapping region ownership is rejected, without nearest-cell extrapolation.
Binding retains vertex count, rest topology and attachment joint and rejects
stale topology, reassigned joints and singular/nonfinite poses. Deformation is
published atomically. Normals are recomputed from the resulting render geometry.

The Cesium example now uses this path. Its illustrative four volumes contain
only 16 of the model's 3273 vertices; a physical step changes exactly those 16
render vertices in the integration test. This is partial reference-space skin
coverage, not a complete anatomically authored character. External collision
sampling still uses the original prescribed skeletal surface; the display
binding does not implement coupled deformable-skin collision or self-contact.
Previously exported Metal frames predate this skin binding.

### Fresh limited GPU qualification of bound skin (2026-10-06)

The existing `body_motion_snapshot` runner accepts `--capture-steps=N` with
`--cesium` (3..=480), retaining the 1/240 s physical timestep. Its output
explicitly reports the limited duration; this does not replace the full clip
qualification. Four observation checkpoints are always retained, and frame
filenames use capture order to avoid collisions on short runs. Default full
capture cadence and duration remain unchanged.

The new bound-skin renderer completes an eight-step contact run on Apple M4 Max
Metal with four distinct frames, 16 bound vertices and no GPU validation errors
in 0.409 s of simulation plus rendering (uncontrolled run). Only the initial
0.0333 s is covered. Images, motion receipts, metadata and interactive preview
are under `artifacts/skin-bound-gpu-2026-10-06/`. This short observation cannot
qualify later stiff contacts, full skin coverage, real-time operation or other
hardware.

### Embedded surface force adjoint (2026-10-06)

`EmbeddedSurface::accumulate_forces_into` transfers forces using the transpose
of the displacement map, retaining existing nodal loads. A private candidate
prevents partial publication on nonfinite input or late shared-node overflow.
Nine embedding tests pass, including an independent potential derivative over
all 12 tetrahedron coordinates, virtual work, resultant and moment for the
unshifted barycentric surface, and atomic accumulation failures.

For relative skin composition the derivative is valid with skeletal reference
and skin base fixed. An authored skin offset changes force lever arms; its
reaction and prescribed-motion work must be carried by the rig before claiming
fully coupled conservation. This API does not yet replace contact geometry or
install new loads in the implicit integrator. Evidence is stored under
`artifacts/skin-force-transfer-2026-10-06/`.

### Relative-skin rig reaction and actuator work (2026-10-06)

`RelativeSurfaceLoads` exposes physical loads on nodes, skeletal reference and
skin base separately: W^T f, -W^T f and f. `actuator_work_j` returns minus the
prescribed physical-force work, matching the energy-increase convention of the
existing implicit ledger. Finite nonlinear steps require path-averaged loads;
an instantaneous response is not a finite-step conservation certificate.
`rig_wrench_about` evaluates the reference/base resultant and moment about an
explicit origin. Ten embedding tests pass. A moving-reference/base fixture
closes an independent quadratic potential with its exact path-average forces
and balances offset skin moment against nodal plus rig moment.

These mechanisms are not yet installed in implicit contact dynamics. Contact
geometry, CCD, path quadrature and rig work must use the same embedded skin
trajectory before claiming coupled deformable-skin collision. Evidence is under
`artifacts/skin-rig-reaction-2026-10-06/`.

### Embedded triangle contact operator (2026-10-06)

`EmbeddedTriangleContact` binds a contained authored skin patch and reuses the
existing prescribed-triangle barrier, closest features and CCD. Its response
transfers physical forces onto tissue nodes, skeletal reference, skin base and
obstacle. All 42 coordinates in the independent potential-difference fixture
match those forces. CCD rejects an intervening crossing even when both endpoints
are admissible. Linear node/reference/base motion yields a linear embedded skin
trajectory; nonlinear rig motion requires sufficiently resolved segments.

The finite-path response reuses adaptive contact quadrature, checks CCD first
and enforces a requested signed work-discrepancy budget across mechanical and
prescribed motions. Endpoint energy differences are estimators, not substituted
work. Path-average response preserves the native midpoint objective. The normal
metric maps the existing PSD frozen-feature blocks as W^T M W; it is explicitly
not a full geometric Hessian. Symmetry, positive action and tangent nullspace
pass. The new five tests plus 31 prescribed-contact and ten embedding tests pass.

This operator does not yet advance the implicit mechanical state or replace
collision sampling in the imported example. Its work budget proves the tested
finite-path balance, not a global error bound for arbitrary meshes/materials.
Evidence is under `artifacts/embedded-skin-contact-2026-10-06/`.

### Stationary embedded contact in existing mechanical stepping (2026-10-06)

`StationaryEmbeddedContact` is now an immutable body potential, installed through
`Body`/`InertialBody::set_stationary_embedded_contact`. Reference nodes, skin base
and obstacle are held fixed. Installation/removal returns separately booked
parameter work at the current mechanical state, not finite-time rig motion.
Rest-node identity and membership of embedded tetrahedra in the actual material
mesh are required; failed installation leaves the entire body unchanged.

The existing Body evaluation supplies embedded potential and its nodal gradient;
InertialBody diagnostics include this contact exactly once alongside plane and
other surface energies. Existing Body path admission includes embedded CCD, so
quasistatic search, ordinary inertial steps and the implicit averaged material
path reuse the same force and crossing guards. No second mechanical stepper was
introduced. Integration tests establish a real contact impulse, parameter-work
accounting, plane coexistence, complete-state rollback on swept crossing, owner
rejection and implicit-step contact response/work admission. 161 focused physics
tests pass; six manual tests remain ignored. Raw evidence is under
`artifacts/stationary-embedded-step-2026-10-06/`.

This does not integrate moving skeletal reference/base work during a timestep,
replace imported-character collision sampling, or establish full skin coverage.
The previously tested relative rig work and path response still need to enter
the same implicit trajectory for moving coupled skin.

### Owner-preserving skin pose transition (2026-10-06)

`StationaryEmbeddedContact::with_pose` stages reference/base/obstacle updates
while preserving binding and obstacle-law identities. `path_response_to` uses
the actual trial body endpoints for embedded CCD and integrated-work admission.
Construction validates reference geometry, while installation admits contact
against current mechanical nodes. A rest-only gap check was removed because it
incorrectly rejected prescribed poses admissible after actual body movement.
Tests confirm atomic rejection before motion, successful installation after an
ordinary inertial translation, owner/size rejection and trajectory work budget.
52 contact/embedding tests pass; evidence is in
`artifacts/skin-pose-transition-2026-10-06/`.

Staging or separately installing a new pose does not integrate moving-rig work
inside the implicit mechanical step; that integration remains pending.

### Moving embedded skin in shared Verlet stepping (2026-10-06)

`InertialBody::step_with_embedded_skin_motion` now stages an owner-preserving
reference/base/obstacle pose through the existing velocity-Verlet pipeline.
The initial and final forces use their respective skin poses. Endpoint-trapezoid
rig and obstacle actuator work is reported separately and included once in
`surface_work_j`; actual endpoint mechanical energy independently admits the
step. Shared evaluation accepts a staged skin without cloning material state.

For moving skin, the stationary embedded CCD check is replaced by the actual
simultaneous linear skin/obstacle path; all other tissue, native surface and
volume guards remain. No second stepper was introduced. Contact owner, positions
and velocities commit together only after complete work admission. Crossing and
energy-budget failures preserve the full previous state, including contact pose.
Tests include simultaneous comotion that would fail the old stationary guard.

174 unique focused tests pass, six manual tests remain ignored. Summed absolute
energy discrepancies over one, two and four segments are approximately
[7.888e-4, 1.992e-4, 4.992e-5] J, showing second-order convergence in the measured
fixture. That convergence test intentionally uses a loose admission budget to
measure truncation; production callers must use their actual budget and refine
when rejected. Evidence is in `artifacts/moving-embedded-step-2026-10-06/`.
This method requires time-independent material and linear discrete trajectories.
Moving-skin integration into the implicit Maxwell path and imported-character
collision authoring remain pending.

### Explicit Maxwell/thermal moving-skin transaction (2026-10-06)

`step_viscoelastic_with_embedded_skin_motion` now uses the existing Maxwell
quarter/half/quarter admission budgets: exact half relaxation, shared frozen-
history Verlet mechanics with moving embedded contact, then exact half relaxation.
Rig/obstacle work is returned alongside the established mechanical/heat receipt.
Only the complete candidate is committed; no independent material or thermal
owner is introduced. Existing surface, plane and implicit APIs retain their
public return types and dispatch through the shared transaction.

A prestressed Ogden-Maxwell specimen releases positive heat, increases cell
temperature, and closes independent mechanical-energy plus heat minus actuator
work within the requested 1e-6 J budget. Both swept crossing and mechanical
work rejection after the first relaxation preserve the entire pre-step state,
including Maxwell history, contact pose and thermal inventory. 176 unique tests
pass; six manual tests remain ignored. Evidence is under
`artifacts/maxwell-moving-skin-2026-10-06/`.

This is the explicit frozen-history Verlet branch, not the implicit moving-skin
Maxwell branch. Imported-character collision integration and full skin coverage
also remain pending.

### Shared skin quadrature preparation (2026-10-06)

The existing implicit material-path evaluator now has an internal staged-skin
variant. It samples skin forces and rig/obstacle work on the same quadrature
nodes; owner-preserving linear pose sampling retains exact endpoint descriptors.
The production implicit step still calls the stationary variant. Moving-skin
implicit stepping and independent qualification of the new quadrature variant
remain unfinished; this preparation is saved with the completed explicit work.
