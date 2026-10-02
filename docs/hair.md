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

The native groom has 203 guides of 20 segments, six nonlinear iterations and at
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
The renderer currently displays guide tubes with simple opaque shading, not a
hair scattering material. CPU throughput is reported by the groom test rather
than claiming a real-time GPU hair system.

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
