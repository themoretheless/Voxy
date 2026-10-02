//! Bounded import of triangulated Wavefront OBJ geometry.
use crate::{SceneMesh, SceneVertex};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
pub struct ObjLimits {
    pub source_bytes: usize,
    pub attributes: usize,
    pub vertices: usize,
    pub triangles: usize,
}
impl Default for ObjLimits {
    fn default() -> Self {
        Self {
            source_bytes: 64 * 1024 * 1024,
            attributes: 1_000_000,
            vertices: 1_000_000,
            triangles: 1_000_000,
        }
    }
}
#[derive(Clone, Debug)]
pub struct ObjAsset {
    pub mesh: SceneMesh,
    /// OBJ normals retained per rendered vertex for future lighting/material passes.
    pub normals: Vec<Option<[f32; 3]>>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjError {
    pub line: usize,
    pub reason: &'static str,
}
impl std::fmt::Display for ObjError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OBJ line {}: {}", self.line, self.reason)
    }
}
impl std::error::Error for ObjError {}
fn error(line: usize, reason: &'static str) -> ObjError {
    ObjError { line, reason }
}
fn numbers<const N: usize>(values: &[&str], line: usize) -> Result<[f32; N], ObjError> {
    if values.len() != N {
        return Err(error(line, "unsupported attribute arity"));
    }
    let mut output = [0.0_f32; N];
    for (slot, value) in output.iter_mut().zip(values) {
        *slot = value.parse().map_err(|_| error(line, "invalid number"))?;
        if !slot.is_finite() {
            return Err(error(line, "non-finite attribute"));
        }
    }
    Ok(output)
}
fn index(value: &str, count: usize, line: usize) -> Result<usize, ObjError> {
    let raw: i64 = value.parse().map_err(|_| error(line, "invalid index"))?;
    let count = i64::try_from(count).map_err(|_| error(line, "index capacity exceeded"))?;
    let resolved = if raw > 0 {
        raw - 1
    } else {
        count.checked_add(raw).unwrap_or(-1)
    };
    if raw == 0 || resolved < 0 || resolved >= count {
        return Err(error(line, "index out of bounds"));
    }
    usize::try_from(resolved).map_err(|_| error(line, "index capacity exceeded"))
}
impl ObjAsset {
    /// # Errors
    /// Rejects exceeded budgets, invalid indices/attributes, non-triangle faces and
    /// unsupported geometry directives. Materials are not loaded implicitly.
    #[allow(clippy::too_many_lines)]
    pub fn parse(source: &str, limits: ObjLimits) -> Result<Self, ObjError> {
        if source.len() > limits.source_bytes {
            return Err(error(0, "source budget exceeded"));
        }
        let (mut positions, mut uvs, mut normals) = (Vec::new(), Vec::new(), Vec::new());
        let (mut vertices, mut vertex_normals, mut indices) = (Vec::new(), Vec::new(), Vec::new());
        let mut unique = HashMap::new();
        for (offset, raw) in source.lines().enumerate() {
            let line = offset + 1;
            let raw = raw.split('#').next().unwrap_or_default();
            let mut words = raw.split_whitespace();
            let Some(kind) = words.next() else { continue };
            // Bound token allocation even for a malicious single face line.
            let values: Vec<_> = words.take(5).collect();
            match kind {
                "v" | "vt" | "vn" => {
                    if positions.len() + uvs.len() + normals.len() >= limits.attributes {
                        return Err(error(line, "attribute budget exceeded"));
                    }
                    match kind {
                        "v" => positions.push(numbers::<3>(&values, line)?),
                        "vt" => uvs.push(numbers::<2>(&values, line)?),
                        _ => normals.push(numbers::<3>(&values, line)?),
                    }
                }
                "f" => {
                    if values.len() != 3 {
                        return Err(error(line, "triangulate faces before import"));
                    }
                    if indices.len() / 3 >= limits.triangles {
                        return Err(error(line, "triangle budget exceeded"));
                    }
                    for value in values {
                        let mut fields = value.split('/');
                        let p = index(fields.next().unwrap_or_default(), positions.len(), line)?;
                        let uv = fields
                            .next()
                            .filter(|v| !v.is_empty())
                            .map(|v| index(v, uvs.len(), line))
                            .transpose()?;
                        let normal = fields
                            .next()
                            .filter(|v| !v.is_empty())
                            .map(|v| index(v, normals.len(), line))
                            .transpose()?;
                        if fields.next().is_some() {
                            return Err(error(line, "invalid face corner"));
                        }
                        let key = (p, uv, normal);
                        let id = if let Some(&id) = unique.get(&key) {
                            id
                        } else {
                            if vertices.len() >= limits.vertices {
                                return Err(error(line, "vertex budget exceeded"));
                            }
                            let id = u32::try_from(vertices.len())
                                .map_err(|_| error(line, "vertex capacity exceeded"))?;
                            vertices.push(SceneVertex {
                                position: positions[p],
                                uv: uv.map_or([0.0; 2], |i| uvs[i]),
                                color: [1.0; 4],
                            });
                            vertex_normals.push(normal.map(|i| normals[i]));
                            unique.insert(key, id);
                            id
                        };
                        indices.push(id);
                    }
                }
                "o" | "g" | "s" | "mtllib" | "usemtl" => {}
                _ => return Err(error(line, "unsupported OBJ directive")),
            }
        }
        let mesh =
            SceneMesh::new(vertices, indices).map_err(|_| error(0, "empty or invalid geometry"))?;
        Ok(Self {
            mesh,
            normals: vertex_normals,
        })
    }
}
#[cfg(test)]
#[allow(clippy::float_cmp)] // Exact parsed integer-valued attributes, no arithmetic.
mod tests {
    use super::*;
    const TRIANGLE: &str =
        "v 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 0 1\nvn 0 0 1\nf -3/1/1 -2/2/1 -1/3/1\n";
    #[test]
    fn relative_indices_and_normals() {
        let asset = ObjAsset::parse(TRIANGLE, ObjLimits::default()).unwrap();
        assert_eq!(asset.mesh.vertices()[1].position, [1.0, 0.0, 0.0]);
        assert_eq!(asset.mesh.vertices()[2].uv, [0.0, 1.0]);
        assert_eq!(asset.normals, vec![Some([0.0, 0.0, 1.0]); 3]);
    }
    #[test]
    fn seams_preserved_and_identical_corners_shared() {
        let source = format!("{TRIANGLE}f 1/1/1 2/2/1 3/3/1\nf 1/2/1 2/2/1 3/3/1");
        let asset = ObjAsset::parse(&source, ObjLimits::default()).unwrap();
        assert_eq!(asset.mesh.vertices().len(), 4);
        assert_eq!(asset.mesh.indices(), &[0, 1, 2, 0, 1, 2, 3, 1, 2]);
    }
    #[test]
    fn invalid_data_and_budgets_reject() {
        for source in [
            "v NaN 0 0",
            "v 0 0 0\nf 0 1 1",
            "v 0 0 0\nf -2 1 1",
            "f 1 2 3 4",
            "curv 1 2 3",
        ] {
            assert!(ObjAsset::parse(source, ObjLimits::default()).is_err());
        }
        for limits in [
            ObjLimits {
                source_bytes: 1,
                ..Default::default()
            },
            ObjLimits {
                attributes: 2,
                ..Default::default()
            },
            ObjLimits {
                vertices: 2,
                ..Default::default()
            },
            ObjLimits {
                triangles: 0,
                ..Default::default()
            },
        ] {
            assert!(ObjAsset::parse(TRIANGLE, limits).is_err());
        }
    }
}
