#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use crate::*;
    use voxy_core::{CHUNK_VOLUME, CancelToken, ChunkPos, LocalPos};
    use voxy_world::{
        BlockDef, BlockRegistry, BlockStateId, ChunkData, ChunkRevision, ChunkSnapshot,
        CollisionShape, InterfaceGroupId, MaterialId, Occlusion, PalettedBlocks, RenderKind,
        ResourceKey, Sample,
    };

    fn block(
        name: &str,
        render: RenderKind,
        occlusion: Occlusion,
        group: Option<u16>,
        material: u16,
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
            face_materials: [MaterialId(material); 6],
            translucent_interface_group: group.map(InterfaceGroupId),
            emission: 0,
            blast_resistance: if render == RenderKind::Invisible {
                0
            } else {
                20
            },
        }
    }

    fn registry() -> Arc<BlockRegistry> {
        Arc::new(
            BlockRegistry::new(vec![
                block("air", RenderKind::Invisible, Occlusion::None, None, 0),
                block("stone", RenderKind::Opaque, Occlusion::FullCube, None, 1),
                block(
                    "glass",
                    RenderKind::Translucent,
                    Occlusion::None,
                    Some(1),
                    2,
                ),
                block(
                    "water",
                    RenderKind::Translucent,
                    Occlusion::None,
                    Some(2),
                    3,
                ),
            ])
            .unwrap(),
        )
    }

    fn state(registry: &BlockRegistry, name: &str) -> BlockStateId {
        registry
            .find(&ResourceKey::parse(format!("voxy:{name}")).unwrap())
            .unwrap()
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

    fn input(registry: Arc<BlockRegistry>, blocks: PalettedBlocks) -> MeshingInput {
        MeshingInput {
            center: snapshot(ChunkPos { x: 0, y: 0, z: 0 }, blocks),
            neighbors: std::array::from_fn(|_| None),
            registry,
            stamp: MeshStamp {
                center: ChunkRevision::default(),
                halo: [None; 26],
                registry_epoch: 0,
                mesher_epoch: 0,
            },
        }
    }

    fn face_areas(mesh: &ChunkMesh) -> [u32; 6] {
        let mut areas = [0_u32; 6];
        for quad in mesh
            .opaque
            .iter()
            .chain(mesh.cutout.iter())
            .chain(mesh.translucent.iter())
        {
            let index = FaceDir::ALL
                .iter()
                .position(|face| *face == quad.face)
                .unwrap();
            areas[index] += u32::from(quad.area());
        }
        areas
    }

    #[test]
    fn halo_order_is_unique_and_canonical() {
        for (index, offset) in HALO_OFFSETS.into_iter().enumerate() {
            assert_eq!(offset.index(), Some(index));
            assert_ne!((offset.dx, offset.dy, offset.dz), (0, 0, 0));
        }
    }

    #[test]
    fn greedy_matches_naive_coverage_and_merges_full_chunk() {
        let registry = registry();
        let stone = state(&registry, "stone");
        let input = input(registry, PalettedBlocks::uniform(stone));
        let naive = build_naive_mesh(&input, &CancelToken::new()).unwrap();
        let greedy = build_mesh(&input, &CancelToken::new()).unwrap();
        assert_eq!(face_areas(&naive), face_areas(&greedy));
        assert_eq!(naive.quad_count(), 6 * 32 * 32);
        assert_eq!(greedy.quad_count(), 6);
        assert_eq!(face_areas(&greedy), [1024; 6]);
    }

    #[test]
    fn one_block_and_empty_chunk_have_canonical_geometry() {
        let registry = registry();
        let stone = state(&registry, "stone");
        let mut dense = vec![BlockStateId::AIR; CHUNK_VOLUME];
        dense[usize::from(LocalPos::new(3, 4, 5).unwrap().index().get())] = stone;
        let one = input(
            Arc::clone(&registry),
            PalettedBlocks::from_dense(dense).unwrap(),
        );
        let one_mesh = build_mesh(&one, &CancelToken::new()).unwrap();
        assert_eq!(one_mesh.quad_count(), 6);
        assert_eq!(
            one_mesh.bounds,
            LocalAabb {
                min: [3, 4, 5],
                max: [4, 5, 6]
            }
        );
        let empty = input(registry, PalettedBlocks::uniform(BlockStateId::AIR));
        let mesh = build_mesh(&empty, &CancelToken::new()).unwrap();
        assert_eq!(mesh.quad_count(), 0);
        assert_eq!(mesh.bounds, LocalAabb::EMPTY);
    }

    #[test]
    fn classifier_obeys_occlusion_and_translucent_groups() {
        let registry = registry();
        let stone = state(&registry, "stone");
        let glass = state(&registry, "glass");
        let water = state(&registry, "water");
        assert_eq!(
            classify_directed_face(glass, &Sample::Loaded(glass), FaceDir::PosX, &registry),
            FaceDecision::Skip
        );
        assert!(matches!(
            classify_directed_face(glass, &Sample::Loaded(water), FaceDir::PosX, &registry),
            FaceDecision::Emit { .. }
        ));
        assert_eq!(
            classify_directed_face(glass, &Sample::Loaded(stone), FaceDir::PosX, &registry),
            FaceDecision::Skip
        );
        assert!(matches!(
            classify_directed_face(
                stone,
                &Sample::Unloaded {
                    chunk: ChunkPos { x: 1, y: 0, z: 0 }
                },
                FaceDir::PosX,
                &registry
            ),
            FaceDecision::Emit { .. }
        ));
    }

    #[test]
    fn loaded_halo_suppresses_shared_boundary() {
        let registry = registry();
        let stone = state(&registry, "stone");
        let mut input = input(Arc::clone(&registry), PalettedBlocks::uniform(stone));
        let offset = HaloOffset {
            dx: 1,
            dy: 0,
            dz: 0,
        };
        input.neighbors[offset.index().unwrap()] = Some(snapshot(
            ChunkPos { x: 1, y: 0, z: 0 },
            PalettedBlocks::uniform(stone),
        ));
        let mesh = build_mesh(&input, &CancelToken::new()).unwrap();
        assert_eq!(mesh.quad_count(), 5);
        assert_eq!(face_areas(&mesh), [1024, 0, 1024, 1024, 1024, 1024]);
    }

    #[test]
    fn cancellation_stops_both_meshers() {
        let token = CancelToken::new();
        token.cancel();
        let input = input(registry(), PalettedBlocks::uniform(BlockStateId::AIR));
        assert_eq!(build_naive_mesh(&input, &token), Err(MeshError::Cancelled));
        assert_eq!(build_mesh(&input, &token), Err(MeshError::Cancelled));
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

    /// Terrain-like chunk: rock below a jittered height, pockets, lamps (opaque but not
    /// occluding), glass and water layers above, and a uniform-rock variant.
    fn terrain_blocks(seed: u64, registry: &BlockRegistry, rock_only: bool) -> PalettedBlocks {
        let stone = state(registry, "stone");
        let glass = state(registry, "glass");
        let water = state(registry, "water");
        let lamp = state(registry, "lamp");
        if rock_only {
            return PalettedBlocks::uniform(stone);
        }
        let mut random = Lcg(seed);
        let mut dense = vec![BlockStateId::AIR; CHUNK_VOLUME];
        for z in 0..32_usize {
            for x in 0..32_usize {
                let jitter = usize::try_from(random.next() % 5).unwrap();
                let height = 6 + ((x * 2 + z * 3 + jitter) % 18);
                for y in 0..32_usize {
                    let index = x + 32 * (z + 32 * y);
                    dense[index] = if y < height {
                        match random.next() % 29 {
                            0 => BlockStateId::AIR,
                            1 => lamp,
                            _ => stone,
                        }
                    } else if y < height + 2 && random.next().is_multiple_of(3) {
                        water
                    } else if random.next().is_multiple_of(41) {
                        glass
                    } else {
                        BlockStateId::AIR
                    };
                }
            }
        }
        PalettedBlocks::from_dense(dense).unwrap()
    }

    #[test]
    fn random_terrain_with_halos_matches_naive() {
        let registry = Arc::new(
            BlockRegistry::new(vec![
                block("air", RenderKind::Invisible, Occlusion::None, None, 0),
                block("stone", RenderKind::Opaque, Occlusion::FullCube, None, 1),
                block(
                    "glass",
                    RenderKind::Translucent,
                    Occlusion::None,
                    Some(1),
                    2,
                ),
                block(
                    "water",
                    RenderKind::Translucent,
                    Occlusion::None,
                    Some(2),
                    3,
                ),
                block("lamp", RenderKind::Opaque, Occlusion::None, None, 4),
            ])
            .unwrap(),
        );
        for (seed, rock_only, with_halo) in [
            (1, false, true),
            (2, false, false),
            (3, true, true),
            (4, true, false),
            (5, false, true),
        ] {
            let mut input = input(
                Arc::clone(&registry),
                terrain_blocks(seed, &registry, rock_only),
            );
            if with_halo {
                for (index, offset) in HALO_OFFSETS.into_iter().enumerate() {
                    if (index + usize::try_from(seed).unwrap()) % 3 == 0 {
                        continue;
                    }
                    input.neighbors[index] = Some(snapshot(
                        ChunkPos {
                            x: i64::from(offset.dx),
                            y: i64::from(offset.dy),
                            z: i64::from(offset.dz),
                        },
                        terrain_blocks(seed + 10 + index as u64, &registry, index % 4 == 0),
                    ));
                }
            }
            let naive = build_naive_mesh(&input, &CancelToken::new()).unwrap();
            let greedy = build_mesh(&input, &CancelToken::new()).unwrap();
            assert_eq!(face_areas(&naive), face_areas(&greedy), "seed {seed}");
            assert_eq!(naive.bounds, greedy.bounds, "seed {seed}");
            assert_eq!(covered_cells(&naive), covered_cells(&greedy), "seed {seed}");
        }
    }

    /// Every unit face with its material and layer, independent of quad merging.
    fn covered_cells(mesh: &ChunkMesh) -> BTreeMap<([u8; 3], FaceDir), (MaterialId, RenderLayer)> {
        let mut cells = BTreeMap::new();
        for quad in mesh
            .opaque
            .iter()
            .chain(mesh.cutout.iter())
            .chain(mesh.translucent.iter())
        {
            let (_, u_axis, v_axis) = quad.face.axes();
            for du in 0..quad.extent_u.get() {
                for dv in 0..quad.extent_v.get() {
                    let mut cell = quad.origin;
                    cell[u_axis] += du;
                    cell[v_axis] += dv;
                    let previous = cells.insert((cell, quad.face), (quad.material, quad.layer));
                    assert!(previous.is_none(), "overlapping quads at {cell:?}");
                }
            }
        }
        cells
    }

    #[test]
    fn complex_terrain_greedy_matches_naive() {
        let registry = registry();
        let stone = state(&registry, "stone");
        let glass = state(&registry, "glass");
        let water = state(&registry, "water");

        let mut dense = vec![BlockStateId::AIR; CHUNK_VOLUME];
        for y in 0..16 {
            for z in 0..32 {
                for x in 0..32 {
                    let idx = x + 32 * (z + 32 * y);
                    if y < 8 {
                        dense[idx] = stone;
                    } else if (x + z) % 3 == 0 {
                        dense[idx] = glass;
                    } else if (x * z) % 7 == 0 {
                        dense[idx] = water;
                    }
                }
            }
        }
        let input = input(
            Arc::clone(&registry),
            PalettedBlocks::from_dense(dense).unwrap(),
        );
        let naive = build_naive_mesh(&input, &CancelToken::new()).unwrap();
        let greedy = build_mesh(&input, &CancelToken::new()).unwrap();
        assert_eq!(face_areas(&naive), face_areas(&greedy));
        assert!(greedy.quad_count() < naive.quad_count());
        let packed = greedy.to_packed_64();
        assert_eq!(packed.quad_count(), greedy.quad_count());
        assert_eq!(packed.bounds, greedy.bounds);
        for &quad in greedy
            .opaque
            .iter()
            .chain(greedy.cutout.iter())
            .chain(greedy.translucent.iter())
        {
            assert_eq!(quad.pack_64().unpack(), Some(quad));
        }
        assert_eq!(select_ao_quad_diagonal([3, 3, 3, 3]), QuadDiagonal::Uv);
        assert_eq!(select_ao_quad_diagonal([1, 3, 3, 1]), QuadDiagonal::Vu);

        let vis = ChunkVisibilityGraph6::OPAQUE
            .with_connection(FaceDir::NegX, FaceDir::PosX)
            .with_connection(FaceDir::NegY, FaceDir::PosZ);
        assert!(vis.can_see_through(FaceDir::PosX, FaceDir::NegX));
        assert!(vis.can_see_through(FaceDir::NegY, FaceDir::PosZ));
        assert!(!vis.can_see_through(FaceDir::NegX, FaceDir::PosY));
        assert_eq!(vis.connected_pair_count(), 2);

        let n = [0.577_350_26_f32, -0.577_350_26, 0.577_350_26];
        let packed_n = pack_octahedral_normal_snorm16(n);
        let unpacked_n = unpack_octahedral_normal_snorm16(packed_n);
        let dot = n[0] * unpacked_n[0] + n[1] * unpacked_n[1] + n[2] * unpacked_n[2];
        assert!(dot > 0.9999);
        assert!(quad_diagonal_flip_for_ao(1, 3, 1, 3));
        assert!(!quad_diagonal_flip_for_ao(3, 1, 3, 1));
    }
}
