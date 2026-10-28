//! Agent-authored reference reconstruction specs compiled to native Voxy projects.
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt::Write as _,
    fs,
    io::Read as _,
    path::Path,
};
use voxy_scene::{ObjectId, SceneDocument, SceneObject};
type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Spec {
    version: u32,
    name: String,
    /// Provenance only; this compiler does not run vision inference.
    reference: String,
    inferred_regions: Vec<String>,
    parts: Vec<Part>,
    details: Vec<Detail>,
    #[serde(default)]
    camera: Option<Camera>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Camera {
    target: [f32; 3],
    distance: f32,
    yaw: f32,
    pitch: f32,
}
#[derive(Deserialize, Serialize)]
#[serde(transparent)]
struct CameraComponent(serde_json::Value);
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Detail {
    description: String,
    part: String,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Part {
    id: String,
    parent: Option<String>,
    translation: [f32; 3],
    rotation: [f32; 4],
    scale: [f32; 3],
    color: [f32; 3],
    geometry: Option<Geometry>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Geometry {
    Box,
    Sphere {
        segments: u32,
        rings: u32,
    },
    Cylinder {
        segments: u32,
    },
    RoundedBox {
        radius: f32,
        segments: u32,
        rings: u32,
    },
    RoundedBoxSection {
        radius: f32,
        segments: u32,
        rings: u32,
        min_y: f32,
        max_y: f32,
    },
}

fn validate(spec: &Spec) -> Result<SceneDocument> {
    if spec.version != 1
        || spec.name.trim().is_empty()
        || spec.reference.trim().is_empty()
        || spec.parts.is_empty()
        || spec.parts.len() > 512
        || spec.details.is_empty()
    {
        return Err(
            "version 1, name, reference, 1..512 parts and a detail inventory are required".into(),
        );
    }
    let mut ids = BTreeSet::new();
    for p in &spec.parts {
        if p.id.is_empty()
            || p.id.len() > 80
            || !p
                .id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
            || !ids.insert(p.id.clone())
        {
            return Err("part IDs must be unique, safe ASCII names (1..80 bytes)".into());
        }
        if !p
            .color
            .iter()
            .all(|x| x.is_finite() && (0.0..=1.0).contains(x))
            || !p
                .scale
                .iter()
                .all(|x| x.is_finite() && *x > 0.0 && *x <= 10000.)
            || !p
                .translation
                .iter()
                .all(|x| x.is_finite() && x.abs() <= 10000.)
            || !p.rotation.iter().all(|x| x.is_finite())
            || (p.rotation.iter().map(|x| x * x).sum::<f32>() - 1.).abs() > 0.0001
        {
            return Err(format!("invalid transform or color on {}", p.id).into());
        }
        match p.geometry {
            Some(Geometry::RoundedBoxSection {
                radius,
                segments,
                rings,
                min_y,
                max_y,
            }) => {
                if !radius.is_finite()
                    || !(0.01..=0.5).contains(&radius)
                    || !(8..=128).contains(&segments)
                    || !(4..=64).contains(&rings)
                    || !min_y.is_finite()
                    || !max_y.is_finite()
                    || min_y < -0.5
                    || max_y > 0.5
                    || max_y - min_y < 0.01
                {
                    return Err("invalid rounded box section".into());
                }
            }
            Some(Geometry::RoundedBox {
                radius,
                segments,
                rings,
            }) if !radius.is_finite()
                || !(0.01..=0.5).contains(&radius)
                || !(8..=128).contains(&segments)
                || !(4..=64).contains(&rings) =>
            {
                return Err("invalid rounded box radius or subdivision budget".into());
            }
            Some(Geometry::Sphere { segments, rings })
                if !(3..=128).contains(&segments) || !(2..=64).contains(&rings) =>
            {
                return Err("sphere subdivision budget exceeded".into());
            }
            Some(Geometry::Cylinder { segments }) if !(3..=128).contains(&segments) => {
                return Err("cylinder subdivision budget exceeded".into());
            }
            _ => {}
        }
    }
    if !spec.parts.iter().any(|p| p.geometry.is_some()) {
        return Err("at least one visible part required".into());
    }
    for d in &spec.details {
        if d.description.trim().is_empty()
            || !spec
                .parts
                .iter()
                .any(|p| p.id == d.part && p.geometry.is_some())
        {
            return Err("every detail must describe a visible part".into());
        }
    }
    let mut scene = SceneDocument {
        version: 1,
        objects: spec
            .parts
            .iter()
            .map(|p| {
                let mut components = BTreeMap::new();
                if p.geometry.is_some() {
                    components.insert("editor.model.v1".into(), json!(p.id));
                    components.insert(
                        "editor.material.v1".into(),
                        json!({"tint": [p.color[0],p.color[1],p.color[2],1.],"lit":true}),
                    );
                }
                SceneObject {
                    id: ObjectId(p.id.clone()),
                    parent: p.parent.clone().map(ObjectId),
                    name: p.id.clone(),
                    active: true,
                    translation: p.translation,
                    rotation: p.rotation,
                    scale: p.scale,
                    components,
                }
            })
            .collect(),
    };
    if let Some(camera) = &spec.camera {
        if ids.contains("reconstruction-camera")
            || spec.parts.len() >= 512
            || !camera
                .target
                .iter()
                .all(|v| v.is_finite() && v.abs() <= 10000.)
            || !camera.distance.is_finite()
            || !(0.01..=10000.).contains(&camera.distance)
            || !camera.yaw.is_finite()
            || !camera.pitch.is_finite()
            || camera.pitch.abs() > 1.5
        {
            return Err("invalid reconstruction camera".into());
        }
        scene.objects.push(SceneObject {
            id: ObjectId("reconstruction-camera".into()),
            parent: None,
            name: "Reconstruction camera".into(),
            active: true,
            translation: [0.; 3],
            rotation: [0., 0., 0., 1.],
            scale: [1.; 3],
            components: BTreeMap::from([(
                "editor.camera.v1".into(),
                json!({
                    "legacy":false,"perspective":true,"target":camera.target,
                    "distance":camera.distance,"yaw":camera.yaw,"pitch":camera.pitch
                }),
            )]),
        });
    }
    // Native scene validation checks parent references and cycles as well as transforms.
    let mut registry = voxy_scene::ComponentRegistry::default();
    registry.register::<String>("editor.model.v1")?;
    registry.register::<serde_json::Value>("editor.material.v1")?;
    registry.register::<CameraComponent>("editor.camera.v1")?;
    scene.load(&registry, 512)?;
    Ok(scene)
}

#[derive(Default)]
struct Mesh {
    vertices: Vec<[f32; 3]>,
    faces: Vec<[usize; 3]>,
}
impl Mesh {
    fn tri(&mut self, a: usize, b: usize, c: usize) {
        self.faces.push([a + 1, b + 1, c + 1]);
    }
    fn obj(&self) -> String {
        let mut s = String::from("# Procedurally generated by img2voxy; metres, Y up\n");
        for v in &self.vertices {
            writeln!(s, "v {} {} {}", v[0], v[1], v[2]).unwrap();
        }
        for f in &self.faces {
            writeln!(s, "f {} {} {}", f[0], f[1], f[2]).unwrap();
        }
        s
    }
}
fn mesh(g: &Geometry) -> Mesh {
    let mut m = Mesh::default();
    match *g {
        Geometry::RoundedBoxSection {
            radius,
            segments,
            rings,
            min_y,
            max_y,
        } => {
            m = mesh(&Geometry::RoundedBox {
                radius,
                segments,
                rings,
            });
            if min_y > -0.5 {
                clip_y(&mut m, min_y, false);
            }
            if max_y < 0.5 {
                clip_y(&mut m, max_y, true);
            }
            let mut used = BTreeMap::new();
            let mut vertices = Vec::new();
            for f in &mut m.faces {
                for index in f {
                    *index = *used.entry(*index).or_insert_with(|| {
                        vertices.push(m.vertices[*index - 1]);
                        vertices.len()
                    });
                }
            }
            m.vertices = vertices;
        }
        Geometry::RoundedBox {
            radius,
            segments,
            rings,
        } => {
            m = mesh(&Geometry::Sphere { segments, rings });
            for v in &mut m.vertices {
                let maximum = v.iter().map(|x| x.abs()).fold(0_f32, f32::max);
                let q = v.map(|x| x * 0.5 / maximum);
                let core = q.map(|x| x.clamp(-0.5 + radius, 0.5 - radius));
                let delta: [f32; 3] = std::array::from_fn(|i| q[i] - core[i]);
                let length = delta.iter().map(|x| x * x).sum::<f32>().sqrt();
                *v = std::array::from_fn(|i| core[i] + radius * delta[i] / length);
            }
        }
        Geometry::Box => {
            m.vertices = vec![
                [-0.5, -0.5, -0.5],
                [0.5, -0.5, -0.5],
                [0.5, 0.5, -0.5],
                [-0.5, 0.5, -0.5],
                [-0.5, -0.5, 0.5],
                [0.5, -0.5, 0.5],
                [0.5, 0.5, 0.5],
                [-0.5, 0.5, 0.5],
            ];
            for [a, b, c, d] in [
                [0, 3, 2, 1],
                [4, 5, 6, 7],
                [0, 4, 7, 3],
                [1, 2, 6, 5],
                [3, 7, 6, 2],
                [0, 1, 5, 4],
            ] {
                m.tri(a, b, c);
                m.tri(a, c, d);
            }
        }
        Geometry::Cylinder { segments } => {
            let n = segments as usize;
            for y in [-0.5, 0.5] {
                for i in 0..n {
                    let a = std::f32::consts::TAU * i as f32 / n as f32;
                    m.vertices.push([0.5 * a.cos(), y, 0.5 * a.sin()]);
                }
            }
            m.vertices.extend([[0., -0.5, 0.], [0., 0.5, 0.]]);
            for i in 0..n {
                let j = (i + 1) % n;
                m.tri(i, n + i, n + j);
                m.tri(i, n + j, j);
                m.tri(2 * n, i, j);
                m.tri(2 * n + 1, n + j, n + i);
            }
        }
        Geometry::Sphere { segments, rings } => {
            let n = segments as usize;
            let r = rings as usize;
            m.vertices.push([0., 0.5, 0.]);
            for row in 1..r {
                let phi = std::f32::consts::PI * row as f32 / r as f32;
                for i in 0..n {
                    let a = std::f32::consts::TAU * i as f32 / n as f32;
                    m.vertices.push([
                        0.5 * phi.sin() * a.cos(),
                        0.5 * phi.cos(),
                        0.5 * phi.sin() * a.sin(),
                    ]);
                }
            }
            let bottom = m.vertices.len();
            m.vertices.push([0., -0.5, 0.]);
            for i in 0..n {
                let j = (i + 1) % n;
                m.tri(0, 1 + j, 1 + i);
                m.tri(bottom, 1 + (r - 2) * n + i, 1 + (r - 2) * n + j);
            }
            for row in 0..r - 2 {
                for i in 0..n {
                    let j = (i + 1) % n;
                    let a = 1 + row * n + i;
                    let b = 1 + row * n + j;
                    m.tri(a, b, b + n);
                    m.tri(a, b + n, a + n);
                }
            }
        }
    }
    m
}

/// Clip a convex primitive and close the new planar boundary; indices stay welded.
fn clip_y(m: &mut Mesh, plane: f32, keep_below: bool) {
    let mut intersections = BTreeMap::new();
    let mut faces = Vec::new();
    for face in std::mem::take(&mut m.faces) {
        let mut polygon = Vec::new();
        for (a, b) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            let va = m.vertices[a - 1];
            let vb = m.vertices[b - 1];
            let inside_a = if keep_below {
                va[1] <= plane
            } else {
                va[1] >= plane
            };
            let inside_b = if keep_below {
                vb[1] <= plane
            } else {
                vb[1] >= plane
            };
            if inside_a {
                polygon.push(a);
            }
            if inside_a != inside_b {
                let index = if va[1] == plane {
                    a
                } else if vb[1] == plane {
                    b
                } else {
                    *intersections
                        .entry((a.min(b), a.max(b)))
                        .or_insert_with(|| {
                            let t = (plane - va[1]) / (vb[1] - va[1]);
                            m.vertices.push([
                                va[0] + t * (vb[0] - va[0]),
                                plane,
                                va[2] + t * (vb[2] - va[2]),
                            ]);
                            m.vertices.len()
                        })
                };
                polygon.push(index);
            }
        }
        polygon.dedup();
        if polygon.first() == polygon.last() {
            polygon.pop();
        }
        for i in 1..polygon.len().saturating_sub(1) {
            faces.push([polygon[0], polygon[i], polygon[i + 1]]);
        }
    }
    let mut edges = BTreeMap::new();
    for f in &faces {
        for (a, b) in [(f[0], f[1]), (f[1], f[2]), (f[2], f[0])] {
            let entry = edges.entry((a.min(b), a.max(b))).or_insert((0, a, b));
            entry.0 += 1;
        }
    }
    let boundary: Vec<_> = edges
        .values()
        .filter(|e| e.0 == 1)
        .map(|e| (e.1, e.2))
        .collect();
    if !boundary.is_empty() {
        m.vertices.push([0., plane, 0.]);
        let center = m.vertices.len();
        for (a, b) in boundary {
            faces.push([center, b, a]);
        }
    }
    m.faces = faces;
}

fn build(spec: &Spec, output: &Path) -> Result<()> {
    let scene = validate(spec)?;
    // A fresh directory prevents overwriting authored projects and previous evidence.
    fs::create_dir(output)?;
    let result = (|| -> Result<()> {
        let mut assets = Vec::new();
        for p in &spec.parts {
            if let Some(g) = &p.geometry {
                let source = format!("{}.obj", p.id);
                fs::write(output.join(&source), mesh(g).obj())?;
                assets.push(json!({"asset":p.id,"source":source}));
            }
        }
        fs::write(output.join("main.scene.json"), scene.to_json()?)?;
        fs::write(
            output.join("assets.json"),
            serde_json::to_vec_pretty(&json!({"version":1,"assets":assets}))?,
        )?;
        fs::write(
            output.join("project.voxy"),
            serde_json::to_vec_pretty(
                &json!({"version":1,"name":spec.name,"asset_manifest":"assets.json","startup_scene":"main.scene.json"}),
            )?,
        )?;
        fs::write(
            output.join("reconstruction.json"),
            serde_json::to_vec_pretty(spec)?,
        )?;
        fs::write(
            output.join("review.json"),
            serde_json::to_vec_pretty(
                &json!({"structural_validation":"passed","visual_review":"pending","reference":spec.reference,"inferred_regions":spec.inferred_regions,"animation_readiness":"unverified","note":"No visual similarity score has been measured. Review a Voxy render against the reference."}),
            )?,
        )?;
        Ok(())
    })();
    // Preserve incomplete output for diagnosis; never report success after an IO failure.
    result
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let (mode, path) = match args.as_slice() {
        [m, p, ..] => (m, p),
        _ => return Err("usage: img2voxy validate SPEC | build SPEC NEW_DIRECTORY".into()),
    };
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("spec exceeds 2 MiB".into());
    }
    let spec: Spec = serde_json::from_slice(&bytes)?;
    match (mode.as_str(), args.len()) {
        ("validate", 2) => {
            validate(&spec)?;
            println!("Structure passed; visual review pending");
        }
        ("build", 3) => {
            build(&spec, Path::new(&args[2]))?;
            println!("Created {}/project.voxy; visual review pending", args[2]);
        }
        _ => return Err("usage: img2voxy validate SPEC | build SPEC NEW_DIRECTORY".into()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Spec {
        serde_json::from_str(include_str!("../examples/desk-lamp.json")).unwrap()
    }
    #[test]
    fn clipped_case_sections_have_closed_edges_and_bounded_vertices() {
        for (min_y, max_y) in [(-0.5, 0.238), (0.248, 0.5), (-0.2, 0.2), (-0.5, 0.)] {
            let m = mesh(&Geometry::RoundedBoxSection {
                radius: 0.3,
                segments: 32,
                rings: 16,
                min_y,
                max_y,
            });
            let mut edges = BTreeMap::new();
            for f in &m.faces {
                for (a, b) in [(f[0], f[1]), (f[1], f[2]), (f[2], f[0])] {
                    assert_ne!(a, b);
                    *edges.entry((a.min(b), a.max(b))).or_insert(0) += 1;
                }
            }
            assert!(
                edges.values().all(|n| *n == 2),
                "bounds={min_y},{max_y} bad={:?}",
                edges
                    .iter()
                    .filter(|(_, n)| **n != 2)
                    .take(10)
                    .collect::<Vec<_>>()
            );
            assert!(
                m.vertices
                    .iter()
                    .all(|v| v[1] >= min_y - 1e-6 && v[1] <= max_y + 1e-6)
            );
        }
    }
    #[test]
    fn native_hierarchy_rejects_cycles_and_unknown_parents() {
        let mut s = fixture();
        validate(&s).unwrap();
        s.parts[0].parent = Some(s.parts[1].id.clone());
        assert!(validate(&s).is_err());
        s.parts[0].parent = Some("absent".into());
        assert!(validate(&s).is_err());
    }
    #[test]
    fn detail_and_geometry_budgets_fail_closed() {
        let mut s = fixture();
        s.details[0].part = "absent".into();
        assert!(validate(&s).is_err());
        let mut s = fixture();
        s.parts[1].geometry = Some(Geometry::Sphere {
            segments: u32::MAX,
            rings: 64,
        });
        assert!(validate(&s).is_err());
    }
    #[test]
    fn primitives_are_closed_outward_and_nondegenerate() {
        for g in [
            Geometry::Box,
            Geometry::RoundedBox {
                radius: 0.3,
                segments: 32,
                rings: 16,
            },
            Geometry::Cylinder { segments: 16 },
            Geometry::Sphere {
                segments: 16,
                rings: 8,
            },
        ] {
            let m = mesh(&g);
            let mut edges = BTreeMap::new();
            for f in &m.faces {
                let [a, b, c] = f.map(|i| m.vertices[i - 1]);
                let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
                let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
                let normal = [
                    u[1] * v[2] - u[2] * v[1],
                    u[2] * v[0] - u[0] * v[2],
                    u[0] * v[1] - u[1] * v[0],
                ];
                assert!(
                    (0..3)
                        .map(|i| normal[i] * (a[i] + b[i] + c[i]))
                        .sum::<f32>()
                        > 0.
                );
                for (a, b) in [(f[0], f[1]), (f[1], f[2]), (f[2], f[0])] {
                    *edges.entry((a.min(b), a.max(b))).or_insert(0) += 1;
                }
            }
            assert!(edges.values().all(|n| *n == 2));
        }
    }
}
