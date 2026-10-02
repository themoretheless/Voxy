//! Transport authored normals through triangle deformation instead of replacing
//! them with normals whose weights depend on the local tessellation density.
use glam::{Mat3, Vec3};
use voxy_render::SceneVertex;

#[derive(Debug)]
struct Triangle {
    ids: [usize; 3],
    reference_transpose: Mat3,
    angles: [f32; 3],
}
#[derive(Debug)]
pub(super) struct PreparedNormals {
    triangles: Vec<Triangle>,
}
impl PreparedNormals {
    pub(super) fn geometric(&self, posed: &[SceneVertex], fallback: &[Vec3]) -> Vec<Vec3> {
        let mut sums = vec![Vec3::ZERO; posed.len()];
        for triangle in &self.triangles {
            let [a, b, c] = triangle.ids.map(|i| Vec3::from_array(posed[i].position));
            let normal = (b - a).cross(c - a);
            for i in triangle.ids {
                sums[i] += normal;
            }
        }
        sums.into_iter()
            .enumerate()
            .map(|(i, n)| n.try_normalize().unwrap_or(fallback[i]))
            .collect()
    }
    pub(super) fn new(rest: &[SceneVertex], indices: &[u32]) -> Self {
        let mut triangles = Vec::with_capacity(indices.len() / 3);
        for triangle in indices.chunks_exact(3) {
            let ids = [
                triangle[0] as usize,
                triangle[1] as usize,
                triangle[2] as usize,
            ];
            let a = Vec3::from_array(rest[ids[0]].position);
            let u = Vec3::from_array(rest[ids[1]].position) - a;
            let v = Vec3::from_array(rest[ids[2]].position) - a;
            let Some(n0) = u.cross(v).try_normalize() else {
                continue;
            };
            let angles = std::array::from_fn(|corner| {
                let i = ids[corner];
                let x = Vec3::from_array(rest[ids[(corner + 1) % 3]].position)
                    - Vec3::from_array(rest[i].position);
                let y = Vec3::from_array(rest[ids[(corner + 2) % 3]].position)
                    - Vec3::from_array(rest[i].position);
                x.cross(y).length().atan2(x.dot(y))
            });
            triangles.push(Triangle {
                ids,
                reference_transpose: Mat3::from_cols(u, v, n0).transpose(),
                angles,
            });
        }
        Self { triangles }
    }
    pub(super) fn transport(&self, posed: &[SceneVertex], authored: &[Vec3]) -> Vec<Vec3> {
        let mut result = vec![Vec3::ZERO; posed.len()];
        for triangle in &self.triangles {
            let ids = triangle.ids;
            let b = Vec3::from_array(posed[ids[0]].position);
            let p = Vec3::from_array(posed[ids[1]].position) - b;
            let q = Vec3::from_array(posed[ids[2]].position) - b;
            let Some(n1) = p.cross(q).try_normalize() else {
                continue;
            };
            let transport =
                Mat3::from_cols(p, q, n1).inverse().transpose() * triangle.reference_transpose;
            for corner in 0..3 {
                let i = ids[corner];
                let normal = (transport * authored[i]).try_normalize().unwrap_or(n1);
                result[i] += triangle.angles[corner] * normal;
            }
        }
        result
            .into_iter()
            .enumerate()
            .map(|(i, n)| n.try_normalize().unwrap_or(authored[i]))
            .collect()
    }
}

#[cfg(test)]
pub(super) fn transported(
    rest: &[SceneVertex],
    posed: &[SceneVertex],
    indices: &[u32],
    authored: &[Vec3],
) -> Vec<Vec3> {
    let mut result = vec![Vec3::ZERO; posed.len()];
    for triangle in indices.chunks_exact(3) {
        let ids = [
            triangle[0] as usize,
            triangle[1] as usize,
            triangle[2] as usize,
        ];
        let a = Vec3::from_array(rest[ids[0]].position);
        let u = Vec3::from_array(rest[ids[1]].position) - a;
        let v = Vec3::from_array(rest[ids[2]].position) - a;
        let b = Vec3::from_array(posed[ids[0]].position);
        let p = Vec3::from_array(posed[ids[1]].position) - b;
        let q = Vec3::from_array(posed[ids[2]].position) - b;
        let Some(n0) = u.cross(v).try_normalize() else {
            continue;
        };
        let Some(n1) = p.cross(q).try_normalize() else {
            continue;
        };
        let reference = Mat3::from_cols(u, v, n0);
        let current = Mat3::from_cols(p, q, n1);
        let transport = current.inverse().transpose() * reference.transpose();
        for corner in 0..3 {
            let i = ids[corner];
            let x = Vec3::from_array(rest[ids[(corner + 1) % 3]].position)
                - Vec3::from_array(rest[i].position);
            let y = Vec3::from_array(rest[ids[(corner + 2) % 3]].position)
                - Vec3::from_array(rest[i].position);
            let angle = x.cross(y).length().atan2(x.dot(y));
            let normal = (transport * authored[i]).try_normalize().unwrap_or(n1);
            result[i] += angle * normal;
        }
    }
    result
        .into_iter()
        .enumerate()
        .map(|(i, n)| n.try_normalize().unwrap_or(authored[i]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn vertex(position: [f32; 3]) -> SceneVertex {
        SceneVertex {
            position,
            uv: [0.; 2],
            color: [1.; 4],
        }
    }
    #[test]
    fn preserves_authored_normals_at_rest_after_nonuniform_refinement() {
        let rest = vec![
            vertex([0., 0., 0.]),
            vertex([1., 0., 0.]),
            vertex([0., 1., 0.]),
            vertex([0.1, 0., 0.]),
        ];
        let authored = vec![Vec3::new(0.1, 0.2, 1.).normalize(); 4];
        let result = transported(&rest, &rest, &[0, 3, 2, 3, 1, 2], &authored);
        for (a, b) in result.iter().zip(&authored) {
            assert!(a.distance(*b) < 1e-6);
        }
    }
    #[test]
    fn follows_rotation_and_nonuniform_affine_stretch() {
        let rest = vec![
            vertex([0., 0., 0.]),
            vertex([1., 0., 0.]),
            vertex([0., 1., 0.]),
        ];
        let rotation = Mat3::from_rotation_x(0.7);
        let posed: Vec<_> = rest
            .iter()
            .map(|v| vertex((rotation * Vec3::from_array(v.position) * 2.).to_array()))
            .collect();
        for n in transported(&rest, &posed, &[0, 1, 2], &[Vec3::Z; 3]) {
            assert!(n.distance(rotation * Vec3::Z) < 1e-6);
        }
        let tilted = vec![
            vertex([0., 0., 0.]),
            vertex([2., 0., 1.]),
            vertex([0., 0.5, 0.]),
        ];
        let expected = Vec3::new(-0.5, 0., 1.).normalize();
        for n in transported(&rest, &tilted, &[0, 1, 2], &[Vec3::Z; 3]) {
            assert!(n.distance(expected) < 1e-6);
        }
    }
    #[test]
    fn cached_actual_body_matches_uncached_transport() {
        let asset = voxy_render::ObjAsset::parse(
            include_str!(
                "../../../assets/characters/blender-female/prepared/body-nipple-refined.obj"
            ),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let rest = asset.mesh.vertices();
        let authored: Vec<_> = asset
            .normals
            .iter()
            .map(|n| Vec3::from_array(n.unwrap()))
            .collect();
        let prepared = PreparedNormals::new(rest, asset.mesh.indices());
        let posed: Vec<_> = rest
            .iter()
            .map(|v| {
                let [x, y, z] = v.position;
                vertex([
                    1.1 * x,
                    y + 0.01 * (x * 30.).sin(),
                    z + 0.003 * (y * 20.).cos(),
                ])
            })
            .collect();
        let expected = transported(rest, &posed, asset.mesh.indices(), &authored);
        assert_eq!(prepared.transport(&posed, &authored), expected);
    }
    #[test]
    #[ignore = "release performance measurement on actual body"]
    fn actual_body_transport_profile() {
        let asset = voxy_render::ObjAsset::parse(
            include_str!(
                "../../../assets/characters/blender-female/prepared/body-nipple-refined.obj"
            ),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let rest = asset.mesh.vertices();
        let authored: Vec<_> = asset
            .normals
            .iter()
            .map(|n| Vec3::from_array(n.unwrap()))
            .collect();
        let prepared = PreparedNormals::new(rest, asset.mesh.indices());
        let posed: Vec<_> = rest
            .iter()
            .map(|v| {
                vertex([
                    v.position[0] * 1.1,
                    v.position[1],
                    v.position[2] + 0.003 * (v.position[1] * 20.).cos(),
                ])
            })
            .collect();
        let mut cached = Vec::new();
        let mut uncached = Vec::new();
        for round in 0..11 {
            let start = std::time::Instant::now();
            let a = std::hint::black_box(prepared.transport(&posed, &authored));
            let first = start.elapsed().as_secs_f64() * 1000.;
            let start = std::time::Instant::now();
            let b =
                std::hint::black_box(transported(rest, &posed, asset.mesh.indices(), &authored));
            let second = start.elapsed().as_secs_f64() * 1000.;
            assert_eq!(a, b);
            if round > 0 {
                cached.push(first);
                uncached.push(second);
            }
        }
        cached.sort_by(f64::total_cmp);
        uncached.sort_by(f64::total_cmp);
        let report = serde_json::json!({"cached_median_ms":cached[5],"uncached_median_ms":uncached[5],"cached_samples_ms":cached,"uncached_samples_ms":uncached,"vertices":rest.len(),"triangles":asset.mesh.indices().len()/3,"bitwise_equal":true,"scope":"normal transport only, no frame rate claim"});
        println!("{report}");
        if let Ok(path) = std::env::var("VOXY_NORMAL_TRANSPORT_REPORT") {
            std::fs::write(path, report.to_string()).unwrap();
        }
    }
}
