//! Deterministic derived voxel lighting with revision-stamped inputs.

mod padded;
mod propagate;
mod types;

pub use propagate::build_light;
pub use types::{
    FaceNeighbor, LightStamp, LightVolume, LightingBudget, LightingError, LightingInput,
    attentuate_tinted_glass_rgb_light, attenuate_filtered_light, bounce_radiosity_injection_level,
};

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, VecDeque};
    use std::sync::Arc;

    use voxy_core::{
        CHUNK_EDGE, CHUNK_VOLUME, CancelToken, LocalIndex, LocalPos, PADDED_EDGE, PADDED_VOLUME,
    };
    use voxy_world::{
        BlockDef, BlockRegistry, BlockStateId, ChunkData, ChunkPos, ChunkRevision, ChunkSnapshot,
        CollisionShape, MaterialId, Occlusion, PalettedBlocks, RenderKind, ResourceKey,
    };

    use crate::propagate::{STRIDE_Y, STRIDE_Z, padded_index, seed_direct_sky, validate};
    use crate::types::MAX_LIGHT;
    use crate::*;

    fn registry() -> Arc<BlockRegistry> {
        Arc::new(
            BlockRegistry::new(vec![
                block("air", RenderKind::Invisible, Occlusion::None, 0, 0),
                block("stone", RenderKind::Opaque, Occlusion::FullCube, 0, 20),
                block("lamp", RenderKind::Opaque, Occlusion::None, 15, 5),
            ])
            .unwrap(),
        )
    }

    fn block(
        name: &str,
        render: RenderKind,
        occlusion: Occlusion,
        emission: u8,
        resistance: u16,
    ) -> BlockDef {
        BlockDef {
            key: ResourceKey::parse(format!("voxy:{name}")).unwrap(),
            render,
            occlusion,
            collision: if occlusion == Occlusion::FullCube {
                CollisionShape::FullCube
            } else {
                CollisionShape::Empty
            },
            face_materials: [MaterialId(0); 6],
            translucent_interface_group: None,
            emission,
            blast_resistance: resistance,
        }
    }

    fn snapshot(pos: ChunkPos, blocks: PalettedBlocks) -> ChunkSnapshot {
        ChunkSnapshot {
            pos,
            revision: ChunkRevision::default(),
            data: Arc::new(ChunkData {
                blocks,
                block_data: BTreeMap::new(),
            }),
        }
    }

    fn input(blocks: PalettedBlocks) -> LightingInput {
        LightingInput {
            center: snapshot(ChunkPos { x: 0, y: 0, z: 0 }, blocks),
            neighbors: std::array::from_fn(|_| None),
            registry: registry(),
            sky_from_above: true,
            stamp: LightStamp {
                center: ChunkRevision::default(),
                faces: [None; 6],
                registry_epoch: 1,
                lighting_epoch: 1,
            },
        }
    }

    #[test]
    fn open_air_receives_full_direct_sky() {
        let light = build_light(
            &input(PalettedBlocks::uniform(BlockStateId::AIR)),
            LightingBudget::default(),
            &CancelToken::new(),
        )
        .unwrap();
        assert!(LocalIndex::all().all(|index| light.sky(index) == 15));
    }

    #[test]
    fn emission_propagates_and_opaque_cells_stop_it() {
        let registry = registry();
        let lamp = registry
            .find(&ResourceKey::parse("voxy:lamp").unwrap())
            .unwrap();
        let stone = registry
            .find(&ResourceKey::parse("voxy:stone").unwrap())
            .unwrap();
        let lamp_pos = LocalPos::new(16, 16, 16).unwrap();
        let stone_pos = LocalPos::new(17, 16, 16).unwrap();
        let blocks = PalettedBlocks::uniform(BlockStateId::AIR)
            .with_updates(&[(lamp_pos.index(), lamp), (stone_pos.index(), stone)])
            .unwrap();
        let light = build_light(
            &input(blocks),
            LightingBudget::default(),
            &CancelToken::new(),
        )
        .unwrap();
        assert_eq!(light.block(lamp_pos.index()), 15);
        assert_eq!(light.block(stone_pos.index()), 0);
        assert_eq!(light.block(LocalPos::new(15, 16, 16).unwrap().index()), 14);
        let mip = light.downsample_2x_max();
        assert_eq!(mip[8 + 16 * (8 + 16 * 8)] & 0x0f, 15);
    }

    #[test]
    fn stamp_budget_and_cancellation_are_fail_closed() {
        let mut invalid = input(PalettedBlocks::uniform(BlockStateId::AIR));
        invalid.stamp.faces[0] = Some(ChunkRevision::default());
        assert_eq!(
            build_light(&invalid, LightingBudget::default(), &CancelToken::new()),
            Err(LightingError::StampMismatch)
        );
        let valid = input(PalettedBlocks::uniform(BlockStateId::AIR));
        assert_eq!(
            build_light(
                &valid,
                LightingBudget {
                    max_propagation_steps: 1
                },
                &CancelToken::new()
            ),
            Err(LightingError::BudgetExceeded)
        );
        let cancel = CancelToken::new();
        cancel.cancel();
        assert_eq!(
            build_light(&valid, LightingBudget::default(), &cancel),
            Err(LightingError::Cancelled)
        );
    }

    /// The previous implementation, kept verbatim as the oracle for exact equivalence:
    /// identical light bytes and identical propagation-budget accounting.
    mod reference {
        use super::*;

        pub fn build_light(
            input: &LightingInput,
            budget: LightingBudget,
            cancel: &CancelToken,
        ) -> Result<LightVolume, LightingError> {
            validate(input, budget)?;
            if cancel.is_cancelled() {
                return Err(LightingError::Cancelled);
            }
            let mut opaque = vec![true; PADDED_VOLUME];
            let mut emission = vec![0_u8; PADDED_VOLUME];
            let mut has_emission = false;
            for y in -1..=CHUNK_EDGE {
                for z in -1..=CHUNK_EDGE {
                    for x in -1..=CHUNK_EDGE {
                        let Some(block) = sample(input, [x, y, z]) else {
                            continue;
                        };
                        let definition = input
                            .registry
                            .get(block)
                            .ok_or(LightingError::UnknownBlock(block.get()))?;
                        let index = padded_index([x, y, z]);
                        opaque[index] = definition.occlusion == Occlusion::FullCube;
                        emission[index] = definition.emission;
                        if definition.emission > 0 {
                            has_emission = true;
                        }
                    }
                }
            }
            let mut sky = vec![0_u8; PADDED_VOLUME];
            if input.sky_from_above {
                seed_direct_sky(&opaque, &mut sky);
            }
            propagate(&opaque, &mut sky, budget, cancel, true)?;
            let mut block = emission;
            if has_emission {
                propagate(&opaque, &mut block, budget, cancel, false)?;
            }
            let mut packed = vec![0_u8; CHUNK_VOLUME];
            for index in LocalIndex::all() {
                let pos = index.position();
                let padded =
                    padded_index([i64::from(pos.x()), i64::from(pos.y()), i64::from(pos.z())]);
                packed[usize::from(index.get())] = (sky[padded] << 4) | block[padded];
            }
            Ok(LightVolume {
                packed: packed.into_boxed_slice(),
                stamp: input.stamp.clone(),
            })
        }

        fn sample(input: &LightingInput, pos: [i64; 3]) -> Option<BlockStateId> {
            let outside = pos.map(|value| !(0..CHUNK_EDGE).contains(&value));
            let outside_count = outside.into_iter().filter(|outside| *outside).count();
            if outside_count > 1 {
                return None;
            }
            let (snapshot, local) = if outside_count == 0 {
                (&input.center, pos)
            } else if pos[0] < 0 {
                (
                    input.neighbors[FaceNeighbor::NegX.index()].as_ref()?,
                    [CHUNK_EDGE - 1, pos[1], pos[2]],
                )
            } else if pos[0] >= CHUNK_EDGE {
                (
                    input.neighbors[FaceNeighbor::PosX.index()].as_ref()?,
                    [0, pos[1], pos[2]],
                )
            } else if pos[1] < 0 {
                (
                    input.neighbors[FaceNeighbor::NegY.index()].as_ref()?,
                    [pos[0], CHUNK_EDGE - 1, pos[2]],
                )
            } else if pos[1] >= CHUNK_EDGE {
                (
                    input.neighbors[FaceNeighbor::PosY.index()].as_ref()?,
                    [pos[0], 0, pos[2]],
                )
            } else if pos[2] < 0 {
                (
                    input.neighbors[FaceNeighbor::NegZ.index()].as_ref()?,
                    [pos[0], pos[1], CHUNK_EDGE - 1],
                )
            } else {
                (
                    input.neighbors[FaceNeighbor::PosZ.index()].as_ref()?,
                    [pos[0], pos[1], 0],
                )
            };
            let local = LocalPos::new(
                u8::try_from(local[0]).ok()?,
                u8::try_from(local[1]).ok()?,
                u8::try_from(local[2]).ok()?,
            )
            .ok()?;
            Some(snapshot.data.blocks.get(local.index()))
        }

        fn propagate(
            opaque: &[bool],
            light: &mut [u8],
            budget: LightingBudget,
            cancel: &CancelToken,
            preserve_direct_down: bool,
        ) -> Result<(), LightingError> {
            let mut queue: VecDeque<_> = light
                .iter()
                .enumerate()
                .filter_map(|(index, &level)| (level > 1).then_some(index))
                .collect();
            if queue.is_empty() {
                return Ok(());
            }
            let mut steps = 0_usize;
            while let Some(index) = queue.pop_front() {
                steps = steps.checked_add(1).ok_or(LightingError::BudgetExceeded)?;
                if steps > budget.max_propagation_steps {
                    return Err(LightingError::BudgetExceeded);
                }
                if steps.is_multiple_of(1024) && cancel.is_cancelled() {
                    return Err(LightingError::Cancelled);
                }
                let current_light = light[index];
                let current_attenuated = current_light.saturating_sub(1);
                let x = index % PADDED_EDGE;
                let yz = index / PADDED_EDGE;
                let z = yz % PADDED_EDGE;
                let y = yz / PADDED_EDGE;
                if y > 0 {
                    let neighbor_index = index - STRIDE_Y;
                    if !opaque[neighbor_index] {
                        let candidate = if preserve_direct_down && current_light == MAX_LIGHT {
                            MAX_LIGHT
                        } else {
                            current_attenuated
                        };
                        if candidate > light[neighbor_index] {
                            light[neighbor_index] = candidate;
                            if candidate > 1 {
                                queue.push_back(neighbor_index);
                            }
                        }
                    }
                }
                if current_attenuated > 0 {
                    let lateral = [
                        (y < PADDED_EDGE - 1, index + STRIDE_Y),
                        (x > 0, index.wrapping_sub(1)),
                        (x < PADDED_EDGE - 1, index + 1),
                        (z > 0, index.wrapping_sub(STRIDE_Z)),
                        (z < PADDED_EDGE - 1, index + STRIDE_Z),
                    ];
                    for (valid, neighbor_index) in lateral {
                        if valid
                            && !opaque[neighbor_index]
                            && current_attenuated > light[neighbor_index]
                        {
                            light[neighbor_index] = current_attenuated;
                            if current_attenuated > 1 {
                                queue.push_back(neighbor_index);
                            }
                        }
                    }
                }
            }
            Ok(())
        }
    }

    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0 >> 33
        }
    }

    /// Height-field terrain with overhangs, caves, lamps and glass for one chunk.
    fn terrain_chunk(seed: u64, registry: &BlockRegistry, pos: ChunkPos) -> ChunkSnapshot {
        let stone = registry
            .find(&ResourceKey::parse("voxy:stone").unwrap())
            .unwrap();
        let lamp = registry
            .find(&ResourceKey::parse("voxy:lamp").unwrap())
            .unwrap();
        let glass = registry
            .find(&ResourceKey::parse("voxy:glass").unwrap())
            .unwrap();
        let mut random = Lcg(seed
            ^ pos.x.cast_unsigned().wrapping_mul(31)
            ^ pos.z.cast_unsigned().wrapping_mul(977));
        let mut dense = vec![BlockStateId::AIR; CHUNK_VOLUME];
        for z in 0..32_usize {
            for x in 0..32_usize {
                let jitter = usize::try_from(random.next() % 7).unwrap();
                let height = 8 + ((x * 3 + z * 5 + jitter) % 20);
                for y in 0..32_usize {
                    let index = x + 32 * (z + 32 * y);
                    let block = if y < height {
                        if random.next().is_multiple_of(23) {
                            BlockStateId::AIR
                        } else if random.next().is_multiple_of(97) {
                            lamp
                        } else {
                            stone
                        }
                    } else if y == height + 3 && random.next().is_multiple_of(5) {
                        stone
                    } else if random.next().is_multiple_of(61) {
                        glass
                    } else {
                        BlockStateId::AIR
                    };
                    dense[index] = block;
                }
            }
        }
        snapshot(pos, PalettedBlocks::from_dense(dense).unwrap())
    }

    fn registry_with_glass() -> Arc<BlockRegistry> {
        Arc::new(
            BlockRegistry::new(vec![
                block("air", RenderKind::Invisible, Occlusion::None, 0, 0),
                block("stone", RenderKind::Opaque, Occlusion::FullCube, 0, 20),
                block("lamp", RenderKind::Opaque, Occlusion::None, 15, 5),
                block("glass", RenderKind::Translucent, Occlusion::None, 0, 3),
            ])
            .unwrap(),
        )
    }

    fn scenario(seed: u64, with_neighbors: bool, sky_from_above: bool) -> LightingInput {
        let registry = registry_with_glass();
        let center = ChunkPos { x: 0, y: 0, z: 0 };
        let neighbors: [Option<ChunkSnapshot>; 6] = if with_neighbors {
            [
                ChunkPos { x: -1, ..center },
                ChunkPos { x: 1, ..center },
                ChunkPos { y: -1, ..center },
                ChunkPos { y: 1, ..center },
                ChunkPos { z: -1, ..center },
                ChunkPos { z: 1, ..center },
            ]
            .map(|pos| Some(terrain_chunk(seed + 1, &registry, pos)))
        } else {
            std::array::from_fn(|_| None)
        };
        let faces = neighbors
            .each_ref()
            .map(|snapshot| snapshot.as_ref().map(|snapshot| snapshot.revision));
        LightingInput {
            center: terrain_chunk(seed, &registry, center),
            neighbors,
            registry,
            sky_from_above,
            stamp: LightStamp {
                center: ChunkRevision::default(),
                faces,
                registry_epoch: 1,
                lighting_epoch: 1,
            },
        }
    }

    fn minimal_budget(
        input: &LightingInput,
        build: fn(
            &LightingInput,
            LightingBudget,
            &CancelToken,
        ) -> Result<LightVolume, LightingError>,
    ) -> usize {
        let (mut low, mut high) = (1_usize, LightingBudget::default().max_propagation_steps);
        assert!(
            build(
                input,
                LightingBudget {
                    max_propagation_steps: high
                },
                &CancelToken::new()
            )
            .is_ok()
        );
        while low < high {
            let middle = low + (high - low) / 2;
            let budget = LightingBudget {
                max_propagation_steps: middle,
            };
            if build(input, budget, &CancelToken::new()).is_ok() {
                high = middle;
            } else {
                low = middle + 1;
            }
        }
        low
    }

    #[test]
    fn matches_reference_bytes_and_budget_on_terrain() {
        let cancel = CancelToken::new();
        for (seed, with_neighbors, sky) in [
            (1, true, true),
            (2, false, true),
            (3, true, false),
            (4, false, false),
            (5, true, true),
        ] {
            let input = scenario(seed, with_neighbors, sky);
            let expected =
                reference::build_light(&input, LightingBudget::default(), &cancel).unwrap();
            let actual = build_light(&input, LightingBudget::default(), &cancel).unwrap();
            assert_eq!(actual, expected, "seed {seed}");
            let budget = minimal_budget(&input, reference::build_light);
            assert_eq!(minimal_budget(&input, build_light), budget, "seed {seed}");
        }
    }

    #[test]
    fn matches_reference_for_uniform_chunks() {
        let registry = registry_with_glass();
        let stone = registry
            .find(&ResourceKey::parse("voxy:stone").unwrap())
            .unwrap();
        let lamp = registry
            .find(&ResourceKey::parse("voxy:lamp").unwrap())
            .unwrap();
        let cancel = CancelToken::new();
        for block in [BlockStateId::AIR, stone, lamp] {
            for with_neighbors in [false, true] {
                let mut input = scenario(9, with_neighbors, true);
                input.center = snapshot(ChunkPos::default(), PalettedBlocks::uniform(block));
                let expected = reference::build_light(&input, LightingBudget::default(), &cancel);
                let actual = build_light(&input, LightingBudget::default(), &cancel);
                assert_eq!(actual, expected);
                let budget = minimal_budget(&input, reference::build_light);
                assert_eq!(minimal_budget(&input, build_light), budget);
            }
        }
        assert_eq!(attenuate_filtered_light(15, 15, 1), 14);
        assert_eq!(attenuate_filtered_light(15, 8, 2), 6);
        assert_eq!(attenuate_filtered_light(15, 0, 1), 0);
        assert_eq!(
            attentuate_tinted_glass_rgb_light([15, 15, 15], [15, 8, 0], 1),
            [14, 7, 0]
        );
        assert_eq!(bounce_radiosity_injection_level(15, 0.8), 6);
        assert_eq!(bounce_radiosity_injection_level(0, 0.8), 0);
    }
}
