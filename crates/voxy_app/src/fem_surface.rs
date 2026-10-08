//! Bridge current FEM surface snapshots into the existing scene renderer.
use physics::plasticity::mesh::QuadraticBody;
use voxy_render::{SceneMesh, SceneVertex};
/// Build a scene mesh from current exposed FEM geometry. color receives accepted
/// component ID and analytic current normal; returns linear straight-alpha RGBA.
/// origin/scale are display coordinates only and do not modify physics state.
pub fn fem_surface_scene_mesh(
    body: &QuadraticBody,
    subdivision_depth: u8,
    max_triangles: usize,
    origin_m: [f64; 3],
    display_scale: f64,
    mut color: impl FnMut(usize, [f64; 3]) -> [f32; 4],
) -> Result<SceneMesh, Box<dyn std::error::Error>> {
    if origin_m.iter().any(|x| !x.is_finite()) || !display_scale.is_finite() || display_scale <= 0.
    {
        return Err("invalid FEM display transform".into());
    }
    let triangles = body.surface_triangles(subdivision_depth, max_triangles)?;
    let count = triangles
        .len()
        .checked_mul(3)
        .ok_or("FEM scene vertex count overflow")?;
    if count > u32::MAX as usize {
        return Err("FEM scene index limit".into());
    }
    let mut vertices = Vec::with_capacity(count);
    let mut indices = Vec::with_capacity(count);
    for triangle in triangles {
        for vertex in triangle.vertices {
            indices.push(vertices.len() as u32);
            vertices.push(SceneVertex {
                position: std::array::from_fn(|a| {
                    ((vertex.position_m[a] - origin_m[a]) * display_scale) as f32
                }),
                uv: [vertex.barycentric[1] as f32, vertex.barycentric[2] as f32],
                color: color(triangle.component, vertex.normal),
            });
        }
    }
    Ok(SceneMesh::new(vertices, indices)?)
}

/// Render the current exposed surface of a finite-deformation tissue body.
/// The display transform never mutates solver coordinates or velocities.
pub fn tissue_surface_scene_mesh(
    body: &physics::biomechanics::Body,
    origin_m: [f64; 3],
    display_scale: f64,
    color: [f32; 4],
) -> Result<SceneMesh, Box<dyn std::error::Error>> {
    if origin_m.iter().any(|x| !x.is_finite()) || !display_scale.is_finite() || display_scale <= 0. {
        return Err("invalid tissue display transform".into());
    }
    if body.positions().len() > u32::MAX as usize {
        return Err("tissue scene index limit".into());
    }
    let vertices = body.positions().iter().map(|p| SceneVertex {
        position: std::array::from_fn(|axis| ((p[axis]-origin_m[axis])*display_scale) as f32),
        uv: [0.; 2], color,
    }).collect();
    let indices = body.surface().into_iter().flatten().map(|i| i as u32).collect();
    // Metre-valued reference coordinates bind procedural material fields to
    // physical tissue, independent of its current deformation/display scale.
    let material_coordinates = body.rest_positions().iter().map(|p| p.map(|x| x as f32)).collect();
    Ok(SceneMesh::new(vertices, indices)?.with_material_coordinates(material_coordinates)?)
}

#[cfg(test)]
mod tissue_surface_tests {
    use super::*;
    use physics::biomechanics::{InertialBody, Material, TesticularGeometry};
    #[test]
    fn current_layered_tissue_surface_tracks_solver_pose_without_mutating_it() {
        let geometry = TesticularGeometry {centers_m:[[-0.02,0.,0.],[0.02,0.,0.]],
            radii_m:[[0.015,0.02,0.025];2],sectors:8,rings:3};
        let material = Material {shear_pa:1000.,bulk_pa:100_000.,fibers:vec![]};
        let [body,_] = geometry.build_layered([0.8;2],
            [material.clone(),material.clone()],[material.clone(),material]).unwrap();
        let cell_count = body.elements().len();
        let vertex_count = body.positions().len();
        let mut dynamic = InertialBody::new(body,&vec![1000.;cell_count],vec![[0.;3];vertex_count]).unwrap();
        dynamic.set_uniform_acceleration([0.,-9.81,0.]).unwrap();
        dynamic.step(0.001,1e-8).unwrap();
        let positions = dynamic.body().positions().to_vec();
        let velocities = dynamic.velocities().to_vec();
        let mesh = tissue_surface_scene_mesh(dynamic.body(),[-0.02,0.,0.],10.,[0.3,0.5,0.7,1.]).unwrap();
        assert_eq!(mesh.indices().len(),dynamic.body().surface().len()*3);
        let reference: Vec<_> = dynamic.body().rest_positions().iter().map(|p|p.map(|x|x as f32)).collect();
        assert_eq!(mesh.explicit_material_coordinates().unwrap(),reference);
        let alternate = tissue_surface_scene_mesh(dynamic.body(),[0.2,-0.1,0.3],25.,[1.;4]).unwrap();
        assert_eq!(alternate.explicit_material_coordinates(),mesh.explicit_material_coordinates());
        for (render,physical) in mesh.vertices().iter().zip(&positions) {
            let expected: [f32;3] = std::array::from_fn(|axis|
                ((physical[axis]-[-0.02,0.,0.][axis])*10.) as f32);
            assert_eq!(render.position,expected);
        }
        assert!(mesh.vertices().iter().zip(dynamic.body().rest_positions()).any(|(p,r)|
            p.position[1] != (r[1]*10.) as f32));
        assert!(tissue_surface_scene_mesh(dynamic.body(),[0.;3],0.,[1.;4]).is_err());
        assert_eq!(dynamic.body().positions(),positions);
        assert_eq!(dynamic.velocities(),velocities);
    }
}
