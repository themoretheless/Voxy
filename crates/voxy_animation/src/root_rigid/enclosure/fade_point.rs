//! Uniform material-point discrepancy from complete fade fields to canonical screws.
use super::*;

/// Borrowed geometric certificate. Both motions start at the same identity pose.
/// Floating playback, body-frame transport and publication errors are separate.
#[derive(Debug)]
pub struct RootRigidFadePointCertificate<'a> {
    candidate: &'a RootScrewEnclosurePath<'a>,
    fields: &'a [RootRigidFadeFieldInterval],
    point: [[f64; 2]; 3],
    origin_error: f64,
    angular_error: f64,
    radius: f64,
}
impl<'a> RootRigidFadePointCertificate<'a> {
    pub fn candidate(&self) -> &RootScrewEnclosurePath<'a> {
        self.candidate
    }
    pub fn fields(&self) -> &[RootRigidFadeFieldInterval] {
        self.fields
    }
    pub fn point_bounds(&self) -> [[f64; 2]; 3] {
        self.point
    }
    pub fn origin_error_bound(&self) -> f64 {
        self.origin_error
    }
    pub fn angular_error_bound(&self) -> f64 {
        self.angular_error
    }
    /// Euclidean error for every prefix and every material point in the box.
    pub fn radius(&self) -> f64 {
        self.radius
    }
}
impl<'a> RootScrewEnclosurePath<'a> {
    /// Compares complete common-frame fade fields with every canonical screw
    /// span. Uniform field discrepancy suffices; no derivative estimate or
    /// point-sampled extrema are assumed. Gaps and incomplete domains reject.
    pub fn certify_fade_point_error(
        &'a self,
        fields: &'a [RootRigidFadeFieldInterval],
        point: [[f64; 2]; 3],
        max_spans: usize,
    ) -> Result<RootRigidFadePointCertificate<'a>, AnimationError> {
        RootRigidEnclosure::IDENTITY.transform_point_box_bounds(point)?;
        if fields.len() != self.path.spans().len() {
            return Err(AnimationError::InvalidSampleTime);
        }
        if fields.len() > max_spans.min(MAX_ROOT_ROTATION_SPANS) {
            return Err(AnimationError::RootRigidBudget);
        }
        let mut error = RootRigidErrorAccumulator::ZERO;
        let mut end = 0.;
        for (index, (span, field)) in self.path.spans().iter().zip(fields).enumerate() {
            if span.start() != end || field.wall_times() != [span.start(), span.end()] {
                return Err(AnimationError::InvalidSampleTime);
            }
            let (nominal, _) = span
                .screw
                .ok_or(AnimationError::RootRotationTransitionUnsupported)?;
            error = error.append_bounded_field_interval(
                span.start(),
                span.end(),
                field.velocity_enclosure().error_bounds(nominal)?,
                &self.prefixes[index],
                nominal,
            )?;
            end = span.end();
        }
        if end != self.path.duration() {
            return Err(AnimationError::InvalidSampleTime);
        }
        let mut point_radius = Scalar::exact(0.);
        for range in point {
            point_radius = point_radius.add(Scalar::exact(range[0].abs().max(range[1].abs())))?;
        }
        // Rotation chord distance is bounded by both the integrated angle and 2.
        let radius = Scalar::exact(error.origin_bound())
            .add(Scalar::exact(error.angular_bound().min(2.)).mul(point_radius)?)?
            .1;
        Ok(RootRigidFadePointCertificate {
            candidate: self,
            fields,
            point,
            origin_error: error.origin_bound(),
            angular_error: error.angular_bound(),
            radius,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fields(path: &RootRigidPath, source: &RootRigidPath) -> Vec<RootRigidFadeFieldInterval> {
        path.spans()
            .iter()
            .map(|span| {
                let wall = [span.start(), span.end()];
                let actual = RootRigidFieldInterval::from_span(
                    &source.spans()[0],
                    wall,
                    wall,
                    RootRigidTransform::IDENTITY,
                    1.,
                )
                .unwrap()
                .unwrap();
                RootRigidFadeFieldInterval::new(actual, actual, [0., path.duration()], [0., 1.])
                    .unwrap()
            })
            .collect()
    }
    #[test]
    fn uniform_fade_point_certificate_owns_every_interval_and_canonical_prefix() {
        let twist = |x| RootRigidTwist {
            linear: DVec3::X * x,
            angular: DVec3::ZERO,
        };
        let candidate =
            RootRigidPath::from_twists(&[(twist(1.), 0.25), (twist(1.), 0.75)], 2).unwrap();
        let source = RootRigidPath::from_twists(&[(twist(2.), 1.)], 1).unwrap();
        let domains = fields(&candidate, &source);
        let cache = candidate.prepare_screw_enclosures(2).unwrap();
        let point = [[-100., 100.], [-2., 3.], [0., 0.]];
        let proof = cache.certify_fade_point_error(&domains, point, 2).unwrap();
        assert!(std::ptr::eq(proof.candidate(), &cache));
        assert!(std::ptr::eq(proof.fields().as_ptr(), domains.as_ptr()));
        assert_eq!(proof.point_bounds(), point);
        assert!(proof.radius() >= 1. && proof.radius() < 1.000000000001);
        assert_eq!(proof.angular_error_bound(), 0.);
        assert!(
            cache
                .certify_fade_point_error(&domains[..1], point, 2)
                .is_err()
        );
        assert!(cache.certify_fade_point_error(&domains, point, 1).is_err());
        let mut reversed = domains.clone();
        reversed.reverse();
        assert!(cache.certify_fade_point_error(&reversed, point, 2).is_err());
        assert!(
            cache
                .certify_fade_point_error(&domains, [[1., -1.]; 3], 2)
                .is_err()
        );
        let stationary = RootRigidPath::from_twists(&[(twist(0.), 1.)], 1).unwrap();
        let turns = RootRigidPath::from_twists(
            &[
                (twist(1.), 0.5),
                (
                    RootRigidTwist {
                        linear: DVec3::ZERO,
                        angular: DVec3::Y * 0.1,
                    },
                    0.5,
                ),
            ],
            2,
        )
        .unwrap();
        let domains = fields(&turns, &stationary);
        let cache = turns.prepare_screw_enclosures(2).unwrap();
        let proof = cache
            .certify_fade_point_error(&domains, [[0., 0.]; 3], 2)
            .unwrap();
        assert!(proof.origin_error_bound() >= 0.525);
        assert!(proof.angular_error_bound() >= 0.05);
        assert!(proof.origin_error_bound() < 0.526);
    }
}
