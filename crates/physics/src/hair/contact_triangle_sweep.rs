//! Continuous surface query for a capsule and a linearly deforming triangle.
//! This does not classify containment in a closed volume or integrate a rod.
use super::continuous::{CapsuleMotion, CapsuleSweep, CapsuleSweepOptions, conservative_advance};
use super::polynomial::*;
use super::{Triangle, TriangleMesh, closest_triangle, segment_pair, segment_triangle};
use crate::hair::math::*;
#[path = "contact_trajectory.rs"]
mod trajectory;
pub(in crate::hair) use trajectory::trajectory_contact;

#[derive(Clone, Copy, Debug)]
pub struct TriangleMotion {
    pub start: [V; 3],
    pub end: [V; 3],
}

fn validate_capsule(
    capsule: CapsuleMotion,
    options: CapsuleSweepOptions,
) -> Result<(), &'static str> {
    if !capsule.radius.is_finite()
        || capsule.radius <= 0.
        || !options.tolerance_m.is_finite()
        || options.tolerance_m <= 0.
        || options.max_iterations == 0
        || capsule
            .start
            .iter()
            .chain(&capsule.end)
            .any(|p| !finite(*p))
    {
        return Err("invalid capsule triangle sweep");
    }
    let scale = capsule
        .start
        .iter()
        .chain(&capsule.end)
        .flatten()
        .map(|x| x.abs())
        .fold(capsule.radius, f64::max);
    if options.tolerance_m < 64. * f64::EPSILON * scale {
        return Err("triangle sweep tolerance is below coordinate precision; recenter the query");
    }
    Ok(())
}

#[derive(Debug)]
pub(super) struct MotionBounds {
    faces: Vec<(V, V)>,
    nodes: Vec<(V, V)>,
}
impl TriangleMesh {
    fn cached_motion_bounds(&self) -> &MotionBounds {
        self.motion_bounds
            .get_or_init(|| std::sync::Arc::new(self.build_motion_bounds()))
            .as_ref()
    }
    fn build_motion_bounds(&self) -> MotionBounds {
        let bounds = |face: &Triangle| {
            let lo = std::array::from_fn(|axis| {
                face.p
                    .iter()
                    .chain(&face.previous_p)
                    .map(|p| p[axis])
                    .fold(f64::INFINITY, f64::min)
            });
            let hi = std::array::from_fn(|axis| {
                face.p
                    .iter()
                    .chain(&face.previous_p)
                    .map(|p| p[axis])
                    .fold(f64::NEG_INFINITY, f64::max)
            });
            (lo, hi)
        };
        let faces: Vec<(V, V)> = self.triangles.iter().map(bounds).collect();
        let mut nodes = vec![([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]); self.nodes.len()];
        // BVH children are allocated after parents. Reuse that topology without
        // changing/refitting the static mesh or invalidating any distance cache.
        for i in (0..self.nodes.len()).rev() {
            let node = &self.nodes[i];
            let mut bound = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
            let mut extend = |(lo, hi): (V, V)| {
                for axis in 0..3 {
                    bound.0[axis] = bound.0[axis].min(lo[axis]);
                    bound.1[axis] = bound.1[axis].max(hi[axis]);
                }
            };
            if let Some(children) = node.children {
                for child in children {
                    extend(nodes[child]);
                }
            } else {
                for face in node.range.clone() {
                    extend(faces[face]);
                }
            }
            nodes[i] = bound;
        }
        MotionBounds { faces, nodes }
    }
    pub(in crate::hair) fn face_motion(&self, ids: [usize; 3]) -> Option<TriangleMotion> {
        self.triangles
            .iter()
            .find(|face| face.ids == ids)
            .map(|face| TriangleMotion {
                start: face.previous_p,
                end: face.p,
            })
    }
    /// All non-clear continuous surface queries in canonical capsule/face order.
    /// Face motion spans `previous_p` to the current collider vertices. Builds
    /// temporal bounds once per batch using the existing immutable BVH topology.
    /// Does not certify closed-volume containment or integrate contact responses.
    pub fn swept_capsule_contacts(
        &self,
        motions: &[CapsuleMotion],
        options: CapsuleSweepOptions,
    ) -> Result<Vec<(usize, [usize; 3], CapsuleSweep)>, &'static str> {
        let mut result: Vec<_> = self
            .swept_capsule_queries(motions, options)?
            .into_iter()
            .map(|(i, face, query)| (i, self.triangles[face].ids, query))
            .collect();
        result.sort_by_key(|(i, face, _)| (*i, *face));
        Ok(result)
    }
    pub(in crate::hair) fn trajectory_constraints(
        &self,
        motions: &[CapsuleMotion],
        options: CapsuleSweepOptions,
    ) -> Result<Vec<(usize, trajectory::TrajectoryContact)>, &'static str> {
        let mut result = Vec::new();
        for (i, face, _) in self.swept_capsule_queries(motions, options)? {
            let face = &self.triangles[face];
            if let Some(contact) = trajectory::trajectory_contact_oriented(
                motions[i],
                TriangleMotion {
                    start: face.previous_p,
                    end: face.p,
                },
                options,
                self.feature_normals.is_some(),
            )? {
                result.push((i, contact));
            }
        }
        Ok(result)
    }
    fn swept_capsule_queries(
        &self,
        motions: &[CapsuleMotion],
        options: CapsuleSweepOptions,
    ) -> Result<Vec<(usize, usize, CapsuleSweep)>, &'static str> {
        for &motion in motions {
            validate_capsule(motion, options)?;
        }
        if !options.tolerance_m.is_finite()
            || options.tolerance_m <= 0.
            || options.max_iterations == 0
        {
            return Err("invalid capsule triangle sweep");
        }
        let MotionBounds { faces, nodes } = self.cached_motion_bounds();
        let mut result = Vec::new();
        let mut stack = Vec::with_capacity(64);
        for (index, &motion) in motions.iter().enumerate() {
            let mut scale = motion.radius;
            let mut min_p = motion.start[0];
            let mut max_p = motion.start[0];
            for p in [motion.start[0], motion.start[1], motion.end[0], motion.end[1]] {
                for axis in 0..3 {
                    min_p[axis] = min_p[axis].min(p[axis]);
                    max_p[axis] = max_p[axis].max(p[axis]);
                    scale = scale.max(p[axis].abs());
                }
            }
            let margin = motion.radius + options.tolerance_m + 64. * f64::EPSILON * scale;
            let lo: V = [min_p[0] - margin, min_p[1] - margin, min_p[2] - margin];
            let hi: V = [max_p[0] + margin, max_p[1] + margin, max_p[2] + margin];
            if !finite(lo) || !finite(hi) {
                return Err("triangle sweep bounds overflow");
            }
            let intersects =
                |(a, b): (V, V)| (0..3).all(|axis| lo[axis] <= b[axis] && hi[axis] >= a[axis]);
            stack.clear();
            stack.push(0);
            while let Some(i) = stack.pop() {
                if !intersects(nodes[i]) {
                    continue;
                }
                let node = &self.nodes[i];
                if let Some(children) = node.children {
                    stack.extend(children);
                    continue;
                }
                for face in node.range.clone().filter(|&face| intersects(faces[face])) {
                    let triangle = &self.triangles[face];
                    let query = sweep_capsule_triangle(
                        motion,
                        TriangleMotion {
                            start: triangle.previous_p,
                            end: triangle.p,
                        },
                        options,
                    )?;
                    if query != CapsuleSweep::Clear {
                        result.push((index, face, query));
                    }
                }
            }
        }
        result.sort_by_key(|(index, face, _)| (*index, *face));
        Ok(result)
    }
}

fn closest_points(
    capsule: CapsuleMotion,
    triangle: TriangleMotion,
    fraction: f64,
) -> Result<(V, V), &'static str> {
    let points: [V; 2] = std::array::from_fn(|i| {
        add(
            capsule.start[i],
            mul(sub(capsule.end[i], capsule.start[i]), fraction),
        )
    });
    let vertices: [V; 3] = std::array::from_fn(|i| {
        add(
            triangle.start[i],
            mul(sub(triangle.end[i], triangle.start[i]), fraction),
        )
    });
    let normal = cross(sub(vertices[1], vertices[0]), sub(vertices[2], vertices[0]));
    let pair = if len(normal) == 0. {
        // A collapsed triangle is still a union of edges: never silently
        // remove a deforming collider when its area vanishes.
        (0..3)
            .map(|i| {
                let (_, _, p, q) =
                    segment_pair(points[0], points[1], vertices[i], vertices[(i + 1) % 3]);
                (p, q)
            })
            .min_by(|(p, q), (a, b)| len(sub(*p, *q)).total_cmp(&len(sub(*a, *b))))
            .unwrap()
    } else {
        let face = Triangle {
            ids: [0, 1, 2],
            p: vertices,
            previous_p: vertices,
            velocity: [[0.; 3]; 3],
            normal: mul(normal, 1. / len(normal)),
            min: [0.; 3],
            max: [0.; 3],
        };
        // Existing closest-feature geometry owns both static and swept queries.
        let (_, p, q) = segment_triangle(points[0], points[1], &face);
        // Explicit endpoint checks also detect non-finite closest-feature math.
        if !finite(closest_triangle(points[0], &face))
            || !finite(closest_triangle(points[1], &face))
        {
            return Err("triangle sweep closest feature overflow");
        }
        (p, q)
    };
    if finite(pair.0) && finite(pair.1) {
        Ok(pair)
    } else {
        Err("triangle sweep closest feature overflow")
    }
}

fn gap(
    capsule: CapsuleMotion,
    triangle: TriangleMotion,
    fraction: f64,
) -> Result<f64, &'static str> {
    let (p, q) = closest_points(capsule, triangle, fraction)?;
    let value = len(sub(p, q)) - capsule.radius;
    if value.is_finite() {
        Ok(value)
    } else {
        Err("triangle sweep distance overflow")
    }
}

// A touching/sliding capsule can be admitted only when its WHOLE trajectory
// stays on one side of the moving infinite face plane within the same physical
// tolerance. Outward-rounded cubic signed distances and degree-six squared
// clearance bounds avoid accepting a few samples or a separating endpoint.
fn fixed_axis_clear(capsule: CapsuleMotion, triangle: TriangleMotion, tolerance: f64) -> bool {
    fixed_axis_clear_at(capsule, triangle, tolerance, 0.)
}
fn fixed_axis_clear_at(
    capsule: CapsuleMotion,
    triangle: TriangleMotion,
    tolerance: f64,
    axis_time: f64,
) -> bool {
    let Ok((p, q)) = closest_points(capsule, triangle, axis_time) else {
        return false;
    };
    let axis = sub(p, q);
    let mut squared = Bound::exact(0.);
    for x in axis {
        squared = squared.add(Bound::exact(x).mul(Bound::exact(x)));
    }
    if !squared.hi.is_finite() || squared.lo <= 0. {
        return false;
    }
    let radius = Bound::exact(capsule.radius).sub(Bound::exact(tolerance));
    if radius.lo <= 0. {
        return false;
    }
    let required = radius.mul(Bound::exact(squared.hi.sqrt().next_up()));
    // Each endpoint/vertex relative projection is affine in time. Bounding
    // BOTH endpoint states proves the bound everywhere, including all convex
    // barycentric points of the moving segment and the finite triangle.
    for (capsule, triangle) in [(capsule.start, triangle.start), (capsule.end, triangle.end)] {
        for p in capsule {
            for q in triangle {
                let mut projection = Bound::exact(0.);
                for i in 0..3 {
                    projection = projection.add(
                        Bound::exact(p[i])
                            .sub(Bound::exact(q[i]))
                            .mul(Bound::exact(axis[i])),
                    );
                }
                if !projection.lo.is_finite() || projection.lo < required.hi {
                    return false;
                }
            }
        }
    }
    true
}

// A changing separating axis is sufficient for all convex points of both
// primitives. Axis endpoints are arbitrary finite vectors; their construction
// need not be exact because the complete projection inequalities are enclosed.
fn moving_axis_clear_interval(
    capsule: CapsuleMotion,
    triangle: TriangleMotion,
    tolerance: f64,
    budget: usize,
    lo: f64,
    hi: f64,
) -> bool {
    let interpolate = |a: V, b: V, t: f64| add(mul(a, 1. - t), mul(b, t));
    let c = CapsuleMotion {
        start: std::array::from_fn(|i| interpolate(capsule.start[i], capsule.end[i], lo)),
        end: std::array::from_fn(|i| interpolate(capsule.start[i], capsule.end[i], hi)),
        radius: capsule.radius,
    };
    let t = TriangleMotion {
        start: std::array::from_fn(|i| interpolate(triangle.start[i], triangle.end[i], lo)),
        end: std::array::from_fn(|i| interpolate(triangle.start[i], triangle.end[i], hi)),
    };
    let (Ok((p0, q0)), Ok((pm, qm)), Ok((p1, q1))) = (
        closest_points(c, t, 0.),
        closest_points(c, t, 0.5),
        closest_points(c, t, 1.),
    ) else {
        return false;
    };
    let first = sub(p0, q0);
    let middle = sub(pm, qm);
    let last = sub(p1, q1);
    // These floating coefficients define an arbitrary axis polynomial. Only
    // the enclosed projection proof grants clearance, not closest queries.
    let axis: [Poly; 3] = std::array::from_fn(|i| {
        let mut p = zero();
        p[0] = Bound::exact(first[i]);
        p[1] = Bound::exact(4. * middle[i] - 3. * first[i] - last[i]);
        p[2] = Bound::exact(2. * first[i] - 4. * middle[i] + 2. * last[i]);
        p
    });
    let mut norm = zero();
    for a in axis {
        norm = sum(norm, product(a, 2, a, 2));
    }
    let radius = Bound::exact(capsule.radius).sub(Bound::exact(tolerance));
    if radius.lo <= 0. {
        return false;
    }
    let squared_radius = radius.mul(radius);
    let mut conditions = Vec::with_capacity(13);
    conditions.push((norm, 4, true));
    for endpoint in 0..2 {
        for vertex in 0..3 {
            let mut projection = zero();
            for i in 0..3 {
                // Restrict the ORIGINAL relative affine motion with outward
                // arithmetic. Rounded resampled positions never enter this proof;
                // common translation cancels before interpolation.
                let first = Bound::exact(capsule.start[endpoint][i])
                    .sub(Bound::exact(triangle.start[vertex][i]));
                let last = Bound::exact(capsule.end[endpoint][i])
                    .sub(Bound::exact(triangle.end[vertex][i]));
                let slope = last.sub(first);
                let mut relative = zero();
                relative[0] = first.add(slope.mul(Bound::exact(lo)));
                relative[1] = slope.mul(Bound::exact(hi).sub(Bound::exact(lo)));
                projection = sum(projection, product(relative, 1, axis[i], 2));
            }
            let clearance = super::polynomial::difference(
                product(projection, 3, projection, 3),
                norm.map(|v| v.mul(squared_radius)),
            );
            conditions.push((projection, 3, false));
            conditions.push((clearance, 6, false));
        }
    }
    certify_nonnegative(&conditions, budget)
}

// Cover the ENTIRE authoritative linear motion with locally separating axes.
// Re-sampling always uses original endpoints; a coordinate error allowance
// tightens each local certificate so rounded interval endpoints cannot admit
// an unsafe exact trajectory. Failed coverage is unknown, never clearance.
fn adaptive_axis_clear(
    capsule: CapsuleMotion,
    triangle: TriangleMotion,
    tolerance: f64,
    budget: usize,
) -> bool {
    let scale = capsule
        .start
        .iter()
        .chain(&capsule.end)
        .chain(&triangle.start)
        .chain(&triangle.end)
        .flatten()
        .map(|x| x.abs())
        .fold(capsule.radius, f64::max);
    // Two three-component lerps and their convex interpolation errors fit
    // within this padding; Bound arithmetic encloses support products/sums.
    let padding = (256. * f64::EPSILON * scale).next_up();
    let local_tolerance = (tolerance - padding).next_down();
    if !local_tolerance.is_finite() || local_tolerance <= 0. {
        return false;
    }
    let mut stack = vec![(0., 1., 0usize)];
    let mut visited = 0;
    while let Some((lo, hi, depth)) = stack.pop() {
        visited += 1;
        if visited > budget {
            return false;
        }
        let interpolate = |a: V, b: V, t: f64| add(mul(a, 1. - t), mul(b, t));
        let c = CapsuleMotion {
            start: std::array::from_fn(|i| interpolate(capsule.start[i], capsule.end[i], lo)),
            end: std::array::from_fn(|i| interpolate(capsule.start[i], capsule.end[i], hi)),
            radius: capsule.radius,
        };
        let t = TriangleMotion {
            start: std::array::from_fn(|i| interpolate(triangle.start[i], triangle.end[i], lo)),
            end: std::array::from_fn(|i| interpolate(triangle.start[i], triangle.end[i], hi)),
        };
        if fixed_axis_clear_at(c, t, local_tolerance, 0.5)
            || moving_axis_clear_interval(capsule, triangle, tolerance, 32, lo, hi)
        {
            continue;
        }
        if depth >= 48 {
            return false;
        }
        let middle = (lo + hi) * 0.5;
        if gap(capsule, triangle, middle).map_or(true, |distance| distance < -tolerance) {
            return false;
        }
        stack.push((middle, hi, depth + 1));
        stack.push((lo, middle, depth + 1));
    }
    true
}

fn supporting_plane_clear(
    capsule: CapsuleMotion,
    triangle: TriangleMotion,
    tolerance: f64,
    budget: usize,
) -> bool {
    let difference = |a: V, b: V, c: V, d: V| -> [Poly; 3] {
        std::array::from_fn(|axis| {
            let mut p = zero();
            let first = Bound::exact(a[axis]).sub(Bound::exact(c[axis]));
            let last = Bound::exact(b[axis]).sub(Bound::exact(d[axis]));
            p[0] = first;
            p[1] = last.sub(first);
            p
        })
    };
    let a = difference(
        triangle.start[1],
        triangle.end[1],
        triangle.start[0],
        triangle.end[0],
    );
    let b = difference(
        triangle.start[2],
        triangle.end[2],
        triangle.start[0],
        triangle.end[0],
    );
    let normal: [Poly; 3] = std::array::from_fn(|i| {
        super::polynomial::difference(
            product(a[(i + 1) % 3], 1, b[(i + 2) % 3], 1),
            product(a[(i + 2) % 3], 1, b[(i + 1) % 3], 1),
        )
    });
    let mut norm = zero();
    for n in normal {
        norm = sum(norm, product(n, 2, n, 2));
    }
    let radius = Bound::exact(capsule.radius).sub(Bound::exact(tolerance));
    if radius.lo <= 0. || !radius.hi.is_finite() {
        return false;
    }
    let radius_squared = radius.mul(radius);
    let first_normal = cross(
        sub(triangle.start[1], triangle.start[0]),
        sub(triangle.start[2], triangle.start[0]),
    );
    let sign = if dot(sub(capsule.start[0], triangle.start[0]), first_normal) < 0. {
        -1.
    } else {
        1.
    };
    let mut conditions = Vec::with_capacity(5);
    conditions.push((norm, 4, true));
    for endpoint in 0..2 {
        let relative = difference(
            capsule.start[endpoint],
            capsule.end[endpoint],
            triangle.start[0],
            triangle.end[0],
        );
        let mut signed = zero();
        for i in 0..3 {
            signed = sum(signed, product(relative[i], 1, normal[i], 2));
        }
        if sign < 0. {
            signed = signed.map(Bound::neg);
        }
        let clearance = super::polynomial::difference(
            product(signed, 3, signed, 3),
            norm.map(|coefficient| coefficient.mul(radius_squared)),
        );
        conditions.push((signed, 3, false));
        conditions.push((clearance, 6, false));
    }
    certify_nonnegative(&conditions, budget)
}

/// Conservative safe fraction for an entire capsule centreline and moving face.
/// Relative endpoint/vertex velocities bound every barycentric relative speed.
/// `InitialContact` and `IterationLimit` never certify an unrestricted step.
/// A touching trajectory may return `Clear` only with a continuous supporting-
/// plane certificate at radius minus the requested physical tolerance.
pub fn sweep_capsule_triangle(
    capsule: CapsuleMotion,
    triangle: TriangleMotion,
    options: CapsuleSweepOptions,
) -> Result<CapsuleSweep, &'static str> {
    validate_capsule(capsule, options)?;
    if triangle
        .start
        .iter()
        .chain(&triangle.end)
        .any(|p| !finite(*p))
    {
        return Err("invalid capsule triangle sweep");
    }
    let mut coordinate_scale = capsule.radius;
    for p in [capsule.start[0], capsule.start[1], capsule.end[0], capsule.end[1]] {
        for axis in 0..3 {
            coordinate_scale = coordinate_scale.max(p[axis].abs());
        }
    }
    for p in [triangle.start[0], triangle.start[1], triangle.start[2], triangle.end[0], triangle.end[1], triangle.end[2]] {
        for axis in 0..3 {
            coordinate_scale = coordinate_scale.max(p[axis].abs());
        }
    }
    let roundoff = 64. * f64::EPSILON * coordinate_scale;
    if options.tolerance_m < roundoff {
        return Err("triangle sweep tolerance is below coordinate precision; recenter the query");
    }
    let va: [V; 2] = [sub(capsule.end[0], capsule.start[0]), sub(capsule.end[1], capsule.start[1])];
    let vb: [V; 3] = [sub(triangle.end[0], triangle.start[0]), sub(triangle.end[1], triangle.start[1]), sub(triangle.end[2], triangle.start[2])];
    let mut max_speed2 = 0.0f64;
    for a in &va {
        for b in &vb {
            let d = sub(*a, *b);
            let d2 = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
            max_speed2 = max_speed2.max(d2);
        }
    }
    let speed = max_speed2.sqrt() * (1. + 16. * f64::EPSILON);
    if !speed.is_finite() {
        return Err("triangle sweep motion overflow");
    }
    let query = conservative_advance(speed, roundoff, options, |t| gap(capsule, triangle, t))?;
    if query != CapsuleSweep::Clear
        && (fixed_axis_clear(capsule, triangle, options.tolerance_m)
            || supporting_plane_clear(
                capsule,
                triangle,
                options.tolerance_m,
                options.max_iterations,
            )
            || adaptive_axis_clear(
                capsule,
                triangle,
                options.tolerance_m,
                options.max_iterations,
            ))
    {
        Ok(CapsuleSweep::Clear)
    } else {
        Ok(query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn face(z: f64) -> [V; 3] {
        [[-2., -2., z], [2., -2., z], [0., 2., z]]
    }
    fn capsule(z: f64) -> [V; 2] {
        [[-0.1, 0., z], [0.1, 0., z]]
    }
    #[test]
    fn cached_motion_bounds_follow_all_geometry_owners() {
        let motions = [CapsuleMotion {
            start: capsule(0.),
            end: capsule(0.),
            radius: 0.01,
        }];
        let query = |mesh: &TriangleMesh| {
            mesh.swept_capsule_contacts(&motions, Default::default())
                .unwrap()
        };
        let original = TriangleMesh::new(&face(-3.), &[[0, 1, 2]]).unwrap();
        assert!(query(&original).is_empty());
        let mut moving = original.clone();
        moving.refit_with_timestep(&face(3.), 1.).unwrap();
        assert!(
            !query(&moving).is_empty(),
            "warm cloned bounds hid a moving face"
        );
        assert!(
            query(&original).is_empty(),
            "clone refit changed the original collider"
        );
        assert!(query(&moving.motion_interval(0., 0.25).unwrap()).is_empty());
        assert!(!query(&moving.motion_interval(0.25, 0.75).unwrap()).is_empty());
        let translated = original
            .transformed([[1., 0., 0., 0.], [0., 1., 0., 0.], [0., 0., 1., 3.]])
            .unwrap();
        assert!(
            !query(&translated).is_empty(),
            "transform reused stale temporal bounds"
        );
        let mut restored = TriangleMesh::new(&face(3.), &[[0, 1, 2]]).unwrap();
        assert!(query(&restored).is_empty());
        restored.restore_replay_motion(&face(-3.), &[[0.; 3]; 3]);
        assert!(
            !query(&restored).is_empty(),
            "replay restoration reused current-only bounds"
        );
        let mut refitted = original.clone();
        assert!(refitted.refit(&[[f64::NAN; 3]; 3]).is_err());
        assert!(
            query(&refitted).is_empty(),
            "rejected refit changed collider motion"
        );
        refitted.refit(&face(3.)).unwrap();
        assert!(
            !query(&refitted).is_empty(),
            "untimed raw refit lost its previous endpoints"
        );
    }
    #[test]
    fn moving_mesh_bvh_matches_exhaustive_face_queries() {
        let mut vertices = Vec::new();
        let mut previous = Vec::new();
        let mut indices = Vec::new();
        for i in 0..24 {
            let offset = [10. * i as f64, 0., 0.];
            indices.push(std::array::from_fn(|j| vertices.len() + j));
            vertices.extend(face(1.).map(|p| add(p, offset)));
            previous.extend(face(-1.).map(|p| add(p, offset)));
        }
        let mut mesh = TriangleMesh::new(&vertices, &indices).unwrap();
        mesh.restore_replay_motion(&previous, &vec![[0.; 3]; vertices.len()]);
        let motions: Vec<_> = [0., 10., 1000.]
            .into_iter()
            .map(|x| {
                let points = capsule(0.).map(|p| add(p, [x, 0., 0.]));
                CapsuleMotion {
                    start: points,
                    end: points,
                    radius: 0.01,
                }
            })
            .collect();
        // Current-only triangle bounds are at z=1 and exclude all the rods.
        let mut current_candidates = Vec::new();
        mesh.query(
            [-1., -1., -0.02],
            [1., 1., 0.02],
            0,
            &mut current_candidates,
        );
        assert!(current_candidates.is_empty());
        let actual = mesh
            .swept_capsule_contacts(&motions, Default::default())
            .unwrap();
        let mut expected = Vec::new();
        for (i, &motion) in motions.iter().enumerate() {
            for t in &mesh.triangles {
                let query = sweep_capsule_triangle(
                    motion,
                    TriangleMotion {
                        start: t.previous_p,
                        end: t.p,
                    },
                    Default::default(),
                )
                .unwrap();
                if query != CapsuleSweep::Clear {
                    expected.push((i, t.ids, query));
                }
            }
        }
        expected.sort_by_key(|(index, face, _)| (*index, *face));
        assert_eq!(actual, expected);
        assert_eq!(actual.len(), 2);
        assert!(
            actual
                .iter()
                .all(|(_, _, query)| matches!(query, CapsuleSweep::Approach { .. }))
        );
    }
    #[test]
    fn invalid_mesh_batch_is_rejected_before_culling() {
        let mesh = TriangleMesh::new(&face(0.), &[[0, 1, 2]]).unwrap();
        let mut c = CapsuleMotion {
            start: capsule(100.),
            end: capsule(100.),
            radius: 0.01,
        };
        c.end[0][0] = f64::NAN;
        assert!(
            mesh.swept_capsule_contacts(&[c], Default::default())
                .is_err()
        );
        assert!(
            mesh.swept_capsule_contacts(
                &[],
                CapsuleSweepOptions {
                    max_iterations: 0,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    #[test]
    fn catches_crossing_with_clear_endpoint_states() {
        let c = CapsuleMotion {
            start: capsule(1.),
            end: capsule(-1.),
            radius: 0.01,
        };
        let t = TriangleMotion {
            start: face(0.),
            end: face(0.),
        };
        assert!(gap(c, t, 0.).unwrap() > 0. && gap(c, t, 1.).unwrap() > 0.);
        let CapsuleSweep::Approach {
            fraction, gap_m, ..
        } = sweep_capsule_triangle(c, t, Default::default()).unwrap()
        else {
            panic!("must retain safe prefix")
        };
        assert!(fraction < 0.495 && fraction > 0.49499999);
        assert!(gap_m >= 0. && gap_m <= 2e-10);
    }
    #[test]
    fn captured_near_tolerance_slide_uses_original_motion_bounds() {
        let c = CapsuleMotion {
            start: [
                [
                    0.09555132078136536,
                    0.5009054421709297,
                    -0.06957728072965613,
                ],
                [
                    0.09931814496478981,
                    0.47274014744110626,
                    -0.07282513217037156,
                ],
            ],
            end: [
                [
                    0.09562983782583627,
                    0.49959910046077904,
                    -0.06965831213880862,
                ],
                [
                    0.09931357676134894,
                    0.47140059711702226,
                    -0.07271631026778565,
                ],
            ],
            radius: 4e-05,
        };
        let t = TriangleMotion {
            start: [
                [
                    0.09505069255828857,
                    0.47713495790958405,
                    -0.07312288135290146,
                ],
                [
                    0.09658217802643776,
                    0.4906931519508362,
                    -0.07076999917626381,
                ],
                [
                    0.10186338424682617,
                    0.4817485362291336,
                    -0.07088272273540497,
                ],
            ],
            end: [
                [
                    0.09505069255828857,
                    0.4759099781513214,
                    -0.07304751127958298,
                ],
                [
                    0.09658218175172806,
                    0.48946648836135864,
                    -0.07069220393896103,
                ],
                [
                    0.10186338424682617,
                    0.4805228114128113,
                    -0.07080645114183426,
                ],
            ],
        };
        assert!(adaptive_axis_clear(c, t, 1e-10, 1024));
        assert!(matches!(
            sweep_capsule_triangle(c, t, Default::default()).unwrap(),
            CapsuleSweep::Clear
        ));
        assert!(!adaptive_axis_clear(
            CapsuleMotion {
                radius: c.radius + 1e-12,
                ..c
            },
            t,
            1e-10,
            1024
        ));
    }
    #[test]
    fn captured_initial_edge_contact_has_continuous_clearance() {
        let c = CapsuleMotion {
            start: [
                [0.10600332119695308, 0.53506232368008, 0.02187988144117896],
                [0.10796079282719731, 0.5228928932727485, 0.03089289073304174],
            ],
            end: [
                [
                    0.10607706365455617,
                    0.5347538323441486,
                    0.022093347415137084,
                ],
                [
                    0.10810187751619427,
                    0.5225468454425485,
                    0.031040577220530503,
                ],
            ],
            radius: 4e-05,
        };
        let t = TriangleMotion {
            start: [
                [
                    0.10320580005645752,
                    0.5298599600791931,
                    0.027216115966439247,
                ],
                [0.1104968786239624, 0.5253394246101379, 0.027562154456973076],
                [
                    0.11163926124572754,
                    0.5299567580223083,
                    0.023570355027914047,
                ],
            ],
            end: [
                [
                    0.10320580005645752,
                    0.5298187136650085,
                    0.027216115966439247,
                ],
                [0.1104968786239624, 0.5252981781959534, 0.027562154456973076],
                [
                    0.11163926124572754,
                    0.5299155116081238,
                    0.023570355027914047,
                ],
            ],
        };
        assert!(adaptive_axis_clear(c, t, 1e-10, 1024));
        assert!(matches!(
            sweep_capsule_triangle(c, t, Default::default()).unwrap(),
            CapsuleSweep::Clear
        ));
        let penetrating = CapsuleMotion {
            radius: c.radius + 1e-8,
            ..c
        };
        assert!(!adaptive_axis_clear(penetrating, t, 1e-10, 1024));
    }
    #[test]
    fn moving_face_crosses_stationary_hair() {
        let c = CapsuleMotion {
            start: capsule(0.),
            end: capsule(0.),
            radius: 0.01,
        };
        let t = TriangleMotion {
            start: face(-1.),
            end: face(1.),
        };
        assert!(
            matches!(sweep_capsule_triangle(c,t,Default::default()).unwrap(),CapsuleSweep::Approach {fraction,..} if fraction<0.495)
        );
    }
    #[test]
    fn common_translation_is_clear() {
        let c = CapsuleMotion {
            start: capsule(1.),
            end: capsule(11.),
            radius: 0.01,
        };
        let t = TriangleMotion {
            start: face(0.),
            end: face(10.),
        };
        assert_eq!(
            sweep_capsule_triangle(c, t, Default::default()).unwrap(),
            CapsuleSweep::Clear
        );
    }
    #[test]
    fn initial_contact_requires_a_continuous_clearance_certificate() {
        let t = TriangleMotion {
            start: face(0.),
            end: face(0.),
        };
        let c = CapsuleMotion {
            start: capsule(0.01),
            end: capsule(0.01).map(|p| add(p, [0.2, 0., 0.])),
            radius: 0.01,
        };
        assert_eq!(
            sweep_capsule_triangle(c, t, Default::default()).unwrap(),
            CapsuleSweep::Clear
        );
        let mut entering = c;
        entering.end = capsule(-0.01);
        assert!(matches!(
            sweep_capsule_triangle(entering, t, Default::default()).unwrap(),
            CapsuleSweep::InitialContact { .. }
        ));
        assert!(!supporting_plane_clear(c, t, 1e-10, 0));
        let mut penetrating = c;
        penetrating.start = capsule(0.01 - 2e-10);
        assert!(!supporting_plane_clear(penetrating, t, 1e-10, 1024));
    }
    #[test]
    fn approach_to_touching_endpoint_requires_the_same_full_path_certificate() {
        let triangle = TriangleMotion {
            start: face(0.),
            end: face(0.),
        };
        let capsule = CapsuleMotion {
            start: capsule(1.),
            end: capsule(0.01),
            radius: 0.01,
        };
        assert_eq!(
            sweep_capsule_triangle(capsule, triangle, Default::default()).unwrap(),
            CapsuleSweep::Clear
        );
        let entering = CapsuleMotion {
            end: capsule.end.map(|p| add(p, [0., 0., -2e-10])),
            ..capsule
        };
        assert_ne!(
            sweep_capsule_triangle(entering, triangle, Default::default()).unwrap(),
            CapsuleSweep::Clear
        );
    }
    #[test]
    fn local_axis_certificates_cover_changing_features_without_sampling_admission() {
        let c = CapsuleMotion {
            start: [[0.4, 0.4, 0.], [0.41, 0.4, 0.]],
            end: [[0.4, 0.4, 0.], [0.41, 0.4, 0.]],
            radius: 0.01,
        };
        let t = TriangleMotion {
            start: [[-1., 0., 0.], [1., 0., 0.], [0., 0., 0.]],
            end: [[0., 1., 0.], [0., -1., 0.], [0., 0., 0.]],
        };
        assert!(!fixed_axis_clear_at(c, t, 1e-10, 0.5));
        assert!(
            !adaptive_axis_clear(c, t, 1e-10, 0),
            "uncovered time intervals cannot be admitted"
        );
        assert!(adaptive_axis_clear(c, t, 1e-10, 3));
        let crossing = TriangleMotion {
            end: [t.end[1], t.end[0], t.end[2]],
            ..t
        };
        assert!(gap(c, crossing, 0.5).unwrap() < 0.);
        assert!(!adaptive_axis_clear(c, crossing, 1e-10, 1024));
    }
    #[test]
    fn vertex_contact_can_slide_without_infinite_plane_separation() {
        let t = TriangleMotion {
            start: face(0.),
            end: face(0.),
        };
        let c = CapsuleMotion {
            start: [[-0.1, 2.01, 0.], [0.1, 2.01, 0.]],
            end: [[0., 2.01, 0.], [0.2, 2.01, 0.]],
            radius: 0.01,
        };
        assert!(!supporting_plane_clear(c, t, 1e-10, 1024));
        assert!(fixed_axis_clear(c, t, 1e-10));
        assert_eq!(
            sweep_capsule_triangle(c, t, Default::default()).unwrap(),
            CapsuleSweep::Clear
        );
        let mut entering = c;
        entering.end = entering.end.map(|p| add(p, [0., -0.02, 0.]));
        assert!(!fixed_axis_clear(entering, t, 1e-10));
        assert!(matches!(
            sweep_capsule_triangle(entering, t, Default::default()).unwrap(),
            CapsuleSweep::InitialContact { .. }
        ));
    }
    #[test]
    fn captured_first_jump_contact_enters_surface_before_ending_clear() {
        // Actual first-frame guide 53 / segment 18, original f64 words retained.
        let c = CapsuleMotion {
            start: [
                [
                    -0.09856575082434922,
                    0.5532094283493131,
                    0.001944126688601915,
                ],
                [
                    -0.10134279356150146,
                    0.5462632012035866,
                    0.008392947137751012,
                ],
            ],
            end: [
                [
                    -0.0979642690407443,
                    0.5544745766931043,
                    0.0008698362845989304,
                ],
                [
                    -0.10075827114148952,
                    0.5474753244209715,
                    0.007253672172510615,
                ],
            ],
            radius: 4e-05,
        };
        let t = TriangleMotion {
            start: [
                [
                    -0.10607147216796875,
                    0.5424706935882568,
                    0.010344523936510086,
                ],
                [
                    -0.0986102819442749,
                    0.5457484722137451,
                    0.010289707221090794,
                ],
                [
                    -0.09887480735778809,
                    0.5483585596084595,
                    0.007142554968595505,
                ],
            ],
            end: [
                [
                    -0.10607147216796875,
                    0.5424645841121674,
                    0.010344523936510086,
                ],
                [
                    -0.0986102819442749,
                    0.5457423627376556,
                    0.010289707221090794,
                ],
                [-0.09887480735778809, 0.54835245013237, 0.007142554968595505],
            ],
        };
        assert!(gap(c, t, 0.).unwrap().abs() < 1e-12);
        assert!(gap(c, t, 1.).unwrap() > 0.0007);
        assert!(gap(c, t, 0.153).unwrap() < -2e-6);
        assert!(matches!(
            sweep_capsule_triangle(c, t, Default::default()).unwrap(),
            CapsuleSweep::InitialContact { .. }
        ));
    }
    #[test]
    fn rotating_face_can_penetrate_between_two_touching_endpoint_states() {
        let start = face(0.);
        let t = TriangleMotion {
            start,
            end: start.map(|p| [p[0], 0., p[1]]),
        };
        let c = CapsuleMotion {
            start: capsule(0.01),
            end: capsule(0.).map(|p| add(p, [0., -0.01, 0.])),
            radius: 0.01,
        };
        assert!(gap(c, t, 0.).unwrap().abs() < 1e-14);
        assert!(gap(c, t, 1.).unwrap().abs() < 1e-14);
        assert!(gap(c, t, 0.5).unwrap() < -0.002);
        assert!(!supporting_plane_clear(c, t, 1e-10, 1024));
        assert!(matches!(
            sweep_capsule_triangle(c, t, Default::default()).unwrap(),
            CapsuleSweep::InitialContact { .. }
        ));
        let safe = CapsuleMotion {
            start: capsule(0.03),
            end: capsule(0.).map(|p| add(p, [0., -0.03, 0.])),
            radius: 0.01,
        };
        assert!(supporting_plane_clear(safe, t, 1e-10, 1024));
    }
    #[test]
    fn deforming_face_keeps_the_entire_accepted_prefix_clear() {
        let c = CapsuleMotion {
            start: capsule(0.),
            end: capsule(0.),
            radius: 0.01,
        };
        let mut end = face(-1.);
        end[2][2] = -3.;
        let t = TriangleMotion {
            start: face(1.),
            end,
        };
        assert!(gap(c, t, 0.).unwrap() > 0. && gap(c, t, 1.).unwrap() > 0.);
        let CapsuleSweep::Approach { fraction, .. } =
            sweep_capsule_triangle(c, t, Default::default()).unwrap()
        else {
            panic!("deforming face crosses capsule")
        };
        assert!(fraction > 0. && fraction < 1.);
        for i in 0..=256 {
            assert!(gap(c, t, fraction * i as f64 / 256.).unwrap() >= 0.);
        }
    }
    #[test]
    fn rejects_invalid_or_unresolvable_queries() {
        let mut c = CapsuleMotion {
            start: capsule(1.),
            end: capsule(-1.),
            radius: 0.01,
        };
        let t = TriangleMotion {
            start: face(0.),
            end: face(0.),
        };
        assert!(
            sweep_capsule_triangle(
                c,
                t,
                CapsuleSweepOptions {
                    tolerance_m: 1e-20,
                    ..Default::default()
                }
            )
            .is_err()
        );
        c.end[0][0] = f64::NAN;
        assert!(sweep_capsule_triangle(c, t, Default::default()).is_err());
    }
    #[test]
    fn collapsed_triangle_retains_edge_collision() {
        let line = [[-2., 0., 0.], [2., 0., 0.], [0., 0., 0.]];
        let c = CapsuleMotion {
            start: capsule(1.),
            end: capsule(-1.),
            radius: 0.01,
        };
        assert!(matches!(
            sweep_capsule_triangle(
                c,
                TriangleMotion {
                    start: line,
                    end: line
                },
                Default::default()
            )
            .unwrap(),
            CapsuleSweep::Approach { .. }
        ));
    }
    #[test]
    fn terminal_states_do_not_certify_clearance() {
        let t = TriangleMotion {
            start: face(0.),
            end: face(0.),
        };
        let c = CapsuleMotion {
            start: capsule(0.),
            end: capsule(1.),
            radius: 0.01,
        };
        assert!(matches!(
            sweep_capsule_triangle(c, t, Default::default()).unwrap(),
            CapsuleSweep::InitialContact { .. }
        ));
        let c = CapsuleMotion {
            start: capsule(1.),
            end: capsule(-1.),
            radius: 0.01,
        };
        assert!(matches!(
            sweep_capsule_triangle(
                c,
                t,
                CapsuleSweepOptions {
                    max_iterations: 1,
                    ..Default::default()
                }
            )
            .unwrap(),
            CapsuleSweep::IterationLimit { .. }
        ));
    }
}
