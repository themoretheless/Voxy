# Prepuce and labial tissue mechanics

The implementation supplies separate neutral adult anatomical mechanical specimens,
not reconstructed patient anatomy or calibrated human tissue predictions. All
lengths are metres, forces newtons, moduli pascals and time seconds.

`skin::PrepuceGeometry::build` constructs one connected shell: an inner annular
sheet, a rounded distal turn and an outer annular sheet. Both proximal boundary
rings are fully fixed in this idealized support condition; the distal crest is
free to deform. There is no duplicated circumferential seam or disconnected inner
leaf. Circumferential material directions feed the existing anisotropic layered
shell. Shell thickness must be less than the radial separation of the sheets.
The model does not yet include a frenulum, dartos activation, a compliant glans,
contact between its own leaves, friction or a retraction/eversion validation.

`skin::LabiaMinoraGeometry::build` produces two mirrored folded shells. Each has
its own geometry, velocity and viscoelastic history. The perimeter is fixed to an
idealized basal support; the raised fold crest is mechanically free. Skin layers
and dimensions are supplied explicitly. These folds are specialized tissues,
including vascular and neural components described by the anatomical study
[Structure and innervation of the labia minora](https://pubmed.ncbi.nlm.nih.gov/22453848/).
The present shell does not model those components or claim that ordinary skin
parameters are measured labial properties.

`biomechanics::LabiaMajoraGeometry::build` supplies two separate half-ellipsoid
volumetric cores, with an interior tetrahedral seed and fully fixed basal disks.
The caller supplies a finite-deformation bulk material. This represents the
mechanical foundation for soft/adipose cores rather than replacing their volume
with a thin sheet. The cores are not yet bonded to overlying skin shells or
registered to the character's pelvis, and have no mutual contact model. The
star-shaped linear-tetrahedron mesh is suitable for this convex specimen only;
quantitative near-incompressible behavior still needs spatial convergence and
assessment of volumetric locking.

The shell uses existing nonlinear matrix/collagen stretch, rest-curvature bending,
incompressible thickness change, Maxwell memory and implicit dynamics. Its existing
positive-gap barrier supports triangle contacts against rigid spheres and planes.
A labial contact regression verifies repulsive force, an accepted nonoverlapping
step and rollback on initial overlap. This is rigid obstacle contact, not soft
labia-to-labia or prepuce-to-glans coupling. No fertility, sensation, injury or
clinical accuracy is implied.

## Runnable mechanical example

```
cargo run --release -p physics --example genital_folds
```

Optional first argument selects the output directory (default
`/tmp/voxy-genital-folds`). The example dynamically loads the prepuce and both
minora fold crests with a total 0.0001 N axial force for four 0.002-second steps,
then releases them for four steps. Shell material values are synthetic:
0.0003 m thickness, density 1000 kg/m3, matrix shear 1000 Pa, collagen modulus
2000 Pa, exponent 4, dispersion 0.1, fiber angle 0.5 rad, and a Maxwell branch
500 Pa/0.02 s. They are not borrowed from vaginal measurements or asserted as
human genital tissue constants. Separate majora cores use synthetic Young's
modulus 3000 Pa and Poisson ratio 0.45, with a 0.001 N apex compression followed
by quasistatic release. Quasistatic majora and dynamic shell phases are distinct;
this is not a coupled whole-genital simulation.

Execution completed and generated ten OBJ surface snapshots (loaded/released
states for five specimens), shell CSV on stdout and `majora.csv` in the output
directory. All OBJ coordinates were independently checked finite and every face
index valid. Majora apex height changed from 0.008 m to 0.007983742314498 m under
load and recovered to 0.007999999953218 m after release. Minimum J under load was
0.9986423901031, with converged residual below 1e-7 N. These values characterize
the synthetic test only.

## Verification and open work

Release tests `genital_folds` (4), `skin` (18) and `biomechanics` (9) passed,
31 total. The new tests cover connected two-boundary prepuce topology,
stress-free folded rest geometry, mirrored minora, positive mass, invalid geometry,
load/release deformation, unchanged roots, positive shell area/thickness,
transactional invalid steps, rigid contact, volumetric majora compression/recovery
and geometric volume refinement against the analytic half-ellipsoid volume.

Anatomical registration, measured layer-specific parameters, frenulum/vascular
attachments, skin-fat bonding, self/mutual contact with friction, complete
retraction/eversion and experimental load-deformation validation remain required.
The specimens are not added to the existing female render mesh yet.

## Urethra and vaginal muscle wall

`biomechanics::UrogenitalWallGeometry` builds bonded four-region solid tubes with
explicit radii, length, mesh resolution and material arrays. `urethra` identifies
regions as mucosa, longitudinal smooth muscle, circular smooth muscle and outer
sphincter/support tissue. `vagina` uses the same numerical topology for mucosa,
longitudinal muscle, circular muscle and outer connective/support tissue. Materials
and excitation are distinct per region; tube template fiber X is circumferential,
Y radial and Z longitudinal. The caller supplies the actual fiber families and
constants; names alone do not fit constitutive properties. Proximal nodes are fixed
and the distal end can shorten. A mathematically closed lumen pressure cavity uses
end caps for pressure forces; this is not a urine-flow or fluid-filled vaginal model.

Human urethral histology distinguishes longitudinal smooth, circular smooth and
striated sphincter structures ([intraurethral ultrasound/histology study](https://pubmed.ncbi.nlm.nih.gov/9464722/)).
Vaginal smooth muscle has active mechanics evaluated in longitudinal and
circumferential directions ([Contractile Properties of Vaginal Tissue](https://pmc.ncbi.nlm.nih.gov/articles/PMC10854261/)).
The implementation is an idealized directional actuator family rather than an
exact reconstruction of interwoven human muscle layers. It omits urethral axial
sphincter distribution, vascular seal/mucosal coaptation, slit-shaped collapsed
vaginal rest geometry, rugae, levator ani and other surrounding pelvic muscles.

Tests separately activate circular region 2 and longitudinal region 1: circular
activation decreases lumen volume, longitudinal activation decreases wall length,
and imposed lumen pressure increases volume. The checks use both urethral and
vaginal mesh dimensions, converged equilibrium, unchanged proximal roots and
positive J. Synthetic constitutive constants are matrix shear 500 Pa, bulk
5000 Pa, fiber stiffness 50 Pa, exponent 2 and active stress 1000 Pa. These are
numerical fixtures, not measured human tissue values.

## Clitoral mechanical substructures

`ClitoralGeometry::build` creates five separate volumetric mechanical bodies in
one local frame: paired continuous corpus/crus solids, a glans ellipsoid and paired
vestibular bulb ellipsoids. Each corpus/crus uses welded sections and conforming
triangular-prism tetrahedral subdivisions along its curved centerline. Supplied
fiber directions are rotated into the local centerline frame segment by segment.
The glans and bulbs use closed ellipsoid surface/star tetrahedral meshes, with
an idealized supported root cap. The parts reflect the distinctions described in
[Anatomy of the clitoris (2005)](https://pubmed.ncbi.nlm.nih.gov/16145367/); their
sizes, exact shapes, material constants and supports are not derived from that
study or registered to the female character.

Continuum corpus-to-glans bonding, reconstructed suspensory/fascial attachments, skin/hood contact,
blood-filled trabecular tissue, vascular circulation and neural control are not
implemented in this specimen. The initial `build` parts move independently; common coordinates
are not proof of a coupled anatomical assembly. The optional spring-coupled assembly
described below now provides discrete mechanical load transfer. No penile tunica material values
are silently used for clitoral tissue. Tests verify all five parts deform under
an applied load with converged positive-J mechanics and closed manifold boundary
meshes; a separate test checks corpus fibers against the actual segment tangents.

## Additional runnable example

```
cargo run --release -p physics --example urogenital
```

Optional first argument selects the output directory (default
`/tmp/voxy-urogenital`). The example applies separate region 1/2 excitation to
urethral/vaginal walls with explicit first-order rise/fall activation and
quasistatic mechanics, then releases excitation. It also applies neutral small
axial loads to the five independent clitoral parts. OBJ files retain rest and
accepted deformed states; CSV reports equilibrium diagnostics. No sexual response,
clinical prediction, physiological vascular filling or full pelvic simulation is
claimed by these mechanical scenarios.

The normal-release `urogenital` example completed: sixteen OBJ surface files and
nine equilibrium records were independently checked for finite coordinates,
valid face indices, positive J and residuals at or below 1e-7 N. Minimum J for
urethral activation/release was 0.9978762105348/0.9985728886116; vaginal
activation/release was 0.9980311913274/0.9986806252200. The lowest clitoral
part J in the example was 0.9813340421793. Release suites `urogenital` (3),
`genital_folds` (4) and `biomechanics` (9) passed, sixteen total. These constitute
numerical mechanical checks, not anatomical registration or measured physiological
validation.

## Discrete mechanical coupling between clitoral parts

`Body::assemble_tissues` combines reference/current geometry, element constitutive
assignments, activations, dead loads, cavities, stationary supports and existing
bonds without rebasing deformed tissues into a new stress-free state. Node ranges
identify the original components; original material-region identities are retained.
Uniform-pressure storage remains rejected by this merge API. Cell-resolved
storage and viscous histories are now preserved, as described below.

`Body::add_tissue_bonds` installs central elastic attachments with energy
`0.5*k*(current_length-reference_length)^2`, stiffness in N/m and reference length
from the original rest coordinates. Both tension and compression are supported.
This central-force energy is invariant under rigid rotation/translation and its
exact gradient enters the ordinary FEM equilibrium and inertial solvers. The
preconditioner includes bond stiffness in addition to solid/fluid terms. A batch
with an invalid or duplicate node pair, collapsed bond or overflow commits nothing.
Bond forces remain discrete attachment forces; element Cauchy diagnostics do not
include an invented smeared attachment stress.

`ClitoralComplex::coupled(k, neighbors)` removes independent root supports from
the glans and bulbs, connects their prior basal caps to nearest corpus/crus
surface nodes, and retains the two crus basal supports. Each basal node gets the
specified total spring stiffness split equally across 3..8 neighbors. This is an
explicit nearest-surface attachment hypothesis, not reconstructed fascia or
measured ligament mechanics. Rest gaps are retained; no vertices are welded or
moved to force an artificial interface. It does not supply self-contact, sliding,
friction, damage, blood filling or neural response.

Tests independently check bond energy against the analytic spring extension,
all selected gradient components against finite differences, rigid-frame
objectivity, transactional installation, load transfer from the coupled glans
into deforming corpus/crus nodes and balancing reactions at fixed crus roots.
A separate free-body inertial test verifies that attachments participate in
motion while preserving linear/angular momentum and the guarded energy balance.

The `urogenital` example now additionally exports coupled-before/coupled-loaded
OBJ states. It clears prior individual demonstration loads before applying a
single 0.0001 N glans load to the combined assembly. Normal-release execution
completed with minimum J 0.9749768435126 and residual 9.466746069050e-8 N. The
assembly's starting state retains the previous accepted individual deformations;
these are not silently rebased. The discrete stiffness used is synthetic 1 N/m
per basal node with six neighbors, not a human clitoral attachment calibration.

Release suites `urogenital` (6), `finite_inertia` (4), `biomechanics` (9) and
`organ_coupling` (6) passed, 25 total. Both coupled OBJ exports were independently
checked for finite coordinates and valid indices (255 vertices, 464 triangles
per export). Full anatomical and physiological validation remains incomplete.

## History-preserving viscoelastic tissue assembly

`Body::assemble_tissues` now accepts Ogden-Maxwell element laws. Cloning the complete
element retains equilibrium parameters, all branch memories and the trial-time
state; node renumbering does not alter the reference tensor or rest geometry.
Geometry, energy and stress are not reset to manufacture an unstressed assembly.
Uniform-pressure inventories remain unsupported; cell-resolved inventories
are now supported as described below. The existing
`relax_step` stages the whole bonded assembly, freezes histories during equilibrium
and commits each viscous branch once after convergence. Inertial dynamics still
rejects viscoelastic cells; this feature concerns timed quasistatic mechanics.

New tests assemble independently loaded tissues with different previous relaxation
histories. Reference/current positions, stresses, response energy and nodal
gradients match the original parts exactly at merge. A subsequent uncoupled
assembly relaxation agrees with separately advanced bodies within specified
numerical tolerances. After installing a mechanical bond, a deliberately rejected
step preserves geometry, energy and both constitutive histories; an accepted step
updates them. Release suites `tissue_assembly_history` (2), `viscoelastic` (8) and
`urogenital` (6) passed, sixteen total.

```
cargo run --release -p physics --example urogenital_relaxation
```

This example assigns synthetic Ogden-Maxwell laws to the glans and both bulbs
(matrix shear 500 Pa, exponent 2, bulk modulus 5000 Pa, one branch 1000 Pa/0.1 s),
then spring-couples them to passive corpus/crus tissues. A constant 0.0001 N glans
load is held for eight 0.02-second steps and released for four further steps.
Normal-release execution completed with converged positive-J mechanics. Independent
CSV inspection verifies finite values, monotonically increasing compression under
load and recovery after release. Glans displacement progressed from
-2.100396120308e-4 m at 0.02 s to -2.215234280869e-4 m at 0.16 s. After release it
was -1.283946797763e-5 m at 0.18 s and -1.072366653080e-5 m at 0.24 s. Minimum J
across recorded steps remained above 0.9981147269761. The exported final OBJ has
255 finite vertices and 464 triangles with valid indices.

These results establish numerical creep and delayed recovery for the chosen
synthetic assembly, not measured human genital viscoelasticity, temporal/spatial
convergence of this anatomical scenario or physiological excitation. The model
still lacks continuum interfaces, skin/hood and soft-body contact, calibrated
material data, vascular filling, nerves and anatomical registration.

## Cell-resolved pore inventory in coupled tissues

`Body::assemble_tissues` now preserves cell-resolved Biot fluid stores when every
part explicitly supplies one store per tetrahedron. Fluid/reference-fluid volumes,
Biot coefficients and storage values remain unchanged and retain element order.
`TissueAssembly::cell_ranges` maps original tissue cells into the combined body,
in addition to node ranges. Dry-only assembly continues to work. A mixture of dry
and cell-stored parts rejects rather than inventing fluid inventories for dry
cells. Uniform-pressure stores also reject: converting independent lumped stores
to one pressure would erase their separate inventory/pressure semantics.

New tests merge predeformed, independently loaded Ogden-Maxwell specimens with
different fluid contents. Per-cell pressure, storage energy, whole-body energy,
nodal gradients, viscous histories and total fluid inventory agree with the
original parts immediately after assembly. A subsequent bonded viscoelastic step
retains all fluid volumes while pressure responds to geometry. This is sealed
storage during relaxation, not transport. Dry/cell-store and uniform-store inputs
are explicitly checked for rejection. Release suites `tissue_assembly_history`
(4), `viscoelastic` (8) and `urogenital` (6) passed, eighteen total.

```
cargo run --release -p physics --example urogenital_pore
```

The example assigns explicit synthetic cell stores to all five clitoral parts
(reference fluid volume 0.3 times cell volume, Biot coefficient 0.8, storage
1e-4 times cell volume in m3/Pa). It spring-couples the assembly and prescribes
added fluid in the two corpus/crus bodies at fractions 0, 0.005, 0.01, 0.005, 0
of their reference tissue volume. There is no imposed fluid change in the glans or
bulbs. Each stage solves the coupled pressure/solid equilibrium. The cells' pore
pressure is a mechanical fluid-storage pressure; it is not an independently
validated cavernous blood pressure or sinusoidal vascular model.

Normal-release execution completed and independent CSV inspection verified
finite values, positive J, converged residuals below 1e-7 N and expanding corpus
volume with added inventory. At fraction 0.01 the corpus volume ratio was
1.006890653313 and reference-volume-weighted mean pore pressure 44.87477349857 Pa.
After inventory returned to its original 9.796466661653e-7 m3, corpus volume ratio
was 0.9999999958133. The small numerical deviation is not material hysteresis.
Fluid totals change by prescribed input/output at the stages; this is not a closed
vascular mass-conservation experiment. Stage numbers are not physiological time.

Independent blood/lymph flow, trabecular microstructure, vascular wall regulation,
nonlinear storage/saturation, soft contact, neural excitation and anatomical
registration remain open. The current network transport adapter still rejects
viscoelastic tissues; this assembly feature preserves storage and history but
does not claim a coupled fluid-transport/viscoelastic time integrator.

Additional release regressions `implicit_pore` (13) and `mixed_darcy` (27) passed.
Together with the eighteen assembly/viscoelastic/urogenital checks above, this
turn verified 58 tests; transport regressions do not by themselves validate the
new prescribed-inventory anatomical example as a vascular network.

## Finite-reservoir porous transport

`PorePerfusion` constructs one transport compartment per explicit FEM cell,
current-geometry RT0 connections on interior faces, and caller-specified passive
ports to finite external reservoirs. Protein inventory and osmotic coefficients
must be supplied explicitly for every cell. Cell-count mismatches and duplicate
ports are rejected. A transport step updates fluid inventory and elastic
mechanical equilibrium transactionally; a failed step preserves both states.

```
cargo run -p physics --example urogenital_perfusion
cargo test -p physics --test perfusion --test urogenital --test genital_folds
```

The perfusion example uses the five-part spring-coupled clitoral specimen,
permeability 1e-12 m2, viscosity 0.001 Pa s, two finite 1e-6 m3 reservoirs
initially at 100 and 0 Pa, and four passive ports of conductance 1e-11
m3/(Pa s). Initial protein concentration is uniformly 10 kg/m3 and osmotic
coefficients are zero. These are synthetic numerical controls. Tissue inventory
changes through exchange flux rather than prescribed inventory increments.
The example checks total fluid/protein conservation and positive element J.

The fixed-solid two-compartment test independently compares transferred volume
with the exact exponential solution for two linear reservoir compliances,
verifies first-order time convergence by halving the step, checks the signed
port ledger and uniform protein concentration, and tests
invalid-step rollback and input-count rejection. The adapter currently requires
elastic cells. It does not establish blood-cell transport, vascular regulation,
physiological perfusion, or coupling to the full female surface model.

Current debug verification passed: `perfusion` (2), `urogenital` (6), and
`genital_folds` (4), twelve tests total. The perfusion example completed five
1 ms intervals with minimum J above 0.99992; absolute total-water drift stayed
below 1.3e-21 m3 and protein drift below 4.1e-20 kg. This short synthetic run
verifies inventory accounting and non-inversion, not physiological calibration.

## Second-order reservoir/FEM transport

`CellPoreTissue::step_mixed_darcy_second_order` and
`PorePerfusion::step_second_order` explicitly select conservative SSPRK2 without
requiring lymphatic wall overrides. Each stage recomputes mechanical equilibrium,
cell pressure and current-geometry RT0 flux. Existing first-order entrypoints
retain their behavior. Both stage solves use the same stored elastic material;
this is not a viscoelastic history integrator.

The perfusion example now uses this second-order path. Release execution completed
and independent CSV inspection checked five finite states, positive J, conserved
printed total volume, absolute water drift below 2e-21 m3 and protein drift below
5e-20 kg. Minimum J was 0.9999234676333. Results are saved in
`docs/urogenital-perfusion-second-order.csv`; the short synthetic run is not an
anatomical validation or a temporal-convergence study of the complete specimen.

The fixed-solid reservoir test checks both the continuous exponential solution
and the exact discrete SSPRK2 amplification factor. With steps 1, 0.5 and 0.25 s
over 10 s, successive absolute-error ratios are within 0.05 of four. A separate
free-apex tetrahedron test checks that successive differences in fluid inventory
and apex height have ratios within 0.15 of four, with positive J and matching
FEM/network inventories. Solver tolerance is 1e-11 N. This distinguishes actual
mechanical coupling from a transport-only convergence check.

Verification passed: five release `perfusion` tests and 27 debug `mixed_darcy`
regressions. The new predictor-failure test limits the solid solver to one
iteration and verifies rollback of positions, cell fluid, network water/protein
inventories and cached pressures after FEM nonconvergence.

## Adaptive coupled perfusion

`PorePerfusion::step_adaptive` applies SSPRK2 step doubling to the complete
elastic tissue/reservoir state. Error control includes volume, protein mass and
nodal position; the position estimate is the Euclidean coarse/fine discrepancy
divided by three. Accepted results use two half steps. Rejected trials and failed
complete intervals cannot partially commit geometry or transport inventories.
This controls local temporal error; it does not bound global error or eliminate
spatial discretization or material/calibration error.

```
cargo run --release -p physics --example urogenital_perfusion -- --adaptive
cargo test --release -p physics --test perfusion
```

Seven release tests passed, including adaptive rejection against a 0.025 s fixed
reference on a deformable tetrahedron, joint signed ledgers, and complete-interval
rollback after trial-budget exhaustion. The five-part anatomical example also
completed: stages 1–4 accepted one step each; stage 5 rejected one trial and
accepted two steps. Accepted normalized error estimates stayed below 0.847.
Independent CSV checks verified five finite states, positive J, water drift below
2e-21 m3 and protein drift below 5e-20 kg. Results are saved in
`docs/urogenital-perfusion-adaptive.csv`.

The example's explicit tolerances are 1e-15 m3 for volume, 1e-14 kg for protein
and 1e-7 m for position, with zero relative tolerance. They are numerical controls,
not measured physiological parameters. Full-body registration, viscoelastic
transport and physiological tissue calibration remain outstanding.

## Oval layered walls

`elliptical_tube` and `UrogenitalWallGeometry::oval_wall` construct homothetic
elliptical layers directly as the reference geometry. The X/Y scale factors
multiply the supplied radii. This preserves distinct radial material regions and
creates no artificial prestrain. Muscle template X/Y/Z axes map to an orthonormal
frame of ellipse tangent, outward normal and longitudinal axis. In particular,
normal fibers are not naively sheared radial vectors. Unit scales reproduce the
existing circular construction. Invalid/nonpositive/nonfinite scales are rejected.

```
cargo run --release -p physics --example urogenital_oval
cargo test --release -p physics --test urogenital --test biomechanics
```

The example builds uncalibrated urethral and vaginal wall specimens with axis
scales [1, 0.6] and [1, 0.3]. Each runs rest, 20 Pa pressure, 0.03 longitudinal
activation and 0.03 circular activation as separate equilibria. Material constants
are the same synthetic values as the earlier wall demonstration, not measured
human fits. All eight release states converged below 1e-7 N residual with positive
J. Independent CSV checks confirmed pressure increases lumen volume, circular
activation decreases lumen volume and longitudinal activation shortens both
specimens. Eight exported OBJ surfaces have finite coordinates, valid indices
and exactly two incident faces per boundary edge. Results are saved in
`docs/urogenital-oval.csv`; meshes are exported to `/tmp/voxy-urogenital-oval`.

Seventeen release tests passed. New checks verify the exact polygonal elliptic
lumen volume, stress-free reference J, tangent/normal fiber orthogonality, unit-
scale compatibility, input rejection, fixed basal supports and directional muscle
response. An oval finite lumen is still an idealization: closed wall contact,
rugae, anatomical axial variation, urine flow, pelvic floor attachment and
registration to the full body are not implemented by this geometry change.

## Explicit axial tissue and muscle zones

`elliptical_tube_axial` and `UrogenitalWallGeometry::axial_wall` accept explicit
materials for every longitudinal segment and radial layer. In a four-layer wall,
region `4 * segment + layer` selects each zone independently. Shared nodes across
segment boundaries preserve the continuous tetrahedral solid, without springs or
mesh duplication. Materials, constitutive validation and preconditioner are built
from the actual cell material. Uniform/circular entrypoints retain their previous
layer-only region IDs. No measured axial distribution is supplied by this API.

```
cargo run --release -p physics --example urogenital_axial
cargo test --release -p physics --test urogenital --test biomechanics --test muscle_activation
```

The synthetic example uses three axial segments and puts an outer circumferential
muscle only in the middle segment (region 7). Rest, pressure, all-segment
longitudinal activation, all-segment circular activation and local outer activation
are separate equilibria for both specimens. Independent CSV inspection verified
ten finite states with residuals below 1e-7 N, positive J, pressure expansion,
longitudinal shortening and lumen-volume reduction with either circular or local
outer activation. Ten OBJ exports have valid indices, finite coordinates and two
incident faces per boundary edge. Results are in `docs/urogenital-axial.csv` and
meshes in `/tmp/voxy-urogenital-axial`.

Release suites `urogenital` (9), `biomechanics` (9) and `muscle_activation` (5)
passed. The axial-profile test checks region/material assignment, mesh and cavity
preservation, localized drive history, fixed basal nodes and rejection of count
mismatch or invalid material in a later segment. This enables localized muscle
scenarios; it does not establish anatomical sphincter extent, neural innervation,
pelvic-floor attachment, wall contact or physiological parameter fits.

## Visible computed stage

`tools/render_urogenital_stage.py` renders the actual exported axial OBJ meshes and
measurements from `docs/urogenital-axial.csv` to
`docs/urogenital-axial-render.png`. It shows rest, 20 Pa pressure and middle outer
muscle activation for both walls, with fixed scale within each specimen row.
The orthographic cutaway removes half the boundary triangles for visibility;
distal-ring insets project the computed five rings onto XY. Geometry and
displacements are not magnified. Four ring-layer bands use presentation colors,
not a stress or anatomical texture map. The figure was visually inspected.

This is a coarse computational specimen, not a photorealistic organ, a full-body
render or evidence of closed-wall contact. The rendering's fixed assumptions
(three axial segments, five rings, eight sectors) are checked before drawing.

## Selected-pair contact barriers

`Body::add_tissue_gaps` installs explicit frictionless node-pair distance barriers.
For gap g = distance - minimum clearance and activation gap h, the potential is
`-k (g-h)^2 ln(g/h)` when 0 < g < h, otherwise zero above h. Closed gaps are
rejected. Its gradient gives equal/opposite central forces; no net internal force
or torque is introduced. The law uses stable logarithms near both activation and
closure. Parameters are caller supplied, not calibrated tissue contact constants.

Body evaluation includes contact energy and force. Tissue assembly remaps and
preserves installed pairs. Quasistatic line search and inertial integration check
the exact minimum pair distance along each straight nodal step, preventing a
step from jumping through a protected pair even with valid endpoint distances.
Muscle midpoint predictors and accepted motion are checked too. Invalid batches
or failed motion retain previous state. Contact forces are discrete nodal forces;
they are not reported as continuum element Cauchy stress.

```
cargo run --release -p physics --example urogenital_contact
cargo test --release -p physics --test tissue_gaps --test urogenital --test finite_inertia --test tissue_assembly_history
cargo test --release -p physics --test muscle_dynamics --test perfusion
```

38 distinct release tests passed across those suites. New tests check analytic
barrier energy, finite-difference force gradient, rotation/translation objectivity,
internal force balance, transactional installation, assembly remapping, compressed
equilibrium and a crossing trajectory whose endpoints alone would appear valid.

The demonstrator compares rest, compression without barriers and compression
with barriers in both oval specimens. Opposing inner-ring Y-axis nodes at three
free axial stations are selected. Minimum distance is 10% of each reference pair
distance; activation gap is 95% of the remaining reference gap; k is 10 N/m.
Dead loads are 0.002 N per selected urethral node and 0.01 N per selected vaginal
node. All six equilibria converged below 1e-7 N residual with positive J. Independent
CSV checks verified active positive barrier energy and increased selected pair
distances under identical loads. The figure `urogenital-contact-render.png` shows
actual computed meshes and centred XY projections of distal rings, with no
magnification of displacement. It was visually inspected. Numerical values are
saved in `docs/urogenital-contact.csv`.

The visibly substantial bending is retained in the result. Protecting selected
pairs does not prevent collisions between other nodes, edges or triangles, and
does not prove global surface injectivity. This is a discrete-contact foundation,
not complete wall coaptation, full-body soft contact or calibrated anatomy.

## Contact load–release cycle and L-BFGS equilibrium

`Body::equilibrate_lbfgs` is an alternative energy minimizer with twelve stored
secant pairs, diagonal preconditioning, curvature checks and an Armijo search.
Fixed-node gradients are masked. The same FEM/contact potential and selected-pair
path restrictions are used; no rest geometry or material history is rebased.
Iterations are numerical optimization, not physiological time. A nonconverged
report retains accepted iterates, like the existing CG method; it must be checked
before treating a state as equilibrium. Invalid initial states/controls do not
commit geometry. The existing CG entrypoint is unchanged.

The strongly bent wall highlighted a practical CG convergence limit during
unloading: at load factor 0.59375 it exhausted 100000 iterations with residual
1.60425e-5 N. Continuing accepted iterates reduced the residual but was slow.
The L-BFGS path completed the full specimen experiment, including unloading in
four equal load decrements with the installed barriers retained. This is a
quasistatic elastic recovery experiment, not viscoelastic recovery timing.
Symmetric buckling can follow a different branch with a different optimizer;
matching residuals do not prove identical loaded trajectories.

```
cargo run --release -p physics --example urogenital_contact
cargo test --release -p physics --test lbfgs --test tissue_gaps --test urogenital
```

17 distinct release checks passed: three L-BFGS checks, five gap checks and nine
urogenital regressions. They include comparison against the small elastic/contact
CG equilibrium, recovery with contact retained, invalid-control preservation, and
a complete bent oval wall load–release regression with fixed supports and
positive selected clearances at every load stage.

Both wall specimens completed eight reported rest/compression/contact/released
states in total. Independent CSV checks verified finite values, positive J and
force residuals below 1e-7 N. Exported rest/released nodes were independently
compared: maximum differences were 1.1912682e-6 m for the urethral wall and
4.3529750e-7 m for the vaginal wall. Released/rest lumen volume ratios were
0.9999992423823554 and 0.9999998087797894. These small residual differences are
numerical tolerances, not material hysteresis.

Measurements and proof are saved in `urogenital-contact-cycle.csv` and
`urogenital-contact-cycle-proof.json`. The visually inspected actual-mesh figure
`urogenital-contact-cycle-render.png` compares rest, loaded barrier and released
states at fixed scale, with no displacement amplification. Distal XY ring
projections are centred for comparison; they do not show absolute translation.
The coarse mesh, point loads, selected-pair contact and synthetic materials remain
substantial limitations. Full triangle/edge collision and anatomical calibration
are still required.

## Angular refinement audit: surface contact is incomplete

The contact example now accepts angular resolution as its second argument after
output directory. Supported counts are 8..128 divisible by four. Pair indices are
computed from the actual resolution, retaining the same three axial stations,
reference opposing points, physical loads, material profile and gap parameters.
Only angular resolution changes; this is not complete spatial refinement.

```
cargo run --release -p physics --example urogenital_contact -- /tmp/voxy-urogenital-contact-16 16
cargo run --release -p physics --example urogenital_contact -- /tmp/voxy-urogenital-contact-32 32
```

Both new runs completed; together with the existing 8-sector run, independent
CSV checks verified 24 finite equilibrium states, positive J and force residuals
below 1e-7 N. Released/rest volume ratios stayed within 1e-5 of one. These checks
DO NOT prove mesh-independent loaded behavior. Loaded/rest vaginal volume ratios
were 0.85543, 0.52381 and 0.02315 for 8, 16 and 32 sectors. Urethral ratios were
0.90367, 0.84903 and 0.80937. The large variations contradict a claim of spatial
convergence for the current point-load/selected-pair experiment.

The existing static BVH audit and triangle classifier were applied to exported
rest, loaded-barrier and released surfaces at every resolution. The loaded
vaginal surfaces contained 20 transverse triangle crossings at 16 sectors and
160 at 32; no crossings were found by this audit in the other sixteen inspected
states. This is floating-point static evidence with shared-vertex pairs excluded,
not CCD certification or a complete proof that the other surfaces are valid.
Positive element J and selected pair clearances are insufficient to ensure global
surface injectivity. Full surface contact is now directly evidenced as necessary.

Results are saved in `urogenital-contact-refinement.csv`,
`urogenital-contact-refinement-proof.json` and
`urogenital-contact-refinement-intersections.json`. The visually inspected figure
`urogenital-contact-refinement-render.png` compares actual meshes at fixed scale
and overlays projected crossing-face outlines in red, including faces hidden by
the presentation cutaway. It is a diagnostic figure showing a defect, not a
validated anatomical result. Renderer refinement mode uses actual ring counts
and optionally reads the static audit; geometry is never synthesized.

## Nonadjacent triangle surface contact

`TissueSurfaceContact` adds a configured set of triangles, clearance, activation
gap and discrete per-pair stiffness to the existing FEM energy. Disjoint triangle
distance evaluates vertex/face and edge/edge closest features; an edge/face
crossing is explicitly detected as zero distance. The barrier gradient is
barycentrically distributed to both triangles, preserving internal force and
torque balance. Degenerate triangles and closed clearance are rejected.

Dynamic AABB sweep-and-prune rejects far face pairs. Straight solver/integrator
steps use conservative advancement with the sum of maximum triangle vertex
speeds as a distance-change bound. Uncertifiable trajectories are rejected after
a bounded number of advances. This prevents endpoint-only tunnelling for the
configured nonadjacent pairs. As elsewhere in this project, calculations are
floating point and do not establish formal exact-arithmetic collision guarantees.

`Body::set_surface_contacts` replaces configured groups transactionally. Assembly
remaps/preserves each group. Separate groups do not interact: to include contact
between assembled tissues, explicitly configure a group containing both surfaces.
`minimum_surface_contact_distance` measures configured nonadjacent pairs only.
Faces sharing any vertex are excluded, so this remains incomplete for adjacent
foldover cases. Stiffness is a discrete pair coefficient, not a mesh-independent
contact constitutive fit. Friction and contact regularization remain outstanding.

```
cargo run --release -p physics --example urogenital_contact -- /tmp/voxy-urogenital-surface-16 16 --surface
cargo test --release -p physics --test surface_contact --test tissue_gaps --test lbfgs --test finite_inertia --test tissue_assembly_history --test urogenital
```

29 distinct release tests passed across those suites: vertex/face and edge/edge
force gradients, energy/objectivity, force/torque balance, true triangle crossing,
transactional configuration, assembly remapping, endpoint-disjoint tunnelling,
and the existing solid/contact/assembly/regression cases. The 16-sector wall
example completed all eight equilibrium states and release with surface contact
enabled in the protected branch. It uses clearance 1e-6 m, activation gap 5e-5 m
and pair stiffness 1 N/m, all synthetic numerical controls.

The independent static classifier detected the previous 20 transverse crossings
in the saved selected-pair vaginal result and none in the new protected loaded
surface, either rest surface or either released surface at 16 sectors. This is
specific evidence for that reproduction, not a proof of mesh convergence or full
anatomical validity. Buckling may select different branches between solves;
changes in loaded volume cannot be attributed exclusively to contact.

`urogenital-surface-16.csv`, `urogenital-surface-comparison.csv` and
`urogenital-surface-comparison-intersections.json` retain measured states and
comparison audit. The actual-mesh figure `urogenital-surface-comparison-render.png`
shows rest, prior point-pair result and new protected result at fixed scale.
Shared-vertex exclusions, point loads, coarse axial/radial resolution and
uncalibrated materials still prevent treating this as physiological anatomy.

## Conservative contact-distance pruning

A two-second macOS process sample of the original active 32-sector run showed
substantial time in exact triangle closest features. The implementation now
checks lower bounds from both triangle planes and the Euclidean separation of
axis-aligned boxes before expensive distance evaluations. It subtracts a
coordinate-scale roundoff allowance. Energy/force evaluations skip pairs whose
bound exceeds the activation range; straight-path checks can certify remaining
motion directly when the bound exceeds clearance plus the vertex-speed travel
bound. The original active process was retained, and a separate optimized run
uses the same physical inputs and its own output directory.

Release checks passed: 10000 random oblique triangle pairs verified the lower
bound against exact closest features; another 10000 configurations compared
pruned and unpruned energy, gradient arrays and contact errors exactly. Four
surface-contact integration tests also passed, including tunnelling rejection.
These are numerical checks in the tested domain, not exact-arithmetic proofs.

```
cargo test --release -p physics --lib lower_bound_tests
cargo test --release -p physics --lib pruning_preserves_unpruned
cargo test --release -p physics --test surface_contact
VOXY_CONTACT_BENCH_OBJ=/tmp/voxy-urogenital-surface-16/vagina-barrier.obj cargo test --release -p physics --lib frozen_wall_pruning_work_and_time -- --ignored --nocapture
```

On the saved 448-triangle protected wall pose, each evaluation considered 3017
AABB candidate pairs. Plane/box bounds culled 2422 (80.28%), leaving 595 exact
feature checks. Pruned/unpruned energy and force arrays agreed exactly. One matched
100-evaluation measurement took 0.058264625 s without pruning and 0.018388875 s
with pruning. This is a contact-energy kernel measurement on one frozen pose,
not whole-solver throughput or evidence that the pending 32-sector equilibrium
has converged. The opt-in fixture benchmark is ignored by default and requires
its OBJ path explicitly.

The source mesh hash and measurements are saved in
`surface-contact-pruning-benchmark.json`; the visually inspected
`surface-contact-pruning-render.png` shows measured work and timing reductions.
The original and optimized 32-sector handles were still active at the latest
observation; their loaded vaginal results remain unverified until they complete.

### Trajectory pruning regression

The accelerated conservative-advancement guard now has an unpruned reference
mode used by its unit test. Across 10,000 deterministic random two-triangle
linear trajectories, 2,871 were accepted and 7,129 rejected. No accelerated
acceptance was rejected by the reference guard. This checks the pruning change;
it is neither an exact-arithmetic CCD proof nor validation of anatomical motion.
The counts are saved in `surface-contact-trajectory-proof.json` and rendered in
`surface-contact-trajectory-render.png`.

Run: `cargo test --release -p physics --lib path_pruning -- --nocapture`.

### Completed 32-sector urethral surface-contact cycle

The optimized run has completed all four urethral stages while its vaginal
surface-contact equilibrium is still running. Rest, protected loaded and released
OBJ meshes are rendered at fixed scale in `urethral-surface-32-render.png`.
The protected loaded/rest signed lumen volume ratio is 0.8086975; the released
ratio is approximately 0.9999977. Minimum J is 0.8121403 under load and
0.9999456 after release; force residuals are below 1e-7 N. Exact CSV rows and
mesh hashes are saved in `urethral-surface-32.csv` and
`urethral-surface-32-proof.json`. This is synthetic wall mechanics, not a fitted
human urethra or evidence of completed full-body integration. Both selected-pair
and nonadjacent-face barriers remain installed during this cycle.

The renderer accepts `--specimen urethra --surface --contact --release --sectors 32`
for a completed specimen without requiring other unfinished specimens.
The four surface-contact integration tests passed again after trajectory pruning.

### Independent static audit of the 32-sector urethral cycle

`tools/audit_urogenital_surface.py` loads the exported OBJ files and uses the
Python geometry BVH/intersection classifier, independently of the Rust solver.
For rest, protected loaded and released urethral states, it found no candidate
nonadjacent-face intersections; no finding limit was reached. Source mesh hashes,
triangle counts and narrow-phase work are saved in
`urethral-surface-32-intersections.json`. The audited visualization is
`urethral-surface-32-audited-render.png`. Shared-vertex pairs are excluded, and
this floating-point static check does not certify continuous motion or anatomy.

Reproduce with `python3 tools/audit_urogenital_surface.py --meshes /tmp/voxy-urogenital-surface-32-optimized --specimen urethra --sectors 32 --output docs/urethral-surface-32-intersections.json`.

### Reject degenerate candidate geometry before trajectory pruning

A stationary collinear triangle whose AABB overlaps a slanted triangle exposed
an accelerated-guard regression: the plane distance bound accepted the candidate
before exact-feature validation rejected its zero area. The trajectory guard now
checks both candidate triangle areas before any distance certificate. The new
regression fails on the pre-fix kernel and passes on the corrected kernel.
An isolated harness assembled from the current distance/contact source and vector
operations passed four kernel tests, with one fixture-dependent benchmark ignored.
Full Cargo verification was blocked by an unrelated unresolved `super::dot` import
in `liquid/film_rebound.rs`; isolated verification does not cover Body integration.
The reproducer geometry and result are recorded in
`surface-contact-degenerate-proof.json` and `surface-contact-degenerate-render.png`.
Previously launched wall processes retain their loaded earlier implementation.

### Unprotected 32-sector vaginal compression is invalid

Independent static classification of the current optimized run's rest and
unprotected compressed vaginal OBJ files found zero rest candidates and 172
transverse segment crossings under load, with no audit limit reached.
`vaginal-unprotected-32-intersections.json` retains the exact mesh hashes and
face pairs. The red outlines in `vaginal-unprotected-32-render.png` identify
these actual findings, including hidden-half projections. The negative signed
lumen measure is not a physical fluid volume. Positive element J and a residual
below 1e-7 N do not establish a geometrically admissible equilibrium.
Protected vaginal equilibrium remains pending in both live runs.

The full crate compiled again after concurrent liquid edits; both trajectory
pruning unit tests passed through Cargo, including the degenerate regression.

### Observed equilibrium progress

`Body::equilibrate_lbfgs_observed` reports iteration, energy (J), and free-node
force residual (N) initially and after every accepted step. The existing method
uses a no-op observer. A regression test verifies identical final geometry,
iteration count and residual, sequential observations including the initial state,
and no observation on invalid controls. All four L-BFGS integration tests passed.
The contact example's optional `--trace` reports every 100 accepted iterations
and final stage residuals to stderr; CSV measurements remain on stdout.
Existing long-running processes cannot acquire this new observer retroactively.

A fresh 8-sector surface-contact diagnostic run reached protected loaded
urethral equilibrium at 3,807 iterations (9.3969e-8 N) and vaginal equilibrium at
5,250 iterations (9.6494e-8 N). The sampled curves are saved in
`urogenital-lbfgs-trace-8.csv` and `urogenital-lbfgs-trace-8-render.png`.
These are iteration histories of synthetic mechanics, not elapsed biological time
or evidence of angular convergence. The separate 32-sector runs remain pending.

### Independent audit of the observed 8-sector full cycle

The fresh run using the corrected degenerate-candidate guard completed all eight
states. Its six rest/protected-load/release boundary meshes were independently
audited: no nonadjacent intersection candidates were found and no audit limit
was reached. The full measurements are in `urogenital-surface-8.csv`, source hashes
and geometry audit in `urogenital-surface-8-intersections.json`, and actual cutaway
meshes in `urogenital-surface-8-cycle-render.png`. Released/rest signed lumen ratios
are 0.9999992424 for the urethral wall and 0.9999998347 for the vaginal wall.
This confirms this synthetic coarse-mesh cycle only; it does not establish
mesh-independent loaded behavior, shared-vertex contact validity or human anatomy.

### Live 32-sector sampling evidence

A two-second macOS stack sample of optimized PID 93762 contained 1,685 main-thread
samples. Disjoint branches accounted for 715 in surface energy evaluation and
664 in the gap/path guard, with 306 elsewhere. This short sample estimates where
CPU time is spent; it does not expose iteration residuals or prove convergence.
Raw sample, counts and rendering are retained in
`surface-contact-32-live-sample.txt`, `surface-contact-32-live-profile.json` and
`surface-contact-32-live-profile-render.png`. Both original and optimized process
handles were confirmed live; no calculation was restarted on observation timeout.

### Prepared per-evaluation contact geometry

Contact energy now prepares each face's coordinate bounds, normal, normal length
and coordinate scale once per evaluation. The cache is rebuilt for every trial
geometry; no state survives deformation. Pair bounds use the same arithmetic as
the original function. Ten thousand deterministic random comparisons matched the
original bounds exactly, and ten thousand energy/gradient comparisons matched the
unpruned reference. Degenerate-candidate and trajectory equivalence tests passed.

On the frozen 448-face mesh, one local timing of 100 evaluations measured
94.460583 ms without pruning versus 15.929292 ms with cached pruning. This is
not a matched timing against the previously pruned implementation and does not
establish whole-simulation speedup. Exact comparison results and fixture hash are
saved in `surface-contact-prepared-benchmark.json`; the plot is
`surface-contact-prepared-render.png`. Existing live processes retain the earlier
loaded code; neither was restarted to obtain this measurement.

The prepared-geometry change also passed 30 integration tests: surface_contact (4),
finite_inertia (4), lbfgs (4), tissue_assembly_history (4), tissue_gaps (5), and
urogenital (9). This covers contact gradients and tunnelling rejection, rigid-motion
invariants, observed equilibrium, material-memory preservation during assembly,
selected-pair loading/release and idealized wall/muscle mechanics. Source hashes
and counts are retained in `surface-contact-prepared-validation.json`.
Rustfmt checks passed for the modified contact/distance/solver source, solver tests
and contact example. These checks do not certify anatomy or complete full-body physics.

### Distributed reference-area external traction

The contact example accepts `--reference-pressure=20` to apply an inward
reference-area dead traction of 20 Pa to the outer lateral wall triangles.
Each triangle's force is minus pressure times its oriented area vector, shared
one-third to each vertex; axial end caps are excluded. Loads are computed once
from reference geometry and scaled through the existing four unloading steps.
They do not update direction or area during deformation, so this is explicitly
not a follower-pressure law. The point-force mode remains the default.
The patch test verifies total force and its first moment under triangulation
subdivision. The fresh 8-sector run completed all eight states with residuals
below 1e-7 N. Protected loaded signed lumen changes were -3.6769 percent for the
urethral wall and -5.6663 percent for the vaginal wall. Both recovered on unloading.
Independent static audits of all six rest/protected-load/release boundary meshes
found no nonadjacent intersection candidates. Measurements, audit source hashes,
and actual rendered meshes are saved in `urogenital-reference-pressure-8.csv`,
`urogenital-reference-pressure-8-intersections.json` and
`urogenital-reference-pressure-8-render.png`. Tissue constants remain synthetic;
no physiological pressure calibration or mesh convergence is claimed.

Run `cargo run --release -p physics --example urogenital_contact -- /tmp/voxy-urogenital-reference-pressure-8 8 --surface --reference-pressure=20`.

### Signed follower-pressure loading

`Body::add_gauge_pressure_cavity` and `set_gauge_pressure` prescribe a finite
signed inside-minus-outside pressure difference on a closed outward-oriented
boundary. Energy remains -p times the current enclosed volume, so the force
tracks current surface direction and area. Negative pressure differences give
inward loading. Existing `add_cavity`/`set_pressure` continue to reject negative
values; invalid topology and nonfinite gauge pressures are rejected atomically.
The external-pressure regression passed: analytic pressure work, numerical energy
gradient, inward equilibrium deformation and recovery on unloading.

The contact example accepts `--follower-pressure=20`, mutually exclusive with
reference-area loading. It constructs an outer lateral boundary and idealized
triangulated end caps, separately from the lumen cavity. Pressure is reduced
through the same four unloading steps. This models a prescribed pressure difference
on a mathematical closed outer hull; it is not pelvic-tissue confinement or a
fluid-flow model. A fresh 8-sector full cycle has been launched for verification.

The follower-pressure 8-sector cycle completed all eight states with residuals
below 1e-7 N and positive J. Protected loaded signed lumen changes were -3.9016
percent (urethral wall) and -6.1369 percent (vaginal wall). Both recovered on
unloading. All six rest/protected-load/release meshes passed the independent static
nonadjacent-face audit with no finding limit reached. Measurements and source
hashes are in `urogenital-follower-pressure-8.csv` and
`urogenital-follower-pressure-8-intersections.json`; actual meshes are rendered in
`urogenital-follower-pressure-8-render.png`. The gauge-pressure regression and all
nine urogenital integration tests passed. These remain synthetic numerical specimens
with idealized caps and supports, not anatomically calibrated pelvic mechanics.

### Angular refinement under 20 Pa follower pressure

Complete rest/compressed/protected/released cycles ran at 8, 16 and 32 sectors,
with the same three axial segments, radii, synthetic materials, pressure and
contact controls. Independent audits found no nonadjacent intersection candidates
in all eighteen rest/protected-load/release surfaces, without reaching finding
limits. The combined exact CSV and source hashes are in
`urogenital-follower-refinement.csv` and
`urogenital-follower-refinement-intersections.json`. The computed loaded meshes,
fixed scale and actual distal rings are rendered in
`urogenital-follower-refinement-render.png`.

Protected signed lumen changes at 8/16/32 sectors were -3.9016/-4.5461/-4.7251
percent for the urethral wall and -6.1369/-7.5183/-8.3689 percent for the vaginal
wall. Residual convergence and absence of detected crossings do not establish
mesh-independent mechanics: the vaginal 16-to-32 change remains 0.8506 percentage
points. Radial/axial refinement, contact-discretization sensitivity and physiological
calibration remain open. These distributed-pressure cycles are different loading
cases from the two still-running older point-compression 32-sector processes.

Both gauge-pressure tests passed, including preservation of signed cavity pressure,
energy and gradient during tissue assembly, and vanishing pressure resultant/torque
on a deformed closed surface.

### Axial refinement and surface-only contact controls

The contact driver now accepts `--segments=N` (3..96, divisible by three).
The middle-third outer-layer material zone retains the same physical extent at
all supported resolutions; its passive fibers are not moved into a shorter zone
when segment count increases. Geometry, outer pressure end caps, node selections
and unload loads use the chosen count. CSV exports include `segments` and
`contact_mode`. `--surface-only` implies full nonadjacent-face contact and omits
selected-pair barriers, while selected distances remain diagnostic outputs.
This distinguishes FEM/pressure response from artificial chosen-pair penalties.
The renderer accepts `--segments` and checks the actual expected vertex count.
Fresh 16-sector, 20 Pa surface-only cycles at 3 and 6 axial segments were launched.

Both new surface-only cycles completed all eight states each with residuals below
1e-7 N and positive J. Independent audits found no nonadjacent intersection
candidates in all twelve rest/protected-load/release surfaces, without reaching
finding limits. At 3 versus 6 axial segments, protected signed lumen changes were
-4.6708 versus -5.8271 percent for the urethral wall and -10.8539 versus -18.3974
percent for the vaginal wall. This substantial axial sensitivity contradicts
mesh-independent mechanics on the previous three-segment template.
Measurements and hashed surface findings are saved in
`urogenital-axial-pressure-comparison.csv` and
`urogenital-axial-pressure-intersections.json`. The actual six-segment cycle is
rendered in `urogenital-surface-only-16-6-render.png`. These comparisons hold the
16 angular sectors, layer radii, middle-third material extent, pressure, supports
and surface contact settings fixed; selected-pair penalties are absent.

The renderer also accepts `--axial-refinement` for directly comparing loaded
3/6/12-segment meshes at fixed angular resolution. It selects measurements and
audit findings by axial count, validates each mesh's actual vertex count and keeps
the within-specimen scale fixed. The 12-segment 20 Pa surface-only cycle is running.

The 12-segment cycle completed all eight states. The combined 3/6/12-segment
series at 16 angular sectors and 20 Pa surface-only pressure has eighteen audited
rest/protected-load/release surfaces, with no nonadjacent intersection candidates
or reached finding limits. Protected signed lumen changes were
-4.6708/-5.8271/-6.3274 percent for the urethral wall and
-10.8539/-18.3974/-22.2072 percent for the vaginal wall. The successive vaginal
change decreases from 7.5435 to 3.8098 percentage points, but remains too large to
claim mesh-independent mechanics. All comparisons retain the same
physical material-zone extents and supports; no selected-pair barriers are present.
Exact measurements, audit source hashes and actual loaded mesh comparison are
saved in `urogenital-axial-pressure-refinement.csv`,
`urogenital-axial-pressure-refinement-intersections.json`, and
`urogenital-axial-pressure-refinement-render.png`. This confirms only convergence
of each discrete equilibrium and the scoped static geometry audit, not anatomical
accuracy, radial/angular convergence or completion of the full-body objective.

A 24-axial-segment cycle at the same 16 angular sectors, 20 Pa follower pressure
and surface-only contact has been launched. The renderer's
`--axial-resolutions 6 12 24` selects the comparison columns without substituting
or rescaling the computed meshes. Measurement and finding selection still use
actual axial metadata, and source vertex counts are checked.

The 24-segment cycle completed with positive J and residuals below 1e-7 N.
All twenty-four rest/protected-load/release surfaces in the 3/6/12/24 series passed
the scoped static audit with no finding limit reached. The latest protected
signed lumen changes were -6.5208 percent for the urethral wall and -23.3692
percent for the vaginal wall. The 12-to-24 differences are 0.1934 and 1.1620
percentage points respectively. The decreasing volume differences do not prove
full convergence; minimum J also changes substantially with refinement.
The latest exact CSV, hashed findings and computed-mesh comparison are in
`urogenital-axial-pressure-refinement-24.csv`,
`urogenital-axial-pressure-refinement-24-intersections.json`, and
`urogenital-axial-pressure-refinement-24-render.png`. Radial/angular convergence,
material calibration, registered anatomy and coupled full-body dynamics remain open.

### Radial subdivision preserving physical layer identities

`UrogenitalWallGeometry::axial_wall_refined` subdivides each of the four physical
layers into one, two or three radial elements. It interpolates inside each layer
without moving its boundaries, clones the specified layer materials, and remaps
all refined sublayers back to the original four-per-segment region IDs. This
preserves activation selection. The driver accepts `--radial=N`, records it in
CSV and derives all ring indices/outer pressure cap topology from that value.
The renderer accepts the same control and projects the actual five physical-layer
boundary rings while rendering the full subdivided boundary mesh.
The new refinement test passed for all three resolutions: exact physical boundary
coordinates, original region/material assignments, element/node counts and invalid
control rejection. A 16-angular, 24-axial, two-elements-per-layer 20 Pa cycle is running.

The old optimized point-compression 32-sector process ended with exit 1 and
`oval specimen did not converge`; no protected vaginal equilibrium was exported.
This is a terminal convergence failure, not an observation timeout. Its old loaded
binary did not report residual/iteration details. Future driver failures now include
specimen, stage, accepted iteration count, residual and minimum J. The separate
original unoptimized process remains live; neither failure nor elapsed time proves
its outcome. This older loading case remains unresolved.

The two-radial-elements-per-layer cycle completed all eight states at 16 angular
and 24 axial subdivisions. All six rest/protected-load/release surfaces passed
the scoped static nonadjacent-face audit. Protected signed lumen changes were
-6.8939 percent (urethral wall) and -23.0931 percent (vaginal wall), versus
-6.5208 and -23.3692 percent at one radial element per layer. The small vaginal
volume difference does not prove local convergence: loaded minimum J changed
from 0.8578 to 0.7773. Exact measurements, mesh hashes and actual boundary-mesh
rendering are retained in `urogenital-radial-2.csv`,
`urogenital-radial-2-intersections.json` and `urogenital-radial-2-render.png`.
A three-radial-elements-per-layer cycle is running separately; its result is not
verified yet. Material calibration and full anatomical registration remain open.

The renderer's `--radial-refinement` compares one/two/three elements per physical
layer. It selects measurements and audit findings by radial metadata and checks
actual full-mesh vertex counts. The section overlays retain the five physical-layer
boundaries, rather than treating numerical sublayers as different tissues.

### Stress diagnostics from saved wall states

The contact driver accepts `--stress-from=DIR` to evaluate saved rest/barrier/
released OBJ node positions against the same reconstructed reference geometry and
materials, without rerunning equilibrium. It verifies boundary face indexing,
reference rest positions and valid stress-evaluation node count. Each export has
one row per tetrahedron: element, physical region, reference volume, J, compressive
pressure, von Mises stress and maximum shear stress. These are continuum material
stress diagnostics; boundary pressure/contact forces are not added as fictitious
cell stresses. The physical loads affect these diagnostics through deformation.

### Completed radial series and continuum stress comparison

All three one/two/three-elements-per-layer cycles completed at 16 angular and
24 axial subdivisions under 20 Pa surface-only follower pressure. All eighteen
rest/protected-load/release surfaces passed the scoped static audit, without
reaching finding limits. Protected signed lumen changes were
-6.5208/-6.8939/-6.9664 percent for the urethral wall and
-23.3692/-23.0931/-23.0197 percent for the vaginal wall. Minimum loaded J in the
vaginal wall was 0.8578/0.7773/0.7346: local deformation remains mesh dependent.
Combined exact measurements, source hashes and actual loaded mesh comparison are
in `urogenital-radial-refinement.csv`,
`urogenital-radial-refinement-intersections.json` and
`urogenital-radial-refinement-render.png`.

Stress diagnostics from the actual saved loaded node coordinates were evaluated
at all three resolutions. Reference-volume-weighted mean von Mises stresses were
21.3531/22.2422/22.4470 Pa for the urethral wall and
53.5184/54.0164/54.1127 Pa for the vaginal wall. Maxima were
238.2507/387.5605/477.5728 Pa and 375.8743/606.5353/819.6522 Pa respectively.
All evaluated fields were finite. Thus stable volume and average stress do not
establish stable peaks. Summary values and raw-stress CSV hashes are retained in
`urogenital-radial-stress-summary.json`; full per-element CSV exports are under
`/tmp/voxy-pressure-stress-radial-{1,2,3}`. These are synthetic constitutive
stress diagnostics, not fitted human physiology or clinical thresholds.

Reproduce stress evaluation for radial 3 with `cargo run --release -p physics --example urogenital_contact -- /tmp/voxy-pressure-stress-radial-3 16 --segments=24 --radial=3 --stress-from=/tmp/voxy-pressure-radial-3`.

### Located loaded stress maxima

Stress CSV exports now include actual deformed tetrahedron centroid coordinates,
reference centroid Z and the number of nodes on the anchored reference base.
For radial 3, both maximum-von-Mises elements were index 27646 and had zero base
nodes. Reference centroid Z was 19.7917 mm in the 20 mm urethral wall and
29.6875 mm in the 30 mm vaginal wall. The peaks are therefore near the distal
end, not located in base-attached elements. This establishes their location;
it does not establish the cause. Artificial pressure end-cap triangulation needs
an independent sensitivity check before interpreting peaks as material behavior.
Source CSV hashes and measured locations are in
`urogenital-stress-location-proof.json`; actual deformed cell centroids colored
by stress, with projected maxima outlined, are rendered in
`urogenital-stress-location-render.png`.

### Pressure cap fan-anchor sensitivity control

The driver accepts `--cap-anchor=K` with K below angular sector count. It changes
only the fan triangulation of the idealized outer pressure caps; wall mesh,
materials, pressure and supports remain unchanged. The control is included in
CSV metadata. A regression using a planar oval cap at anchors 0 and 8 passed:
reference pressure energy (therefore enclosed reference volume) agrees, while
nodal pressure gradients differ substantially. This demonstrates that virtual
cap load distribution depends on the arbitrary fan choice even before solving.
It is not evidence that real anatomy has such a dependence.
The 16-angular/24-axial/radial-3, 20 Pa anchor-8 cycle is running; urethral states
have completed while vaginal protected equilibrium/unloading remains pending.
Both cap-gradient and reference-area subdivision example tests passed.

The anchor-8 protected vaginal equilibrium converged below 1e-7 N and passed the
independent static nonadjacent-face audit. Actual continuum stress maxima were
819.65216 Pa at deformed centroid x=+11.65687 mm (element 27646) for anchor 0,
and 819.65364 Pa at x=-11.65739 mm (element 27598) for anchor 8. Both reference
centroid Z values are 29.6875 mm. Moving only the cap fan anchor moves the
hotspot to the opposite side at nearly unchanged magnitude. This confirms that
the arbitrary virtual cap load distribution controls this location; it does not
validate either peak as physical tissue behavior. Loaded stress CSV hashes and
coordinates are in `urogenital-cap-anchor-peak-proof.json`; actual centroid/stress
projections are in `urogenital-cap-anchor-peak-render.png`.
The new case's unloading remains a separate pending check at this observation.

`--stress-stages=barrier` supports analysis of a completed loaded state while later
states are running. The source rest mesh is always read and checked against the
reconstructed reference before processing the selected states, even when no rest
stress CSV is requested. Invalid stage selections are rejected.

The anchor-8 process subsequently completed unloading and exited successfully.
All six rest/protected-load/release surface states passed the scoped static audit.
Full measurements are in `urogenital-cap-anchor-8.csv`; source hashes and findings
are in `urogenital-cap-anchor-8-{urethra,vagina}-intersections.json`.
The earlier pending-unload note records the prior observation, not current status.

### Fan-independent averaged virtual pressure caps

`--balanced-caps` installs all cyclic fan triangulations of each virtual outer
pressure cap and assigns each closed hull pressure difference -p/sectors. The
potential is -p times the average enclosed volume. Repeated lateral faces carry
only their fraction of the pressure, so the total prescribed load is not multiplied.
The pressure remains a follower load with consistent energy gradient. Reordering
fan anchors does not select a preferred vertex. The CSV records `balanced_caps`.

Three example tests passed. The new averaged-cap test checks analytic reference
pressure work, energy/gradient invariance under anchor reordering on a deformed
nonplanar ring and finite-difference energy derivatives. Existing tests retain the
single-fan sensitivity control and reference-traction area subdivision invariant.
A 16-angular/24-axial/radial-3, 20 Pa surface-only cycle with averaged caps is
running. This removes the arbitrary fan vertex as a modeling choice; it does not
make the virtual caps anatomical tissue or prove physiological boundary conditions.

### Averaged caps: completed cycle

The 16-sector, 24-segment, radial-3, 20 Pa surface-only cycle completed for both specimens. Independent static audits found no intersections at rest, loaded and released states (shared-vertex pairs excluded). Vaginal signed lumen change was -23.0544%; min J was 0.926842; recovery error was -0.0004%. Actual-mesh render: `urogenital-balanced-caps-render.png`. Stress summary: `urogenital-balanced-caps-stress-summary.json`. These are synthetic wall specimens with idealized virtual caps, not anatomically calibrated full-body results.

Averaged-cap radial convergence runs started at radial subdivisions 1 and 2, with all other settings fixed. Radial-1 full cycle completed; radial-2 remains running. Loaded stress peaks at radial 1 versus 3: urethra 75.13 versus 101.56 Pa; vaginal wall 199.60 versus 274.71 Pa (see actual CSVs for exact values). Peak comparison render: `urogenital-balanced-caps-peak-render.png`. Convergence is not yet established.

### Averaged-cap radial refinement: three completed cycles

All three resolutions completed loading and unloading for both specimens. Independent static audits of all 18 rest/loaded/released meshes found no intersections among eligible nonadjacent faces; these do not prove continuous trajectories or anatomical calibration.

| Specimen | Radial subdivisions | Mean VM (Pa) | Peak VM (Pa) | Signed lumen change (%) | Min J |
|---|---:|---:|---:|---:|---:|
| urethra | 1 | 20.27573 | 75.12581 | -6.50169 | 0.977053 |
| vagina | 1 | 50.42482 | 199.59624 | -23.37356 | 0.930523 |
| urethra | 2 | 21.08557 | 88.24930 | -6.87490 | 0.974039 |
| vagina | 2 | 50.71797 | 253.28065 | -23.11560 | 0.927787 |
| urethra | 3 | 21.26725 | 101.56184 | -6.94918 | 0.972972 |
| vagina | 3 | 50.74822 | 274.70579 | -23.05436 | 0.926842 |

Vaginal mean stress changes by 0.0596% from radial 2 to 3 while peak stress changes by 8.4615%. This does not establish local stress convergence. A separate 48-axial-segment radial-3 run was started with the same physical middle-third material region and pressure, with accepted-iteration tracing enabled. Evidence: `urogenital-balanced-caps-refinement-proof.json`, CSV and actual mesh render.

### Stress distribution diagnostics

`tools/summarize_wall_stress.py` now exports finite/positive-volume checked, reference-volume-weighted means and quantiles plus actual peak region/location and source SHA256. Applied to all three completed vaginal loaded states: p95 = 109.2473, 109.1359, 110.4839 Pa; p99 = 179.1110, 192.4122, 185.2877 Pa; p99.9 = 197.5980, 241.4887, 254.6382 Pa. Thus tail convergence remains unproven despite stable mean stress. Peak region IDs 84/80/80 map to inner radial layer in axial segments 21/20/20, with no base nodes. No cause of this tail dependence has yet been established. The 48-segment run remains live with accepted-iteration tracing; no restart was performed.

### Axial-48 preliminary loaded-state diagnostics

Stress postprocessing now accepts the explicitly selected `compressed` stage, retaining the same topology/reference checks. The 48-segment compressed state converged for both specimens and independent rest/compressed static audits had no intersection findings. Comparison against radial-3 axial-24 protected states is preliminary because the axial-48 protected solve remains live: mean VM urethra 21.2672 -> 21.8463 Pa, peak 101.5618 -> 150.4236 Pa; vaginal mean 50.7482 -> 51.3308 Pa, peak 274.7058 -> 321.9610 Pa. Axial-48 peaks are in outer layer at distal segment 47, without pinned base nodes. These measurements suggest an end-boundary sensitivity but do not establish its cause. Full tail statistics and hashes: `urogenital-balanced-caps-axial-stress-preliminary.json`. Actual compressed mesh render and independent audits accompany it.

### Matched protected axial stress comparison

Axial-48 protected loaded equilibria have converged for both specimens. Their stress distributions are identical to their unprotected equilibria in this low-pressure case (no active barrier energy); this is not validation of strong wall coaptation. Independent static protected audits found no intersections. At fixed 16 sectors/radial 3/averaged caps, 24 -> 48 segments changes urethral mean VM by 2.7226% and peak by 48.1104%; vaginal mean by 1.1479% and peak by 17.2021%. Local peak convergence remains unproven. `urogenital-balanced-caps-axial-stress-protected.json` preserves statistics and hashes; `urogenital-balanced-caps-axial-peak-render.png` shows actual deformed element centroids. Vaginal axial-48 unloading remains live. A controlled end-load comparison is needed before attributing the effect to virtual caps.

### Axial-48 cycle completion and boundary sensitivity controls

The same live axial-48 process completed successfully. Both specimens completed unloading; all six rest/protected/released static meshes passed independent intersection audits. Actual cycle render and hashes/recovery metrics are in `urogenital-balanced-caps-axial-48-cycle-render.png`, CSV and cycle-proof JSON. Reference-area lateral-only dead-traction controls were started at 24 and 48 segments, radial 3, 16 sectors, 20 Pa, surface-only contact. These change both cap inclusion and pressure law relative to closed-hull follower runs; they assess boundary/loading sensitivity but cannot alone isolate the cause of peak growth. No anatomical validity is implied.

### Lateral reference traction: preliminary urethral boundary control

Added validated `--specimen=urethra|vagina` selection to the example, allowing completed specimen stress postprocessing without reading unfinished other specimens. Both original lateral-control cycles remain live. Their converged unprotected urethral states were postprocessed with exact reference/topology checks and independently audited at rest/compressed: no eligible-face intersections found. For 24 -> 48 segments, reference-volume mean VM is 235.3340 -> 232.0678 Pa; peak VM 919.3071 -> 904.6427 Pa. Peak region 7 corresponds to outer layer of axial segment 1, reference z 1.25 -> 0.625 mm, near the base rather than the free end. This differs sharply from closed-hull follower loading, but simultaneously changes load law and cap inclusion; neither anatomical validity nor isolated causation is established. Stress hashes/statistics: `urogenital-reference-lateral-urethra-preliminary-stress.json`. Actual mesh render: `urogenital-reference-lateral-48-urethra-preliminary-render.png`.

### Lateral reference traction: matched protected urethral states

Both protected urethral states converged and static rest/barrier audits found no intersections. At 24/48 axial segments, mean VM is 235.3241/232.0894 Pa and peak VM 919.2773/904.7383 Pa. These differ only slightly from unprotected states; a missing contact barrier does not explain the much larger stress relative to the closed-hull follower cases. The comparison still changes both pressure law and end loading, so isolated causation and anatomical validity remain unproven. Actual deformed stress centroids are visualized in `urogenital-reference-lateral-urethra-peak-render.png`; statistics/hashes are in `urogenital-reference-lateral-urethra-protected-stress.json`. Both full control cycles remain live.

### Isolating cap inclusion under fixed reference traction

Added `--reference-caps` (requires `--reference-pressure`), which averages the same closed outer-hull fan triangulations but freezes their reference-area tractions. Relative to lateral-only reference traction this changes cap inclusion while retaining the load law, geometry, material and supports. A new test verifies the frozen closed load equals negative follower-energy gradient minus baseline at reference at every node and has zero net force. All four example tests passed. A 24-segment radial-3, 16-sector, 20 Pa surface-only closed reference-traction cycle was started (`/tmp/voxy-reference-closed-24`); no equilibrium result is claimed yet. Original lateral controls remain live. This control is for numerical boundary sensitivity, not physiological calibration.

### Isolated loading-law control: protected urethral equilibrium

The closed-reference-traction protected urethral state converged and its independent rest/barrier audit found no intersections. Matched 24 axial segments/radial 3/16 sectors/20 Pa/material/support cases: lateral dead traction mean/peak VM 235.3241/919.2773 Pa; averaged closed dead traction 242.0127/920.8323 Pa; averaged closed follower pressure 21.2672/101.5618 Pa. Thus cap inclusion at fixed dead law changes this peak by only about 0.17%, while updating the load with the deformed surface at fixed closed geometry has a much larger effect on the attained equilibrium. No uniqueness/global-minimum proof or physiological calibration is claimed. Evidence: `urogenital-pressure-boundary-control-urethra.json`, peak-proof JSON and actual deformed-centroid render. Full control cycles remain live.

### Lateral controls: vaginal geometry is not covered by urethral convergence

The 24-segment full lateral-reference-traction cycle completed successfully for both specimens. Independent static audits of all six rest/protected/released states found no intersections. Actual vaginal cycle render: `urogenital-reference-lateral-24-vagina-cycle-render.png`; CSV archived. Protected vaginal loaded means VM at 24/48 segments are 289.2951/308.3807 Pa; peaks 893.6601/1109.1125 Pa, both with reference centroid z 1.5625 mm. Unlike the urethral case, this peak does not stabilize under the tested axial refinement. Source hashes/statistics: `urogenital-reference-lateral-vagina-protected-stress.json`. The 48-segment full lateral cycle and 24-segment closed-reference control remain live; the latter has not exported a loaded vaginal state yet. No conclusion from urethral cap controls is generalized to vaginal geometry.

### Isolated loading-law control: vaginal protected equilibrium

The 24-segment closed-reference vaginal protected equilibrium converged (residual 9.8047e-8 N, min J 0.769066, barrier energy 4.7394e-10 J), and independent rest/barrier static audits found no eligible-face intersections. At fixed 24 axial segments/radial 3/16 sectors/20 Pa/material/supports, lateral dead traction mean/peak VM is 289.2951/893.6601 Pa; averaged closed dead traction 304.9272/874.6434 Pa; averaged closed follower pressure 50.7482/274.7058 Pa. The closed dead control activates surface contact whereas the other two have zero barrier energy; contact law remains fixed. Updating pressure direction/area has a larger effect on attained equilibrium than adding caps at fixed dead law in this tested setup. No unique/global equilibrium or anatomical calibration is proven. Actual stress centroids and source hashes are preserved in `urogenital-pressure-boundary-control-vagina-render.png`, peak-proof JSON and summary JSON. Both pending full cycles remain live.

### Explicit loading metadata

Stage CSVs now append `load_mode`, `load_pressure_pa`, `reference_caps` to distinguish frozen lateral/closed reference loads from follower loads without interpreting folder names. A zero-pressure closed-reference urethral full-cycle smoke run completed with four correctly labeled states. Existing live binaries retain their old schema; historical CSVs were not silently rewritten. The example segment range now matches the builder limit while preserving thirds: 3..63 divisible by three. Pending physical controls are still running; no unloading convergence is inferred from elapsed time.

### Closed reference-traction control: completed cycles

The original closed-reference process completed successfully for both specimens. All six rest/protected/released meshes passed independent static intersection audits. Vaginal loaded signed lumen change was -41.7271%; post-unloading recovery error was approximately -0.0001%. Exact metrics, load mode, pressure, cap mode and CSV SHA256 are preserved in `urogenital-reference-closed-24-cycle-proof.json`; original CSV archived unchanged. Actual computed cycle render: `urogenital-reference-closed-24-cycle-render.png`. This completes the 24-segment numerical load-law/cap-inclusion control; it does not establish anatomical boundary conditions or peak mesh convergence. The original lateral-48 process remains live.

### Surrounding-tissue support foundation inspection

Current `tissue_bonds.rs` provides objective rest-length node-to-node springs and `assemble_tissues`, retaining world positions, supports, loads, element laws, cavities and material history. Existing tissue-assembly-history tests (4) and tissues tests (11) passed in release. The history suite checks preservation of deformation/distinct viscous histories, matching separate relaxation, bonded failure rollback and cell-pore inventories. These checks support reuse of the existing mechanism but do not validate pelvic attachments, surface-density scaling, inter-tissue contact or a coupled surrounding-wall simulation. No surrounding-tissue result is claimed yet. Lateral-48 unloading remains live.

### First deformable surrounding-wall assembly

`cargo run --release -p physics --example supported_wall` now constructs a synthetic 16-sector/6-segment four-layer wall and separate two-layer surrounding tube, both base-anchored, assembled in world coordinates. Objective rest-length radial bonds connect matching outer-wall/inner-support nodes; stiffness is 1e7 N/m^3 times lumped reference surface area, rather than fixed stiffness per node. Wall-only lateral reference traction is 20 Pa. Both loaded/unloaded equilibria converged: maximum wall/support displacement 0.180210/0.178045 mm under load, recovery displacement 6.049/5.934 nm; min loaded J 0.977420. Actual rest/loaded/released OBJ and CSV were produced and rendered. This is a new simplified material setup, not directly comparable with previous anisotropic wall controls. Inter-tissue collision/contact, calibrated support parameters, angular/axial convergence and anatomical registration remain unimplemented/unverified in this example. No intersection-free claim is made.

### Coupled support: combined surface contact

The supported-wall example now installs a single combined surface group including wall and support faces, so cross-tissue pairs participate alongside self-contact. Clearance 1 um, activation gap 50 um, discrete stiffness 1 N/m. Loaded/released solves completed with identical low-load deformations to the prior unprotected case; minimum eligible surface distances 121.824/149.732 um remain above activation, so this is not an active-contact-force validation. Independent static audits of actual rest/loaded/released combined meshes found no intersections (shared-vertex pairs excluded). Actual mesh render and CSV are archived as `supported-wall-contact-*`; strong closing loads, friction, mesh convergence and anatomical calibration remain open.

### Coupled support closing-load experiment started

Supported-wall now accepts finite nonnegative `--support-pressure` as reference-area inward traction on outer support faces, while wall traction remains 20 Pa. A 200 Pa support-pressure full loading/unloading cycle is live (`/tmp/voxy-supported-wall-closing-200`); no converged closing state is exported yet. Diagnostic output now includes total surface-barrier energy and cross-tissue contact energy, obtained by subtracting the same-state energy with separate wall/support self-contact groups from the combined group. This distinguishes contact across tissues from self-contact; material/bond/load state remains fixed for subtraction. A zero-support-pressure diagnostic cycle is separately running to check the inactive baseline. The active-contact claim remains unproven until output and independent geometry audit are available.

### Cross-tissue energy diagnostic regression

Added a regression in `tests/surface_contact.rs` using two distinct tissue triangles within the activation range. Separate contact groups match plain constitutive energy/gradient exactly; the combined group adds the analytically expected positive cross-barrier energy and nonzero forces. This validates the diagnostic partition on an active small fixture, not the pending supported-wall closing equilibrium. Supported-wall future runs now log every 100 accepted L-BFGS iterations via the observed API. The original 200 Pa run remains live with its original binary and was not restarted solely due to waiting.

### Active-barrier tests and ramped closing experiment

All five `surface_contact` tests passed, including analytic cross-tissue barrier partition, finite-difference gradient/objective balanced forces, invalid-state atomicity and continuous-path tunnelling rejection. The original instantaneous 200 Pa supported-wall run remains live and has not exported a converged loaded state. Added validated `--load-increments=1..32`: loads and unloading advance from the same accepted body state without rebasing geometry or resetting constitutive history. A separate four-increment 200 Pa loading/unloading experiment was started with accepted-iteration tracing (`/tmp/voxy-supported-wall-closing-200-ramp`). This changes the load path deliberately; it is not a restart of the original live run. Stage CSV writes are now flushed after export for future runs. Neither closing equilibrium nor active inter-tissue contact is claimed yet.

### Closing-load convergence snapshot

The four-step ramp remains at its first increment: the accepted-iteration trace snapshot through iteration 19100 has residual approximately 7.978e-4 N, well above 1e-7 N despite small energy changes. Trace JSON and log-residual plot are saved as `supported-wall-closing-convergence-*`. Both original instantaneous and ramped processes are still live; no restart or false equilibrium export was performed. These data indicate a solver convergence issue to investigate, not a verified contact configuration.

### Nonconverged geometry capture for solver investigation

Supported-wall accepts validated `--max-iterations=1..100000`. If the solve does not reach the unchanged 1e-7 N tolerance, it now exports only a distinctly named `{stage}-unconverged.obj` and text report with converged=false, iteration count, residual, min J and minimum surface distance, then returns an error without attempting unloading. No normal loaded/released equilibrium artifact is written for the failure. A deliberately short 200-iteration, four-step-ramp, 200 Pa diagnostic experiment was started to inspect the first accepted trial state; original full-budget instantaneous and ramp runs remain live. This diagnostic is not an equilibrium or a replacement for either run.

### First diagnostic active-surface state: not equilibrium

The 200-iteration first-increment diagnostic terminated with exit 1 as intended: residual 1.5252178e-3 N, min J 0.931452, minimum eligible surface distance 16.6880 um (clearance 1 um, activation gap 50 um). Actual accepted geometry is explicitly unconverged. Independent static audit found no eligible-face intersections. Distance is inside the barrier activation range, but does not identify cross-tissue versus self-contact; active inter-tissue contact remains unproven. Actual mesh render/report and hashed audit saved as `supported-wall-unconverged-*`. Both full-budget closing experiments remain separate/live.

### Active pair localization on unconverged supported tissues

Added nonmutating `Body::active_surface_pairs_at`, validating trial energy/geometry and reporting actual active group/node triples, distance and per-pair barrier energy using the same closest-feature law. `supported_wall --probe-from` validates the exported mesh topology against reconstructed reference before probing, without solving or committing the checkpoint. The 200-iteration unconverged state contains 572 active cross-tissue pairs, summed energy 2.97752945e-7 J, and zero active within-tissue pairs. Thus this snapshot does have active wall/support contact; it is still not equilibrium and does not establish why L-BFGS stagnates. Pair CSV and summary archived under `supported-wall-active-pairs*`. The existing active barrier regression now verifies diagnostic pair count/distance/energy, empty separate groups and rejection of wrong-sized coordinates. All five surface-contact tests passed.

### Actual active-contact gradient probe

The nonmutating supported-wall probe now ranks free-node gradient components and tests the six largest against central/one-sided finite differences at 10, 1 and 0.1 nm on the same reconstructed-load/reference mesh and saved unconverged state. At 1 nm maximum central-gradient error is 2.5133e-12 N; at 10 nm 1.9767e-10 N; at 0.1 nm 5.2349e-12 N. Left/right derivative separation shrinks proportionally to step (7.9463e-6 -> 7.9463e-7 -> 7.9469e-8 N). This finds no force/energy mismatch on these six components of the 200-iteration state; it does not prove smoothness or correct gradients at the later plateau iterate. Actual CSV and summary saved as `supported-wall-gradient-probe*`. Full instantaneous/ramp runs remain live; plateau cause remains unproven.

### Accepted-step L-BFGS instrumentation

Added `equilibrate_lbfgs_steps`, observing accepted step multiplier, number of rejected line-search trials and maximum accepted node displacement in addition to iteration/energy/residual. Existing observer and plain APIs delegate without changing search/acceptance/contact guards, preconditioning, history, tolerance or updates. Supported-wall future runs include those fields in trace lines. The observer regression compares positions, residual and per-iterate old fields bit-for-bit between plain, old observer and step observer; it also checks dyadic accepted multipliers and finite displacement. Original instantaneous and four-step closing binaries continue unchanged. Instrumentation alone does not establish the stagnation cause.

### Restarted optimizer from diagnostic geometry

Added atomic `restore_diagnostic_positions`: validates energy/geometry and unchanged pinned positions, preserves reference/material/load state, and does not certify the imported trajectory. Supported-wall `--resume-from` checks exact exported topology and uses this import before the existing ramp schedule. The imported configuration is labeled `imported.obj`, not rest. A separate 2000-iteration instrumented continuation from the 200-iteration checkpoint was started with identical first-step loads; L-BFGS history begins empty, so this is an explicit numerical history-reset experiment, not continuation of the live solver memory. Original full-budget runs remain live. Regression checks reference/energy preservation and invalid/pinned/nonfinite import atomicity.

### Plateau checkpoint: persistent one-sided derivative split

The explicit history-reset diagnostic continuation terminated unconverged after 2000 accepted iterations: residual 9.10295836e-4 N, min J 0.930495, minimum eligible gap 16.4958 um. Last accepted node displacement was 1.65557e-15 m despite step multiplier 0.5. Five L-BFGS/import regression tests passed. A same-state finite-difference probe at the six largest free components now shows maximum left/right derivative split 4.84327e-4, 4.78970e-4, 4.78429e-4 N at 10, 1, 0.1 nm; maximum analytic-versus-central discrepancy stays near 2.392e-4 N. Unlike the earlier 200-iteration smooth probe, this persistent split is evidence of local nonsmoothness at these coordinates, not proof of a globally incorrect gradient or a specific contact feature. Actual probe CSV/summary and unconverged report archived as `supported-wall-plateau-*`. Next investigation must isolate which energy terms/pairs produce the kink. Original full-budget processes remain independent.

### Energy-term isolation of the plateau kink

The same-state six-component finite-difference probe now evaluates both complete energy and a cloned body without surface contact. Without surface contact, maximum one-sided split shrinks 4.8677e-6 -> 4.8677e-7 -> 4.8704e-8 N as step shrinks 10 -> 1 -> 0.1 nm; central-gradient error is about 2.076e-12 N at 1 nm. The isolated surface-contact term instead retains split 4.7946e-4 -> 4.7848e-4 -> 4.7838e-4 N and central-gradient mismatch near 2.392e-4 N. This localizes the observed nonsmoothness to the surface-contact energy in the tested coordinates, not the tissue constitutive/bond/dead-load terms. It does not yet identify the individual feature pair or justify a particular replacement law. Raw and summary data archived as `supported-wall-gradient-terms*`.

### Per-pair plateau localization

Probe now reports left/right energy derivatives per active triangle pair, using the union of baseline/perturbed active keys and zero energy for inactive keys. The sum of individual derivative splits reconstructs total contact-term split with maximum discrepancy 8.0320e-12 N across the tested components/steps. Largest 0.1 nm split occurs at node 547, y: wall faces [467,548,547] and [467,547,466] each against support face [803,851,852], about 1.35978e-4 N per pair. Both wall faces share edge [467,547]; node 547 is on distal outer-wall ring (z index 6). Similar mirrored pairs occur around node 555. This identifies concrete neighboring-face pairs responsible for much of the kink but does not yet identify their closest-feature switches or validate a replacement potential. Full pair derivatives and partition proof saved under `supported-wall-pair-derivatives*`. Original full-budget processes remain live.

### Closest-feature switching probe started

Added validated nonmutating `surface_pair_closest_at`, returning barycentric weights of the two closest points and distance for an explicit node-triple pair. The supported-wall probe selects the largest 0.1 nm derivative split and records nearest points at -0.1 nm, zero, +0.1 nm; the run is pending, so no feature-switch result is claimed yet. Extended contact regression checks normalized weights, distance agreement and invalid index rejection; test execution is pending. Original ramp trace remains on load-1 around iteration 52700 at residual 7.108e-4 N.

Closest-feature probe completed: for wall face [467,548,547] versus support [803,851,852], node-547 y shift -0.1 nm selects edge/edge points (wall weights [0.932542,0,0.067458], support [0.957768,0,0.042232]); at zero/+0.1 nm it selects wall vertex 547 against support face interior (wall [0,0,1], support approximately [0.026784,0.932025,0.041191]). Distances remain near 17.2595 um. This verifies discontinuous closest-feature selection at nearly tied distances in the concrete problematic pair. Five contact tests passed; exact weights/distances archived in `supported-wall-closest-feature.csv`. A smooth/consistent potential replacement still requires design and verification.

### IPC parallel-edge mollifier foundation

Reviewed primary IPC Toolkit source (`https://github.com/ipc-sim/ipc-toolkit/blob/main/src/ipc/distance/edge_edge_mollifier.hpp`) and tutorial. Implemented a standalone `edge_contact_mollifier` building block with fixed-reference threshold 1e-3 times squared reference edge lengths, polynomial multiplier below threshold, and analytic spatial gradient including the multiplier derivative. Added finite-difference, resultant balance, translation, parallel/inactive/threshold and invalid-geometry tests. Test run is currently waiting for the shared Cargo build-directory lock; no passing claim yet. This helper is not integrated into the triangle-minimum barrier and does not fix the plateau by itself; full primitive contact aggregation, degeneracy handling, potential/gradient verification and CCD-preserving integration remain required. Original live computations are preserved.

### Parallel-edge smoothing is inactive for the observed kink

The initial two mollifier tests passed. Evaluating the concrete switched edge pair [467,547] / [803,852] at the plateau gives cross-squared/threshold ratio 472.8051 and squared sine angle 0.471351, hence multiplier exactly 1 and zero multiplier gradient. This is not a near-parallel-edge degeneracy. A fixture regression records these actual reference/current coordinates to prevent treating this helper as a fix for the observed triangle-minimum kink; its test run is pending. The necessary next change is primitive potential aggregation/feature handling, not simply multiplying the existing minimum by the parallel-edge mollifier. Exact measured case is in `supported-wall-mollifier-case.json`.

### Primitive barrier sum removes observed single-pair kink

Implemented diagnostic `surface_pair_primitive_barrier_at`: sum of six point-triangle and nine edge-edge barriers, with analytic gradients, retaining the old body contact law. On the previously localized nonparallel switched pair, left/right derivative split shrinks 1.49250e-7 -> 1.49249e-8 -> 1.49146e-9 N as step shrinks 10 -> 1 -> 0.1 nm (old pair split approximately 1.35978e-4 N persisted). At 1 nm analytic/central discrepancy is about 3.017e-13 N. This verifies removal of the observed kink for that pair under this diagnostic sum. All six surface-contact tests passed, including finite differences for all 18 pair coordinates and balanced force. Full mesh integration still requires deduplication, reference-based edge mollification/product derivatives, activation handling, mesh scaling and CCD-preserving verification; this is not full IPC or calibrated anatomical contact. Raw measurements archived as `supported-wall-primitive-probe.csv`.

### Unique primitive topology foundation

Added `surface_primitive_stencils_at`, which uses current group-local broad-phase face candidates and adjacency exclusions, then canonicalizes/deduplicates vertex-face and edge-edge identities across neighboring faces. Diagnostic topology only: the solver contact law is unchanged. A fixture with two adjacent faces versus one opposing face produces 10 unique vertex-face and 15 edge-edge stencils instead of 30 raw entries, invariant under face order/winding reversal; invalid coordinate counts are rejected. All seven surface-contact tests passed. A full-mesh plateau stencil probe is running; active feature classification, endpoint/edge degeneracy deduplication, mollified product gradients and energy/CCD integration remain open.

### Mesh-wide primitive diagnostic potential

Added `surface_primitive_energy_at`: uses canonical broad-phase stencils, point/triangle and edge/edge distances and analytic distance gradients. Edge terms include the fixed-reference parallel-edge multiplier and full product derivative (multiplier gradient times barrier). Still diagnostic-only: solver contact law unchanged, endpoint-feature deduplication and mesh scaling not established. All eight surface-contact tests passed, including numerical derivatives across all specimen coordinates and balanced resultant. A same-state finite-difference probe of the full plateau mesh primitive energy has been launched; no full-mesh smoothness or solver-convergence claim yet.

### Full-mesh primitive derivative probe completed

On the frozen load-1 plateau mesh, tested the same six free coordinate components that exposed the old contact kink. The mesh-wide diagnostic primitive potential gives maximum analytic/central error 7.0656e-11, 6.2930e-13 and 1.5558e-12 N for perturbations 10, 1 and 0.1 nm. Maximum left/right derivative split shrinks 4.13872e-7 -> 4.13871e-8 -> 4.13696e-9 N, whereas the old triangle-minimum full energy split stays approximately 4.78e-4 N. This supports removal of the observed local kink for these components; it does not establish global smoothness, mesh independence or equilibrium. Raw values, checksum/summary and measured comparison are `supported-wall-mesh-primitive-probe.csv`, `supported-wall-mesh-primitive-probe-summary.json` and `supported-wall-mesh-primitive-probe-render.png`.

Original instantaneous 200 Pa computation terminated unsuccessfully: residual 0.002609995359556973 N, compared with required 1e-7 N. No converged loaded state is claimed. The original four-step computation remains running (last inspected accepted iteration 61900, first-stage residual 7.108139758819e-4 N). Next integration must retain the geometric path guard, avoid recursion through diagnostic evaluation, and explicitly select the experimental law; endpoint-feature multiplicity and calibration remain unresolved.

### Experimental primitive law integrated into body evaluation

Added explicit `SurfaceContactLaw::{TriangleMinimum,ExperimentalPrimitiveSum}` with atomic selection, keeping the former default. Experimental evaluation replaces only surface energy/gradient; material, pore, bond, gap, cavity and external-force terms retain their existing implementation. Primitive assembly now has internal nonrecursive helpers. The exact triangle contact geometry validator and continuous straight-step path guard are retained independently of mollified energy. Tissue assembly preserves a common law and rejects mixed-law inputs rather than silently changing their physics. `supported_wall --contact-law=primitive` records the selected law in `contact-law.txt`; discrete coefficients have not been calibrated across the two laws.

All nine surface-contact integration tests passed, including analytic/finite-difference total energy gradients on a deformed specimen, equality of total energy/forces with plain-body plus primitive terms, and single-body assembly preservation. The continuous-tunnelling regression has additionally been extended to both modes and is pending rerun. A four-increment 200 Pa full load/release run was launched with unchanged 1e-7 N tolerance and a 10000 accepted-iteration budget per stage; no equilibrium result is yet available. Endpoint feature multiplicity, mesh scaling and anatomical/material calibration remain open.

### First active-contact load stages verified

Experimental primitive-law four-increment computation converged at actual support pressures 50, 100 and 150 Pa (wall pressures 5, 10 and 15 Pa). Residuals respectively 9.52841e-8, 9.86628e-8 and 9.62173e-8 N; minimum J 0.930279, 0.854646 and 0.779403; minimum eligible surface gap 6.34554, 2.40045 and 1.73552 micrometres. Cross-tissue contact energies are positive. Independent static geometry classification of reference and all three accepted load meshes found no nonadjacent intersections and did not reach the audit limit. Actual distal-ring coordinates rendered without displacement exaggeration in `supported-wall-primitive-load-stages-render.png`; archived stage CSV explicitly records applied pressure/factor, because the original binary CSV pressure field was requested maximum pressure, not stage pressure.

Full 200 Pa stage exhausted 10000 accepted iterations with residual 6.57905447245352e-5 N, above the unchanged 1e-7 N tolerance. Diagnostic state saved; it is not a converged loaded result and no release was run by that process. A separate explicit continuation from this terminal checkpoint at full 200 Pa, fresh optimizer history, same reference/configuration/forces and a 20000-iteration budget has been launched. This does not establish full-load equilibrium or recovery yet.

Nine contact tests, expanded tunnelling test for both laws, five L-BFGS regressions, four assembly/history regressions and example compilation check passed. The current example CSV includes load factor, actual applied support pressure and selected contact law to avoid confusing requested maximum with a ramp stage.

### Full-load derivative validation and accepted-coordinate snapshots

At the experimental 200 Pa stage's first terminal 10000-iteration checkpoint, finite differences of the six largest free gradient components show total analytic/central error decreasing 2.08259e-6 -> 2.08275e-8 -> 4.63318e-10 N for steps 10 -> 1 -> 0.1 nm. Left/right split decreases proportionally 2.20567e-4 -> 2.20476e-5 -> 2.20473e-6 N. This is compatible with high curvature rather than a persistent kink at these tested components. Primitive contact alone accounts for most curvature; this is a local diagnostic, not a proof of global smoothness or the cause of all remaining solver difficulty. Raw probe files and summaries are archived with prefix `supported-wall-primitive-200-`.

The live continuation later lowers energy substantially while force residual rises; no equilibrium or buckling mechanism is established without inspecting the accepted geometry. Added read-only `equilibrate_lbfgs_states` observer exposing initial/accepted coordinates while preserving existing observer wrappers and the minimization algorithm. Bit-exact observer regression passed: final coordinates, iteration count and residual equal the plain observer, every callback energy equals independent body evaluation, pins remain fixed, and final snapshot equals committed state. Example compilation passed. The example now writes explicitly uncertified intermediate OBJ/metadata every 1000 accepted iterations by default (`--snapshot-every=0` disables); file errors are reported. This will apply to future binaries, not retroactively to the already-running continuation. A one-iteration file-output smoke test at full load is pending; nonconvergence is expected and is not a physics result.

Snapshot smoke test completed with expected nonconvergence at the deliberately one-iteration budget. The saved iterate OBJ is byte-identical to the final unconverged OBJ; metadata records `equilibrium_certified=false`, full applied support pressure 200 Pa, selected law, iteration, energy and actual residual. Archived metadata/geometry `supported-wall-snapshot-smoke.txt` / `.obj` are output verification only, not a converged anatomical result.

### Full-load continuation terminal result: large bending, not equilibrium

The authoritative full-200-Pa continuation terminated unsuccessfully at its 20000 accepted-iteration budget. Residual 0.04430630511682473 N, minimum J 0.6296152562797, minimum eligible surface gap 1.060007540909 micrometres. Energy fell from -1.522619431173e-4 to -3.342290862866e-4 J. No converged loaded state or release result exists for this run. The exported diagnostic mesh shows substantial bending compared with the imported checkpoint; numerical minimization iterations are not physiological time, and this does not prove a specific instability mechanism.

Independent static audit of both unconverged full-load checkpoints found no nonadjacent face intersections and did not reach its limit (`supported-wall-primitive-200-unconverged-intersections.json`). Actual coordinate views share a physical scale per view in `supported-wall-primitive-200-unconverged-render.png`; neither panel is labelled an equilibrium. A full-body/contact derivative probe of the final bent geometry has been launched to test consistency after this deformation transition before selecting another solver intervention. No blind longer equilibrium rerun is active.

### Bent-state consistency and optional contact-curvature preconditioning

The authoritative bent-state derivative probe completed successfully. Total analytic/central gradient error decreases 2.44943e-4 -> 2.41975e-6 -> 2.42087e-8 N as perturbation shrinks 10 -> 1 -> 0.1 nm, with left/right split shrinking 5.15361e-3 -> 5.10168e-4 -> 5.10117e-5 N. Again this supports consistency/high local curvature at the six tested components, not a complete mesh smoothness proof. Measurements/checksums archived in `supported-wall-primitive-bent-gradient-summary.json` and its CSV files.

Added optional `equilibrate_lbfgs_preconditioned_states` using positive lumped normal-barrier curvature added to the existing material/bond/pore diagonal at each accepted iterate. Curvature is the analytic scalar second derivative of the barrier multiplied by the edge mollifier and squared closest-point node weights. Distance and mollifier Hessians are omitted; this is an approximate positive numerical preconditioner, not the true Hessian or a physical material fit. Actual energy/gradient, force tolerance, line search and continuous geometric guard are retained. Existing observer methods select the original diagonal; experimental primitive law is required for the new option. Nonfinite/nonpositive preconditioner values are rejected. Example flag `--contact-preconditioner` records the choice and probe exports per-node curvature. Compilation check passed; curvature finite-difference and active/inactive solver comparison tests are still pending in the shared Cargo build queue.

The original unpruned point-load calculation (authoritative session 22403) has now terminated with `oval specimen did not converge`; it is not still running and did not establish a protected full-resolution vaginal equilibrium. Original old-law supported-wall ramp remains live separately; it is preserved.

### Contact-curvature tests passed; matched full-load run launched

All ten surface-contact tests passed, including the optional active-contact preconditioned/plain small-specimen equilibrium comparison and energy-preserving curvature queries. Scalar barrier curvature passed finite differences at gaps 0.06, 0.3, 1, 10 and 40 micrometres. The inactive-contact preconditioner test passed with bit-identical final coordinates, residual and iteration count against the original solver. Example compilation passed and changed tracked files pass whitespace diff checks.

Launched a matched full-200-Pa comparison from the same `primitive-ramp/loaded-unconverged.obj` checkpoint as the previous failed 20000-iteration continuation, with the same model/loads/force tolerance and accepted-iteration budget. The only numerical change is the explicitly selected dynamic normal-contact curvature diagonal. It also saves uncertified snapshots every 1000 accepted iterations. Output root `/tmp/voxy-supported-wall-contact-curvature-200`; no convergence or performance benefit is claimed until observed. Existing triangle-law ramp remains preserved separately.

Matched dynamic-curvature full-load run is authoritatively live. At accepted iteration 1200, residual was 5.285567007702e-6 N (above 1e-7 N tolerance); it subsequently rose as energy decreased, so no equilibrium claim follows. The first 1000-iteration snapshot was saved with uncertified metadata, including preconditioner selection. The derivative comparison plot `supported-wall-primitive-bent-gradient-render.png` uses actual archived measurements before/after large bending, with no equilibrium/anatomical validation claim.

### Pressure direction law controls added for supported wall

Added explicit `--load-law=reference-lateral` (existing default), `closed-reference` and `closed-follower` to the two-tissue supported-wall example. Both closed modes use identical outer lateral faces and identical virtual caps averaged over every ring fan anchor; actual pressure is divided over these equivalent closed hulls. The follower uses conservative signed pressure-volume work on current coordinates. The closed-reference mode freezes the negative gradient of that same work at the original reference mesh, resets all hull gauge pressures to zero, and ramps these fixed forces. Thus comparing the two closed modes isolates updating the load with deformation instead of confounding it with cap inclusion. Both tissue pressure levels are ramped together, without constitutive rebasing; law selection is saved and included in stage/snapshot metadata. Caps remain a synthetic benchmark assumption, not verified pelvic boundary conditions.

Example compilation and two load-law tests passed: closed-reference and follower gradients agree at rest to numerical precision, then differ after deformation; averaged follower energy passes finite differences on a nonplanar end ring. This does not establish anatomical realism or full-load convergence. The new modes are alternatives for matched controls, not an assertion that the old reference load was numerically incorrect.

Independent static audit of dynamic-curvature accepted snapshots 1000,2000,4000,6000 found no nonadjacent intersections or audit-limit truncation. Outer wall ring centres give maximum lateral shifts 0.00975,4.47937,7.27985,11.95861 mm respectively; distal centre z changes 30.2124,30.0088,29.5030,27.5927 mm. Residuals remain above tolerance. Full metadata/checksums/coordinates archived as `supported-wall-curvature-snapshots-audit.json`. These are uncertified numerical iterates, not time-resolved physiological states.

Matched closed-follower and closed-reference control runs launched from the same undeformed reference, with identical geometry/material/bonds/contact law, the original (non-curvature) L-BFGS diagonal, four load/unload increments, 1e-7 N tolerance and a 10000 accepted-iteration budget per stage. Both are currently active. At 50 Pa support / 5 Pa wall they converged with residuals 9.58488e-8 / 9.52616e-8 N respectively. The follower also converged at 100 Pa support / 10 Pa wall, residual 9.47397e-8 N. No full-200-Pa or release claim yet. Rendered audited curvature snapshots are in `supported-wall-curvature-snapshots-render.png` and remain explicitly labelled uncertified iterates.

### Closed-follower 200 Pa active-contact load/release cycle verified

Authoritative closed-follower process exited successfully. All four loading and four unloading stages converged with the unchanged 1e-7 N force tolerance. Full applied support pressure 200 Pa / wall pressure 20 Pa: residual 9.559964189246e-8 N, minimum J 0.7628612760437, maximum wall displacement 1.540430584204 mm, support displacement 1.697504324634 mm, minimum eligible surface distance 1.382502326173 micrometres, positive total/cross-tissue contact energy 1.364683172469e-6 J. The cycle used the original material/pore L-BFGS diagonal, not contact-curvature preconditioning.

After zero-pressure release: residual 9.815683900838e-8 N, minimum J 0.9999994705649, maximum wall recovery error 26.52963114351 nm and support error 21.32975795619 nm, zero contact energy, minimum eligible distance 149.7283965935 micrometres. No constitutive reference rebasing or force-tolerance change was used. Independent static audit of reference plus all eight converged meshes found no nonadjacent intersections and no audit-limit truncation. CSV and coordinate rendering archived as `supported-wall-closed-follower-cycle.csv` and `supported-wall-closed-follower-cycle-render.png`; static audit is in `supported-wall-pressure-law-intersections.json`.

This establishes a full active-contact elastic load/recovery cycle for the explicitly synthetic homogeneous two-tissue benchmark with its virtual closed hulls. It does not establish anatomical registration, calibrated tissue parameters, dynamic/viscoelastic recovery, whole-organ physiology, mesh convergence or uniqueness/global stability. The matched frozen closed-hull control and the original dynamic-curvature/reference-direction control remain active separately; their final outcome is not yet established.

### Transactional L-BFGS physical steps and heterogeneous layer foundation

Added `relax_step_lbfgs_states`: uses L-BFGS for the same real-time Ogden-Maxwell increment as the original CG `relax_step`, freezes branch histories for the full nonlinear solve, advances them once only after convergence, and atomically leaves geometry/memory unchanged on failure. Both methods now share the commit implementation. Added atomic batch material assignment with one body clone, preserving unassigned laws/memories and rejecting duplicate/invalid indices. All ten viscoelastic tests passed, including successive CG/L-BFGS physical-step agreement and failed-step memory/geometry preservation. A further committed-force-residual assertion and example layer-profile tests are queued.

The supported-wall example has explicit `--viscoelastic`, `--dt`, `--hold-steps`, `--recovery-steps`. It assigns distinct synthetic layer moduli and branch times to four wall and two support layers, records every coefficient in `material-profile.csv`, marks calibration false, uses actual physical time in CSV, and inserts held-load and zero-load recovery stages. Fresh reference is required: geometry-only imports do not restore Maxwell memory, so viscoelastic resume/probe inputs are rejected. The time mode uses the standard L-BFGS diagonal. On a failed physical solve, the example restores the previous committed body including boundary loads and exports distinctly labelled last-committed geometry plus failure metadata; uncertified nonlinear trial snapshots cannot be used as history checkpoints. Example compilation passed before the latest boundary-load rollback addition; current example tests are pending.

Matched frozen closed-hull load control terminated unsuccessfully at 150 Pa support pressure after 10000 accepted iterations, residual 0.7134277859336 N, minimum J 0.6783771340999, minimum surface distance 1.004613454379 micrometres. No 200 Pa or release equilibrium exists for that run. The dynamic-curvature reference-lateral control also terminated unsuccessfully at its 20000-iteration budget; its terminal values require inspection before reporting. Successful closed-follower elastic cycle remains separate evidence for its specific boundary model, not a universal solver or anatomy validation.

Additional committed-state force validation passed: after every L-BFGS Maxwell memory advance, independently recomputed free-node residual satisfies the same 1e-8 N coupon tolerance. Three supported-wall example tests passed, including distinct layered shear response with unchanged reference coordinates. The latest example restores prior boundary loads as well as geometry/memory after a failed physical solve; release build is currently running to verify that final addition before the full contact/time experiment.

Terminal dynamic normal-curvature/reference-lateral continuation: residual 0.42888091675539075 N at 20000 accepted iterations; no loaded/released equilibrium result. This numerical preconditioner has not demonstrated a full-load benefit for that benchmark, so it remains optional and is not used for the verified elastic follower cycle or planned viscous physical-time run.

Final release build passed including boundary-load rollback. Launched full-resolution heterogeneous viscous supported-wall experiment: support maximum 200 Pa / wall maximum 20 Pa, four physical load/unload increments, dt=0.1 s, five full-load hold steps and ten zero-load recovery steps, primitive contact, closed-follower pressure, original L-BFGS diagonal, unchanged 1e-7 N force tolerance, at most 10000 accepted iterations per physical step. Original geometry/material reference is preserved; coefficients remain explicitly synthetic/unfitted. Authoritative session 49487 is live; committed load, hold, unload and initial recovery stages have been observed; the full schedule has not yet terminated. A separate deliberately one-iteration smoke run checks failed physical-time rollback/output semantics; it is not a convergence experiment or anatomical result.

### Heterogeneous viscous contact/time protocol completed

Authoritative session 49487 exited successfully. All 23 physical increments (4 loading, 5 full-load holds, 4 unloading, 10 zero-load recovery) converged over 2.3 s at dt=0.1 s with maximum reported residual 9.980316738881e-8 N. Minimum J across committed states 0.7233315220467; minimum eligible surface gap 1.358899277789 micrometres. Static audit of reference and all 23 committed OBJ states found no nonadjacent intersections and no truncation. Material coefficients, raw physical-time CSV, checksums and summary archived with prefix `supported-wall-viscous-`; actual measured time curve is `supported-wall-viscous-time-render.png`.

At t=0.4 s (full support pressure 200 Pa / wall 20 Pa), maximum wall displacement 1.845008814049 mm. During full-pressure hold to t=0.9 s, displacement increases to 2.013301732030 mm (+9.1215 percent), demonstrating creep for this explicit synthetic profile. At zero-load release t=1.3 s, wall displacement is still 0.316726674843 mm; after 1 s additional zero-load recovery, t=2.3 s, it falls to 0.121787454372 mm. Thus recovery is delayed and incomplete in this observation window; this is not a permanent-damage prediction, calibrated anatomical result or proof of temporal convergence. No reference geometry rebasing is used.

One-iteration failure smoke run exited with the expected physical-step nonconvergence. Failure metadata identifies no committed physical increment, time remaining 0 s, attempted t=0.1 s at support pressure 50 Pa. Last-committed exported OBJ is byte-identical to its rest OBJ, confirming geometry rollback; constitutive-memory and force-after-commit invariants are covered by the passing library tests. This smoke run is output/rollback validation only.

### Pressure-preserving physical timestep refinement

Added `--time-substeps=N` (viscous mode only, 1..16). Every original pressure interval is divided into N increments with identical load factor and duration dt/N; the coarse endpoint keeps its original stage name. It does not change ramp/hold/unload/recovery durations or pressure history. Metadata records nominal/actual dt and subdivision count. Four example tests passed, including exact pressure-factor/end-label/duration preservation; release build passed.

Added `tools/compare_supported_wall_time.py`: compares complete committed schedules at common physical times without interpolation or geometry welding, verifies equal reference topology/coordinates and identical material-profile hashes, checks both endpoint and every intermediate pressure/load-law interval, and reports per-node maximum/RMS differences with input hashes. It rejects missing times, mismatched durations, nonfinite/inverted/uncertified rows and geometry differences. Self-comparison of the 23-step baseline yields exactly zero difference at all 23 common times; this validates the oracle plumbing, not temporal convergence.

Launched unchanged 200-Pa physical protocol with actual dt=0.05 s (`--dt=0.1 --time-substeps=2`), 46 committed increments expected over the same 2.3 s. Geometry/materials/contact law, pressure intervals and 1e-7 N residual tolerance are unchanged. Output root `/tmp/voxy-supported-wall-viscous-200-half-dt`; no refinement result is yet available.

Also launched actual dt=0.025 s (four subdivisions, 92 physical increments) under the same pressure intervals and 2.3 s observation window. A third timestep level is required to check whether differences decrease, rather than interpreting one coarse/fine difference as convergence proof. Both refinements are independent fresh-reference computations; no completed process was restarted solely because observation timed out. No new anatomic material defaults were inferred from local atlas geometry: inspected HRA tetrahedral provenance confirms reference geometry exists, while measured tissue properties/attachments and registration remain absent.

Comparator negative-control fixture rejected a changed intermediate pressure while all coarse endpoint pressures were kept equal. The fixture is software validation data only; it is not a simulation or plotted measurement. Actual dt=0.05 run remains live and has reached zero-load recovery; dt=0.025 run remains live separately. The original triangle-minimum/reference-lateral four-step ramp is now authoritatively terminal: exhausted 100000 accepted iterations on first stage, residual 7.108139758818712e-4 N. It is no longer described as running and yielded no converged loaded/released state.

### Half timestep protocol completed and full-field difference measured

Authoritative dt=0.05 s process exited successfully after all 46 physical increments over exactly the same 2.3 s protocol. Comparator verified identical reference points/topology/material profile hashes, pressure on every subinterval and load/contact law, with all committed residuals <=1e-7 N. At the 23 shared coarse endpoint times, maximum Euclidean nodal difference is 22.253143128068 micrometres at t=1.1 s (unload-2, support pressure 100 Pa), RMS difference there 2.709473481721 micrometres. Maximum nodal difference relative to coarse peak displacement at that time is 1.6016832551 percent. This is full-field comparison, not merely the change in the scalar maximum-displacement metric.

Measurements/checksums are in `supported-wall-viscous-dt-0.1-vs-0.05.json`; actual fine CSV archived as `supported-wall-viscous-half-dt-stages.csv`. Static audit of reference and all 46 committed fine meshes found no nonadjacent crossings or limit truncation (`supported-wall-viscous-half-dt-intersections.json`). These results quantify time sensitivity; one pair of timesteps does not establish convergence order or an acceptable anatomical error. Actual dt=0.025 s computation remains live, so the coarse timestep is not yet treated as numerically certified.

The comparator records the node and both coordinates at each maximum difference. The t=1.1 s peak is at node 708, in the support mesh (wall nodes are 0..559), predominantly in its x displacement. The measured maximum and RMS curves are rendered in `supported-wall-viscous-time-refinement-render.png`; this plot explicitly marks the third timestep as pending. Localization alone does not identify the physical or numerical cause of this peak.

`tools/render_supported_wall_time_error.py` renders the actual raw fine-step mesh in X-Z and Y-Z projections, coloring all nodes by coarse/fine Euclidean coordinate difference. It verifies topology and the reported peak before plotting; it does not interpolate, weld, magnify displacement or infer anatomical properties. The inspected `supported-wall-viscous-time-error-map.png` localizes the peak to the support's middle axial section. Python compilation and diff whitespace checks passed. The independent dt=0.025 s process is still live during unloading; no third-level convergence conclusion is recorded.

### Third timestep: completed prefix comparison

Added an explicit `--until-time` comparator mode requiring an already committed common endpoint. Reports label this as a prefix and record both source observation endpoints; the default still requires equal full durations. Zero-difference self comparison passed for 24 shared times through 1.2 s. Nonfinite, future, and nonmatching prefix endpoints were rejected.

For the independently calculated dt=0.05/0.025 s pair, all 24 common times through 1.2 s meet the reference/material/pressure/constitutive-law/residual checks. Maximum coordinate difference is 2.883386771982 micrometres at t=0.6 s (node 540, wall), RMS there 1.158606724372 micrometres. At t=1.1 s, where the dt=0.1/0.05 pair had its 22.253 micrometre peak, the new pair's maximum difference is 1.634276529405 micrometres, RMS 0.591714730149 micrometres. This establishes reduced time sensitivity on the completed prefix, not an error bound for the unfinished recovery interval or an anatomical calibration. Report: `supported-wall-viscous-dt-0.05-vs-0.025-prefix.json`; inspected raw-mesh plot: `supported-wall-viscous-quarter-prefix-error-map.png`, explicitly labeled as a prefix. The authoritative quarter-step process remains live.

Extended the verified prefix to 1.6 s (32 dt=0.05 endpoints), including initial zero-load recovery. Maximum difference remains 2.883386771982 micrometres. At all 16 common times shared by all three timestep levels, the finer pair's maximum nodal difference is strictly lower than the coarser pair's. `tools/render_supported_wall_refinement.py` plots these identical sampled times without interpolation; inspected output `supported-wall-viscous-three-dt-prefix-render.png` labels the partial observation window explicitly. This still supplies no formal convergence order or complete-cycle error bound. Comparator hashes now refer to the exact CSV byte snapshots parsed, avoiding provenance drift while a producer appends new rows. Python compilation and diff whitespace checks passed; quarter-step recovery remains live.

### Third timestep completed: full-cycle comparison

The authoritative dt=0.025 s process exited successfully with all 92 physical increments through 2.3 s. Maximum committed residual is 9.985864479859e-8 N; minimum element J across the cycle is 0.7227745954086. At final recovery, wall displacement is 0.1188047905865 mm and support displacement 0.1292797576410 mm. The unchanged zero-load observation window does not imply permanent residual strain.

Complete dt=0.05/0.025 comparison verifies all 46 matching times and the unchanged pressure/material/reference protocol; maximum nodal difference remains 2.883386771982 micrometres. At all 23 shared three-level times, the finer pair has smaller maximum nodal difference than the dt=0.1/0.05 pair. This supports reduced timestep sensitivity for this synthetic benchmark, without claiming formal convergence order, mesh convergence or calibrated human anatomy. Complete CSV: `supported-wall-viscous-quarter-dt-stages.csv`; full comparison: `supported-wall-viscous-dt-0.05-vs-0.025.json`; inspected full-cycle plot: `supported-wall-viscous-three-dt-render.png`. Static audit of rest and all 92 committed meshes found zero nonadjacent-face findings and no limit truncation (`supported-wall-viscous-quarter-dt-intersections.json`). Prefix artifacts remain historical snapshots, not live-status claims. No own benchmark process remains running.
