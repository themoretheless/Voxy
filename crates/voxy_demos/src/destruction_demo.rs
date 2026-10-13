//! End-to-end destruction chain (plan step A5): a fractured quadratic body
//! releases rigid pieces, the pieces claim and remove the voxels inside their
//! hulls atomically, fall and land on the ground with an audited energy
//! ledger, and the surface energy that rigid motion cannot carry becomes dust
//! in the participating medium whose extinction field the renderer reads.

use std::collections::BTreeMap;
use std::sync::Arc;

use physics::debris::{DebrisBody, DebrisGround, DebrisStepReport, step_debris};
use physics::liquid::{DropletExtinctionGrid, FiniteDropletGasGrid, VaporCell};
use physics::medium::{
    MediumSpecies, MediumSpeciesProperties, MediumStepControl, ParticipatingMedium,
};
use physics::plasticity::Material;
use physics::plasticity::mesh::{DebrisRelease, QuadraticBody};
use physics::wear::{FractureDust, FractureDustSettings, fracture_dust};
use physics_voxel::{FractureFrame, FracturePlan, plan_fracture};
use voxy_core::{ChunkPos, WorldEpoch};
use voxy_world::{
    BlockDef, BlockRegistry, ChunkData, CollisionShape, EditSource, GeneratedChunk, MaterialId,
    Occlusion, PalettedBlocks, RenderKind, ResourceKey, World, WorldLimits,
};

/// Ground plane height in metres; the body sits above it.
pub const GROUND_Y: f64 = -2.;
const VOXEL_SIZE: f64 = 0.1;
const DENSITY: f64 = 2400.;

#[derive(Debug)]
pub struct DestructionDemo {
    pub world: World,
    pub release: DebrisRelease,
    pub plan: FracturePlan,
    pub bodies: Vec<DebrisBody>,
    pub ledger: DebrisStepReport,
    pub dust: FractureDust,
    pub medium: ParticipatingMedium,
    pub time_s: f64,
}

impl DestructionDemo {
    /// Builds the world, fractures the body and performs the irreversible
    /// stages (release, voxel removal, dust emission) once.
    /// # Errors
    /// Any physics or world failure; nothing is partially applied to the world.
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let registry = Arc::new(BlockRegistry::new(vec![
            BlockDef {
                key: ResourceKey::parse("voxy:air")?,
                render: RenderKind::Invisible,
                occlusion: Occlusion::None,
                collision: CollisionShape::Empty,
                face_materials: [MaterialId(0); 6],
                translucent_interface_group: None,
                emission: 0,
                blast_resistance: 0,
            },
            BlockDef {
                key: ResourceKey::parse("voxy:stone")?,
                render: RenderKind::Opaque,
                occlusion: Occlusion::FullCube,
                collision: CollisionShape::FullCube,
                face_materials: [MaterialId(1); 6],
                translucent_interface_group: None,
                emission: 0,
                blast_resistance: 20,
            },
        ])?);
        let stone = registry
            .find(&ResourceKey::parse("voxy:stone")?)
            .ok_or("missing stone")?;
        let mut world = World::new(
            WorldEpoch::new(1).ok_or("invalid epoch")?,
            registry.clone(),
            WorldLimits::default(),
        );
        world.insert_generated(GeneratedChunk {
            pos: ChunkPos { x: 0, y: 0, z: 0 },
            data: ChunkData {
                blocks: PalettedBlocks::uniform(stone),
                block_data: BTreeMap::new(),
            },
        })?;

        // Two tetrahedra already separated by a fully failed interface,
        // flying apart with a non-rigid (stretching) velocity field.
        let material = Material::new(1e6, 0.3, 1e9, 0.)?;
        let body = QuadraticBody::from_linear(
            vec![
                [0.; 3],
                [1., 0., 0.],
                [0., 1., 0.],
                [0., 0., 1.],
                [0.; 3],
                [1., 0., 0.],
                [0., 1., 0.],
                [0., 0., -1.],
            ],
            vec![([0, 1, 2, 3], material), ([4, 5, 6, 7], material)],
        )?;
        let groups = body.fragment_nodes();
        let mut velocities = vec![[0.; 3]; body.positions().len()];
        for (g, nodes) in groups.iter().enumerate() {
            let sign = if g == 0 { 1. } else { -1. };
            for &i in nodes {
                let p = body.positions()[i];
                velocities[i] = [0.4 * p[0], 0.2, sign * 1.5 + 0.3 * p[2]];
            }
        }
        let release = body.release_debris(&[DENSITY, DENSITY], &velocities, 0.)?;
        let frame = FractureFrame {
            // The body spans [-1, 1] m; map it into chunk (0, 0, 0).
            origin_m: [-1., -1., -1.],
            voxel_size_m: VOXEL_SIZE,
            max_writes: 100_000,
        };
        let plan = plan_fracture(&world, &registry, EditSource::Simulation, &release, frame)?
            .ok_or("no solid voxels under the fracture")?;
        world.commit(plan.txn.clone())?;
        let dust = fracture_dust(
            &release,
            FractureDustSettings {
                density_kg_m3: DENSITY,
                surface_energy_j_m2: 1.,
                energy_fraction: 1.,
                extra_energy_j: 0.,
                grain_radius_m: 5e-5,
                max_mass_fraction: 0.05,
            },
        )?;
        let bodies = release
            .pieces
            .iter()
            .map(|piece| {
                let mut piece = piece.clone();
                // The fines left the pieces.
                piece.mass_kg *= 1. - dust.donor_mass_fraction;
                DebrisBody::new(piece)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let n = [24usize, 24, 24];
        let h = 0.25;
        let volume = h * h * h;
        let gas = FiniteDropletGasGrid::new(
            [-3.; 3],
            [h; 3],
            n,
            vec![
                VaporCell {
                    mass: 1.2 * volume,
                    volume,
                    temperature: 300.,
                    velocity: [0.; 3],
                    specific_heat_cv: 718.,
                };
                n[0] * n[1] * n[2]
            ],
        )?;
        let mut medium = ParticipatingMedium::new(
            gas,
            vec![MediumSpeciesProperties {
                kind: MediumSpecies::Dust,
                mass_extinction_m2_kg: 3. * 2. / (4. * DENSITY * 5e-5),
                albedo: 0.7,
                settling_velocity_m_s: 0.05,
                diffusivity_m2_s: 1e-3,
                buoyancy_per_kg_m3: 0.,
            }],
        )?;
        medium.deposit_particles(0, &dust.particles)?;
        Ok(Self {
            world,
            release,
            plan,
            bodies,
            ledger: DebrisStepReport::default(),
            dust,
            medium,
            time_s: 0.,
        })
    }

    /// Advances debris and dust by `dt`.
    /// # Errors
    /// Propagates solver failures; the demo state is unchanged on failure.
    pub fn advance(&mut self, dt: f64) -> Result<(), Box<dyn std::error::Error>> {
        let mut bodies = self.bodies.clone();
        let mut ledger = self.ledger;
        step_debris(
            &mut bodies,
            dt,
            [0., -9.81, 0.],
            &DebrisGround {
                normal: [0., 1., 0.],
                offset_m: GROUND_Y,
                friction: 0.6,
            },
            &mut ledger,
        )?;
        self.medium.advance(
            dt,
            MediumStepControl {
                advance_gas: false,
                ..MediumStepControl::default()
            },
        )?;
        self.bodies = bodies;
        self.ledger = ledger;
        self.time_s += dt;
        Ok(())
    }

    /// Dust extinction field for `voxy_demos::medium_optics::extinction_view`.
    /// # Errors
    /// Numerical overflow.
    pub fn extinction(&self) -> Result<DropletExtinctionGrid, physics::liquid::Error> {
        self.medium.extinction_grid()
    }

    #[must_use]
    pub fn removed_voxels(&self) -> usize {
        self.plan.txn.writes.len()
    }

    /// Lowest hull vertex of every piece, for landing checks.
    #[must_use]
    pub fn lowest_points(&self) -> Vec<f64> {
        self.bodies
            .iter()
            .map(|b| {
                b.world_hull()
                    .iter()
                    .map(|p| p[1])
                    .fold(f64::INFINITY, f64::min)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxy_world::{BlockStateId, Sample, VoxelView};

    #[test]
    fn fracture_removes_voxels_drops_pieces_and_seeds_dust_in_the_medium() {
        let mut demo = DestructionDemo::new().unwrap();
        assert_eq!(demo.release.pieces.len(), 2);
        assert!(demo.removed_voxels() > 100, "{}", demo.removed_voxels());
        for (pos, _) in demo.plan.pieces.iter().flat_map(|p| &p.voxels) {
            assert_eq!(demo.world.sample(*pos), Sample::Loaded(BlockStateId::AIR));
        }
        // Dust left the pieces and is now in the medium.
        let piece_mass: f64 = demo.bodies.iter().map(|b| b.piece.mass_kg).sum();
        let released: f64 = demo.release.pieces.iter().map(|p| p.mass_kg).sum();
        assert!((piece_mass + demo.dust.mass_kg - released).abs() < 1e-9 * released);
        assert!((demo.medium.total_mass_kg(0) - demo.dust.mass_kg).abs() < 1e-12);
        let before = demo.extinction().unwrap();
        assert!(before.extinction_m_inverse().iter().any(|e| *e > 0.));
        // Pieces fall to the ground and come to rest with a closed ledger.
        let initial_energy = {
            let mut probe = demo.bodies.clone();
            let mut ledger = DebrisStepReport::default();
            step_debris(
                &mut probe,
                0.,
                [0., -9.81, 0.],
                &DebrisGround {
                    normal: [0., 1., 0.],
                    offset_m: GROUND_Y,
                    friction: 0.6,
                },
                &mut ledger,
            )
            .unwrap();
            ledger.kinetic_j + ledger.potential_j
        };
        for _ in 0..1500 {
            demo.advance(2e-3).unwrap();
        }
        let l = demo.ledger;
        let total = l.kinetic_j + l.potential_j + l.dissipated_j - l.correction_j - l.integration_j;
        assert!(
            (total - initial_energy).abs() < 1e-6 * initial_energy.abs(),
            "{l:?}"
        );
        assert!(l.dissipated_j > 0.);
        for lowest in demo.lowest_points() {
            assert!(lowest >= GROUND_Y - 1e-6, "piece below ground: {lowest}");
            assert!(lowest < GROUND_Y + 0.5, "piece still airborne: {lowest}");
        }
        assert!(
            demo.bodies
                .iter()
                .all(|b| b.velocity_m_s.iter().all(|v| v.abs() < 0.05))
        );
        // Dust is still conserved apart from settling through the floor.
        let after = demo.medium.total_mass_kg(0);
        assert!(after <= demo.dust.mass_kg + 1e-12 && after > 0.5 * demo.dust.mass_kg);
        assert!(
            demo.extinction()
                .unwrap()
                .extinction_m_inverse()
                .iter()
                .any(|e| *e > 0.)
        );
    }
}
