# Destruction, crumbling and dust

Status: 2026-10-13, plan steps A1–A4 landed. This covers the chain from a
fractured continuum body to rigid debris, voxel removal and airborne fines.
Nothing here is a calibrated material model; every quantity is conserved or
booked explicitly so a caller can audit it.

## Pipeline

1. **Fracture** happens in the quadratic cohesive mesh
   (`physics::plasticity::mesh::QuadraticBody`, `DynamicBody`): cohesive faces
   reach full damage and `fragment_nodes()` reports connected components.
2. **Release** (`QuadraticBody::release_debris(densities, velocities, min_mass)`,
   `crates/physics/src/plasticity/mesh/quadratic/debris.rs`): every fragment at
   or above the mass threshold becomes a `DebrisPiece` with the consistent-mass
   centre, inertia tensor, momentum and spin from `fragments.rs`, plus a convex
   hull (`convex_hull`, incremental, outward CCW faces). Smaller fragments stay
   in `retained_nodes`. `DebrisRelease` reports rigid kinetic energy, the
   residual deformable kinetic energy and the fully broken cohesive area between
   released pieces. Mass, linear and angular momentum of the pieces equal the
   fragment integrals to 1e-12 (`tests/debris_release.rs`).
3. **Debris dynamics** (`physics::debris::step_debris`, `crates/physics/src/debris.rs`):
   pieces fall under constant gravity with an exact ballistic update, land on
   a plane with inelastic normal impulses and Coulomb friction, and a contact
   patch (≥3 touching hull vertices) pins the face so resting bodies do not creep.
   `DebrisStepReport` keeps `kinetic + potential + dissipated - correction -
   integration` equal to the initial energy by construction; `integration_j` is
   the scheme's own first-order residual (≈ ½·m·g²·dt² per resting step) and is
   reported rather than hidden. Tests: exact free fall, landing at rest with a
   residual that halves with the step, stick/slip on an incline against
   `μ = tan θ`.
4. **Dust** (`physics::wear::fracture_dust`, `crates/physics/src/wear/fracture_dust.rs`):
   the deformable energy a rigid piece cannot carry (times `energy_fraction`,
   plus caller energy) buys fresh surface at `3γ/(ρr)` J/kg, so
   `m = E·ρ·r / (3γ)`, capped by `max_mass_fraction` of the released mass. Grains
   are `suspension::Particle`s at hull vertices with piece velocities;
   `donor_mass_fraction` tells the caller how much mass to take off each piece.
5. **Voxel removal** (`physics_voxel::plan_fracture`, `crates/physics_voxel/src/fracture.rs`):
   with a `FractureFrame` (origin, voxel size, write budget) every piece claims
   the solid voxels whose centres lie inside its hull. The result is one
   atomic `EditTxn` (revision-checked like `plan_explosion`) plus a
   `VoxelChunkPiece` inventory per piece, so the removed voxels can be rendered
   as debris following `DebrisBody` motion. Claimed volume matches the hull
   volume within one voxel layer.

## Update 2026-10-14

- **Piece–piece, hull–obstacle & hull–voxel-world contacts**:
  - `step_debris` resolves piece–piece SAT contacts over both hulls' face normals
    with support-patch centroid contact points, shared inelastic impulses,
    Coulomb friction, and mass-split push-out.
  - `step_debris_with_obstacles` resolves SAT contacts between debris convex hulls
    and static axis-aligned voxel/box obstacles (`DebrisBoxObstacle`).
  - `step_debris_in_world` sweeps debris hull AABBs against any
    `physics::CollisionWorld` (including `physics_voxel::World` via
    `VoxelCollisionWorld`) with sliding response and piece–piece contact.
- **Secondary impact comminution / crumbling (`crumble_debris_on_impact`)**:
  - Splits high-impact debris pieces along their longest local hull axis into
    smaller child `DebrisBody` pieces and emits airborne `FractureDust`
    (`suspension::Particle`) while conserving total mass and linear momentum.
- `voxy_demos::destruction_demo::DestructionDemo` runs the whole chain in one
  object (fracture release → `plan_fracture` commit → `step_debris` landing with
  a closed ledger → `fracture_dust` → `ParticipatingMedium` extinction); its
  test removes >100 voxels, lands both pieces at rest and keeps dust mass
  conserved apart from floor settling.
- Medium transport backends: `voxy_gpu::GpuMediumProgram` (wgpu, oracle test
  `tests/medium_reference.rs`: max cell difference 3e-8 vs the CPU reference,
  settled mass matched) and `voxy_cuda::CudaCompute::medium_transport` on the
  same packed word ABI (`pack_medium_job`/`unpack_medium_result`).
- Drying: `ParticipatingMedium::exchange_film_vapor` moves film solvent into the
  containing gas cell with exact mass bookkeeping.

