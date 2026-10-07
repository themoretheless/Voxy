//! Immutable CPU reference geometry for medium boundaries derived from SceneMesh.
//! No simulation inventory, lighting integration or alternate application runner.
use glam::DVec3;
use std::collections::BTreeMap;

/// Snapshot-local medium identity; the caller owns the identity registry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OpticalMediumId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MediumBoundaryHit {
    pub distance_m: f64,
    pub position_m: [f64; 3],
    /// Points into the incident medium, matching dielectric_boundary_sample.
    pub incident_normal: [f64; 3],
    pub incident_medium: OpticalMediumId,
    pub transmitted_medium: OpticalMediumId,
    pub triangle_index: usize,
}
#[derive(Clone, Debug)]
struct Triangle {
    vertices: [DVec3; 3],
    outward: DVec3,
}
/// Closed, consistently oriented indexed boundary, expressed in metres.
/// Immutable derivative of an existing scene mesh, not a second physical owner.
#[derive(Debug)]
pub struct MediumBoundaryMesh {
    triangles: Vec<Triangle>,
    inside: OpticalMediumId,
    outside: OpticalMediumId,
    lower: DVec3,
    upper: DVec3,
}
impl MediumBoundaryMesh {
    pub(crate) fn append_gpu_records(
        &self,
        words: &mut Vec<u32>,
        inside: u32,
        outside: u32,
        object: u32,
    ) -> Result<(), crate::ComputeError> {
        for triangle in &self.triangles {
            crate::medium_geometry_compute::append_triangle(
                words,
                triangle.vertices.map(|v| v.to_array()),
                [0.; 3],
                [0, inside, outside, object],
            )?;
        }
        Ok(())
    }
    pub(crate) fn transport_metadata(
        &self,
    ) -> (OpticalMediumId, OpticalMediumId, [f64; 3], [f64; 3], usize) {
        (
            self.inside,
            self.outside,
            self.lower.to_array(),
            self.upper.to_array(),
            self.triangles.len(),
        )
    }
    /// Admit closed manifold topology and outward orientation for every connected
    /// component before publishing geometry. Self-intersection and overlap between
    /// disconnected components must be excluded by the source geometry owner.
    /// Shared edges must use shared indices;
    /// render meshes split at UV seams require a topology-preserving source mesh.
    /// # Errors
    /// Invalid scale/identity, triangle budget, degenerate faces, open/nonmanifold
    /// or inconsistent winding, inward/zero-volume components, nonfinite geometry.
    pub fn from_scene_mesh(
        mesh: &crate::SceneMesh,
        inside: OpticalMediumId,
        outside: OpticalMediumId,
        metres_per_world_unit: f64,
        max_triangles: usize,
    ) -> Result<Self, &'static str> {
        let count = mesh.indices().len() / 3;
        if count == 0 || count > max_triangles {
            return Err("medium boundary triangle budget");
        }
        if inside == outside || !metres_per_world_unit.is_finite() || metres_per_world_unit <= 0. {
            return Err("invalid medium boundary scale or identity");
        }
        let (triangles, lower, upper) =
            admit_triangles(mesh, metres_per_world_unit, max_triangles)?;
        let mut edges = BTreeMap::<(u32, u32), Vec<(usize, bool)>>::new();
        for (face, indices) in mesh.indices().chunks_exact(3).enumerate() {
            for (a, b) in [
                (indices[0], indices[1]),
                (indices[1], indices[2]),
                (indices[2], indices[0]),
            ] {
                let edge = edges.entry((a.min(b), a.max(b))).or_default();
                if edge.len() == 2 {
                    return Err("nonmanifold medium boundary");
                }
                edge.push((face, a < b));
            }
        }
        let mut neighbours = vec![Vec::new(); count];
        for edge in edges.values() {
            if edge.len() != 2 || edge[0].1 == edge[1].1 {
                return Err("open or inconsistently wound medium boundary");
            }
            neighbours[edge[0].0].push(edge[1].0);
            neighbours[edge[1].0].push(edge[0].0);
        }
        // Validate each component, not just the sum: an inward disconnected
        // component must not hide behind the volume of an outward component.
        let mut seen = vec![false; count];
        for root in 0..count {
            if seen[root] {
                continue;
            }
            let origin = triangles[root].vertices[0];
            let mut pending = vec![root];
            seen[root] = true;
            let mut volume6 = 0.;
            while let Some(face) = pending.pop() {
                let v = triangles[face].vertices.map(|v| v - origin);
                volume6 += v[0].dot(v[1].cross(v[2]));
                for &other in &neighbours[face] {
                    if !seen[other] {
                        seen[other] = true;
                        pending.push(other);
                    }
                }
            }
            if !volume6.is_finite() || volume6 <= 0. {
                return Err("inward or zero-volume medium boundary component");
            }
        }
        Ok(Self {
            triangles,
            inside,
            outside,
            lower,
            upper,
        })
    }
    /// First crossing on a unit ray, in the explicitly supplied metric interval
    /// (minimum_distance_m, maximum_distance_m]. No hidden bias or origin offset.
    /// Does not infer camera occupancy, pick optical coefficients or integrate light.
    /// O(triangles) CPU oracle; not a production acceleration structure.
    /// # Errors
    /// Invalid ray/range, unrepresentable arithmetic or numerically ambiguous hit.
    pub fn first_hit(
        &self,
        origin_m: [f64; 3],
        direction: [f64; 3],
        minimum_distance_m: f64,
        maximum_distance_m: f64,
    ) -> Result<Option<MediumBoundaryHit>, &'static str> {
        Ok(first_triangle_hit(
            &self.triangles,
            self.lower,
            self.upper,
            origin_m,
            direction,
            minimum_distance_m,
            maximum_distance_m,
            true,
        )?
        .map(|h| MediumBoundaryHit {
            distance_m: h.distance_m,
            position_m: h.position_m,
            incident_normal: h.incident_normal,
            incident_medium: if h.entering {
                self.outside
            } else {
                self.inside
            },
            transmitted_medium: if h.entering {
                self.inside
            } else {
                self.outside
            },
            triangle_index: h.triangle_index,
        }))
    }
}

#[derive(Clone, Copy)]
struct TriangleIntersection {
    distance_m: f64,
    position_m: [f64; 3],
    incident_normal: [f64; 3],
    entering: bool,
    triangle_index: usize,
}
fn admit_triangles(
    mesh: &crate::SceneMesh,
    scale: f64,
    max_triangles: usize,
) -> Result<(Vec<Triangle>, DVec3, DVec3), &'static str> {
    let count = mesh.indices().len() / 3;
    if count == 0 || count > max_triangles {
        return Err("medium boundary triangle budget");
    }
    if !scale.is_finite() || scale <= 0. {
        return Err("invalid optical geometry scale");
    }
    let mut triangles = Vec::with_capacity(count);
    let mut lower = DVec3::splat(f64::INFINITY);
    let mut upper = DVec3::splat(f64::NEG_INFINITY);
    for indices in mesh.indices().chunks_exact(3) {
        let v: [DVec3; 3] = std::array::from_fn(|i| {
            DVec3::from_array(mesh.vertices()[indices[i] as usize].position.map(f64::from)) * scale
        });
        if v.iter().any(|p| !p.is_finite()) {
            return Err("nonfinite medium boundary geometry");
        }
        let area = (v[1] - v[0]).cross(v[2] - v[0]);
        let length = area.length();
        if !length.is_finite() || length == 0. {
            return Err("degenerate medium boundary triangle");
        }
        for p in v {
            lower = lower.min(p);
            upper = upper.max(p);
        }
        triangles.push(Triangle {
            vertices: v,
            outward: area / length,
        });
    }
    Ok((triangles, lower, upper))
}
#[allow(clippy::too_many_arguments)]
fn first_triangle_hit(
    triangles: &[Triangle],
    lower: DVec3,
    upper: DVec3,
    origin_m: [f64; 3],
    direction: [f64; 3],
    minimum_distance_m: f64,
    maximum_distance_m: f64,
    reject_crease: bool,
) -> Result<Option<TriangleIntersection>, &'static str> {
    let origin = DVec3::from_array(origin_m);
    let direction = DVec3::from_array(direction);
    if !origin.is_finite()
        || !direction.is_finite()
        || (direction.length() - 1.).abs() > 1e-10
        || !minimum_distance_m.is_finite()
        || !maximum_distance_m.is_finite()
        || minimum_distance_m < 0.
        || maximum_distance_m <= minimum_distance_m
    {
        return Err("invalid medium boundary ray");
    }
    let direction = direction.normalize();
    let mut near = minimum_distance_m;
    let mut far = maximum_distance_m;
    for axis in 0..3 {
        if direction[axis] == 0. {
            if origin[axis] < lower[axis] || origin[axis] > upper[axis] {
                return Ok(None);
            }
        } else {
            let a = (lower[axis] - origin[axis]) / direction[axis];
            let b = (upper[axis] - origin[axis]) / direction[axis];
            near = near.max(a.min(b));
            far = far.min(a.max(b));
            if far < near {
                return Ok(None);
            }
        }
    }
    let mut hit: Option<TriangleIntersection> = None;
    let mut best = maximum_distance_m;
    for (triangle_index, triangle) in triangles.iter().enumerate() {
        let v = triangle.vertices;
        let denominator = direction.dot(triangle.outward);
        if denominator == 0. {
            continue;
        }
        let distance = (v[0] - origin).dot(triangle.outward) / denominator;
        if !distance.is_finite() {
            return Err("medium boundary intersection overflow");
        }
        if distance <= minimum_distance_m || distance > best {
            continue;
        }
        let mut position = origin + direction * distance;
        // Reconstruct the dominant coordinate from the face plane. Merely
        // evaluating origin+t*direction can leave an axis-aligned hit a few
        // ulps outside the surface, producing a spurious second entry.
        // This is rounding the geometric hit, not moving the ray by a bias.
        let axis = (0..3)
            .max_by(|&a, &b| {
                triangle.outward[a]
                    .abs()
                    .total_cmp(&triangle.outward[b].abs())
            })
            .unwrap();
        let residual: f64 = (0..3)
            .filter(|&i| i != axis)
            .map(|i| triangle.outward[i] * (position[i] - v[0][i]))
            .sum();
        position[axis] = v[0][axis] - residual / triangle.outward[axis];
        if !position.is_finite() {
            return Err("medium boundary intersection overflow");
        }
        // Oriented edge tests against the geometric face normal. Shared
        // edge ties choose the lowest face index deterministically.
        if (0..3).any(|i| {
            (v[(i + 1) % 3] - v[i])
                .cross(position - v[i])
                .dot(triangle.outward)
                < 0.
        }) {
            continue;
        }
        if let Some(previous) = hit.as_ref() {
            if distance == best {
                let oriented = if denominator < 0. {
                    triangle.outward
                } else {
                    -triangle.outward
                };
                if reject_crease
                    && DVec3::from_array(previous.incident_normal).dot(oriented) < 1. - 1e-12
                {
                    return Err("ambiguous medium boundary edge crossing");
                }
                continue;
            }
        }
        let entering = denominator < 0.;
        best = distance;
        hit = Some(TriangleIntersection {
            distance_m: distance,
            position_m: position.to_array(),
            incident_normal: if entering {
                triangle.outward
            } else {
                -triangle.outward
            }
            .to_array(),
            entering,
            triangle_index,
        });
    }
    Ok(hit)
}
/// Immutable open or closed opaque triangles with prescribed two-sided outgoing
/// linear RGB radiance. This is not a diffuse/specular lighting solver.
#[derive(Debug)]
pub struct OpaqueRadianceMesh {
    triangles: Vec<Triangle>,
    lower: DVec3,
    upper: DVec3,
    radiance: [f64; 3],
}
impl OpaqueRadianceMesh {
    pub(crate) fn append_gpu_records(
        &self,
        words: &mut Vec<u32>,
        object: u32,
    ) -> Result<(), crate::ComputeError> {
        for triangle in &self.triangles {
            crate::medium_geometry_compute::append_triangle(
                words,
                triangle.vertices.map(|v| v.to_array()),
                self.radiance,
                [1, u32::MAX, u32::MAX, object],
            )?;
        }
        Ok(())
    }
    /// Geometry is a derivative of SceneMesh. Both sides terminate the ray;
    /// outgoing radiance is physical L, not reduced L/n^2 or reflectance.
    /// # Errors
    /// Triangle budget, invalid scale/radiance or degenerate geometry.
    pub fn from_scene_mesh(
        mesh: &crate::SceneMesh,
        metres_per_world_unit: f64,
        max_triangles: usize,
        radiance: [f64; 3],
    ) -> Result<Self, &'static str> {
        if radiance.iter().any(|x| !x.is_finite() || *x < 0.) {
            return Err("invalid opaque surface radiance");
        }
        let (triangles, lower, upper) =
            admit_triangles(mesh, metres_per_world_unit, max_triangles)?;
        Ok(Self {
            triangles,
            lower,
            upper,
            radiance,
        })
    }
    pub(crate) fn transport_metadata(&self) -> ([f64; 3], [f64; 3], usize, [f64; 3]) {
        (
            self.lower.to_array(),
            self.upper.to_array(),
            self.triangles.len(),
            self.radiance,
        )
    }
    pub(crate) fn first_distance(
        &self,
        origin: [f64; 3],
        direction: [f64; 3],
        maximum: f64,
    ) -> Result<Option<f64>, &'static str> {
        Ok(first_triangle_hit(
            &self.triangles,
            self.lower,
            self.upper,
            origin,
            direction,
            0.,
            maximum,
            false,
        )?
        .map(|h| h.distance_m))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    const WATER: OpticalMediumId = OpticalMediumId(17);
    const AIR: OpticalMediumId = OpticalMediumId(4);
    pub(crate) fn box_mesh(reverse: bool, open: bool) -> crate::SceneMesh {
        let positions = [
            [-10., -10., -1.],
            [10., -10., -1.],
            [10., 10., -1.],
            [-10., 10., -1.],
            [-10., -10., 1.],
            [10., -10., 1.],
            [10., 10., 1.],
            [-10., 10., 1.],
        ];
        let mut indices = vec![
            0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7,
            6, 3, 0, 4, 3, 4, 7,
        ];
        if reverse {
            for t in indices.chunks_exact_mut(3) {
                t.swap(1, 2);
            }
        }
        if open {
            indices.truncate(indices.len() - 3);
        }
        crate::SceneMesh::new(
            positions
                .into_iter()
                .map(|position| crate::SceneVertex {
                    position,
                    uv: [0.; 2],
                    color: [1.; 4],
                })
                .collect(),
            indices,
        )
        .unwrap()
    }
    #[test]
    fn real_entry_exit_snell_distance_and_absorbing_media() {
        let mesh = box_mesh(false, false);
        let boundary = MediumBoundaryMesh::from_scene_mesh(&mesh, WATER, AIR, 1., 12).unwrap();
        for degrees in [0_f64, 10., 15., 30., 45., 60.] {
            let angle = degrees.to_radians();
            let ray = [angle.sin(), 0., -angle.cos()];
            let entry = boundary
                .first_hit([0., 0., 3.], ray, 0., 100.)
                .unwrap()
                .unwrap();
            assert_eq!(entry.incident_medium, AIR);
            assert_eq!(entry.transmitted_medium, WATER);
            assert_eq!(entry.incident_normal, [0., 0., 1.]);
            assert!((entry.distance_m - 2. / angle.cos()).abs() < 1e-12);
            let sample =
                crate::dielectric_boundary_sample(ray, entry.incident_normal, 1., 1.333).unwrap();
            let inside = sample.transmitted_direction().unwrap();
            let exit = boundary
                .first_hit(entry.position_m, inside, 0., 100.)
                .unwrap()
                .unwrap();
            assert_eq!(exit.incident_medium, WATER);
            assert_eq!(exit.transmitted_medium, AIR);
            assert!((exit.position_m[2] + 1.).abs() < 1e-12);
            let cosine = (1. - (angle.sin() / 1.333).powi(2)).sqrt();
            assert!((exit.distance_m - 2. / cosine).abs() < 1e-12);
            assert!(
                (exit.position_m[0] - (2. * angle.tan() + 2. * (angle.sin() / 1.333) / cosine))
                    .abs()
                    < 1e-12
            );
            let exit_sample =
                crate::dielectric_boundary_sample(inside, exit.incident_normal, 1.333, 1.).unwrap();
            for (a, b) in exit_sample
                .transmitted_direction()
                .unwrap()
                .into_iter()
                .zip(ray)
            {
                assert!((a - b).abs() < 1e-12);
            }
            // The explicitly selected direct transmitted branch, with black
            // reflected branch radiance. Not the infinite-reflection solution.
            let fog =
                crate::OpticalSegment::homogeneous([0.1, 0.2, 0.3], [0.; 3], entry.distance_m)
                    .unwrap();
            let water =
                crate::OpticalSegment::homogeneous([0.4, 0.7, 1.], [0.; 3], exit.distance_m)
                    .unwrap();
            let far = fog.apply_rgba([1.; 4]).unwrap();
            let at_exit = exit_sample
                .camera_radiance([0.; 3], Some([far[0], far[1], far[2]]))
                .unwrap();
            let at_entry = water
                .apply_rgba([at_exit[0], at_exit[1], at_exit[2], 1.])
                .unwrap();
            let through = sample
                .camera_radiance([0.; 3], Some([at_entry[0], at_entry[1], at_entry[2]]))
                .unwrap();
            let actual = fog
                .apply_rgba([through[0], through[1], through[2], 1.])
                .unwrap();
            for axis in 0..3 {
                let expected = sample.transmitted_power_fraction()
                    * exit_sample.transmitted_power_fraction()
                    * (-[0.1, 0.2, 0.3][axis] * (4. / angle.cos())
                        - [0.4, 0.7, 1.][axis] * (2. / cosine))
                        .exp();
                assert!((actual[axis] - expected).abs() < 1e-12);
            }
        }
        let inside_ray = [80_f64.to_radians().sin(), 0., -80_f64.to_radians().cos()];
        let hit = boundary
            .first_hit([0.; 3], inside_ray, 0., 100.)
            .unwrap()
            .unwrap();
        assert_eq!(hit.incident_medium, WATER);
        let sample =
            crate::dielectric_boundary_sample(inside_ray, hit.incident_normal, 1.333, 1.).unwrap();
        assert!(sample.transmitted_direction().is_none());
        assert_eq!(sample.reflected_power_fraction(), 1.);
    }
    #[test]
    fn topology_budget_metric_scale_and_ranges_are_explicit() {
        let mesh = box_mesh(false, false);
        assert!(MediumBoundaryMesh::from_scene_mesh(&mesh, WATER, AIR, 1., 11).is_err());
        assert!(
            MediumBoundaryMesh::from_scene_mesh(&box_mesh(false, true), WATER, AIR, 1., 12)
                .is_err()
        );
        assert!(
            MediumBoundaryMesh::from_scene_mesh(&box_mesh(true, false), WATER, AIR, 1., 12)
                .is_err()
        );
        for scale in [0., -1., f64::INFINITY, f64::NAN] {
            assert!(MediumBoundaryMesh::from_scene_mesh(&mesh, WATER, AIR, scale, 12).is_err());
        }
        assert!(MediumBoundaryMesh::from_scene_mesh(&mesh, WATER, WATER, 1., 12).is_err());
        let boundary = MediumBoundaryMesh::from_scene_mesh(&mesh, WATER, AIR, 0.5, 12).unwrap();
        assert_eq!(
            boundary
                .first_hit([0., 0., 1.5], [0., 0., -1.], 0., 100.)
                .unwrap()
                .unwrap()
                .distance_m,
            1.
        );
        assert!(
            boundary
                .first_hit([0., 0., 1.5], [0., 0., -1.], 0., 0.9)
                .unwrap()
                .is_none()
        );
        // Explicit minimum excludes entry and exposes exit; no implicit bias.
        assert_eq!(
            boundary
                .first_hit([0., 0., 1.5], [0., 0., -1.], 1., 100.)
                .unwrap()
                .unwrap()
                .distance_m,
            2.
        );
        assert!(
            boundary
                .first_hit([6., 0., 1.5], [0., 0., -1.], 0., 100.)
                .unwrap()
                .is_none()
        );
        assert!(boundary.first_hit([0.; 3], [0.; 3], 0., 1.).is_err());
        assert!(boundary.first_hit([0.; 3], [0., 0., -1.], -1., 1.).is_err());
        assert!(
            boundary
                .first_hit([0.; 3], [0., 0., -1.], 0., f64::INFINITY)
                .is_err()
        );
        let mut flipped = box_mesh(false, false).indices().to_vec();
        flipped.swap(0, 1);
        let inconsistent = crate::SceneMesh::new(mesh.vertices().to_vec(), flipped).unwrap();
        assert!(MediumBoundaryMesh::from_scene_mesh(&inconsistent, WATER, AIR, 1., 12).is_err());
    }
    #[test]
    fn disconnected_inward_components_nonmanifold_edges_and_creases_are_rejected() {
        let mesh = box_mesh(false, false);
        let small_inward = box_mesh(true, false);
        let mut vertices = mesh.vertices().to_vec();
        vertices.extend(small_inward.vertices().iter().copied().map(|mut v| {
            v.position = [
                v.position[0] * 0.25 + 100.,
                v.position[1] * 0.25,
                v.position[2] * 0.25,
            ];
            v
        }));
        let mut indices = mesh.indices().to_vec();
        indices.extend(small_inward.indices().iter().map(|i| i + 8));
        let combined = crate::SceneMesh::new(vertices, indices).unwrap();
        assert_eq!(
            MediumBoundaryMesh::from_scene_mesh(&combined, WATER, AIR, 1., 24).unwrap_err(),
            "inward or zero-volume medium boundary component"
        );
        let mut duplicated = mesh.indices().to_vec();
        duplicated.extend_from_slice(&mesh.indices()[..3]);
        let nonmanifold = crate::SceneMesh::new(mesh.vertices().to_vec(), duplicated).unwrap();
        assert_eq!(
            MediumBoundaryMesh::from_scene_mesh(&nonmanifold, WATER, AIR, 1., 13).unwrap_err(),
            "nonmanifold medium boundary"
        );
        let boundary = MediumBoundaryMesh::from_scene_mesh(&mesh, WATER, AIR, 1., 12).unwrap();
        let component = -1. / 2_f64.sqrt();
        assert_eq!(
            boundary
                .first_hit([11., 0., 2.], [component, 0., component], 0., 100.)
                .unwrap_err(),
            "ambiguous medium boundary edge crossing"
        );
    }
}
