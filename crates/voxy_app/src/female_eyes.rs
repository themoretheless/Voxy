//! Eye pigmentation in bind coordinates, separate from lighting and gaze.
use glam::{Vec2, Vec3};

pub(crate) const MATERIAL_SHADER: &str = include_str!("female_material.wgsl");
pub(crate) fn center(x: f32) -> Vec3 {
    if x > 0. {
        Vec3::new(0.032_892_3, 0.712_416, 0.121_601)
    } else {
        Vec3::new(-0.032_848_5, 0.712_416, 0.121_413)
    }
}
pub(crate) fn uv(point: Vec3) -> [f32; 2] {
    let mut offset = point - center(point.x);
    if offset.z < 0. {
        // Keep the same material/normal encoding across the equator. Posterior
        // coordinates remain in the sclera annulus instead of the pupil/iris.
        let projected = Vec2::new(offset.x, offset.y);
        if projected.length() < 0.007 {
            let direction = projected.try_normalize().unwrap_or(Vec2::X);
            offset.x = direction.x * 0.007;
            offset.y = direction.y * 0.007;
        }
    }
    [
        0.1 + 0.2 * ((offset.x / 0.024) + 0.5).clamp(0., 1.),
        0.004 + 0.04 * (0.5 - offset.y / 0.024).clamp(0., 1.),
    ]
}
pub(crate) fn is_eye_uv(uv: [f32; 2]) -> bool {
    (0.1..=0.3).contains(&uv[0]) && (0.004..=0.044).contains(&uv[1])
}
fn smooth(a: f32, b: f32, value: f32) -> f32 {
    let t = ((value - a) / (b - a)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}
fn scleral_pigment(p: Vec2) -> Vec3 {
    let radius = p.length();
    let angle = p.y.atan2(p.x);
    let mut vessels: f32 = 0.;
    for branch in 0..9 {
        let phase = branch as f32 * 2.399963;
        let axis = phase + 0.09 * (radius * 950. + phase).sin();
        let wrapped = (angle - axis).sin().atan2((angle - axis).cos());
        let main = 1. - smooth(0.000045, 0.00016, wrapped.abs() * radius);
        let split_axis = axis + 0.12 * smooth(0.0072, 0.0095, radius);
        let split = (angle - split_axis).sin().atan2((angle - split_axis).cos());
        let fork =
            (1. - smooth(0.000025, 0.00011, split.abs() * radius)) * smooth(0.0073, 0.0082, radius);
        vessels = vessels.max(main.max(fork) * smooth(0.0063, 0.008, radius));
    }
    Vec3::new(0.78, 0.79, 0.75).lerp(Vec3::new(0.78, 0.30, 0.28), vessels * 0.08)
}
pub(crate) fn pigment(uv: Vec2) -> Vec3 {
    let p = Vec2::new((uv.x - 0.1) / 0.2 - 0.5, 0.5 - (uv.y - 0.004) / 0.04) * 0.024;
    let radius = p.length();
    let sclera = scleral_pigment(p);
    if radius > 0.0059 {
        return sclera;
    }
    let angle = p.y.atan2(p.x);
    // Integer angular frequencies close at atan2's seam. Unequal phases and
    // radial warping break the mechanically repeated spiral pattern.
    let warp = 0.055 * (angle * 7. + 1.7).sin() + 0.028 * (angle * 13. - 0.8).sin();
    let theta = angle + warp * smooth(0.002, 0.0055, radius);
    let bundle_a = 0.45
        + 0.55
            * smooth(
                -0.65,
                0.75,
                (radius * 4100. + (angle * 17.).sin() * 2.3).sin(),
            );
    let bundle_b = 0.35
        + 0.65
            * smooth(
                -0.7,
                0.8,
                (radius * 5700. + (angle * 29. + 1.1).sin() * 1.8).sin(),
            );
    let fibers = (0.5
        + 0.19 * bundle_a * (theta * 79. + 0.7 * (radius * 1800.).sin()).sin()
        + 0.13 * bundle_b * (theta * 137. + 2.1).sin()
        + 0.09 * (theta * 43. - radius * 900. + 0.3).sin()
        + 0.07 * (theta * 19. + 1.9).sin()
        + 0.08 * (angle * 5. + radius * 1300.).sin())
    .clamp(0., 1.);
    let iris = Vec3::new(0.075, 0.12, 0.065).lerp(Vec3::new(0.25, 0.285, 0.155), fibers);
    let collarette =
        0.0028 + 0.00018 * (angle * 11. + 0.4).sin() + 0.00009 * (angle * 23. - 1.2).sin();
    let inner = smooth(collarette - 0.00045, collarette + 0.00035, radius);
    let warm = Vec3::new(0.17, 0.13, 0.055) * (0.7 + 0.6 * fibers);
    let iris = warm.lerp(iris, inner);
    let crypts = smooth(0.84, 0.98, 0.5 + 0.5 * (angle * 31. + 1.1).sin())
        * (1. - smooth(0.00008, 0.0005, (radius - collarette).abs()));
    let iris = iris * (1. - 0.45 * crypts);
    let ring = smooth(0.00505, 0.0056, radius);
    let iris = iris.lerp(Vec3::new(0.025, 0.039, 0.026), ring * 0.9);
    let color = iris.lerp(sclera, smooth(0.0055, 0.0059, radius));
    Vec3::new(0.0015, 0.002, 0.0025).lerp(color, smooth(0.0019, 0.00215, radius))
}
/// Refine the globe before animation so vertex ray visibility has finer support.
pub(crate) fn refined_globe(
    mesh: &voxy_render::SceneMesh,
) -> (Vec<voxy_render::SceneVertex>, Vec<u32>) {
    let mut vertices = mesh.vertices().to_vec();
    let mut indices = mesh.indices().to_vec();
    let origin = center(vertices[0].position[0]);
    for _ in 0..2 {
        let mut edges = std::collections::HashMap::new();
        let mut refined = Vec::with_capacity(indices.len() * 4);
        for triangle in indices.chunks_exact(3) {
            let mut midpoint = |a: u32, b: u32| {
                let key = (a.min(b), a.max(b));
                *edges.entry(key).or_insert_with(|| {
                    let va = vertices[a as usize];
                    let vb = vertices[b as usize];
                    let pa = Vec3::from_array(va.position) - origin;
                    let pb = Vec3::from_array(vb.position) - origin;
                    let radius = 0.5 * (pa.length() + pb.length());
                    let direction = (pa + pb).try_normalize().unwrap_or(Vec3::Z);
                    let point = origin + direction * radius;
                    let index = u32::try_from(vertices.len()).expect("eye vertices");
                    vertices.push(voxy_render::SceneVertex {
                        position: point.to_array(),
                        uv: uv(point),
                        color: va.color,
                    });
                    index
                })
            };
            let [a, b, c] = [triangle[0], triangle[1], triangle[2]];
            let ab = midpoint(a, b);
            let bc = midpoint(b, c);
            let ca = midpoint(c, a);
            refined.extend([a, ab, ca, ab, b, bc, ca, bc, c, ab, bc, ca]);
        }
        indices = refined;
    }
    (vertices, indices)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equator_keeps_a_single_material_and_continuous_sclera_coordinates() {
        let center = center(1.);
        let front = uv(center + Vec3::new(0.009, 0.004, 0.000001));
        let rear = uv(center + Vec3::new(0.009, 0.004, -0.000001));
        assert!(is_eye_uv(front) && is_eye_uv(rear));
        assert!(Vec2::from_array(front).distance(Vec2::from_array(rear)) < 1e-6);
        assert!(pigment(Vec2::from_array(front)).min_element() > 0.7);
    }
    #[test]
    fn refinement_preserves_globe_radius_and_valid_topology() {
        let asset = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/eye-l.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let (vertices, indices) = refined_globe(&asset.mesh);
        assert_eq!(indices.len(), asset.mesh.indices().len() * 16);
        assert!(indices.iter().all(|&i| (i as usize) < vertices.len()));
        let origin = center(vertices[0].position[0]);
        let radii: Vec<_> = asset
            .mesh
            .vertices()
            .iter()
            .map(|v| Vec3::from_array(v.position).distance(origin))
            .collect();
        let min = radii.iter().copied().fold(f32::INFINITY, f32::min);
        let max = radii.iter().copied().fold(0., f32::max);
        assert!(vertices.iter().all(|v| {
            let r = Vec3::from_array(v.position).distance(origin);
            r.is_finite() && r >= min - 1e-6 && r <= max + 1e-6
        }));
    }
    #[test]
    fn fibers_close_across_angular_seam() {
        for radius in [0.0024, 0.003, 0.004, 0.0052] {
            let sample = |y: f32| {
                pigment(Vec2::new(
                    0.2 - 0.2 * radius / 0.024,
                    0.024 - 0.04 * y / 0.024,
                ))
            };
            assert!(sample(1e-8).distance(sample(-1e-8)) < 0.001);
        }
    }
    #[test]
    fn pupil_iris_and_sclera_have_distinct_materials() {
        let coordinates = |x: f32| Vec2::new(0.2 + 0.2 * x / 0.024, 0.024);
        let pupil = pigment(coordinates(0.));
        let iris = pigment(coordinates(0.004));
        let sclera = pigment(coordinates(0.009));
        assert!(pupil.max_element() < 0.01);
        assert!(iris.max_element() > 0.05 && iris.max_element() < 0.4);
        assert!(sclera.min_element() > 0.7);
        assert!(is_eye_uv(uv(center(1.) + Vec3::Z * 0.012)));
        let posterior = uv(center(1.) - Vec3::Z * 0.012);
        assert!(is_eye_uv(posterior));
        assert!(pigment(Vec2::from_array(posterior)).min_element() > 0.7);
        assert!(!is_eye_uv([0., 0.]));
        assert!(!is_eye_uv([0.5, 0.5]));
    }
}
