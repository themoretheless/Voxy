//! Browser geometry from the authoritative halo-correct voxel mesh.
use voxy_mesher::{FaceDir, QuadDiagonal};
use voxy_render::{SceneMesh, SceneVertex};

pub(crate) fn mesh(scene: &voxy_runtime::BootstrapScene) -> Result<SceneMesh, String> {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for chunk in &scene.chunks {
        // Keep offsets relative to the scene anchor and exactly representable in f32.
        let component = |value: i64, anchor: i64| -> Result<f32, String> {
            let delta = value
                .checked_sub(anchor)
                .ok_or("browser chunk offset overflow")?;
            let delta =
                i16::try_from(delta).map_err(|_| "browser chunk offset outside render range")?;
            Ok(f32::from(delta) * 32.0)
        };
        let chunk_offset = [
            component(chunk.pos.x, scene.anchor.x)?,
            component(chunk.pos.y, scene.anchor.y)?,
            component(chunk.pos.z, scene.anchor.z)?,
        ];
        for quad in chunk
            .mesh
            .opaque
            .iter()
            .chain(&chunk.mesh.cutout)
            .chain(&chunk.mesh.translucent)
        {
            let (u, v) = match quad.face {
                FaceDir::NegX | FaceDir::PosX => (2, 1),
                FaceDir::NegY | FaceDir::PosY => (0, 2),
                FaceDir::NegZ | FaceDir::PosZ => (0, 1),
            };
            let color = match quad.material.0 {
                2 => [0.28, 0.65, 0.20, 1.0],
                3 => [0.46, 0.28, 0.13, 1.0],
                4 => [0.15, 0.48, 0.85, 0.65],
                _ => [0.55, 0.58, 0.62, 1.0],
            };
            let base = u32::try_from(vertices.len()).map_err(|e| e.to_string())?;
            for (corner, [cu, cv]) in [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]
                .into_iter()
                .enumerate()
            {
                let mut position = quad.origin.map(f32::from);
                position[u] += cu * f32::from(quad.extent_u.get());
                position[v] += cv * f32::from(quad.extent_v.get());
                position = std::array::from_fn(|a| {
                    (position[a] + chunk_offset[a] - [16.0, 10.0, 16.0][a]) / 16.0
                });
                let shade = 0.6 + 0.4 * f32::from(quad.ao[corner]) / 3.0;
                vertices.push(SceneVertex {
                    position,
                    uv: [cu, cv],
                    color: [
                        color[0] * shade,
                        color[1] * shade,
                        color[2] * shade,
                        color[3],
                    ],
                });
            }
            let offsets = match quad.diagonal {
                QuadDiagonal::Uv => [0, 1, 2, 0, 2, 3],
                QuadDiagonal::Vu => [0, 1, 3, 1, 2, 3],
            };
            indices.extend(offsets.map(|offset| base + offset));
        }
    }
    SceneMesh::new(vertices, indices).map_err(|e| e.to_string())
}

pub(crate) fn destroy_center(world: &mut voxy_world::World) -> Result<u32, String> {
    use voxy_world::{CollisionShape, Sample, VoxelView};
    let mut center = None;
    for y in (0..32).rev() {
        let pos = voxy_world::VoxelPos { x: 16, y, z: 16 };
        if let Sample::Loaded(block) = world.sample(pos)
            && world
                .registry()
                .get(block)
                .is_some_and(|def| def.collision == CollisionShape::FullCube)
        {
            center = Some(pos);
            break;
        }
    }
    let center = center.ok_or("no destructible center block")?;
    let plan = physics_voxel::plan_explosion(
        world,
        world.registry(),
        voxy_world::EditSource::Player(1),
        physics_voxel::Explosion {
            center,
            radius: 3,
            ..physics_voxel::Explosion::default()
        },
    )
    .map_err(|e| e.to_string())?;
    let physics_voxel::DestructionPlan::Transaction(edit) = plan else {
        return Err("center destruction unavailable".into());
    };
    let receipt = world.commit(edit).map_err(|e| e.to_string())?;
    u32::try_from(receipt.inverse.writes.len()).map_err(|e| e.to_string())
}
