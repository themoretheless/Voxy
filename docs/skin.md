# Nonlinear skin shell

`physics::skin` is the continuum successor to the illustrative edge-spring skin in `physics::tissue`. It uses a reference triangle metric, layered constitutive energies, signed dihedral bending and implicit dynamics. All geometry is in metres; mass is kilograms; forces are newtons; energies are joules.

## Material

For each triangle, construct the surface deformation gradient F (3 by 2) from its stress-free chart. C = transpose(F) F has components C11, C22, C12. The surface area stretch is J_s = sqrt(det(C)). The incompressible thickness stretch is 1/J_s, so t_current = t_reference/J_s and the 3D first invariant is I1 = C11 + C22 + 1/det(C).

Each layer contributes reference thickness times:

```
W_matrix = mu / 2 * (I1 - 3)
I4 = a^T C a
E_f = max(0, kappa * (I1 - 3) + (1 - 3*kappa) * (I4 - 1))
W_fibers = sum(two families) k1 / (2*k2) * (exp(k2 * E_f^2) - 1)
```

The two fiber directions lie at +/- fiber_angle relative to a caller-supplied rest-space tangential axis on each triangle. Both angles and dispersion are independent material inputs. Fiber recruitment follows the positive generalized GOH strain; a compressed mean direction can still recruit dispersed transverse fibers. No exponential clamping silently changes the constitutive law: excessive input causes numerical overflow and a rejected step.

`SkinMaterial::response` exposes energy per reference area, its exact metric gradient/tangent, and current thickness. Mass is lumped from reference triangle area times the sum of layer density times thickness. Defaults are illustrative epidermal/dermal layers, not calibrated constants for an individual or an anatomical region.

Bending uses the change in signed dihedral angle, retaining the rest angle for curved surfaces. Hinge energy is 0.5 * D * 3 * rest_edge_length² / (sum adjacent reference areas) * delta_angle². D is calculated through the thickness of the laminate about its stiffness-weighted neutral axis with E=3*mu and nu=0.5. This is a matrix small-strain bending law; it does not recruit collagen through a finite-strain thickness integration.

Each Maxwell branch stores the tangential Green strain q. Its energy density is 0.5 * modulus * ||E-q||², with the off-diagonal strain counted twice. Internal history updates implicitly as q_new = (q_old + dt/tau * E_new)/(1+dt/tau). Eliminating q yields the reduced incremental energy with modulus/(1+dt/tau). This creates stress relaxation and hysteresis without damping rigid translations or rotations. It is an objective tangential generalized Maxwell model, not a fitted full 3D nonlinear viscoelastic constitutive law.

## Dynamics and contact

The solver minimizes the backward-Euler inertial potential plus hyperelastic, incremental viscoelastic, bending, optional attachment and contact energies. Exact first and second derivatives are computed with local second-order automatic differentiation. Newton uses matrix-free preconditioned conjugate gradients; non-positive search curvature triggers Hessian regularization. Armijo line search decreases the incremental objective. Steps commit positions, velocities and viscoelastic history only after force-residual convergence.

`Attachment` is a spring/dashpot relative to a moving target in deeper tissue. It is an explicit coupling interface, not a volumetric model of fascia, fat or muscle. Hard pins are stored as future targets so moving a skeleton attachment does not teleport free particles.

`ContactScene` supplies analytic moving spheres and halfspaces. A positive-gap logarithmic barrier activates within a configurable distance. Sphere contact uses closest triangle features, including face interiors and edges. Barrier energies are integrated over reference area. The shell has a conservative collision offset equal to half the reference total thickness. It currently does not decrease that offset with local thinning. Conservative advancement certifies complete linear trajectories for sphere/triangle contact and triangle collapse. Exhausted certificates and fast sweeps that cannot be resolved fail closed. They do not publish a tunneled frame. Callers should reduce the timestep on failure; automatic adaptive stepping is not yet supplied.

There is currently no Coulomb friction, vertex/triangle or edge/edge self-contact, volumetric subcutaneous material, tearing/damage, fluid coupling or anatomical parameter fitting. A maximum realistic skin simulation still requires these features and experiments for the intended body region. The implementation does not claim patient-specific or clinical accuracy.

## Verification and real mesh

```sh
cargo test -p physics --test skin --test skin_embedding --release
cargo test -p voxy_app --lib female_demo
VOXY_SKIN_ONLY=1 cargo run -p voxy_app --example female --release
VOXY_SKIN_ONLY=1 cargo run -p voxy_app --example female --release -- --smoke
VOXY_SKIN_ONLY=1 cargo run -p voxy_app --example skin_render --release
```

Tests cover finite-difference material gradients/tangents; shell force derivatives including hinges; zero rest stress; frame objectivity and total force/torque; anisotropy and nonlinear recruitment; volume-preserving thinning; rigid translation/free fall; fixed-strain stress relaxation; transactional failures; topology validation; face-interior sphere contact; loaded halfspace equilibrium; and rejected fast sphere crossing.

The female example uses the actual Blender Studio anatomical geometry. Its full-body reduced shell has 636 nodes and 1268 triangles, with complete displacement bindings to all 42,342 vertices of the detailed body mesh. Eyes and strand hair remain separate systems. The export preserves the high-resolution shape through displacement offsets; the maximum distance from a detailed vertex to the reduced shell is 9.76 mm. This is a mapping-resolution measurement, not a physiological accuracy estimate.

`SkinEmbedding` is a renderer-independent library interface. It validates full coverage, barycentric weights and indices, rejects duplicate bindings, and preserves independent coincident seam vertices. At rest it leaves the detailed mesh unchanged. During animation it transfers `physical shell - current skeleton reference` displacements onto the independently posed render mesh, avoiding a second application of skeletal motion.

Thin forearm and hand skin uses up to four times the torso foundation stiffness,
with a smooth spatial transition across the elbow. This keeps the physical shell
close to the rig in bony regions while retaining compliant torso motion.

The whole shell follows moving spring/dashpot attachments driven by the procedural 16-bone rig and existing secondary tissue support motion. Attachment targets are the positions at the beginning of the step; their velocity advances them to the endpoint. The skin is solved in world space, so it can lag behind moving supports. Gravity acts on the shell. The optional spatially localized anterior abdominal load cycles smoothly when enabled with P; the ordinary character starts with this diagnostic load disabled. Reference-area loads are prescribed tractions, not a coupled internal-pressure or moving-probe simulation. The demo supports an optional rigid spherical probe; self-contact and full body-to-body contact remain unsupported.

The implicit step follows accumulated frame time, with at least 1/120 s between solves, a maximum 50 ms step and a maximum 100 ms backlog. Only one implicit skin solve runs per rendered frame. Hair independently subdivides to at most 1/240 s. This preserves the clip's elapsed-time rate at ordinary 20–120 Hz input cadences; long stalls beyond the backlog limit still discard excess time. The title displays measured wall time per step and the last force residual. This is not a claim of 120 steps per wall-clock second. `M` shows displacement magnitude from the moving support reference, blue to red over 0–10 mm; `P` toggles the abdominal load; Space pauses; mouse drag and arrows control the view.

The default layer properties, rest pose, foundation stiffness and body-up collagen axis are illustrative. There are no measured region-specific Langer directions, patient-specific prestress or parameter fits. A high-resolution visible mesh does not establish high-resolution physical accuracy.

References: [Gasser-Ogden-Holzapfel model and skin fitting discussion](https://pmc.ncbi.nlm.nih.gov/articles/PMC5042997/); [collagen dispersion measurements](https://arxiv.org/abs/1203.4733); [Discrete Shells](https://multires.caltech.edu/pubs/ds.pdf). The material uses the undamaged GOH-type energy, not the damage extension in the first paper.


## Library audit fixes

The Maxwell update now uses bounded retention/update factors `tau/(tau+dt)` and
`dt/(tau+dt)` instead of forming `dt/tau`. This prevents valid subnormal relaxation
times from overflowing the ratio and committing NaN material history. Tests cover
both extremely short and long relaxation times. Laminate validation also checks
aggregate thickness, areal density and bending rigidity before publishing results.
Surface embedding tests cover full coverage, render seams, unchanged rest detail,
no doubled skeleton motion, invalid inputs and numerical overflow. Integration
checks require physical displacement on the head, arms, torso and legs, positive
skin thickness and noncollapsed triangles on the supplied anatomical model.

Topology validation rejects disconnected vertex fans even when each edge is manifold.

`VOXY_SKIN_ONLY=1` isolates skin validation from the optional Cosserat hair solver.
Hair follows the head rigidly in this mode. Female skin integration tests use this
mode; hair dynamics have their own tests. Combined hair simulation currently has
an unresolved CPU performance issue and is not covered by the skin smoke result.

Validation on the supplied model: 17 shell/material/contact tests, 2 embedding
tests and 3 female integration tests pass in release mode. The isolated offscreen
Metal render passed after 60 nonlinear skin steps and produced the skin surface
and displacement map. The native 120-frame smoke attempt was blocked by window
occlusion (`SkippedOccluded`); it is not reported as a passing window test.
An observed native step took about 230 ms, so this demo is not real-time.

## Scalar line search and strain diagnostics

Armijo trials evaluate scalar energy with zero-dimensional automatic differentiation,
without building gradients, Hessian blocks or CG diagonals. Face material and Maxwell
energy share the same generic implementation with the full Newton evaluation.
Newton derivatives and convergence tolerances are unchanged. Stored-energy queries
and committed Maxwell metric updates also avoid unused derivative work.

A regression compares scalar and full energies on 20 deformed states with nonzero
Maxwell memory, a hard pin, moving spring/dashpot support, sphere and halfspace barriers.
The scalar path also rejects collapsed elements. An affine-stretch test checks
principal stretches, area ratio and incompressible thickness under rigid rotations.

`Skin::surface_metrics()` exposes ordered principal stretches, area ratio and thickness
per physical triangle. `K` displays `max(abs(lambda_min-1), abs(lambda_max-1))`
from blue to red over 0–10%; it includes compression as well as extension. Values are
piecewise constant on the coarse physical triangles. `M` retains the displacement
map and switches off the strain map, and vice versa.

Reproduce the isolated evaluation benchmark:

```sh
cargo test -p physics --lib skin::shell::objective_tests::benchmark_objective_evaluations --release -- --ignored --nocapture
VOXY_SKIN_ONLY=1 cargo run -p voxy_app --example skin_render --release -- --strain
```

On this machine, 100 evaluations on a 20×20 patch averaged 15.15 ms for full
second derivatives and 0.052 ms for scalar energy. This measures only objective
evaluation; CG, collision trajectory checks, Newton assembly and rendering remain.
It does not establish the same acceleration for a complete step or real-time simulation.
The strain preview is `/tmp/voxy-skin-strain-preview.png`.

`Skin::set_element_workers(1..=64)` controls parallel evaluation of independent
membrane and hinge derivatives. The library defaults to one worker; the female
demo uses up to eight available CPU workers. Small meshes remain sequential.
Element results are assembled in source order, preserving the arithmetic order
of energy, gradient and Hessian accumulation. Local Hessians use contiguous
row-major storage. Contact evaluation, CG and line search retain their existing
semantics.

The `parallel_elements_preserve_derivatives_and_integrated_state_exactly`
regression compares energies, element derivatives, assembled gradients and
three integrated steps against the sequential path, including exact position
and velocity equality. This is a deterministic CPU acceleration, not a
real-time guarantee for the complete skin and hair pipeline.

The female demo evaluates skin and hair concurrently within a physical step,
then joins both before committing its playback time. Both use the same sampled
skeletal pose. Hair contacts use the posed anatomical mesh, rather than the
skin solver's displaced surface; this remains a one-way coupling approximation.
The physical timestep and solver tolerances are unchanged.

Run `cargo run --release -p voxy_app --example skin_render -- --sequence` to
validate 720 physical steps spanning the six-second rig clip and render 120
samples to `/tmp/voxy-full-character-frames`. The example reports physical-step
and mesh-build CPU timing separately. These saved frames are an offline
trajectory inspection and do not prove real-time native playback or hairstyle
quality.

Current checks: 18 skin integration tests, 2 embedding tests, 1 scalar/full regression,
3 female integration tests and the separately invoked measurement pass. Offscreen Metal
rendering of the model and strain map passed after 60 nonlinear steps.

## Interactive rigid probe

`C` toggles a 20 mm-radius rigid sphere against the anterior abdominal shell.
The support rig carries its anchor; indentation advances/retracts at 20 mm/s,
with a maximum prescribed travel of 12 mm from the initially clear position.
The initial center offset is 31 mm from the selected reduced-shell node.
The contact barrier activates within 3 mm and uses stiffness 100,000 N/m³.
`P` independently controls the prescribed abdominal traction. Turn it off to
isolate the probe. The title reports the absolute axial contact reaction `Fz`
in newtons, evaluated at the accepted endpoint; it is not a calibrated load-cell prediction.
The sphere remains present during retraction and is hidden only after returning
completely. Its render allocation/topology stays constant across toggles.

A conservative triangle-AABB distance check excludes spheres beyond the barrier's
finite influence. Closest features still determine every retained barrier term;
swept conservative advancement remains unchanged. A deterministic 2,000-case check
compares AABB rejection against exact closest-feature distances. Contact-force
queries evaluate barrier derivatives directly, avoiding two complete material/hinge
assemblies and their subtraction. A regression compares the direct force against
that reference calculation.

The actual model integration test advances 80 fixed steps into contact and 80
through retraction, requires nonzero reaction, valid triangle area, complete
retraction, zero released contact force, finite positions and stable render topology.
An initial run measured about 4.99 N peak axial reaction with the illustrative parameters.
The moving/viscoelastic surface is not expected to return instantly to its original
rest pose. The collision guarantee applies to the reduced physical shell, including
its thickness offset, not every detailed render triangle. The retained high-resolution
offsets have up to 9.76 mm mapping distance; clinical accuracy remains unvalidated.

```sh
VOXY_SKIN_ONLY=1 cargo run -p voxy_app --example skin_render --release -- --probe --strain
```

This exports a close-up to `/tmp/voxy-skin-probe-preview.png`: earlier approach
at left and the loaded strain map at right. The native render and force test exercise
physical sphere contact, rather than substituting a visual indentation.

`cargo run --release -p voxy_app --example skin_render -- --sequence --frame-step`
renders the full six-second loop using 120 physical steps of 50 ms each, with
hair substeps, instead of the reference sequence's 720 steps of 1/120 s.
The frame-cadence regression validates full skin/hair integration at 20 and 30 Hz.

### Editing rest shape and regional properties

`Skin::rebased(rest)` returns a new stress-free shell with rebuilt geometric metrics, lumped masses, pin positions and bending hinges. It retains material and regional scales, but resets velocities and relaxation history. Caller bindings and animation targets must be rebuilt or transformed consistently before adopting the returned shell.

`set_face_properties(stiffness_scales, density_scales)` validates per-triangle positive finite multipliers before updating state. Membrane/relaxation energy scales per face; shared-edge bending uses a harmonic stiffness mean. Surface mass is reassembled from triangle area and density; contact penalty area remains independent of density. These are material editing operations, not physiological calibration.
