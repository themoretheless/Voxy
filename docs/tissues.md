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

Each particle has position x, velocity v, and inverse mass w. Zero inverse mass is a skeleton attachment, moved explicitly with `move_pin`. Free tissue remains behind when an attachment moves, producing secondary motion. The integrator predicts positions with acceleration and exponential velocity damping exp(-damping * dt); corrected velocities are reconstructed from position differences.

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
