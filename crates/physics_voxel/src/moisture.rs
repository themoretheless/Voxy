//! Quantized world-liquid withdrawal into conservative continuous moisture sources.
use physics::{
    moisture::{Body, Calibration, CohesiveCalibration, SupplyTransfer, WaterSupply},
    plasticity::mesh::{FiniteQuadraticDynamics, QuadraticCohesiveWetUpdate, QuadraticWetUpdate},
};
use voxy_core::{VoxelPos, split_voxel};
use voxy_world::{
    BlockRegistry, CommitReceipt, EditSource, EditTxn, Sample, VoxelView, VoxelWrite, World,
};
#[derive(Clone, Copy, Debug)]
pub struct VoxelMoistureContact {
    pub pos: VoxelPos,
    /// Whole eighth-voxel units withdrawn; excess remains in the returned source.
    pub units: u8,
    pub side_m: f64,
    pub liquid_density_kg_m3: f64,
    pub cell: usize,
    pub conductance_kg_s: f64,
}
#[must_use = "retain the remaining water source to conserve liquid mass"]
#[derive(Clone, Debug)]
pub struct VoxelMoistureStep {
    pub receipt: CommitReceipt,
    /// Unabsorbed withdrawn water remains here; caller must retain this inventory.
    /// It is not still present in the voxel and must not be counted twice.
    pub remaining_source: WaterSupply,
    pub transfer: SupplyTransfer,
    pub bulk: QuadraticWetUpdate,
    pub cohesive: QuadraticCohesiveWetUpdate,
}
/// Atomically withdraw quantized liquid and apply finite-source wet mechanics.
/// Surplus water is a returned continuous inventory, not silently discarded.
/// # Errors
/// Invalid/missing liquid, inventories, mechanics or world commit; all three
/// live states remain unchanged. Routing/contact discovery remains caller-owned.
#[allow(clippy::too_many_arguments)]
pub fn advance_voxel_moisture(
    world: &mut World,
    registry: &BlockRegistry,
    states: crate::LiquidStates,
    contact: VoxelMoistureContact,
    dt_s: f64,
    solid: &mut FiniteQuadraticDynamics,
    water: &mut Body,
    dry_mass_kg: &[f64],
    bulk_laws: &[Calibration],
    incoming_velocity: &[[f64; 3]],
    face_laws: &[CohesiveCalibration],
    minus_weights: &[f64],
) -> Result<VoxelMoistureStep, &'static str> {
    states
        .validate(registry)
        .map_err(|_| "invalid source liquid states")?;
    if contact.units == 0
        || contact.units > 8
        || !contact.side_m.is_finite()
        || contact.side_m <= 0.
        || !contact.liquid_density_kg_m3.is_finite()
        || contact.liquid_density_kg_m3 <= 0.
    {
        return Err("invalid voxel moisture source geometry");
    }
    let Sample::Loaded(block) = world.sample(contact.pos) else {
        return Err("voxel moisture source unavailable");
    };
    let level = states.level(block).ok_or("voxel is not source liquid")?;
    if contact.units > level {
        return Err("insufficient voxel liquid inventory");
    }
    let mass =
        contact.side_m.powi(3) * contact.liquid_density_kg_m3 * f64::from(contact.units) / 8.;
    if !mass.is_finite() || mass <= 0. {
        return Err("unrepresentable voxel source mass");
    }
    let chunk = split_voxel(contact.pos).0;
    let revision = world
        .chunk(chunk)
        .ok_or("voxel moisture chunk unavailable")?
        .revision;
    let mut next = solid.clone();
    let mut next_water = water.clone();
    let mut supplies = [WaterSupply {
        cell: contact.cell,
        water_kg: mass,
        conductance_kg_s: contact.conductance_kg_s,
    }];
    let (transfer, bulk, cohesive) = next.advance_moisture_supplies_with_cohesion(
        dt_s,
        &mut next_water,
        &mut supplies,
        dry_mass_kg,
        bulk_laws,
        incoming_velocity,
        face_laws,
        minus_weights,
    )?;
    let receipt = world
        .commit(EditTxn {
            source: EditSource::Simulation,
            expected: vec![(chunk, revision)],
            writes: vec![VoxelWrite {
                pos: contact.pos,
                block: states.state(level - contact.units),
            }],
        })
        .map_err(|_| "voxel moisture world commit failed")?;
    *solid = next;
    *water = next_water;
    Ok(VoxelMoistureStep {
        receipt,
        remaining_source: supplies[0],
        transfer,
        bulk,
        cohesive,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::BTreeMap, sync::Arc};
    use voxy_core::{ChunkPos, WorldEpoch};
    use voxy_world::*;
    #[test]
    fn world_liquid_wet_mass_and_surplus_conserve_inventory_with_rollback() {
        let base = crate::test_support::test_registry();
        let mut definitions = vec![base.get(BlockStateId::AIR).unwrap().clone()];
        for i in 1..=8 {
            definitions.push(BlockDef {
                key: ResourceKey::parse(format!("voxy:water_{i}")).unwrap(),
                render: RenderKind::Translucent,
                occlusion: Occlusion::None,
                collision: CollisionShape::Empty,
                face_materials: [MaterialId(0); 6],
                translucent_interface_group: None,
                emission: 0,
                blast_resistance: 0,
            });
        }
        let registry = Arc::new(BlockRegistry::new(definitions).unwrap());
        let states = crate::WaterStates(std::array::from_fn(|i| {
            registry
                .find(&ResourceKey::parse(format!("voxy:water_{}", i + 1)).unwrap())
                .unwrap()
        }));
        let mut world = World::new(
            WorldEpoch::new(1).unwrap(),
            Arc::clone(&registry),
            WorldLimits::default(),
        );
        world
            .insert_generated(GeneratedChunk {
                pos: ChunkPos { x: 0, y: 0, z: 0 },
                data: ChunkData {
                    blocks: PalettedBlocks::uniform(states.0[7]),
                    block_data: BTreeMap::new(),
                },
            })
            .unwrap();
        let mesh = physics::plasticity::mesh::QuadraticBody::from_linear(
            vec![[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            vec![(
                [0, 1, 2, 3],
                physics::plasticity::Material::new(1e5, 0.3, 1e9, 0.).unwrap(),
            )],
        )
        .unwrap();
        let mut solid = FiniteQuadraticDynamics::new(
            mesh,
            vec![physics::biomechanics::Material::from_young_poisson(1e5, 0.3).unwrap()],
            &[6.],
            vec![[0.; 3]; 10],
            &[false; 10],
        )
        .unwrap();
        let mut water = Body::new(
            vec![physics::moisture::Cell {
                capacity_kg: 0.1,
                water_kg: 0.,
            }],
            vec![],
        )
        .unwrap();
        let p = physics::moisture::Properties {
            young_pa: 1e5,
            poisson: 0.3,
            yield_pa: 1e9,
            hardening_pa: 0.,
            hardness_pa: 1e8,
            wear_coefficient: 1e-3,
        };
        let laws = [Calibration::new(p, p).unwrap()];
        let contact = VoxelMoistureContact {
            pos: VoxelPos { x: 8, y: 8, z: 8 },
            units: 1,
            side_m: 1.,
            liquid_density_kg_m3: 1.,
            cell: 0,
            conductance_kg_s: 1.,
        };
        assert!(
            advance_voxel_moisture(
                &mut world,
                &registry,
                states,
                contact,
                1.,
                &mut solid,
                &mut water,
                &[1.],
                &laws,
                &[],
                &[],
                &[]
            )
            .is_err()
        );
        assert_eq!(world.sample(contact.pos), Sample::Loaded(states.0[7]));
        assert_eq!(water.cells()[0].water_kg, 0.);
        assert_eq!(solid.energy().unwrap().mass_kg, 1.);
        let mut limited_world = World::new(
            WorldEpoch::new(2).unwrap(),
            Arc::clone(&registry),
            WorldLimits {
                max_writes_per_transaction: 0,
                max_chunks_per_transaction: 1,
            },
        );
        limited_world
            .insert_generated(GeneratedChunk {
                pos: ChunkPos { x: 0, y: 0, z: 0 },
                data: ChunkData {
                    blocks: PalettedBlocks::uniform(states.0[7]),
                    block_data: BTreeMap::new(),
                },
            })
            .unwrap();
        assert!(
            advance_voxel_moisture(
                &mut limited_world,
                &registry,
                states,
                contact,
                1.,
                &mut solid,
                &mut water,
                &[1.],
                &laws,
                &[[0.; 3]; 10],
                &[],
                &[]
            )
            .is_err()
        );
        assert_eq!(
            limited_world.sample(contact.pos),
            Sample::Loaded(states.0[7])
        );
        assert_eq!(water.cells()[0].water_kg, 0.);
        assert_eq!(solid.energy().unwrap().mass_kg, 1.);
        let r = advance_voxel_moisture(
            &mut world,
            &registry,
            states,
            contact,
            1.,
            &mut solid,
            &mut water,
            &[1.],
            &laws,
            &[[0.; 3]; 10],
            &[],
            &[],
        )
        .unwrap();
        assert_eq!(world.sample(contact.pos), Sample::Loaded(states.0[6]));
        assert!((water.cells()[0].water_kg - 1. / 11.).abs() < 1e-14);
        assert!(
            (7. / 8. + water.cells()[0].water_kg + r.remaining_source.water_kg - 1.).abs() < 1e-14
        );
        assert!((solid.energy().unwrap().mass_kg - 1. - water.cells()[0].water_kg).abs() < 1e-12);
        assert_eq!(r.receipt.inverse.writes[0].block, states.0[7]);
        let mut bank = VoxelMoistureBank::new(r.remaining_source).unwrap();
        let bank_before = bank.water_kg();
        assert!(
            bank.advance(1., &mut solid, &mut water, &[1.], &laws, &[], &[], &[])
                .is_err()
        );
        assert_eq!(bank.water_kg(), bank_before);
        let (transfer, _, _) = bank
            .advance(
                1.,
                &mut solid,
                &mut water,
                &[1.],
                &laws,
                &[[0.; 3]; 10],
                &[],
                &[],
            )
            .unwrap();
        assert!((water.cells()[0].water_kg - 12. / 121.).abs() < 1e-14);
        assert!((bank_before - bank.water_kg() - transfer.supplied_water_kg[0]).abs() < 1e-14);
        assert!((7. / 8. + water.cells()[0].water_kg + bank.water_kg() - 1.).abs() < 1e-14);
        let bank_before = bank.water_kg();
        assert_eq!(
            bank.return_to_world(&mut world, &registry, states, contact.pos, 1., 1.)
                .unwrap()
                .returned_units,
            0
        );
        assert_eq!(bank.water_kg(), bank_before);
        let mut surplus = bank.source;

        let initial = surplus.water_kg;
        let returned = return_liquid_surplus(
            &mut world,
            &registry,
            states,
            contact.pos,
            1.,
            1.,
            &mut surplus,
        )
        .unwrap();
        assert_eq!(returned.returned_units, 0);
        assert_eq!(surplus.water_kg, initial);
        // A larger detached bank can refill the one available world unit;
        // capacity caps prevent overfilling, with excess still owned by the bank.
        surplus.water_kg = 0.3;
        let empty_pos = VoxelPos { x: 40, y: 8, z: 8 };
        limited_world
            .insert_generated(GeneratedChunk {
                pos: split_voxel(empty_pos).0,
                data: ChunkData {
                    blocks: PalettedBlocks::uniform(BlockStateId::AIR),
                    block_data: BTreeMap::new(),
                },
            })
            .unwrap();
        assert!(
            return_liquid_surplus(
                &mut limited_world,
                &registry,
                states,
                empty_pos,
                1.,
                1.,
                &mut surplus
            )
            .is_err()
        );
        assert_eq!(surplus.water_kg, 0.3);
        assert_eq!(
            limited_world.sample(empty_pos),
            Sample::Loaded(BlockStateId::AIR)
        );

        let returned = return_liquid_surplus(
            &mut world,
            &registry,
            states,
            contact.pos,
            1.,
            1.,
            &mut surplus,
        )
        .unwrap();
        assert_eq!(returned.returned_units, 1);
        assert_eq!(world.sample(contact.pos), Sample::Loaded(states.0[7]));
        assert!((surplus.water_kg + returned.returned_mass_kg - 0.3).abs() < 1e-14);
        let retained = surplus.water_kg;
        assert_eq!(
            return_liquid_surplus(
                &mut world,
                &registry,
                states,
                contact.pos,
                1.,
                1.,
                &mut surplus
            )
            .unwrap()
            .returned_units,
            0
        );
        assert_eq!(surplus.water_kg, retained);
    }
}

#[derive(Clone, Debug)]
pub struct LiquidSurplusReturn {
    pub returned_units: u8,
    pub returned_mass_kg: f64,
    pub receipt: Option<CommitReceipt>,
}
/// Return whole eighth-voxel units from a continuous surplus inventory. Fractional
/// water remains in the source, and a full voxel accepts no additional mass.
/// Caller must use the same SI voxel scale and liquid density as withdrawal.
/// # Errors
/// Invalid inventory/scale, incompatible or missing voxel, precision loss or
/// failed world commit. Source inventory only changes after successful commit.
#[allow(clippy::too_many_arguments)]
pub fn return_liquid_surplus(
    world: &mut World,
    registry: &BlockRegistry,
    states: crate::LiquidStates,
    pos: VoxelPos,
    side_m: f64,
    density_kg_m3: f64,
    source: &mut WaterSupply,
) -> Result<LiquidSurplusReturn, &'static str> {
    states
        .validate(registry)
        .map_err(|_| "invalid source liquid states")?;
    if !side_m.is_finite()
        || side_m <= 0.
        || !density_kg_m3.is_finite()
        || density_kg_m3 <= 0.
        || !source.water_kg.is_finite()
        || source.water_kg < 0.
    {
        return Err("invalid liquid surplus inventory or scale");
    }
    let unit_mass = side_m.powi(3) * density_kg_m3 / 8.;
    if !unit_mass.is_finite() || unit_mass <= 0. || !(unit_mass * 8.).is_finite() {
        return Err("unrepresentable liquid unit mass");
    }
    let Sample::Loaded(block) = world.sample(pos) else {
        return Err("liquid return voxel unavailable");
    };
    let level = if block == voxy_world::BlockStateId::AIR {
        0
    } else {
        states
            .level(block)
            .ok_or("liquid return voxel incompatible")?
    };
    let slots = 8 - level;
    let mut units = if source.water_kg >= unit_mass * f64::from(slots) {
        slots
    } else {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let units = (source.water_kg / unit_mass).floor() as u8;
        units
    };
    // A rounded quotient must never return more mass than actually held.
    while units > 0 && unit_mass * f64::from(units) > source.water_kg {
        units -= 1;
    }
    if units == 0 {
        return Ok(LiquidSurplusReturn {
            returned_units: 0,
            returned_mass_kg: 0.,
            receipt: None,
        });
    }
    let mass = unit_mass * f64::from(units);
    let remainder = source.water_kg - mass;
    if !remainder.is_finite() || remainder < 0. || remainder >= source.water_kg {
        return Err("unrepresentable liquid surplus decrement");
    }
    let chunk = split_voxel(pos).0;
    let revision = world
        .chunk(chunk)
        .ok_or("liquid return chunk unavailable")?
        .revision;
    let receipt = world
        .commit(EditTxn {
            source: EditSource::Simulation,
            expected: vec![(chunk, revision)],
            writes: vec![VoxelWrite {
                pos,
                block: states.state(level + units),
            }],
        })
        .map_err(|_| "liquid surplus world commit failed")?;
    source.water_kg = remainder;
    Ok(LiquidSurplusReturn {
        returned_units: units,
        returned_mass_kg: mass,
        receipt: Some(receipt),
    })
}

/// Owned continuous water remainder between simulation ticks. Inventory is
/// private so absorption/return operations cannot silently overwrite its mass.
#[must_use = "retain the water bank between simulation ticks"]
#[derive(Clone, Debug)]
pub struct VoxelMoistureBank {
    source: WaterSupply,
}
impl VoxelMoistureBank {
    /// # Errors
    /// Invalid mass or conductance. Cell index is validated against the target
    /// network when advancing; retargeting is explicit and preserves mass.
    pub fn new(source: WaterSupply) -> Result<Self, &'static str> {
        if !source.water_kg.is_finite()
            || source.water_kg < 0.
            || !source.conductance_kg_s.is_finite()
            || source.conductance_kg_s < 0.
        {
            return Err("invalid moisture bank inventory");
        }
        Ok(Self { source })
    }
    #[must_use]
    pub fn water_kg(&self) -> f64 {
        self.source.water_kg
    }
    pub fn retarget(&mut self, cell: usize) {
        self.source.cell = cell;
    }
    /// Absorb this bank into the solid without another world-liquid withdrawal.
    /// # Errors
    /// Any moisture/mechanical failure preserves bank, network and solid.
    #[allow(clippy::too_many_arguments)]
    pub fn advance(
        &mut self,
        dt_s: f64,
        solid: &mut FiniteQuadraticDynamics,
        water: &mut Body,
        dry_mass_kg: &[f64],
        bulk_laws: &[Calibration],
        incoming_velocity: &[[f64; 3]],
        face_laws: &[CohesiveCalibration],
        minus_weights: &[f64],
    ) -> Result<
        (
            SupplyTransfer,
            QuadraticWetUpdate,
            QuadraticCohesiveWetUpdate,
        ),
        &'static str,
    > {
        solid.advance_moisture_supplies_with_cohesion(
            dt_s,
            water,
            std::slice::from_mut(&mut self.source),
            dry_mass_kg,
            bulk_laws,
            incoming_velocity,
            face_laws,
            minus_weights,
        )
    }
    /// # Errors
    /// Same validation/transaction errors as `return_liquid_surplus`; bank only
    /// decrements after a successful world edit.
    pub fn return_to_world(
        &mut self,
        world: &mut World,
        registry: &BlockRegistry,
        states: crate::LiquidStates,
        pos: VoxelPos,
        side_m: f64,
        density_kg_m3: f64,
    ) -> Result<LiquidSurplusReturn, &'static str> {
        return_liquid_surplus(
            world,
            registry,
            states,
            pos,
            side_m,
            density_kg_m3,
            &mut self.source,
        )
    }
}
