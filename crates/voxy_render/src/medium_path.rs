//! Bounded deterministic CPU reference for all dielectric camera branches.
//! Operates on borrowed immutable optical derivatives, with no physical inventory.
use crate::{MediumBoundaryMesh, OpticalMediumId, OpticalSegment};
use glam::DVec3;

/// Homogeneous local optical coefficients in SI units; not simulation mass.
#[derive(Clone, Copy, Debug)]
pub struct HomogeneousOpticalMedium {
    id: OpticalMediumId,
    ior: f64,
    extinction: [f64; 3],
    source_per_m: [f64; 3],
}
impl HomogeneousOpticalMedium {
    /// # Errors
    /// Invalid index/coefficient, unrepresentable n^2 or no coefficient-based
    /// stationary radiance bound (nonzero emission with zero extinction).
    pub fn new(
        id: OpticalMediumId,
        ior: f64,
        extinction: [f64; 3],
        source_per_m: [f64; 3],
    ) -> Result<Self, &'static str> {
        if !ior.is_finite() || ior <= 0. || !(ior * ior).is_finite() || ior * ior == 0. {
            return Err("invalid transport refractive index");
        }
        OpticalSegment::homogeneous(extinction, source_per_m, 0.)?;
        for axis in 0..3 {
            if extinction[axis] == 0. && source_per_m[axis] > 0. {
                return Err("stationary medium source bound unavailable");
            }
            if extinction[axis] > 0.
                && !(source_per_m[axis] / extinction[axis] / (ior * ior)).is_finite()
            {
                return Err("unrepresentable medium radiance bound");
            }
        }
        Ok(Self {
            id,
            ior,
            extinction,
            source_per_m,
        })
    }
}
#[derive(Clone, Copy, Debug)]
pub struct MediumTransportBudget {
    pub max_rays: usize,
    /// Conservative charge: all admitted triangles for each processed ray,
    /// even when a geometry bounding box avoids its actual triangle scan.
    pub max_triangle_tests: usize,
    pub absolute_error_rgb: [f64; 3],
}
#[derive(Clone, Copy, Debug)]
pub struct MediumTransportEstimate {
    pub radiance: [f64; 3],
    /// Bound on unprocessed nonnegative branch radiance, excluding f64 roundoff.
    /// This does not certify geometry intersection accuracy or source modelling.
    pub unresolved_upper_bound: [f64; 3],
    pub traced_rays: usize,
    pub charged_triangle_tests: usize,
}
/// Finite metric domain with an emissive terminal boundary on its six faces.
/// Environment is specified as reduced radiance L/n^2; its actual radiance on
/// a terminal branch is that value times the branch medium's index squared.
/// This is an explicit optical boundary, not a missing-geometry screen fallback.
#[derive(Debug)]
pub struct CpuMediumTransportScene<'a> {
    media: &'a [HomogeneousOpticalMedium],
    boundaries: &'a [MediumBoundaryMesh],
    opaque: &'a [crate::OpaqueRadianceMesh],
    lower: DVec3,
    upper: DVec3,
    environment_reduced_radiance: [f64; 3],
    maximum_reduced_radiance: [f64; 3],
    triangle_count: usize,
}
#[derive(Clone, Copy)]
struct Branch {
    origin: [f64; 3],
    direction: [f64; 3],
    medium: OpticalMediumId,
    weight: [f64; 3],
}
impl<'a> CpuMediumTransportScene<'a> {
    pub(crate) fn gpu_transport_parameters(&self) -> ([f64; 3], [f64; 3], [f64; 3], [f64; 3]) {
        (
            self.lower.to_array(),
            self.upper.to_array(),
            self.environment_reduced_radiance,
            self.maximum_reduced_radiance,
        )
    }
    pub(crate) fn gpu_medium_parameters(&self) -> impl Iterator<Item = [f64; 12]> + '_ {
        self.media.iter().map(|m| {
            [
                m.ior,
                m.ior * m.ior,
                0.,
                0.,
                m.extinction[0],
                m.extinction[1],
                m.extinction[2],
                0.,
                m.source_per_m[0],
                m.source_per_m[1],
                m.source_per_m[2],
                0.,
            ]
        })
    }
    pub(crate) fn gpu_triangle_count(&self) -> usize {
        self.triangle_count
    }
    pub(crate) fn gpu_medium_ids(&self) -> impl Iterator<Item = OpticalMediumId> + '_ {
        self.media.iter().map(|m| m.id)
    }
    pub(crate) fn append_gpu_geometry(
        &self,
        words: &mut Vec<u32>,
    ) -> Result<(), crate::ComputeError> {
        for (object, boundary) in self.boundaries.iter().enumerate() {
            let (inside, outside, _, _, _) = boundary.transport_metadata();
            let index = |id| {
                self.media
                    .iter()
                    .position(|m| m.id == id)
                    .and_then(|i| u32::try_from(i).ok())
                    .ok_or(crate::ComputeError::InvalidBuffer)
            };
            boundary.append_gpu_records(words, index(inside)?, index(outside)?, object as u32)?;
        }
        for (object, surface) in self.opaque.iter().enumerate() {
            surface.append_gpu_records(words, object as u32)?;
        }
        Ok(())
    }
    /// Caller excludes overlapping/self-intersecting source boundaries and
    /// explicitly authors incident/transmitted medium identities for nesting.
    /// # Errors
    /// Duplicate/missing identities, invalid domain/environment, boundary not
    /// strictly inside the finite domain, or unrepresentable geometry count.
    pub fn new(
        media: &'a [HomogeneousOpticalMedium],
        boundaries: &'a [MediumBoundaryMesh],
        lower: [f64; 3],
        upper: [f64; 3],
        environment_reduced_radiance: [f64; 3],
    ) -> Result<Self, &'static str> {
        Self::with_opaque(
            media,
            boundaries,
            &[],
            lower,
            upper,
            environment_reduced_radiance,
        )
    }
    /// Add open/closed opaque geometry with prescribed outgoing radiance.
    /// # Errors
    /// Preserves medium/domain admission and rejects nonfinite surface bounds.
    pub fn with_opaque(
        media: &'a [HomogeneousOpticalMedium],
        boundaries: &'a [MediumBoundaryMesh],
        opaque: &'a [crate::OpaqueRadianceMesh],
        lower: [f64; 3],
        upper: [f64; 3],
        environment_reduced_radiance: [f64; 3],
    ) -> Result<Self, &'static str> {
        let lower = DVec3::from_array(lower);
        let upper = DVec3::from_array(upper);
        if media.is_empty()
            || !lower.is_finite()
            || !upper.is_finite()
            || !(upper - lower).is_finite()
            || (0..3).any(|i| lower[i] >= upper[i])
            || environment_reduced_radiance
                .iter()
                .any(|x| !x.is_finite() || *x < 0.)
        {
            return Err("invalid finite medium domain");
        }
        let mut maximum_reduced_radiance = environment_reduced_radiance;
        for (index, medium) in media.iter().enumerate() {
            if media[..index].iter().any(|m| m.id == medium.id) {
                return Err("duplicate optical medium identity");
            }
            for axis in 0..3 {
                if medium.extinction[axis] > 0. {
                    maximum_reduced_radiance[axis] = maximum_reduced_radiance[axis].max(
                        medium.source_per_m[axis]
                            / medium.extinction[axis]
                            / (medium.ior * medium.ior),
                    );
                }
            }
        }
        let mut triangle_count = 0_usize;
        for boundary in boundaries {
            let (inside, outside, a, b, count) = boundary.transport_metadata();
            if !media.iter().any(|m| m.id == inside) || !media.iter().any(|m| m.id == outside) {
                return Err("missing optical medium identity");
            }
            if (0..3).any(|i| a[i] <= lower[i] || b[i] >= upper[i]) {
                return Err("medium boundary outside terminal domain");
            }
            triangle_count = triangle_count
                .checked_add(count)
                .ok_or("medium geometry capacity overflow")?;
        }
        for surface in opaque {
            let (a, b, count, radiance) = surface.transport_metadata();
            if (0..3).any(|i| a[i] <= lower[i] || b[i] >= upper[i]) {
                return Err("opaque geometry outside terminal domain");
            }
            triangle_count = triangle_count
                .checked_add(count)
                .ok_or("medium geometry capacity overflow")?;
            for medium in media {
                for axis in 0..3 {
                    let bound = radiance[axis] / (medium.ior * medium.ior);
                    if !bound.is_finite() {
                        return Err("opaque surface radiance bound overflow");
                    }
                    maximum_reduced_radiance[axis] = maximum_reduced_radiance[axis].max(bound);
                }
            }
        }
        Ok(Self {
            media,
            boundaries,
            opaque,
            lower,
            upper,
            environment_reduced_radiance,
            maximum_reduced_radiance,
            triangle_count,
        })
    }
    fn medium(&self, id: OpticalMediumId) -> Result<&HomogeneousOpticalMedium, &'static str> {
        self.media
            .iter()
            .find(|m| m.id == id)
            .ok_or("unknown current optical medium")
    }
    fn domain_exit(&self, branch: Branch) -> Result<f64, &'static str> {
        let mut exit = f64::INFINITY;
        for axis in 0..3 {
            let d = branch.direction[axis];
            if d != 0. {
                let plane = if d > 0. {
                    self.upper[axis]
                } else {
                    self.lower[axis]
                };
                exit = exit.min((plane - branch.origin[axis]) / d);
            }
        }
        if !exit.is_finite() || exit < 0. {
            return Err("invalid terminal domain crossing");
        }
        Ok(exit)
    }
    /// Trace both reflected and transmitted branches, prioritizing largest
    /// unresolved weight. Computation uses reduced radiance L/n^2, where each
    /// lossless interface is a convex combination R+T=1. Homogeneous source
    /// bounds and the explicit environment bound therefore bound every subtree.
    /// # Errors
    /// Invalid ray/occupancy, ambiguous geometry, arithmetic overflow or budget
    /// exhausted before the requested unresolved-radiance bound is achieved.
    /// No partial estimate is returned as an accepted result on budget failure.
    pub fn estimate(
        &self,
        origin: [f64; 3],
        direction: [f64; 3],
        current_medium: OpticalMediumId,
        budget: MediumTransportBudget,
    ) -> Result<MediumTransportEstimate, &'static str> {
        let o = DVec3::from_array(origin);
        let d = DVec3::from_array(direction);
        if !o.is_finite()
            || !d.is_finite()
            || (d.length() - 1.).abs() > 1e-10
            || (0..3).any(|i| o[i] < self.lower[i] || o[i] > self.upper[i])
            || budget
                .absolute_error_rgb
                .iter()
                .any(|x| !x.is_finite() || *x < 0.)
        {
            return Err("invalid medium transport ray or tolerance");
        }
        let camera_ior = self.medium(current_medium)?.ior;
        let camera_scale = camera_ior * camera_ior;
        let mut pending = vec![Branch {
            origin,
            direction: d.normalize().to_array(),
            medium: current_medium,
            weight: [1.; 3],
        }];
        let mut radiance = [0.; 3];
        let mut rays = 0_usize;
        let mut charged = 0_usize;
        loop {
            let mut bound = [0.; 3];
            for branch in &pending {
                for axis in 0..3 {
                    bound[axis] +=
                        branch.weight[axis] * self.maximum_reduced_radiance[axis] * camera_scale;
                }
            }
            if bound.iter().chain(&radiance).any(|x| !x.is_finite()) {
                return Err("medium transport radiance overflow");
            }
            if (0..3).all(|i| bound[i] <= budget.absolute_error_rgb[i]) {
                return Ok(MediumTransportEstimate {
                    radiance,
                    unresolved_upper_bound: bound,
                    traced_rays: rays,
                    charged_triangle_tests: charged,
                });
            }
            if rays >= budget.max_rays {
                return Err("medium transport ray budget exhausted");
            }
            charged = charged
                .checked_add(self.triangle_count)
                .filter(|n| *n <= budget.max_triangle_tests)
                .ok_or("medium transport triangle budget exhausted")?;
            let best = pending
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| {
                    a.weight
                        .iter()
                        .copied()
                        .fold(0_f64, f64::max)
                        .total_cmp(&b.weight.iter().copied().fold(0_f64, f64::max))
                })
                .map(|(i, _)| i)
                .ok_or("missing unresolved medium branch")?;
            let branch = pending.swap_remove(best);
            rays += 1;
            let medium = self.medium(branch.medium)?;
            let terminal = self.domain_exit(branch)?;
            let mut opaque_hit: Option<(f64, [f64; 3])> = None;
            if terminal > 0. {
                for surface in self.opaque {
                    if let Some(distance) =
                        surface.first_distance(branch.origin, branch.direction, terminal)?
                    {
                        if opaque_hit.is_some_and(|h| h.0 == distance) {
                            return Err("simultaneous opaque surfaces");
                        }
                        if opaque_hit.is_none_or(|h| distance < h.0) {
                            opaque_hit = Some((distance, surface.transport_metadata().3));
                        }
                    }
                }
            }
            let visible_limit = opaque_hit.map_or(terminal, |h| h.0);
            let mut hit: Option<crate::MediumBoundaryHit> = None;
            if terminal > 0. {
                for boundary in self.boundaries {
                    if let Some(candidate) =
                        boundary.first_hit(branch.origin, branch.direction, 0., visible_limit)?
                    {
                        if opaque_hit.is_some_and(|h| h.0 == candidate.distance_m) {
                            return Err("opaque surface coincides with medium interface");
                        }
                        if hit.is_some_and(|h| h.distance_m == candidate.distance_m) {
                            return Err("simultaneous medium interfaces");
                        }
                        if hit.is_none_or(|h| candidate.distance_m < h.distance_m) {
                            hit = Some(candidate);
                        }
                    }
                }
            }
            let length = hit.map_or(visible_limit, |h| h.distance_m);
            let segment =
                OpticalSegment::homogeneous(medium.extinction, medium.source_per_m, length)?;
            let transmission = segment.transmission();
            let source = segment.source_radiance();
            let mut weight = [0.; 3];
            for axis in 0..3 {
                radiance[axis] +=
                    branch.weight[axis] * (source[axis] / (medium.ior * medium.ior)) * camera_scale;
                weight[axis] = branch.weight[axis] * transmission[axis];
            }
            if let Some(hit) = hit {
                if branch.medium != hit.incident_medium {
                    return Err("medium boundary occupancy mismatch");
                }
                let next = self.medium(hit.transmitted_medium)?;
                let sample = crate::dielectric_boundary_sample(
                    branch.direction,
                    hit.incident_normal,
                    medium.ior,
                    next.ior,
                )?;
                let reflected = weight.map(|x| x * sample.reflected_power_fraction());
                if reflected.iter().any(|x| *x > 0.) {
                    pending.push(Branch {
                        origin: hit.position_m,
                        direction: DVec3::from_array(sample.reflected_direction())
                            .normalize()
                            .to_array(),
                        medium: branch.medium,
                        weight: reflected,
                    });
                }
                if let Some(direction) = sample.transmitted_direction() {
                    let transmitted = weight.map(|x| x * sample.transmitted_power_fraction());
                    if transmitted.iter().any(|x| *x > 0.) {
                        pending.push(Branch {
                            origin: hit.position_m,
                            direction: DVec3::from_array(direction).normalize().to_array(),
                            medium: next.id,
                            weight: transmitted,
                        });
                    }
                }
            } else {
                let terminal_reduced = opaque_hit.map_or(self.environment_reduced_radiance, |h| {
                    h.1.map(|x| x / (medium.ior * medium.ior))
                });
                for axis in 0..3 {
                    radiance[axis] += weight[axis] * terminal_reduced[axis] * camera_scale;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const WATER: OpticalMediumId = OpticalMediumId(17);
    const AIR: OpticalMediumId = OpticalMediumId(4);
    pub(super) fn boundary() -> MediumBoundaryMesh {
        MediumBoundaryMesh::from_scene_mesh(
            &crate::medium_geometry::tests::box_mesh(false, false),
            WATER,
            AIR,
            1.,
            12,
        )
        .unwrap()
    }
    pub(super) fn budget(tolerance: f64) -> MediumTransportBudget {
        MediumTransportBudget {
            max_rays: 4096,
            max_triangle_tests: 4096 * 12,
            absolute_error_rgb: [tolerance; 3],
        }
    }
    #[test]
    fn all_reflection_orders_match_independent_absorbing_emitting_slab() {
        let boundaries = [boundary()];
        let sigma = [0.2, 0.7, 1.5];
        let q = [0.08, 0.21, 0.6];
        let fog = [0.1, 0.2, 0.3];
        let n = 1.333;
        let media = [
            HomogeneousOpticalMedium::new(AIR, 1., fog, [0.; 3]).unwrap(),
            HomogeneousOpticalMedium::new(WATER, n, sigma, q).unwrap(),
        ];
        let scene = CpuMediumTransportScene::new(
            &media,
            &boundaries,
            [-12., -12., -3.],
            [12., 12., 3.],
            [1.; 3],
        )
        .unwrap();
        let r = ((1_f64 - n) / (1. + n)).powi(2);
        let t = 1. - r;
        let expected: [f64; 3] = std::array::from_fn(|i| {
            let a = (-2_f64 * sigma[i]).exp();
            let front = (-fog[i]).exp();
            let far = (-2. * fog[i]).exp();
            let s = q[i] / sigma[i] * (1. - a);
            // Infinite series of all interior reflection orders, with both
            // exterior terminal faces emitting equal reduced radiance.
            front * (far * (r + t * t * a / (1. - r * a)) + t * s / (n * n * (1. - r * a)))
        });
        let mut previous = [0.; 3];
        let mut previous_rays = 0;
        for tolerance in [1e-3, 1e-7, 1e-11] {
            let result = scene
                .estimate([0., 0., 2.], [0., 0., -1.], AIR, budget(tolerance))
                .unwrap();
            for axis in 0..3 {
                assert!(result.radiance[axis] >= previous[axis]);
                assert!(result.radiance[axis] <= expected[axis] + 1e-14);
                assert!(
                    expected[axis] - result.radiance[axis]
                        <= result.unresolved_upper_bound[axis] + 1e-14,
                    "axis={axis} estimate={result:?} expected={expected:?}"
                );
                assert!(result.unresolved_upper_bound[axis] <= tolerance);
            }
            assert!(result.traced_rays >= previous_rays);
            previous_rays = result.traced_rays;
            previous = result.radiance;
            assert_eq!(result.charged_triangle_tests, result.traced_rays * 12);
            println!(
                "MEDIUM ALL BRANCHES tolerance={tolerance} estimate={result:?} expected={expected:?}"
            );
        }
    }
    #[test]
    fn refractive_equilibrium_and_trapped_emitting_internal_reflections() {
        let boundaries = [boundary()];
        let n = 1.5;
        let media = [
            HomogeneousOpticalMedium::new(AIR, 1., [0.1; 3], [0.1; 3]).unwrap(),
            HomogeneousOpticalMedium::new(WATER, n, [0.2; 3], [0.2 * n * n; 3]).unwrap(),
        ];
        let scene = CpuMediumTransportScene::new(
            &media,
            &boundaries,
            [-12., -12., -3.],
            [12., 12., 3.],
            [1.; 3],
        )
        .unwrap();
        for (origin, direction, id, expected) in [
            ([0., 0., 2.], [0., 0., -1.], AIR, 1.),
            ([0., 0., 0.], [0., 0., -1.], WATER, n * n),
        ] {
            let result = scene
                .estimate(origin, direction, id, budget(1e-10))
                .unwrap();
            for i in 0..3 {
                assert!(
                    (result.radiance[i] - expected).abs()
                        <= result.unresolved_upper_bound[i] + 1e-13
                );
            }
        }
        // Equal-direction components make every cube face exceed the critical
        // angle: no branch reaches the terminal environment. Absorption/emission
        // still makes the full infinite path converge to its stationary source.
        let media = [
            HomogeneousOpticalMedium::new(AIR, 1., [0.; 3], [0.; 3]).unwrap(),
            HomogeneousOpticalMedium::new(WATER, n, [0.2; 3], [0.2 * n * n; 3]).unwrap(),
        ];
        let scene = CpuMediumTransportScene::new(
            &media,
            &boundaries,
            [-12., -12., -3.],
            [12., 12., 3.],
            [0.; 3],
        )
        .unwrap();
        let component = 1. / 3_f64.sqrt();
        let result = scene
            .estimate([0.123, 0.456, 0.03], [component; 3], WATER, budget(1e-10))
            .unwrap();
        for i in 0..3 {
            assert!((result.radiance[i] - n * n).abs() <= result.unresolved_upper_bound[i] + 1e-13);
        }
        assert!(result.traced_rays > 2);
    }
    #[test]
    fn insufficient_budget_is_an_error_and_does_not_mutate_scene() {
        let boundaries = [boundary()];
        let media = [
            HomogeneousOpticalMedium::new(AIR, 1., [0.; 3], [0.; 3]).unwrap(),
            HomogeneousOpticalMedium::new(WATER, 1.5, [0.; 3], [0.; 3]).unwrap(),
        ];
        let scene = CpuMediumTransportScene::new(
            &media,
            &boundaries,
            [-12., -12., -3.],
            [12., 12., 3.],
            [1.; 3],
        )
        .unwrap();
        let small = MediumTransportBudget {
            max_rays: 1,
            ..budget(1e-10)
        };
        assert_eq!(
            scene
                .estimate([0., 0., 2.], [0., 0., -1.], AIR, small)
                .unwrap_err(),
            "medium transport ray budget exhausted"
        );
        let small = MediumTransportBudget {
            max_triangle_tests: 11,
            ..budget(1e-10)
        };
        assert_eq!(
            scene
                .estimate([0., 0., 2.], [0., 0., -1.], AIR, small)
                .unwrap_err(),
            "medium transport triangle budget exhausted"
        );
        assert_eq!(
            scene
                .estimate([0.; 3], [0., 0., -1.], AIR, budget(1e-10))
                .unwrap_err(),
            "medium boundary occupancy mismatch"
        );
        let accepted = scene
            .estimate([0., 0., 2.], [0., 0., -1.], AIR, budget(1e-10))
            .unwrap();
        assert!((accepted.radiance[0] - 1.).abs() <= accepted.unresolved_upper_bound[0] + 1e-13);
        // A lossless trapped path has no finite termination certificate here.
        let component = 1. / 3_f64.sqrt();
        let small = MediumTransportBudget {
            max_rays: 3,
            ..budget(1e-10)
        };
        assert_eq!(
            scene
                .estimate([0.123, 0.456, 0.03], [component; 3], WATER, small)
                .unwrap_err(),
            "medium transport ray budget exhausted"
        );
    }
    #[test]
    fn source_and_identity_admission_is_explicit() {
        assert!(HomogeneousOpticalMedium::new(AIR, 1., [0.; 3], [1.; 3]).is_err());
        assert!(HomogeneousOpticalMedium::new(AIR, f64::MAX, [0.; 3], [0.; 3]).is_err());
        let air = HomogeneousOpticalMedium::new(AIR, 1., [0.; 3], [0.; 3]).unwrap();
        assert!(
            CpuMediumTransportScene::new(&[air, air], &[], [-1.; 3], [1.; 3], [1.; 3]).is_err()
        );
        assert!(
            CpuMediumTransportScene::new(
                &[air],
                &[boundary()],
                [-12., -12., -3.],
                [12., 12., 3.],
                [1.; 3]
            )
            .is_err()
        );
        let scene = CpuMediumTransportScene::new(
            std::slice::from_ref(&air),
            &[],
            [-1.; 3],
            [1.; 3],
            [1.; 3],
        )
        .unwrap();
        assert!(
            scene
                .estimate([2., 0., 0.], [0., 0., -1.], AIR, budget(1e-10))
                .is_err()
        );
        assert!(
            scene
                .estimate([0.; 3], [0.; 3], AIR, budget(1e-10))
                .is_err()
        );
        assert!(
            scene
                .estimate([0.; 3], [0., 0., -1.], OpticalMediumId(999), budget(1e-10))
                .is_err()
        );
        let exact = scene
            .estimate([0.; 3], [0., 0., -1.], AIR, budget(0.))
            .unwrap();
        assert_eq!(exact.radiance, [1.; 3]);
        assert_eq!(exact.unresolved_upper_bound, [0.; 3]);
    }
    #[test]
    fn nested_liquid_species_replace_medium_occupancy_in_order() {
        let oil = OpticalMediumId(29);
        let mesh = crate::medium_geometry::tests::box_mesh(false, false);
        let vertices = mesh
            .vertices()
            .iter()
            .copied()
            .map(|mut v| {
                v.position = v.position.map(|x| x * 0.5);
                v
            })
            .collect();
        let inner_mesh = crate::SceneMesh::new(vertices, mesh.indices().to_vec()).unwrap();
        let boundaries = [
            boundary(),
            MediumBoundaryMesh::from_scene_mesh(&inner_mesh, oil, WATER, 1., 12).unwrap(),
        ];
        let sigma_air = [0.1, 0.2, 0.3];
        let sigma_water = [0.4, 0.5, 0.6];
        let sigma_oil = [1.3, 1.7, 2.1];
        // Index identity isolates spatial medium occupancy from interface
        // reflection. Exterior fog is replaced by water/oil, not double-counted.
        let media = [
            HomogeneousOpticalMedium::new(AIR, 1., sigma_air, [0.; 3]).unwrap(),
            HomogeneousOpticalMedium::new(WATER, 1., sigma_water, [0.; 3]).unwrap(),
            HomogeneousOpticalMedium::new(oil, 1., sigma_oil, [0.; 3]).unwrap(),
        ];
        let scene = CpuMediumTransportScene::new(
            &media,
            &boundaries,
            [-12., -12., -3.],
            [12., 12., 3.],
            [1.; 3],
        )
        .unwrap();
        let result = scene
            .estimate([0., 0., 2.], [0., 0., -1.], AIR, budget(0.))
            .unwrap();
        assert_eq!(result.traced_rays, 5);
        assert_eq!(result.charged_triangle_tests, 120);
        assert_eq!(result.unresolved_upper_bound, [0.; 3]);
        for i in 0..3 {
            let expected = (-3. * sigma_air[i] - sigma_water[i] - sigma_oil[i]).exp();
            assert!((result.radiance[i] - expected).abs() < 1e-14);
        }
        let inside = scene
            .estimate([0.; 3], [0., 0., -1.], oil, budget(0.))
            .unwrap();
        assert_eq!(inside.traced_rays, 3);
        for i in 0..3 {
            let expected = (-2. * sigma_air[i] - 0.5 * sigma_water[i] - 0.5 * sigma_oil[i]).exp();
            assert!((inside.radiance[i] - expected).abs() < 1e-14);
        }
        assert_eq!(
            scene
                .estimate([0.; 3], [0., 0., -1.], WATER, budget(0.))
                .unwrap_err(),
            "medium boundary occupancy mismatch"
        );
    }
}

#[cfg(test)]
mod opaque_tests {
    use super::tests::{boundary, budget};
    use super::*;
    use crate::OpaqueRadianceMesh;
    const AIR: OpticalMediumId = OpticalMediumId(4);
    const WATER: OpticalMediumId = OpticalMediumId(17);
    fn plane(z: f32, radiance: [f64; 3]) -> OpaqueRadianceMesh {
        let vertices = [
            [-11., -11., z],
            [11., -11., z],
            [11., 11., z],
            [-11., 11., z],
        ]
        .into_iter()
        .map(|position| crate::SceneVertex {
            position,
            uv: [0.; 2],
            color: [1.; 4],
        })
        .collect();
        let mesh = crate::SceneMesh::new(vertices, vec![0, 1, 2, 0, 2, 3]).unwrap();
        OpaqueRadianceMesh::from_scene_mesh(&mesh, 1., 2, radiance).unwrap()
    }
    #[test]
    fn opacity_before_inside_and_behind_water_matches_independent_transport() {
        let boundaries = [boundary()];
        let fog = [0.1, 0.2, 0.3];
        let water = [0.4, 0.7, 1.1];
        let n = 1.5;
        let media = [
            HomogeneousOpticalMedium::new(AIR, 1., fog, [0.; 3]).unwrap(),
            HomogeneousOpticalMedium::new(WATER, n, water, [0.; 3]).unwrap(),
        ];
        let emitted = [2., 3., 4.];
        let r = ((1_f64 - n) / (1. + n)).powi(2);
        let t = 1. - r;
        for z in [1.5_f32, 0., -2.] {
            let surfaces = [plane(z, emitted)];
            let scene = CpuMediumTransportScene::with_opaque(
                &media,
                &boundaries,
                &surfaces,
                [-12., -12., -3.],
                [12., 12., 3.],
                [1.; 3],
            )
            .unwrap();
            let result = scene
                .estimate([0., 0., 2.], [0., 0., -1.], AIR, budget(1e-11))
                .unwrap();
            for i in 0..3 {
                let f = (-fog[i]).exp();
                let front = (-2. * fog[i]).exp();
                let a = (-2. * water[i]).exp();
                let expected = if z > 1. {
                    emitted[i] * (-fog[i] * (2. - f64::from(z))).exp()
                } else if z == 0. {
                    f * (r * front + (t / (n * n)) * emitted[i] * (-water[i]).exp())
                } else {
                    let back = emitted[i] * (-fog[i]).exp();
                    // Independent sum over alternating front/back exits after
                    // any number of internal water reflections.
                    f * (r * front + t * t * a * (back + r * a * front) / (1. - r * r * a * a))
                };
                assert!(
                    result.radiance[i] <= expected + 1e-13,
                    "z={z} result={result:?} expected={expected}"
                );
                assert!(
                    expected - result.radiance[i] <= result.unresolved_upper_bound[i] + 1e-13,
                    "z={z} result={result:?} expected={expected}"
                );
            }
            assert_eq!(result.charged_triangle_tests, result.traced_rays * 14);
            if z > 1. {
                assert_eq!(result.traced_rays, 1);
            }
            if z == 0. {
                assert_eq!(result.traced_rays, 3);
            }
            println!("MEDIUM OPAQUE z={z} result={result:?}");
        }
    }
    #[test]
    fn two_sided_opaque_surface_and_sources_stop_at_real_hit() {
        let boundaries = [boundary()];
        let n = 1.5;
        let emitted = [2., 3., 4.];
        let q = [0.04, 0.08, 0.12];
        let sigma = [0.4, 0.7, 1.1];
        let media = [
            HomogeneousOpticalMedium::new(AIR, 1., [0.1; 3], [0.; 3]).unwrap(),
            HomogeneousOpticalMedium::new(WATER, n, sigma, q).unwrap(),
        ];
        let surfaces = [plane(0., emitted)];
        let scene = CpuMediumTransportScene::with_opaque(
            &media,
            &boundaries,
            &surfaces,
            [-12., -12., -3.],
            [12., 12., 3.],
            [1.; 3],
        )
        .unwrap();
        let result = scene
            .estimate([0., 0., 2.], [0., 0., -1.], AIR, budget(1e-11))
            .unwrap();
        let r = ((1_f64 - n) / (1. + n)).powi(2);
        let t = 1. - r;
        for i in 0..3 {
            let a = (-sigma[i]).exp();
            let source = q[i] / sigma[i] * (1. - a);
            let expected =
                (-0.1_f64).exp() * (r * (-0.2_f64).exp() + t / (n * n) * (source + a * emitted[i]));
            assert!((result.radiance[i] - expected).abs() < 1e-13);
        }
        let surfaces = [plane(-2., emitted)];
        let scene = CpuMediumTransportScene::with_opaque(
            &media,
            &boundaries,
            &surfaces,
            [-12., -12., -3.],
            [12., 12., 3.],
            [0.; 3],
        )
        .unwrap();
        let back = scene
            .estimate([0., 0., -2.5], [0., 0., 1.], AIR, budget(0.))
            .unwrap();
        assert_eq!(back.traced_rays, 1);
        for i in 0..3 {
            assert!((back.radiance[i] - emitted[i] * (-0.05_f64).exp()).abs() < 1e-13);
        }
    }
    #[test]
    fn opaque_geometry_limits_and_coincident_interfaces_reject_atomically() {
        let boundaries = [boundary()];
        let media = [
            HomogeneousOpticalMedium::new(AIR, 1., [0.; 3], [0.; 3]).unwrap(),
            HomogeneousOpticalMedium::new(WATER, 1.5, [0.; 3], [0.; 3]).unwrap(),
        ];
        let surfaces = [plane(1., [1.; 3])];
        let scene = CpuMediumTransportScene::with_opaque(
            &media,
            &boundaries,
            &surfaces,
            [-12., -12., -3.],
            [12., 12., 3.],
            [1.; 3],
        )
        .unwrap();
        assert_eq!(
            scene
                .estimate([0., 0., 2.], [0., 0., -1.], AIR, budget(1e-10))
                .unwrap_err(),
            "opaque surface coincides with medium interface"
        );
        let surfaces = [plane(1.5, [1.; 3])];
        let scene = CpuMediumTransportScene::with_opaque(
            &media,
            &boundaries,
            &surfaces,
            [-12., -12., -3.],
            [12., 12., 3.],
            [1.; 3],
        )
        .unwrap();
        let small = MediumTransportBudget {
            max_triangle_tests: 13,
            ..budget(1e-10)
        };
        assert_eq!(
            scene
                .estimate([0., 0., 2.], [0., 0., -1.], AIR, small)
                .unwrap_err(),
            "medium transport triangle budget exhausted"
        );
        assert_eq!(
            scene
                .estimate([0., 0., 2.], [0., 0., -1.], AIR, budget(0.))
                .unwrap()
                .radiance,
            [1.; 3]
        );
        // This ray would hit an ambiguous water-box crease at (10,0,1),
        // but the opaque plane at z=1.5 terminates it first. Hidden geometry
        // cannot reject an otherwise valid visible optical path.
        let c = -1. / 2_f64.sqrt();
        assert_eq!(
            scene
                .estimate([11., 0., 2.], [c, 0., c], AIR, budget(0.))
                .unwrap()
                .radiance,
            [1.; 3]
        );
        let outside = [plane(4., [1.; 3])];
        assert!(
            CpuMediumTransportScene::with_opaque(
                &media,
                &boundaries,
                &outside,
                [-12., -12., -3.],
                [12., 12., 3.],
                [1.; 3]
            )
            .is_err()
        );
        let mesh = crate::medium_geometry::tests::box_mesh(false, false);
        assert!(OpaqueRadianceMesh::from_scene_mesh(&mesh, 1., 11, [1.; 3]).is_err());
        assert!(OpaqueRadianceMesh::from_scene_mesh(&mesh, 1., 12, [f64::NAN; 3]).is_err());
        assert!(OpaqueRadianceMesh::from_scene_mesh(&mesh, 1., 12, [-1.; 3]).is_err());
    }
}
