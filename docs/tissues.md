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
implicit stepping remains unfinished; this preparation is saved with the
completed explicit work.

The shared-quadrature variant is now independently qualified on a moving
embedded triangle with all four tissue nodes free: central differences of the
search objective match all 12 averaged-gradient coordinates within
1e-6*(1+abs(derivative)). Endpoint potential change minus averaged gradient
work and prescribed rig/obstacle work is below 1e-10 J. Both actuator work
terms are nonzero. Endpoint samples preserve descriptor bits; invalid phases
and mismatched sample counts are rejected without mutating the body.
103 library tests and 48 embedded/native contact tests pass; six manual tests
remain ignored. This fixture does not qualify stiff implicit convergence or
production time stepping. Evidence: `artifacts/shared-skin-quadrature-2026-10-06/`.

### Implicit moving embedded skin (2026-10-06)

`step_implicit_with_surface_and_skin_motion` now stages an installed embedded
skin alongside an installed native prescribed surface in the existing implicit
solver. Both owners are required by this coupled API. Material and embedded
forces and rig/obstacle work share quadrature nodes. Moving skin uses its actual
simultaneous trajectory for all feasibility, line-search and final CCD checks;
internal contact, native obstacle and volume guards remain active. Final energy
is evaluated at the new skin pose. Embedded actuator work is added once to
`surface_work_j`; no endpoint-energy correction replaces forces.

On energy rejection, moving-skin samples can be uniformly refined within the
existing 128-panel bound. Contact poses, node positions and velocities commit
only after nonlinear convergence, independent energy admission and support-work
representability. Existing stationary APIs retain their behavior.

The independent moving-skin test admits endpoint kinetic/potential energy minus
actuator work within 1e-8 J and verifies the committed pose. A stiffness sweep
(100, 10,000, 1,000,000 N/m; dt=1 ms) passes the same budget. Crossing and
unrepresentably tight 1e-30 J work budgets preserve the complete previous state.
The native surface is inactive in these new fixtures to isolate embedded work;
simultaneously active native/embedded contacts and long stiff trajectories need
additional qualification. 179 focused tests pass, six manual tests remain ignored.
Evidence: `artifacts/implicit-moving-skin-2026-10-06/`.

This is time-independent material integration. Moving embedded skin in the
implicit Maxwell/thermal transaction and imported-character collision authoring
remain pending. No real-time, full-model coverage or all-hardware claim is made.

### Implicit Maxwell/thermal moving skin (2026-10-06)

`step_viscoelastic_implicit_with_surface_and_skin_motion` now uses the existing
exact half-relaxation / frozen-history implicit mechanics / exact half-relaxation
transaction. Both installed native and embedded contact owners move together.
The shared implicit core returns its actual averaged rig/obstacle work alongside
the original support receipt; embedded work is already included in total surface
work and must not be added twice. Existing public stationary receipts are unchanged.
Quarter/half/quarter admission budgets and outer candidate ownership remain intact.

A prestressed Ogden-Maxwell specimen with thermal storage and simultaneously
active tilted native and embedded contacts passes independent kinetic + potential
+ released heat - actuator work within 1e-6 J at dt=1 ms. All three actuator terms
(native obstacle, skin rig, skin obstacle) are nonzero. Released heat matches
thermal storage growth; temperature rises and the admitted contact pose commits.
Crossing rejection after the first relaxation preserves the full original state.
181 focused tests pass; six manual tests remain ignored. Evidence is under
`artifacts/implicit-maxwell-moving-skin-2026-10-06/`.

A parallel native triangle near multiple parallel tissue faces fails the existing
strict nonlinear residual criterion (weighted residual about 2.624e-10 J versus
1e-12 J). Its trace and executable rollback reproduction are retained. The
closest-feature ambiguity is a suspected cause, not yet proven or fixed. No
residual/work tolerance was relaxed. Robust convergence for this geometry, long
stiff trajectories, imported-character collision and full skin coverage remain
unqualified. This limits production-readiness claims despite the new transaction.

### Parallel-contact non-smoothness diagnosis (2026-10-06)

The captured terminal native-contact trajectory now has an executable local
diagnostic. At all five Gauss nodes, one-sided potential slopes with respect to
node 2 z differ by about 3.556 N: forward about -3.556 N, backward about -7.112 N.
The returned force gradient lies within those slopes. Central differences of the
path objective differ from the selected averaged branch gradient by about
0.506 N for midpoint perturbations from 1e-9 to 1e-12 m. This is evidence of a
non-smooth closest-feature switch, not evidence that the selected branch force
is an invalid potential derivative. The previous inference of a smooth
potential/gradient inconsistency is therefore not supported.

An experimental cross-product denominator for nearly parallel edge projection
left this discrepancy unchanged and was reverted. No production contact law,
force, equilibrium threshold or work tolerance changed during this diagnosis.
A solver that handles active closest-feature switches is still required for this
reproduction; simply loosening residual admission or replacing the potential
would not qualify the original behavior. Evidence and the rejected experiment
are under `artifacts/parallel-contact-kink-2026-10-06/`.

### Generalized contact-branch balance reference (2026-10-06)

A test-only two-branch reference projects the free-node inertial residual onto
a convex segment of candidate residuals, minimizing sum(r^2/inertia). The exact
closed-form coefficient is clamped to [0,1]. An independent dense search checks
the minimum for a multidimensional fixture; pinned reactions do not influence
free equilibrium. Same-sign branches cannot invent a zero residual, identical
branches retain their residual, and invalid dimensions/inertia are rejected.
Evidence: `artifacts/contact-branch-projection-2026-10-06/`.

This is a reference oracle, not a production fix. Before admission can use it,
the native contact evaluator must expose co-active feature gradients at the
identical quadrature pose. Their body/obstacle derivatives and actuator work
must use identical convex coefficients. Gradients sampled at nearby but
different positions are not silently treated as valid coincident branches.
The parallel-contact convergence reproduction therefore remains unresolved.

### Same-pose native contact branch extraction (2026-10-06)

`contact_branch_bundles` exposes native feature alternatives grouped by original
body/obstacle face identity at a single pose. The currently selected branch is
first; alternatives require bit-identical evaluated distance and compensated gap
and distinct barycentric weights. Each branch retains the original barrier
potential/derivative and PSD normal stencil. `gradients()` applies matching
coefficients to body and obstacle. This is a read-only geometry diagnostic;
bit equality does not certify exact real-arithmetic co-activity and no solver
force mixture is selected by this API.

A parallel contained triangle exposes at least three branches with identical
potential and independently balanced total force and torque. A tilted triangle
retains one branch. The selected branch reproduces the existing response; input
geometry and invalid topology handling are preserved. 158 focused tests pass,
six manual tests remain ignored. Evidence: `artifacts/same-pose-contact-branches-2026-10-06/`.

At the captured failed trajectory, all five quadrature nodes return three active
face-pair bundles with exactly one retained branch each. Consequently the
previous central/one-sided finite-difference kink evidence does not establish
co-activity at the stored terminal pose. Event localization and a justified
distance-uncertainty treatment are required before a branch mixture may be
admitted. The original parallel-contact convergence failure remains unresolved.

### Captured native event localization (2026-10-06)

The captured terminal trajectory now brackets the native closest-feature switch
at every Gauss node by binary search of node 2 z. Each bracket ends at adjacent
representable f64 coordinates (`lower.next_up() == upper`), after 44-48
iterations. Widths range from 4.136e-25 to 6.617e-24 m. Gradients differ by about
3.556 N across every bracket. This localizes the implemented feature event, not
an exact real-arithmetic coincidence or an admission certificate.

A compensated squared-distance ordering experiment retained the same terminal
nonconvergence and captured derivative values. Its source snapshots and logs
are preserved, and both production source files were restored. No equilibrium
threshold, physical contact law or work guard changed. 23 embedded-contact
regressions pass, and formatting/diff checks pass. Evidence is under
`artifacts/contact-event-localization-2026-10-06/`.

The next solver change must handle this event with verified generalized
stationarity and coupled body/obstacle work. The original convergence failure
remains open; the event bracket alone does not authorize blending forces.

### Full-interval adaptive implicit moving skin (2026-10-06)

`step_viscoelastic_implicit_adaptive_with_surface_and_skin_motion` retries the
original full linear prescribed interval with 1,2,4,... equal substeps, up to a
caller-selected power-of-two limit (maximum 256). It reuses the same implicit
Maxwell transaction, divides the original time and budget, interpolates original
native/skin/support trajectories, and retains exact authored endpoint poses.
Numerical convergence/work failures permit refinement; invalid controls, owners
and crossing do not. Every attempt starts from the original state. Only the
complete admitted interval commits; no partial substeps escape failed attempts.

`ViscoelasticDynamicStep::absolute_energy_defect_j` now records absolute defects
from both relaxation and thermal stages plus mechanics, before signed sums can
cancel. The adaptive receipt aggregates all work/heat components, bounds this
absolute total, checks receipt finiteness, and independently checks actual
endpoint kinetic + potential + heat minus all external work against the
original full-interval budget. Existing per-stage admission guards remain.

The original parallel-contact reproduction admits its full 1 ms interval with
64 substeps: independent defect -3.40563e-11 J and absolute stage defects
9.62582e-11 J versus the original 1e-6 J budget. Heat matches thermal growth,
native and skin poses match requested endpoints, and limits of 1 or 32 preserve
the complete initial state on rejection. Invalid subdivision limits also roll
back. 185 focused tests pass, six manual tests remain ignored; the final receipt
finiteness change additionally passes the targeted full-interval test. Evidence:
`artifacts/adaptive-implicit-moving-skin-2026-10-06/`.

This enables full-interval integration for the measured fixture without changing
the contact potential, forces or residual thresholds. It does not fix the
single-substep generalized-stationarity failure at the feature event. Long-run
accuracy, timestep convergence, imported-character integration, full skin
coverage and real-time performance remain unqualified.

### Adaptive trajectory ledger and refinement limitation (2026-10-06)

The parallel native/embedded Maxwell specimen now runs the same 8 ms linear
actuator trajectory with 8, 16 and 32 nominal intervals. All runs independently
balance endpoint mechanical energy, released heat and external work within
the original aggregate 8e-6 J budget. Absolute stage defects total approximately
6.312e-10, 8.950e-10 and 8.723e-10 J; thermal storage growth matches released heat.
The adaptive schedules use 512, 306 and 278 actual substeps respectively.

Relative to the 32-interval endpoint, position discrepancies decrease from
1.153e-10 to 4.636e-11 m, while velocity discrepancies increase from
7.612e-7 to 1.052e-6 m/s. The original monotonic-refinement assertion failed;
its log is preserved. The passing test explicitly records the velocity
limitation and qualifies only ledgers, not temporal convergence. Common fixed
fine schedules and longer trajectories are required for a convergence claim.
Evidence: `artifacts/adaptive-contact-trajectory-2026-10-06/`.

### Fixed-grid trajectory qualification limit (2026-10-06)

Fixed 512, 1024, 2048, 4096 and 8192 steps now execute the same 8 ms
parallel-contact Maxwell trajectory with the same original aggregate 8e-6 J
budget. All five runs close independent work/heat ledgers and thermal storage;
absolute stage defects decrease from 6.312e-10 to 5.740e-11 J. Relative to the
8192-step numerical endpoint, position discrepancies are 3.408e-11, 3.001e-12,
1.990e-12 and 2.727e-13 m. Velocity discrepancies are 1.114e-7, 5.606e-7,
1.645e-7 and 2.469e-8 m/s, which are not monotonic. The 8192-step endpoint is
a comparison trajectory, not an independent exact solution.

The initial three-level monotonic-velocity assertion failed and is retained
in evidence. Adaptive and fixed tests now reuse one fixture with identical
geometry, controls, ledger checks and distance measurement. Both tests explicitly
retain the observed velocity limitation. Thus differing adaptive schedules are
not the sole explanation; branch selection and nonlinear residual accuracy need
further investigation. No solver threshold or physical law was modified.
Evidence: `artifacts/fixed-contact-refinement-2026-10-06/`; two common-fixture
tests pass, formatting and diff checks pass. Temporal convergence remains open.

### Nonlinear budget sensitivity qualification limit (2026-10-06)

The common trajectory runner now accepts an explicit aggregate budget and
reports indexed rejection instead of hiding failed comparisons. The 8192-step
fixed 8 ms trajectory passes at 8e-6 J, but at 8e-8 J rejects step 2 with
`implicit contact nonlinear nonconvergence`. A full pre/post snapshot verifies
that the rejected step preserves material history, thermal state, contacts,
positions and velocities. The accepted first step is a trajectory prefix; it
does not qualify the full stricter-budget interval. Its displacement defect
sum is 3.989e-15 m.

The runner also sums norms of dx - dt*(v_old+v_new)/2 per nominal interval and
free node. On the original fixed 8192-step run this totals 8.632e-11 m. For
adaptive runs, this is an endpoint trapezoidal discrepancy across multiple
internal steps, not the residual of any one implicit solve. No causal velocity
claim is inferred from it. The requested stricter-budget velocity comparison
is unavailable because that trajectory is not admitted. 27 embedded-contact
tests pass; formatting/diff checks pass. Solver thresholds remain unchanged.
Evidence: `artifacts/contact-budget-sensitivity-2026-10-06/`.

Production temporal accuracy remains unqualified. A failure at a stricter
budget does not prove why the original velocity comparison is nonmonotonic;
feature-event handling and nonlinear-error controls remain required work.

### Bounded nonlinear quadrature retry (2026-10-06)

The shared implicit core now retries nonlinear-limit and line-search failures
with uniformly bisected quadrature panels, up to the existing 128-panel cap.
Each retry starts from the same uncommitted state and re-evaluates original
material, native and embedded gradients/work at actual common quadrature poses.
No gradient mixture, smoothing, physical-potential change or relaxed residual
threshold is introduced. CCD, volume, nonlinear and independent endpoint energy
guards still admit the result. Exhausted refinement retains the original failure.

The previously failing full 1 ms parallel native/embedded Maxwell step now
admits at 16 panels: independent endpoint energy/heat/work defect 1.457e-12 J
versus the original 1e-6 J budget. Thermal growth and exact requested contact
poses are verified. The adaptive wrapper now accepts that fixture with one
time substep; prior 64-substep evidence remains historical.

The stricter 8e-8 J, 8192-step full 8 ms trajectory now also admits instead of
rejecting step 2. The 8/16/32 nominal trajectories use exactly 8/16/32 time
steps and show decreasing discrepancies against the 32-step endpoint: position
1.830e-9 -> 3.847e-10 m and velocity 3.130e-8 -> 2.127e-8 m/s. These are numerical
comparison results on this fixture, not a universal convergence-order proof.

Fine fixed-grid velocity nonmonotonicity remains. Against a stricter-budget
16384-step numerical endpoint, the original-budget 8192-step velocity differs
by 1.982e-7 m/s, and the stricter-budget 8192-step velocity by 3.598e-7 m/s.
Thus stricter admission does not qualify velocity accuracy by itself. General
feature-event convergence and independent long-run accuracy remain open.

188 focused physics tests pass; six manual tests remain ignored. Tests formerly
expecting the original rejection now require independent full-interval work/heat
admission and endpoint pose ownership. Invalid controls and tiny-budget failures
still preserve all state. Formatting/diff checks pass. Evidence:
`artifacts/nonlinear-contact-quadrature-2026-10-06/`.

### Refined-kernel example and Metal capture (2026-10-06)

The existing body-motion example passes 37 release tests (two manual tests
ignored) after the bounded nonlinear quadrature retry. This includes the
68-step imported contact/energy gate and parallel-region equivalence. A fresh
Metal/Apple M4 Max capture executes 48 steps at the unchanged 1/240 s physical
step: 0.2 simulated seconds, seven distinct rendered frames and 28 finite
secondary-motion receipts. The pose strip was visually inspected; the existing
HTML preview exporter validates the frame/receipt timeline. Evidence:
`artifacts/refined-skin-gpu-2026-10-06/`.

This remains a partial capture and only 16/3273 imported model vertices have
tissue binding. The original prescribed skeletal collision path is used;
moving embedded collision in the imported character is still unintegrated.
The separate full 480-step imported-contact qualification is now running
under `artifacts/refined-full-contact-2026-10-06/tests.log`; no full-clip success
is inferred from this short render or the earlier source revision.

### Native-only regression and scoped quadrature retry (2026-10-06)

The full 480-step qualification on the unconditional nonlinear-quadrature retry
terminated with failure at nominal step 71: `implicit contact quadrature
nonconvergence` (248.49 s). No completed energy audit was emitted. This
contradicts full-clip qualification of that source; the failed log and launch
snapshot remain in `artifacts/refined-full-contact-2026-10-06/`.

Nonlinear-limit/line-search quadrature retries are now scoped to moving embedded
skin, where the common pose/work quadrature was qualified. Native-only contact
retains its prior temporal-refinement path and all admission guards. No
constitutive law, CCD, volume or energy threshold was relaxed. The ordinary
imported-contact regression is extended from 68 to 72 nominal steps to cover
the observed step-71 failure. That 72-step test passes with independent absolute
frame errors [6.950e-8, 7.876e-7, 8.705e-6, 6.550e-8] J versus 7.2e-4 J per
body. The moving embedded/native Maxwell full-step test also passes.
Evidence: `artifacts/contact-quadrature-scope-2026-10-06/`.

`tools/verify_imported_contact_audit.py` validates the emitted aggregate scope,
step/body counts, unchanged requested budget, finite values, independent
absolute-error budget and final reported-ledger closure. For logs, successful
test completion must follow the audit; failed, partial and duplicate-audit logs
are rejected. The tool does not independently reconstruct per-frame rounding
or physics. Three tests exercise a historical successful audit, fifteen
contaminated reports (including numeric and balance overflow) and terminal/partial-log cases. It accepts the new 72-step
audit and rejects the failed full-run log.

The scoped-source full 480-step qualification passed in 540.40 s. All 1098
launch-manifest source hashes remained unchanged. Independent absolute frame
errors were [2.990e-5, 2.693e-6, 5.680e-5, 3.119e-7] J, each below the
original 4.8e-3 J per-body budget; reported defects were not subtracted from
these errors. The aggregate verifier also passed. Evidence:
`artifacts/scoped-full-contact-2026-10-06/{tests.log,verified-audit.json,result.json}`.
This qualifies native-only imported contact after retry scoping, not collision
through the physically deformed imported skin or a fresh full GPU render.


### Mixed skeletal/tissue contact vertices

`EmbeddedSurface::bind_relative` now takes a fixed authored ownership mask.
Tissue-owned vertices must remain inside the authored tetrahedral mesh at binding;
prescribed vertices need no extrapolation and retain their supplied base pose
bit for bit. Relative deformation and transpose force transfer use the same W.
Absolute deformation rejects mixed surfaces without a base pose. Trial-direction
mapping explicitly returns zero at prescribed vertices.

`EmbeddedTriangleContact::new_mixed` uses that map with the existing triangle
contact law, CCD, integrated actuator work and implicit solver. A mixed triangle
with two vertices outside the tetrahedron passes an independent finite-difference
check of nodal, reference and base loads. A prescribed vertex crossing the obstacle
is rejected even with fixed tissue nodes. A moving mixed-skin implicit step closes
independent K+U-actuator-work within its original 1e-8 J budget; a subsequent
crossing leaves the full body state unchanged.

The embedding has one global mechanical node space plus prescribed vertices.
That node space can contain multiple disconnected FEM regions. It is not yet
integrated into the imported character example: its separate regional dynamics
must be assembled into that global space. Treating another dynamic region as
prescribed would lose its force transfer and must not be presented as full skin
contact.
Evidence: `artifacts/mixed-skin-contact-2026-10-06/`.


A two-region fixture now qualifies the existing global assembly route. Its
contact point lies on an edge whose endpoints belong to different disconnected
FEM regions, with heterogeneous material stiffness and densities. Both regions
receive nonzero force and velocity; all 24 nodal potential derivatives pass an
independent finite-difference check. A trial direction confined to the first
region creates nonzero contact metric action in the second (maximum 134.099),
so this fixture specifically checks coupling that independent regional solves
would miss. Distinct prescribed reference motions for both regions and a mixed
skeletal base motion close the full-step independent mechanical balance to
2.6923e-12 J within the original 1e-8 J budget. Independent CCD rejects the
prescribed-vertex crossing; the implicit solver rejects that interval at its
initial trajectory guard and preserves the entire previous body state.

Evidence: `artifacts/multi-region-skin-contact-2026-10-06/`. This is a two-tetrahedron
qualification, not imported model integration, production mesh convergence or
Maxwell/thermal qualification across multiple regions. No second dynamics solver
was added. The imported example still needs global regional assembly, stable
node/cell mappings, contact-domain exclusions and a fresh render/energy run.

### Dynamic regional assembly

`InertialBody::assemble_tissues` now reuses `Body::assemble_tissues` and returns
one global `InertialBody` plus stable node/cell ranges. It concatenates existing
nodal/cell masses and velocities directly, without rebuilding density or resetting
support motion. Complete element laws and Maxwell memories are retained by the
existing Body assembly. Cell thermal capacities, immutable reference temperatures,
heat inventories and sub-ULP compensation are copied separately and exactly.

Tests assemble regions with distinct prestrain, relaxation histories, moving
supports, density, heat capacity and temperature; verify exact ranges, positions,
velocities, masses, constitutive probe responses and thermal data; and compare
the next uncoupled global advance against independently advanced source regions.
A common active native contact surface keeps its identity and additive energy.
Different obstacle geometry owners/poses, accelerations/planes or thermal ownership are
rejected without changing any source. Global skin contact must be installed after
assembly. Body assembly previously omitted installed embedded contact: it now
rejects that case explicitly instead of silently discarding its energy and forces.

The two-region shared-skin coupling test now constructs its global state through
this new assembly API before installing the mixed embedded contact. This still
leaves imported example integration open: regional native contact exclusions need
an explicit global mapping rather than merging their different surface masks.
Evidence: `artifacts/inertial-tissue-assembly-2026-10-06/`.


### Global body-triangle contact domains

`PrescribedTriangleSurface::with_body_contact_domains` authors obstacle-face masks
for groups of body triangles in one global node space. Canonical body node IDs
retain group membership under cyclic/reversed triangle indexing. Mask rows share
immutable storage by group. Unlisted triangles use the global obstacle mask;
listed triangles require both masks. Fully excluded body rows are allowed without
removing either geometry. Invalid/duplicate body identities and inconsistent mask
sizes are rejected. Domain configuration is part of contact-owner identity;
changing it during a path is rejected, and pose staging retains it.

The same pair filter now serves forces/potential, the normal preconditioner,
nearest-contact diagnostics, branch bundles and CCD. Tests compare grouped
responses to independently masked regional responses; check potential derivatives,
path-averaged forces and actuator work; and demonstrate that an excluded crossing
is allowed while the corresponding enabled crossing is rejected.

`InertialBody::assemble_tissues` can now combine distinct regional masks when all
parts share the same original obstacle geometry, law and pose. It remaps each
local body-face identity by its global node offset, composes existing body and
obstacle masks, and retains source obstacle-face indices. Identical common owners
without body-specific rows retain their identity; remapped domains create a new
global contact owner explicitly. The assembly test verifies exact force transfer,
additive contact/mechanical energy, obstacle geometry identity and agreement of
path work with separate regions. No body face is omitted from the mapped domain.

This closes the domain-mapping prerequisite, but the imported example still runs
separate regional dynamics. Its controller and skin binding must adopt the
assembled global state before claiming full imported deformed-skin contact.
Evidence: `artifacts/global-contact-domains-2026-10-06/`.

### Assembled imported animation controller and render skin

The imported GPU snapshot entrypoint now assembles its four continuum regions
before skin binding. It replaces the regional dynamics with one global owner,
retaining immutable node/cell ranges and all original joint/pin metadata. The
existing thermal/mechanical/refinement controller is shared through
`step_continuum_targets`; a complete global target list drives all four joints.
Original regional obstacle domains remain pose-validation templates. Incoming
poses must retain each template's exact law/domain owner and share identical
obstacle positions before staging the admitted global obstacle pose. Regional
contact authoring must precede assembly; late rebinding is rejected atomically.

Global skin binding checks reference-space membership separately in every region,
rejects overlap, then creates one mixed relative map over the global FEM cells.
Every range builds its skeletal reference using its own authored joint. Rendering
adds physical displacement exactly once; prescribed exterior vertices retain
base coordinates. Binding checks retain rest/cell/range/joint identities. Tests
check all regional displacements, rigid-pose composition, exterior vertices,
singular/reassigned joints and actual imported model membership (16/3273 vertices).
The actual imported test preserves its previous deformed render positions across
assembly, then advances and renders the assembled owner.

Energy, volume and thermal diagnostics now describe the single global owner in
this entrypoint. The displayed secondary offset is its mass-weighted deviation
from per-region skeletal reference positions, rather than a regional free-node
offset. This scope change must be retained when interpreting CSV receipts.

The app example regression passed 40 tests (two manual qualifications ignored).
Three focused assembled-owner tests also pass after the late-binding guard.
Independent controller balance was 7.7343e-10 J within its unchanged 1e-5 J
frame budget. A fresh Metal run admitted 48 physical steps (0.2 s), 192 controller
substeps, and seven captured frames. Sparse output-interval energy balances from
rounded CSV receipts sum to 1.5091e-7 J in absolute value without subtracting the
reported solver defect; this is not an audit of every internal/physical step.

Evidence: `artifacts/assembled-controller-2026-10-06/` and
`artifacts/assembled-skin-final-gpu-2026-10-06/`. Collision still uses the skeletal
obstacle surface, not the physically deformed imported skin. Full assembled clip,
production mesh coverage/convergence and real-time performance remain unproven.


### Physical render-skin contact boundary

The imported snapshot now shares one immutable embedded-surface map between
rendering and contact. After initial assembly, 56 responsive imported triangles
use the physical skin boundary (16 of 3273 vertices are tissue-bound). Native FEM
envelope contact is disabled to avoid duplicate forces. The remaining obstacle
is prescribed by the skeletal pose in f64. Incident triangles, authored coordinate
aliases and skeletal copies of dynamic patches are excluded at initialization.
Reciprocal contact between two dynamic skin patches remains unimplemented.

The existing adaptive Maxwell controller stages skin and obstacle motion together,
books their work once, and preserves atomic rollback including thermal history.
A moving-plate fixture measured an independent energy defect of 3.9368e-10 J
and nonzero obstacle work of -4.2406e-6 J; a subsequent crossing rejected without
changing state. The app example suite passed 41 tests with two manual tests ignored;
shared-map regressions passed 30 embedded-contact and 11 surface tests.

A fresh Metal capture completed 48 physical steps (0.2 s), 192 controller substeps
and seven frames in 11.557 s including rendering/readback. This is a short runtime
qualification, not proof of real-time performance or a full 480-step skin clip.
Earlier native-only full-clip results retain their original scope.
Evidence: `artifacts/physical-skin-gpu-48-2026-10-06/` and
`artifacts/physical-skin-controller-2026-10-06/`.


### Per-frame imported physical-skin qualification

The imported qualification helper now runs either the original regional FEM
boundary or the assembled render-skin boundary, recording the selected mode in
the audit. A new normal regression covers 72 steps at 240 Hz, including the
previously problematic initial trajectory interval. It asserts the actual
16 tissue-bound vertices, 56 responsive triangles and one assembled energy owner.
Every frame independently checks delta stored energy minus actuator work plus
released heat against the unchanged 1e-5 J frame budget; reported numerical
defect is never subtracted. The sum of absolute frame errors was 1.11610514e-7 J
over 72 steps (requested aggregate budget 7.2e-4 J). The final reported-ledger
closure was -3.65e-15 J. This is actual per-frame evidence, unlike sparse GPU CSV.

Evidence: `artifacts/physical-skin-path-2026-10-06/`. A separate ignored test
qualifies all 480 steps; its existence alone is not completion evidence.

The full physical-skin test was executed and **failed at step 279** with
`closed surface contact gap`, after 278 committed frames. No completed full-clip
audit was emitted. The source manifest remained unchanged during both runs.
This contradicts full-clip readiness of the new boundary; earlier native-only
480-step success does not qualify this mode. CCD and energy tolerances remain
unchanged. Evidence: `artifacts/physical-skin-full-clip-2026-10-06/`.
The next diagnosis must identify the enabled skin/obstacle pair and whether the
closure originates in prescribed motion, FEM response or path interpolation.


### Closed initial trial and adaptive retry

A read-only query on `StationaryEmbeddedContact` now reports the nearest enabled
physical-skin pair, including closed gaps, using the exact force embedding/domain.
Its regression verifies source identities, known open/closed distances, unchanged
contact state and agreement with force rejection. All 31 embedded tests passed.

Trace of the step-279 rejection identifies iteration zero in the averaged material
path: skin face [651, 666, 2591], obstacle triangle 1341. The admitted initial
gap was +4.7486234e-5 m; the stationary trial in the next prescribed pose had
gap -6.6259148e-5 m. The velocity predictor was also inadmissible. This is not
evidence that the committed state crossed. The adaptive Maxwell wrapper now
retries a closed trial gap with smaller substeps, after validating initial
diagnostics. Accepted substeps still require the original CCD and proportional
energy budget; exhausted trials preserve the original interval state.
Evidence: `artifacts/physical-skin-gap-diagnosis-2026-10-06/`. The changed full
clip is running separately in `artifacts/physical-skin-refined-clip-2026-10-06/`;
no successful result is claimed until that process terminates and its audit passes.

A focused exhausted-trial regression additionally stages a closed skin pose over
a physically tiny interval. With one and two allowed subdivisions it must reject
and preserve the complete Maxwell/thermal/contact state. This passed after the
retry change; the 31-test embedded suite also passed. These rollback results do
not by themselves establish full-clip success.

The refined full clip terminated with the same `closed surface contact gap` at
step 279 (121.57 s in this run). Allowing trial-gap retries up to 256 substeps
therefore does not establish clip readiness. The failed result is preserved;
the next diagnostic examines whether the nearest feature is dynamically owned.

The source-vertex owner trace confirms only vertex 666 of face [651, 666, 2591]
is tissue-owned. The rejected nearest feature has weights [0, 0, 1], so it lies
at prescribed skeletal vertex 2591. That vertex cannot be displaced by FEM forces;
a triangle containing it cannot be separated from an obstacle closer than the
minimum allowed distance to that vertex. Refinement alone cannot fix this source
constraint. Physical coverage currently binds only 16/3273 vertices. A production
solution needs physically covered skin patches or collision-aware rig motion;
excluding the offending pair would merely hide the prescribed intersection.


### Prescribed-point obstruction witness

`StationaryEmbeddedContact::prescribed_contact_obstruction` inspects the nearest
enabled physical-skin feature with the same geometry/domain query as contact.
It returns a sufficient obstruction witness only for a closed gap whose exact
barycentric weights vanish on every tissue-owned vertex. Its remaining point
is prescribed and stays on the triangle under all nodal deformation, so FEM
forces cannot separate this pair. No epsilon mobility classification, pair
exclusion or new contact force law is introduced. None is not a general proof
of feasibility, and this is not an exact-arithmetic geometric certificate.

The adaptive Maxwell step checks this witness after admitted initial diagnostics
and returns `prescribed skin obstacle gap is closed` before subdivision. The
regression distinguishes a fixed mixed-edge closure from a fully dynamic closed
trial, checks invariance under large tissue translations, rejects a changed
domain owner and preserves the complete Maxwell/thermal state. All 33 embedded
contact and 11 surface tests passed. This admission/diagnostic improvement does
not repair the imported rig trajectory or expand the current tissue coverage.
Evidence: `artifacts/prescribed-skin-obstruction-2026-10-06/`.

The imported obstruction replay completed: it now returns the explicit
`prescribed skin obstacle gap is closed` at step 279 with the original source
face identities. This confirms admission classification, not clip readiness.

### Authored convex skin to conforming tissue volume

`TetraMesh::from_convex_surface` constructs one radial tetrahedron per explicitly
authored outward boundary triangle around a strictly interior point. Original
skin vertices/indices are preserved and the interior node is appended, so the
complete source boundary can use the existing embedding and force transpose.
Admission checks finite unique/used points, closed opposite-edge topology, sphere
Euler characteristic, connectedness, convex half-spaces and the existing positive
volume/interface/boundary validation. Validation work is bounded at 16 million
point/face checks. No hull, welding, exterior extrapolation or anatomical fit is
inferred. Floating checks are conservative without an added geometry epsilon;
this is not exact-arithmetic certification or a general nonconvex body mesher.

Tests verify all eight source cube vertices embed, affine skin motion agrees with
its nodal motion, boundary loads transfer once and a 1 m3 cube at 1000 kg/m3 has
1000 kg total nodal mass. A sheared/scaled cube has analytic volume 24 m3 and
24000 kg mass both before and after conforming refinement. Open/inward/nonconvex
surfaces, invalid indices, duplicate/unused points and noninterior centers reject.
All 19 checks across authored-volume, anatomical interchange, continuum geometry
and surface embedding suites passed. Evidence:
`artifacts/convex-tissue-surface-2026-10-06/`. This provides volume authoring for
complete convex tissue patches; the imported character still has its original
16/3273 bound vertices until correctly authored patches/decomposition are integrated.


### Authored volumes use the existing animation controller

`TissueDemo::body_from_regions` admits supplied tetrahedral regions, explicit
three-node supports and joint identities. The neutral mannequin's ellipsoids
now delegate to this same construction path; there is no separate authored-volume
solver. Topology, node indices and duplicate supports reject before publication.
`TetraMesh::into_body` now validates its boundary/interface topology as well as
Body's mechanical geometry, including for manually constructed public mesh data.
The demo retains its illustrative material/density/thermal constants. This is
not anatomical material calibration, arbitrary support-count authoring or rig fitting.

An end-to-end convex-volume fixture binds every one of eight skin vertices,
assembles the single physical owner, enables all 12 skin triangles and advances
the existing controller against a moving prescribed plate. The independent
frame balance is -4.0268934e-7 J within the unchanged 1e-5 J budget, without
subtracting reported solver defect. Render skin moves with the solved tissue;
the three prescribed obstacle vertices retain their supplied pose. Missing mesh
boundaries and invalid supports reject. The prior full app suite passed 42 tests
(three manual qualifications ignored), followed by the new authored-volume
fixture; eight anatomical/convex/continuum regressions also passed. Formatting
and source whitespace checks passed. Evidence:
`artifacts/authored-tissue-controller-2026-10-06/`. Imported Cesium skin coverage
is unchanged until suitable volumes/skin ownership are authored and integrated.


### Observed file-backed tissue volume authoring

The existing imported snapshot accepts `--tissue-regions=MANIFEST.json` with
`--cesium --contact`. Its version-1 manifest declares `scene_phase_0_metres`,
`illustrative-manikin-v1`, the exact source-model BLAKE3 digest, and a list of
VXTM meshes with BLAKE3 digests, joint names, three support-node indices and
explicit source obstacle-face exclusions. Unknown settings, incompatible
coordinates/profiles/hashes/joints/supports and invalid/duplicate exclusions reject.

The importer reuses `voxy_assets::FileInputs` and `ImportInputs` rather than a
parallel file/cache system. Mesh paths are normal paths relative to the manifest
root, existing provider symlink/root restrictions apply, observations retain
model/manifest/mesh hashes, and all inputs are reread before publication. Reads
are bounded at 1 MiB per manifest, 32 MiB per mesh and 64 MiB total. Native
contact begins as an inactive integration owner; after assembly, the shared
physical-skin contact is installed before stepping. No runtime exclusions are
computed from animated intersections. The current preset remains illustrative.

`TetraMesh::to_bytes` exports validated authored volumes through existing VXTM v1.
Tests verify exact geometry/cell/boundary round trips and reject invalid exports.
A real file-backed convex pad attached to CesiumMan's torso passes eight headless
physical steps, with independent per-frame balances checked against 1e-5 J.
The complete example suite passed 44 tests (three manual tests ignored), and six
mesh/interchange tests passed. A fresh Metal invocation loaded the saved manifest,
completed three physical steps/12 controller substeps and produced four frames.
The pad binds only one of 3273 model vertices and six contact triangles: this
qualifies asset/controller plumbing, not increased full-character coverage or
repair of the step-279 intersection. Frames cover only 0.0125 s.

Evidence and a runnable illustrative manifest are in
`artifacts/authored-tissue-import-2026-10-06/`. Reproduction:

```sh
cargo run -p voxy_app --release --example body_motion_snapshot -- \
  /tmp/authored-poses.png /tmp/authored-frames --cesium --contact --capture-steps=3 \
  --tissue-regions=artifacts/authored-tissue-import-2026-10-06/fixture/regions.json
```


### Explicit material profiles and arbitrary support-node lists

`TissueRegionSpec` now owns geometry, support nodes, joint identity and explicit
SI density, Ogden terms/bulk modulus, Maxwell spectrum, specific heat and initial
Kelvin temperature. The controller's support metadata uses vectors; no fixed
three-pin storage remains. Duplicate/out-of-range supports reject using one
linear pinned mask. The illustrative constructor delegates to this same owner
with its previous constants and three supports, preserving existing defaults.

Manifest version 1 remains compatible. Version 2 selects
`authored-ogden-maxwell-v1`; every region requires a `material` object with
`density_kg_m3`, `specific_heat_j_kg_k`, `temperature_kelvin`, `bulk_pa`,
`ogden_terms` (each `shear_pa`, `exponent`) and `maxwell_branches` (each
`shear_pa`, `relaxation_seconds`). Its `supports` is a variable-length list.
All physical parameters are required: missing parameters do not fall back to
demonstration values. Shared Ogden/inertia/thermal validation rejects inadmissible
parameters and spectra; unknown fields also reject. These are caller-supplied
parameters, not a claim of independently measured anatomical calibration.

A four-support fixture verifies all requested nodes follow the rig, checks
1200 kg/m3 density against an independent octahedron-volume mass of 0.0128 kg,
and initializes cells at 299 K with 2000 J/(kg K) specific heat. Released heat
was 3.58628324e-9 J; temperature changes independently imply 3.58631951e-9 J,
within the 6.80e-12 J rounding allowance. The heat magnitude is over 100 times
that allowance, so the capacity check distinguishes configured values from a
hidden default. Independent mechanical frame defect was 9.73160435e-8 J within
the unchanged 1e-5 J budget. Invalid density/capacity/temperature/moduli/spectra
and supports reject.

Both manifest versions are tested through observed file input and the shared
controller. A v2 manifest initializes 295 K, four supports and explicit constitutive
parameters, then advances the imported physical-skin path. The example suite
passed 45 tests (three manual qualifications ignored), and a fresh Metal v2
invocation completed three physical steps/12 controller substeps. The main
application executable also compiles; its filtered run contained zero matching
tests and is compilation evidence only. Evidence:
`artifacts/authored-tissue-material-2026-10-06/`. The fixture remains one
illustrative pad with 1/3273 source vertices bound; default full-character coverage
and the step-279 rig obstruction remain unchanged.

The production application library's matching controller-owner test also passed
(with the same independent mass/heat/frame-balance values). This verifies the
shared app module, in addition to the snapshot example and executable compilation.


### Source-index skin coverage admission

Both tissue manifest versions can declare a `coverage` object containing
`minimum_bound_vertices` and `required_vertices` (original concatenated scene
vertex indices at phase zero). Counts/indices must be valid and required indices
unique. Immutable skin bindings now expose their sorted tissue-owned source IDs.
The file importer instantiates and assembles candidate volumes, evaluates their
actual reference-space membership, checks the contract, and only then publishes
the observed asset. This happens before GPU creation in the snapshot entrypoint.
The runtime repeats the same contract against its final shared binding and prints
a `TISSUE_COVERAGE` receipt including actual source/bound counts and source IDs.
Omitting the contract retains legacy compatibility; the report explicitly records
whether a contract was supplied. This is membership admission, not a guarantee
of contact mobility, rig feasibility, calibrated anatomy or collision convergence.

The fixture's one bound vertex is exactly source 666; this explicit requirement
passes. Requests for a higher minimum, source 2591, duplicate/out-of-range IDs
and impossible counts reject. A full-coverage request for 3273 vertices and
[651,666,2591] reports missing [651,2591]. The actual CLI rejected it without
GPU initialization or an output image. A valid contracted Metal invocation
completed three physical steps and emitted the expected membership receipt.
The 45-test example suite passed, followed by a source-identity check confirming
the same bound vertex IDs before and after region assembly. No source geometry
was expanded and the original step-279 obstruction remains unresolved.
Evidence: `artifacts/authored-skin-coverage-2026-10-06/`.

## Nonradial imported volume boundary normals (2026-10-06)

The debug continuum renderer now caches exterior faces from the canonical
`Body::surface()` topology instead of treating the last three nodes of every
tetrahedron as an exterior face. The previous radial-mesh assumption omitted
exterior faces and included internal interfaces for general VXTM volumes.

The regression uses two tetrahedra sharing a face, checks all six exterior
triangles and the normal at the origin, permutes cell node order, and verifies
assembly preserves the result. After two refinements an interior node has zero
boundary normal. The example suite passes 46 tests (3 ignored); the matching
production library regression also passes. This fix does not increase character
skin coverage or resolve the physical-skin clip obstruction at frame 279.
