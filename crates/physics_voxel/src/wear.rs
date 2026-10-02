//! Coarse voxel removal driven by finite Archard wear inventory.
mod suspension;
use physics::wear::{Layer, Material, Removal};
use std::collections::BTreeMap;
pub use suspension::{VoxelWearSuspension, VoxelWearSuspensionStep};
use voxy_core::{VoxelPos, split_voxel};
use voxy_world::{
    BlockStateId, ChunkRevision, CommitReceipt, EditSource, EditTxn, Sample, VoxelView, VoxelWrite,
    World,
};

#[derive(Clone, Debug)]
pub struct VoxelWear {
    pos: VoxelPos,
    block: BlockStateId,
    revision: ChunkRevision,
    layer: Layer,
    material: Material,
    removed: bool,
    side_m: f64,
    face: [i8; 3],
}
#[derive(Clone, Debug)]
pub struct VoxelWearStep {
    pub removal: Removal,
    /// World invalidation, inverse edit and durability ticket on full removal.
    pub receipt: Option<CommitReceipt>,
}
impl VoxelWear {
    /// Bind wear inventory to one loaded solid voxel. Side length is in meters;
    /// area is one complete exposed face and thickness one voxel side.
    /// # Errors
    /// Missing/air voxel, invalid geometry or unavailable revision.
    pub fn new(
        view: &impl VoxelView,
        pos: VoxelPos,
        side_m: f64,
        density_kg_m3: f64,
        material: Material,
    ) -> Result<Self, &'static str> {
        if !side_m.is_finite() || side_m <= 0. {
            return Err("invalid wear voxel size");
        }
        let Sample::Loaded(block) = view.sample(pos) else {
            return Err("wear voxel unavailable");
        };
        if block == BlockStateId::AIR {
            return Err("cannot wear air voxel");
        }
        let revision = view
            .chunk(split_voxel(pos).0)
            .ok_or("wear chunk unavailable")?
            .revision;
        let layer = Layer::new(side_m * side_m, side_m, density_kg_m3)?;
        Ok(Self {
            pos,
            block,
            revision,
            layer,
            material,
            removed: false,
            side_m,
            face: [0, 1, 0],
        })
    }
    /// Select one outward axis-aligned face. One binding represents one
    /// uniformly receding face; simultaneous multi-face removal needs a new model.
    /// # Errors
    /// Invalid normal or the same geometry/world errors as `new`.
    pub fn new_on_face(
        view: &impl VoxelView,
        pos: VoxelPos,
        side_m: f64,
        density_kg_m3: f64,
        material: Material,
        face: [i8; 3],
    ) -> Result<Self, &'static str> {
        if face.iter().any(|v| !(-1..=1).contains(v))
            || face.iter().filter(|v| **v != 0).count() != 1
        {
            return Err("invalid wear face normal");
        }
        let mut wear = Self::new(view, pos, side_m, density_kg_m3, material)?;
        wear.face = face;
        Ok(wear)
    }
    /// Remaining bounds in normalized voxel-local coordinates.
    #[must_use]
    pub fn collision_bounds(&self) -> ([f64; 3], [f64; 3]) {
        let mut min = [0.; 3];
        let mut max = [1.; 3];
        let thickness = self.thickness_m() / self.side_m;
        for axis in 0..3 {
            if self.face[axis] > 0 {
                max[axis] = thickness;
            }
            if self.face[axis] < 0 {
                min[axis] = 1. - thickness;
            }
        }
        (min, max)
    }
    #[must_use]
    pub fn remaining_mass_kg(&self) -> f64 {
        self.layer.remaining_mass_kg()
    }
    #[must_use]
    pub fn debris_mass_kg(&self) -> f64 {
        self.layer.debris_mass_kg()
    }
    #[must_use]
    pub fn thickness_m(&self) -> f64 {
        self.layer.thickness_m()
    }
    /// Accumulate physical partial wear; remove voxel only at exhaustion.
    /// Partial recession is available through the wear-aware collision adapter. Any intervening chunk revision invalidates this binding.
    /// World edit is committed before wear state, preserving rollback on failure.
    /// # Errors
    /// Stale/unavailable voxel, invalid loading or failed world transaction.
    pub fn advance(
        &mut self,
        world: &mut World,
        load_n: f64,
        distance_m: f64,
    ) -> Result<VoxelWearStep, &'static str> {
        if self.removed {
            return Err("wear voxel already removed");
        }
        let chunk = split_voxel(self.pos).0;
        if world.sample(self.pos) != Sample::Loaded(self.block)
            || world.chunk(chunk).ok_or("wear chunk unavailable")?.revision != self.revision
        {
            return Err("stale wear voxel binding");
        }
        let mut layer = self.layer.clone();
        let removal = layer.advance(self.material, load_n, distance_m)?;
        let receipt = if removal.exhausted {
            Some(
                world
                    .commit(EditTxn {
                        source: EditSource::Simulation,
                        expected: vec![(chunk, self.revision)],
                        writes: vec![VoxelWrite {
                            pos: self.pos,
                            block: BlockStateId::AIR,
                        }],
                    })
                    .map_err(|_| "wear world commit failed")?,
            )
        } else {
            None
        };
        self.layer = layer;
        self.removed = removal.exhausted;
        Ok(VoxelWearStep { removal, receipt })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::BTreeMap, sync::Arc};
    use voxy_core::{ChunkPos, WorldEpoch};
    use voxy_world::{ChunkData, GeneratedChunk, PalettedBlocks, ResourceKey, WorldLimits};
    fn fixture() -> (World, VoxelWear, BlockStateId) {
        fixture_with_limits(WorldLimits::default())
    }
    fn fixture_with_limits(limits: WorldLimits) -> (World, VoxelWear, BlockStateId) {
        let registry = Arc::new(crate::test_support::test_registry());
        let stone = registry
            .find(&ResourceKey::parse("voxy:stone").unwrap())
            .unwrap();
        let mut world = World::new(WorldEpoch::new(1).unwrap(), registry, limits);
        world
            .insert_generated(GeneratedChunk {
                pos: ChunkPos { x: 0, y: 0, z: 0 },
                data: ChunkData {
                    blocks: PalettedBlocks::uniform(stone),
                    block_data: BTreeMap::new(),
                },
            })
            .unwrap();
        let wear = VoxelWear::new(
            &world,
            VoxelPos { x: 8, y: 8, z: 8 },
            1.,
            2000.,
            Material::new(1., 1.).unwrap(),
        )
        .unwrap();
        (world, wear, stone)
    }
    #[test]
    fn coupled_wear_updates_geometry_cloud_and_heat_with_atomic_failure() {
        use physics::liquid::{
            Config, Liquid, LiquidField, Material as FluidMaterial, Particle, TransportMaterial,
        };
        use physics::wear::WearSuspensionInput;
        let (mut world, old_binding, stone) = fixture();
        let binding = VoxelWear::new(
            &world,
            old_binding.pos,
            1.,
            2000.,
            Material::new(1e8, 1e-3).unwrap(),
        )
        .unwrap();
        let mut liquid = Liquid::new(
            vec![Particle {
                position: [0.; 3],
                velocity: [0.; 3],
                mass: 1e-5,
                material: 0,
            }],
            vec![FluidMaterial {
                rest_density: 1000.,
                sound_speed: 2.,
                viscosity: 0.001,
            }],
            Config {
                gravity: [0.; 3],
                ..Config::default()
            },
        )
        .unwrap();
        liquid
            .configure_transport(
                vec![LiquidField {
                    temperature: 10.,
                    concentration: 0.,
                }],
                vec![TransportMaterial {
                    specific_heat: 1.,
                    conductivity: 0.,
                    ..TransportMaterial::default()
                }],
            )
            .unwrap();
        liquid.set_viscous_heating(true).unwrap();
        liquid.set_pressure_work(true).unwrap();
        let mut coupled = VoxelWearSuspension::new(
            binding,
            physics::friction::Material::new(1e9, 1e9, 0.5).unwrap(),
            liquid,
            100.,
        )
        .unwrap();
        let initial = coupled.suspension().energy();
        let input = WearSuspensionInput {
            gap_m: [1e-4, -1e-4, 0.],
            normal: [0., 1., 0.],
            emission_positions_m: &[[0.; 3]; 16],
            inherited_velocity_m_s: [0.; 3],
            heat_weights: &[1.],
            dt_s: 1e-4,
            coupling_steps: 1,
        };
        let result = coupled.step_contact(&mut world, input).unwrap();
        assert!(result.receipt.is_none());
        assert_eq!(world.sample(old_binding.pos), Sample::Loaded(stone));
        assert_eq!(coupled.suspension().grains().len(), 16);
        let emitted_mass: f64 = coupled
            .suspension()
            .grains()
            .iter()
            .map(|p| p.mass_kg())
            .sum();
        assert!((emitted_mass - result.physics.wear.mass_kg).abs() < 1e-20);
        let (_, max) = coupled.binding().collision_bounds();
        assert!(max[1] < 1.);
        assert_eq!(max[1], coupled.binding().surface().unwrap().vertices[7][1]);
        assert!(coupled.suspension().energy().enthalpy_j > initial.enthalpy_j);
        let before = coupled.clone();
        assert!(
            coupled
                .step_contact(
                    &mut world,
                    WearSuspensionInput {
                        gap_m: [2e-4, -1e-4, 0.],
                        dt_s: 0.2,
                        ..input
                    }
                )
                .is_err()
        );
        assert_eq!(
            coupled.binding().collision_bounds(),
            before.binding().collision_bounds()
        );
        assert_eq!(coupled.suspension().energy(), before.suspension().energy());
        assert_eq!(
            coupled.suspension().contact_state(),
            before.suspension().contact_state()
        );
        assert_eq!(
            coupled.suspension().grains().len(),
            before.suspension().grains().len()
        );
        assert_eq!(world.sample(old_binding.pos), Sample::Loaded(stone));
        world
            .commit(EditTxn {
                source: EditSource::Editor,
                expected: vec![(split_voxel(old_binding.pos).0, old_binding.revision)],
                writes: vec![VoxelWrite {
                    pos: VoxelPos {
                        x: old_binding.pos.x + 1,
                        ..old_binding.pos
                    },
                    block: BlockStateId::AIR,
                }],
            })
            .unwrap();
        assert!(coupled.step_contact(&mut world, input).is_err());
        assert_eq!(coupled.suspension().energy(), before.suspension().energy());
        assert_eq!(
            coupled.binding().collision_bounds(),
            before.binding().collision_bounds()
        );
    }
    #[test]
    fn partial_inventory_then_world_removal_and_mesh_invalidation() {
        let (mut world, mut wear, stone) = fixture();
        let first = wear.advance(&mut world, 1., 0.25).unwrap();
        assert!(first.receipt.is_none());
        assert_eq!(world.sample(wear.pos), Sample::Loaded(stone));
        assert_eq!(wear.remaining_mass_kg(), 1500.);
        assert_eq!(wear.debris_mass_kg(), 500.);
        let last = wear.advance(&mut world, 1., 1.).unwrap();
        assert_eq!(last.removal.remaining_sliding_distance_m, 0.25);
        assert_eq!(world.sample(wear.pos), Sample::Loaded(BlockStateId::AIR));
        let receipt = last.receipt.unwrap();
        assert_eq!(receipt.chunks.len(), 1);
        assert_eq!(receipt.inverse.writes[0].block, stone);
        assert_eq!(wear.debris_mass_kg(), 2000.);
        assert!(wear.surface().is_none());
        assert!(wear.advance(&mut world, 1., 1.).is_err());
    }
    #[test]
    fn partial_wear_lowers_the_collision_surface() {
        let (mut world, old, _) = fixture();
        let above = VoxelPos {
            y: old.pos.y + 1,
            ..old.pos
        };
        world
            .commit(EditTxn {
                source: EditSource::Editor,
                expected: vec![(split_voxel(old.pos).0, old.revision)],
                writes: vec![VoxelWrite {
                    pos: above,
                    block: BlockStateId::AIR,
                }],
            })
            .unwrap();
        let mut wear =
            VoxelWear::new(&world, old.pos, 1., 2000., Material::new(1., 1.).unwrap()).unwrap();
        wear.advance(&mut world, 1., 0.25).unwrap();
        let registry = crate::test_support::test_registry();
        let body = crate::AnchoredAabb {
            anchor: wear.pos,
            min: [0.2, 1.5, 0.2],
            max: [0.8, 1.7, 0.8],
        };
        let ordinary = crate::sweep_aabb(
            &world,
            &registry,
            body,
            [0., -1., 0.],
            crate::SweepConfig::default(),
        )
        .unwrap();
        let worn = sweep_worn_voxels(
            &world,
            &registry,
            std::slice::from_ref(&wear),
            body,
            [0., -1., 0.],
            crate::SweepConfig::default(),
        )
        .unwrap();
        assert_eq!(ordinary.fraction, 0.5);
        assert_eq!(worn.fraction, 0.75);
        assert_eq!(worn.normal, [0, 1, 0]);
        let mut character = crate::CharacterState {
            body,
            velocity: [0., -5., 0.],
            grounded: false,
        };
        let input = crate::CharacterInput {
            planar_velocity: [0.; 2],
            jump_pressed: false,
        };
        let config = crate::CharacterConfig {
            gravity: 0.,
            step_height: 0.,
            ground_snap_distance: 0.,
            ..Default::default()
        };
        let report = crate::step_character_with_wear(
            &world,
            &registry,
            std::slice::from_ref(&wear),
            &mut character,
            input,
            0.2,
            config,
        )
        .unwrap();
        assert!(report.grounded);
        assert!((report.applied_displacement[1] + 0.75).abs() < 1e-6);
        assert_eq!(character.velocity[1], 0.);
        let before = character;
        let bad =
            crate::VoxelWear::new(&world, wear.pos, 1., 2000., Material::new(1., 1.).unwrap())
                .unwrap();
        world
            .commit(EditTxn {
                source: EditSource::Editor,
                expected: vec![(split_voxel(wear.pos).0, wear.revision)],
                writes: vec![VoxelWrite {
                    pos: wear.pos,
                    block: BlockStateId::AIR,
                }],
            })
            .unwrap();
        assert!(
            crate::step_character_with_wear(
                &world,
                &registry,
                &[bad],
                &mut character,
                input,
                0.2,
                config
            )
            .is_err()
        );
        assert_eq!(character, before);
    }
    #[test]
    fn all_six_faces_recede_inward_and_invalid_normals_fail() {
        let (mut world, original, _) = fixture();
        for axis in 0..3 {
            for sign in [-1, 1] {
                let mut face = [0; 3];
                face[axis] = sign;
                let mut wear = VoxelWear::new_on_face(
                    &world,
                    original.pos,
                    1.,
                    2000.,
                    Material::new(1., 1.).unwrap(),
                    face,
                )
                .unwrap();
                wear.advance(&mut world, 1., 0.25).unwrap();
                let (min, max) = wear.collision_bounds();
                assert_eq!(max[axis] - min[axis], 0.75);
                let surface = wear.surface().unwrap();
                let mut signed_volume = 0.;
                for [a, b, c] in surface.triangles {
                    let a = surface.vertices[usize::from(a)];
                    let b = surface.vertices[usize::from(b)];
                    let c = surface.vertices[usize::from(c)];
                    signed_volume += (a[0] * (b[1] * c[2] - b[2] * c[1])
                        + a[1] * (b[2] * c[0] - b[0] * c[2])
                        + a[2] * (b[0] * c[1] - b[1] * c[0]))
                        / 6.;
                }
                assert!((signed_volume - 0.75).abs() < 1e-14);

                assert_eq!(
                    if sign > 0 { min[axis] } else { max[axis] },
                    if sign > 0 { 0. } else { 1. }
                );
                for other in 0..3 {
                    if other != axis {
                        assert_eq!((min[other], max[other]), (0., 1.));
                    }
                }
            }
        }
        for face in [[0, 0, 0], [1, 1, 0], [2, 0, 0]] {
            assert!(
                VoxelWear::new_on_face(
                    &world,
                    original.pos,
                    1.,
                    2000.,
                    Material::new(1., 1.).unwrap(),
                    face
                )
                .is_err()
            );
        }
    }
    #[test]
    fn batch_removal_refreshes_neighbor_and_commits_all_or_nothing() {
        let (mut world, first, _) = fixture();
        let second = VoxelWear::new(
            &world,
            VoxelPos {
                x: first.pos.x + 1,
                ..first.pos
            },
            1.,
            2000.,
            Material::new(1., 1.).unwrap(),
        )
        .unwrap();
        let mut wear = [first, second];
        assert!(
            advance_voxel_wear_batch(&mut world, &mut wear, &[(1., 1.), (1., f64::NAN)]).is_err()
        );
        assert_eq!(wear[0].remaining_mass_kg(), 2000.);
        assert_eq!(world.sample(wear[0].pos), Sample::Loaded(wear[0].block));
        let r = advance_voxel_wear_batch(&mut world, &mut wear, &[(1., 1.), (1., 0.25)]).unwrap();
        assert_eq!(r.receipt.unwrap().chunks.len(), 1);
        assert_eq!(wear[1].remaining_mass_kg(), 1500.);
        validate_wear_bindings(&world, &wear).unwrap();
        let r = advance_voxel_wear_batch(&mut world, &mut wear, &[(0., 0.), (1., 0.75)]).unwrap();
        assert!(r.removals[0].is_none());
        assert_eq!(wear[1].remaining_mass_kg(), 0.);
        assert_eq!(
            wear.iter().map(VoxelWear::debris_mass_kg).sum::<f64>(),
            4000.
        );
        assert_eq!(world.sample(wear[1].pos), Sample::Loaded(BlockStateId::AIR));
    }
    #[test]
    fn world_budget_rejection_rolls_back_complete_batch() {
        let (mut world, first, stone) = fixture_with_limits(WorldLimits {
            max_writes_per_transaction: 1,
            max_chunks_per_transaction: 1,
        });
        let second = VoxelWear::new(
            &world,
            VoxelPos {
                x: first.pos.x + 1,
                ..first.pos
            },
            1.,
            2000.,
            Material::new(1., 1.).unwrap(),
        )
        .unwrap();
        let mut wear = [first, second];
        let revision = wear[0].revision;
        assert!(advance_voxel_wear_batch(&mut world, &mut wear, &[(1., 1.), (1., 1.)]).is_err());
        for w in &wear {
            assert_eq!(w.remaining_mass_kg(), 2000.);
            assert_eq!(w.debris_mass_kg(), 0.);
            assert!(w.surface().is_some());
            assert_eq!(world.sample(w.pos), Sample::Loaded(stone));
            assert_eq!(
                world.chunk(split_voxel(w.pos).0).unwrap().revision,
                revision
            );
        }
        let r = advance_voxel_wear_batch(&mut world, &mut wear, &[(1., 1.), (0., 0.)]).unwrap();
        assert_eq!(r.receipt.unwrap().inverse.writes.len(), 1);
        validate_wear_bindings(&world, &wear).unwrap();
    }
    #[test]
    fn two_chunk_removal_uses_one_transaction_and_keeps_mass() {
        let (mut world, first, stone) = fixture();
        let pos = VoxelPos { x: 40, y: 8, z: 8 };
        world
            .insert_generated(GeneratedChunk {
                pos: split_voxel(pos).0,
                data: ChunkData {
                    blocks: PalettedBlocks::uniform(stone),
                    block_data: BTreeMap::new(),
                },
            })
            .unwrap();
        let second =
            VoxelWear::new(&world, pos, 0.5, 1000., Material::new(1., 1.).unwrap()).unwrap();
        let mut wear = [first, second];
        let initial: f64 = wear.iter().map(VoxelWear::remaining_mass_kg).sum();
        let r = advance_voxel_wear_batch(&mut world, &mut wear, &[(1., 1.), (1., 1.)]).unwrap();
        let receipt = r.receipt.unwrap();
        assert_eq!(receipt.chunks.len(), 2);
        assert_eq!(receipt.inverse.writes.len(), 2);
        assert_eq!(
            wear.iter().map(VoxelWear::debris_mass_kg).sum::<f64>(),
            initial
        );
        for w in &wear {
            assert_eq!(world.sample(w.pos), Sample::Loaded(BlockStateId::AIR));
        }
        // The half-meter voxel occupies 0.125 m³: its exhaustion consumes 0.125 m of the accepted path.
        assert_eq!(r.removals[1].unwrap().remaining_sliding_distance_m, 0.875);
    }
    #[test]
    fn competing_edit_preserves_wear_inventory() {
        let (mut world, mut wear, _) = fixture();
        world
            .commit(EditTxn {
                source: EditSource::Editor,
                expected: vec![(split_voxel(wear.pos).0, wear.revision)],
                writes: vec![VoxelWrite {
                    pos: wear.pos,
                    block: BlockStateId::AIR,
                }],
            })
            .unwrap();
        assert!(wear.advance(&mut world, 1., 2.).is_err());
        assert_eq!(wear.remaining_mass_kg(), 2000.);
        assert_eq!(wear.debris_mass_kg(), 0.);
    }
}

/// Sweeps against partially worn axis-aligned full-face voxel layers.
/// Geometry recedes inward from each selected face; other blocks keep registry shapes.
/// # Errors
/// Invalid sweeps, duplicate/excessive wear bindings or stale world revisions.
pub fn sweep_worn_voxels(
    view: &impl VoxelView,
    registry: &voxy_world::BlockRegistry,
    wear: &[VoxelWear],
    aabb: crate::AnchoredAabb,
    displacement: [f64; 3],
    config: crate::SweepConfig,
) -> Result<crate::SweepResult, crate::SweepError> {
    let bindings = validate_wear_bindings(view, wear)?;
    crate::collision::sweep_aabb_with_bounds(
        view,
        registry,
        aabb,
        displacement,
        config,
        |pos, _| {
            Ok(bindings
                .get(&pos)
                .map_or(([0.; 3], [1.; 3]), |w| w.collision_bounds()))
        },
    )
}

/// Wear-aware backend for the shared character/vehicle collision controller.
#[derive(Debug)]
pub struct WornVoxelCollisionWorld<'a, V> {
    pub view: &'a V,
    pub registry: &'a voxy_world::BlockRegistry,
    pub wear: &'a [VoxelWear],
}
impl<V: VoxelView> physics::CollisionWorld for WornVoxelCollisionWorld<'_, V> {
    type Obstacle = crate::SweepObstacle;
    type Error = crate::SweepError;
    fn sweep_aabb(
        &self,
        body: physics::AnchoredAabb,
        displacement: [f64; 3],
        max_candidates: usize,
    ) -> Result<physics::SweepResult<Self::Obstacle>, Self::Error> {
        let hit = sweep_worn_voxels(
            self.view,
            self.registry,
            self.wear,
            crate::AnchoredAabb {
                anchor: VoxelPos {
                    x: body.anchor.x,
                    y: body.anchor.y,
                    z: body.anchor.z,
                },
                min: body.min,
                max: body.max,
            },
            displacement,
            crate::SweepConfig {
                max_candidate_voxels: max_candidates,
            },
        )?;
        Ok(physics::SweepResult {
            fraction: hit.fraction,
            normal: hit.normal,
            obstacle: hit.obstacle,
        })
    }
}

pub(crate) fn validate_wear_bindings<'a>(
    view: &impl VoxelView,
    wear: &'a [VoxelWear],
) -> Result<std::collections::BTreeMap<VoxelPos, &'a VoxelWear>, crate::SweepError> {
    if wear.len() > 4096 {
        return Err(crate::SweepError::InvalidCollisionBounds);
    }
    let mut bindings = std::collections::BTreeMap::new();
    for w in wear {
        if w.removed {
            continue;
        }
        if bindings.insert(w.pos, w).is_some() {
            return Err(crate::SweepError::InvalidCollisionBounds);
        }
        if view.sample(w.pos) != Sample::Loaded(w.block)
            || view
                .chunk(split_voxel(w.pos).0)
                .is_none_or(|c| c.revision != w.revision)
        {
            return Err(crate::SweepError::StaleWearGeometry);
        }
    }
    Ok(bindings)
}

/// Closed surface in normalized voxel-local coordinates, for geometry/render
/// adapters. Winding is outward; no neighboring-face occlusion is applied.
#[derive(Clone, Debug)]
pub struct WearSurface {
    pub vertices: [[f64; 3]; 8],
    pub triangles: [[u8; 3]; 12],
}
impl VoxelWear {
    /// Exact partial-wear surface using the same bounds as collision queries.
    /// Fully exhausted geometry has no surface. Renderer must replace the old
    /// full voxel mesh and handle material/neighbor visibility separately.
    #[must_use]
    pub fn surface(&self) -> Option<WearSurface> {
        if self.removed {
            return None;
        }
        let (min, max) = self.collision_bounds();
        let vertices = std::array::from_fn(|i| {
            std::array::from_fn(|axis| {
                if i & (1 << axis) == 0 {
                    min[axis]
                } else {
                    max[axis]
                }
            })
        });
        Some(WearSurface {
            vertices,
            triangles: [
                [0, 4, 6],
                [0, 6, 2], // -X
                [1, 3, 7],
                [1, 7, 5], // +X
                [0, 1, 5],
                [0, 5, 4], // -Y
                [2, 6, 7],
                [2, 7, 3], // +Y
                [0, 2, 3],
                [0, 3, 1], // -Z
                [4, 5, 7],
                [4, 7, 6], // +Z
            ],
        })
    }
}

#[derive(Clone, Debug)]
pub struct VoxelWearBatch {
    /// Removed bindings with zero loading return None; active entries return reports.
    pub removals: Vec<Option<Removal>>,
    pub receipt: Option<CommitReceipt>,
}
/// Advance a bounded set of wear inventories and atomically remove exhausted
/// blocks in one world transaction. Surviving bindings in changed chunks adopt
/// the receipt's new revision, avoiding false stale errors caused by this batch.
/// All affected wear bindings must be included; unrelated external edits remain stale.
/// # Errors
/// Invalid/duplicate/stale bindings, loading or failed commit; no state changes.
pub fn advance_voxel_wear_batch(
    world: &mut World,
    wear: &mut [VoxelWear],
    loading: &[(f64, f64)],
) -> Result<VoxelWearBatch, &'static str> {
    if wear.len() != loading.len() {
        return Err("wear loading count mismatch");
    }
    validate_wear_bindings(world, wear).map_err(|_| "invalid wear batch bindings")?;
    let mut next = wear.to_vec();
    let mut removals = Vec::with_capacity(wear.len());
    let mut writes = Vec::new();
    let mut expected = BTreeMap::new();
    for (w, &(load, distance)) in next.iter_mut().zip(loading) {
        if !load.is_finite() || load < 0. || !distance.is_finite() || distance < 0. {
            return Err("invalid wear loading");
        }
        if w.removed {
            if load > 0. && distance > 0. {
                return Err("loading removed wear voxel");
            }
            removals.push(None);
            continue;
        }
        let removal = w.layer.advance(w.material, load, distance)?;
        if removal.exhausted {
            writes.push(VoxelWrite {
                pos: w.pos,
                block: BlockStateId::AIR,
            });
            expected.insert(split_voxel(w.pos).0, w.revision);
            w.removed = true;
        }
        removals.push(Some(removal));
    }
    let receipt = if writes.is_empty() {
        None
    } else {
        Some(
            world
                .commit(EditTxn {
                    source: EditSource::Simulation,
                    expected: expected.into_iter().collect(),
                    writes,
                })
                .map_err(|_| "wear batch world commit failed")?,
        )
    };
    if let Some(receipt) = &receipt {
        for w in &mut next {
            if let Some(delta) = receipt
                .chunks
                .iter()
                .find(|d| d.pos == split_voxel(w.pos).0)
            {
                w.revision = delta.after_revision;
            }
        }
    }
    wear.clone_from_slice(&next);
    Ok(VoxelWearBatch { removals, receipt })
}
