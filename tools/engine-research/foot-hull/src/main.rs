//! Offline reference-mesh audit. No runtime collision or stance inference.
use parry3d_f64::{math::Vector, transformation::try_convex_hull};
use serde_json::json;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("expected native point log")?;
    let input = std::fs::read_to_string(path)?;
    let mut results = Vec::new();
    for line in input
        .lines()
        .filter_map(|line| line.strip_prefix("FOOT_REFERENCE_POINTS "))
    {
        let record: serde_json::Value = serde_json::from_str(line)?;
        let points: Vec<Vector> = record["points"]
            .as_array()
            .ok_or("missing points")?
            .iter()
            .map(|p| {
                Ok(Vector::new(
                    p[0].as_f64().ok_or("x")?,
                    p[1].as_f64().ok_or("y")?,
                    p[2].as_f64().ok_or("z")?,
                ))
            })
            .collect::<Result<_, &str>>()?;
        if points.is_empty() || points.len() > 65536 || points.iter().any(|p| !p.is_finite()) {
            return Err("invalid point cloud".into());
        }
        let mut source_hash = blake3::Hasher::new();
        source_hash.update(b"voxy-foot-reference-v1\0");
        let bone = format!("{}_foot", record["side"].as_str().ok_or("missing side")?);
        source_hash.update(bone.as_bytes());
        source_hash.update(&[0]);
        source_hash.update(&(points.len() as u64).to_le_bytes());
        for point in &points {
            for value in [point.x, point.y, point.z] {
                source_hash.update(&value.to_bits().to_le_bytes());
            }
        }
        let source_hash = source_hash.finalize().to_hex().to_string();
        let (hull, faces) = try_convex_hull(&points).map_err(|e| format!("hull failed: {e:?}"))?;
        if hull.iter().any(|vertex| !points.contains(vertex)) {
            return Err("hull vertex is not an original mesh point".into());
        }
        let center = hull.iter().copied().fold(Vector::ZERO, |sum, p| sum + p) / hull.len() as f64;
        let mut max_outside = 0f64;
        for face in &faces {
            let [a, b, c] = face.map(|i| hull[i as usize]);
            let mut normal = (b - a).cross(c - a).normalize();
            if !normal.is_finite() {
                return Err("degenerate hull face".into());
            }
            if normal.dot(center - a) > 0. {
                normal = -normal;
            }
            for point in &points {
                max_outside = max_outside.max(normal.dot(*point - a));
            }
        }
        if max_outside > 1e-9 {
            return Err("hull does not enclose source points".into());
        }
        let native = voxy_animation::FootSupportPoints::new(
            hull.iter()
                .map(|v| glam::DVec3::new(v.x, v.y, v.z))
                .collect(),
        )?;
        let mut max_native_error = 0f64;
        let mut max_support_error = 0f64;
        // Deterministic 3D directions: includes slopes and lateral orientations.
        for y in -12..=12 {
            for azimuth in 0..48 {
                let height = y as f64 / 12.;
                let radius = (1. - height * height).max(0.).sqrt();
                let angle = azimuth as f64 * std::f64::consts::TAU / 48.;
                let n = Vector::new(radius * angle.cos(), height, radius * angle.sin());
                let minimum = |vertices: &[Vector]| {
                    vertices
                        .iter()
                        .map(|v| v.dot(n))
                        .fold(f64::INFINITY, f64::min)
                };
                let selected =
                    native.support(glam::DMat3::IDENTITY, glam::DVec3::new(n.x, n.y, n.z))?;
                max_native_error =
                    max_native_error.max((minimum(&points) - selected.projection).abs());
                max_support_error =
                    max_support_error.max((minimum(&points) - minimum(&hull)).abs());
            }
        }
        // Compare the native transpose query with independently transformed full
        // mesh vertices, including shear and reflected nonuniform scale.
        let mut affine_cases = Vec::new();
        for angle in [-0.6, 0.0, 0.6] {
            for reflected in [false, true] {
                let scale = glam::DVec3::new(if reflected { -2.0 } else { 2.0 }, 0.5, 3.0);
                let shear = glam::DMat3::from_cols(
                    glam::DVec3::X,
                    glam::DVec3::new(0.2, 1.0, 0.0),
                    glam::DVec3::new(0.0, -0.15, 1.0),
                );
                let linear = glam::DMat3::from_quat(glam::DQuat::from_rotation_x(angle))
                    * shear
                    * glam::DMat3::from_diagonal(scale);
                let transformed: Vec<_> = points
                    .iter()
                    .map(|p| linear * glam::DVec3::new(p.x, p.y, p.z))
                    .collect();
                let mut error = 0.0f64;
                for y in -12..=12 {
                    for azimuth in 0..48 {
                        let height = y as f64 / 12.0;
                        let radius = (1.0 - height * height).max(0.0).sqrt();
                        let angle = azimuth as f64 * std::f64::consts::TAU / 48.0;
                        let n =
                            glam::DVec3::new(radius * angle.cos(), height, radius * angle.sin())
                                .normalize();
                        let expected = transformed
                            .iter()
                            .map(|p| n.dot(*p))
                            .fold(f64::INFINITY, f64::min);
                        let selected = native.support(linear, n)?;
                        error = error.max((expected - selected.projection).abs());
                        error = error.max((n.dot(linear * selected.local) - expected).abs());
                    }
                }
                if error > 1e-9 {
                    return Err("affine native support changes mesh envelope".into());
                }
                affine_cases.push(json!({"rotation_x":angle,"reflected":reflected,
                    "scale":scale.to_array(),"shear":true,"directions":1200,"max_error_m":error}));
            }
        }
        if max_native_error > 1e-9 {
            return Err("native support changes mesh envelope".into());
        }
        if max_support_error > 1e-9 {
            return Err("hull changes measured support envelope".into());
        }
        results.push(json!({"side":record["side"],"source_vertices":points.len(),"source_geometry_blake3":source_hash,
            "hull_vertices":hull.iter().map(|v|[v.x,v.y,v.z]).collect::<Vec<_>>(),
            "affine_cases":affine_cases,"native_support_error_m":max_native_error,"faces":faces,"max_face_outside_m":max_outside,"vertices_are_original_points":true,"sampled_support_directions":1200,"max_support_error_m":max_support_error}));
    }
    if results.len() != 2 {
        return Err("expected exactly two native foot clouds".into());
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({"parry_version":"0.28.0",
        "reference_mesh_only":true,"continuous_collision_verified":false,"feet":results}))?
    );
    Ok(())
}
