//! Directional fade discrepancy bound tied to an immutable canonical screw path.
use super::*;

#[derive(Debug)]
pub struct RootRigidCoordinateCertificate<'a> {
    path: &'a RootRigidPath,
    axis: usize,
    error_bound: f64,
}
impl RootRigidCoordinateCertificate<'_> {
    pub fn path(&self) -> &RootRigidPath {
        self.path
    }
    pub fn axis(&self) -> usize {
        self.axis
    }
    /// Uniform bound for every prefix and every material point, assuming equal
    /// initial coordinates. Floating evaluation/publication errors are separate.
    /// World coordinate margin after an exact signed coordinate-row mapping.
    /// Evaluation radius is a caller-proven bound in world units.
    pub fn enclosed_scaled_error_bound(
        &self,
        scale: f64,
        evaluation_radius: f64,
    ) -> Result<f64, AnimationError> {
        if !scale.is_finite() || !evaluation_radius.is_finite() || evaluation_radius < 0. {
            return Err(AnimationError::NumericalOverflow);
        }
        Ok(Scalar::exact(self.error_bound)
            .mul(Scalar::exact(scale.abs()))?
            .add(Scalar::exact(evaluation_radius))?
            .1)
    }
    pub fn error_bound(&self) -> f64 {
        self.error_bound
    }
}
impl RootRigidPath {
    /// Compare complete fade intervals with this exact stored screw trajectory.
    /// Every interval must match one trajectory span's stored endpoints in order.
    /// Both angular fields must remain parallel to the requested axis throughout.
    /// Any unsupported field returns None; no partial certificate is published.
    pub fn enclose_fade_coordinate_error<'a>(
        &'a self,
        fields: &[RootRigidFadeFieldInterval],
        axis: usize,
        max_spans: usize,
    ) -> Result<Option<RootRigidCoordinateCertificate<'a>>, AnimationError> {
        if axis >= 3 || fields.len() != self.spans().len() {
            return Err(AnimationError::InvalidSampleTime);
        }
        if fields.len() > max_spans {
            return Err(AnimationError::RootRotationBudget);
        }
        let mut error = Scalar::exact(0.);
        for (span, field) in self.spans().iter().zip(fields) {
            if field.wall_times() != [span.start(), span.end()] {
                return Err(AnimationError::InvalidSampleTime);
            }
            let Some((twist, _)) = span.screw else {
                return Ok(None);
            };
            let Some(bound) = field.coordinate_displacement_error_bound(axis, twist)? else {
                return Ok(None);
            };
            error = error.add(Scalar::exact(bound))?;
        }
        Ok(Some(RootRigidCoordinateCertificate {
            path: self,
            axis,
            error_bound: error.1,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn zero_fields(path: &RootRigidPath) -> Vec<RootRigidFadeFieldInterval> {
        let end = path.spans().last().unwrap().end();
        path.spans()
            .iter()
            .map(|span| {
                let zero = RootRigidFieldInterval::frozen([span.start(), span.end()]).unwrap();
                RootRigidFadeFieldInterval::new(zero, zero, [0., end], [0., 1.]).unwrap()
            })
            .collect()
    }
    #[test]
    fn certificate_is_path_bound_and_accumulates_every_ordered_interval() {
        let nominal = RootRigidTwist {
            linear: DVec3::new(4., 0.25, -7.),
            angular: DVec3::Y,
        };
        let path = RootRigidPath::from_twists(&[(nominal, 0.3), (nominal, 0.7)], 2).unwrap();
        let fields = zero_fields(&path);
        let proof = path
            .enclose_fade_coordinate_error(&fields, 1, 2)
            .unwrap()
            .unwrap();
        assert!(std::ptr::eq(proof.path(), &path));
        assert_eq!(proof.axis(), 1);
        assert!(proof.error_bound() >= 0.25 && proof.error_bound() - 0.25 < 1e-12);
        assert!(path.enclose_fade_coordinate_error(&fields, 1, 1).is_err());
        assert!(path
            .enclose_fade_coordinate_error(&fields[..1], 1, 2)
            .is_err());
        let mut reversed = fields.clone();
        reversed.reverse();
        assert!(path.enclose_fade_coordinate_error(&reversed, 1, 2).is_err());
        let tilted = RootRigidPath::from_twists(
            &[
                (nominal, 0.3),
                (
                    RootRigidTwist {
                        angular: DVec3::new(1e-300, 1., 0.),
                        ..nominal
                    },
                    0.7,
                ),
            ],
            2,
        )
        .unwrap();
        assert!(tilted
            .enclose_fade_coordinate_error(&fields, 1, 2)
            .unwrap()
            .is_none());
        let planar = RootRigidPath::from_twists(
            &[
                (
                    RootRigidTwist {
                        linear: DVec3::X,
                        ..nominal
                    },
                    0.3,
                ),
                (
                    RootRigidTwist {
                        linear: DVec3::Z,
                        ..nominal
                    },
                    0.7,
                ),
            ],
            2,
        )
        .unwrap();
        assert_eq!(
            planar
                .enclose_fade_coordinate_error(&fields, 1, 2)
                .unwrap()
                .unwrap()
                .error_bound(),
            0.
        );
        let empty = RootRigidPath::from_twists(&[], 0).unwrap();
        assert_eq!(
            empty
                .enclose_fade_coordinate_error(&[], 1, 0)
                .unwrap()
                .unwrap()
                .error_bound(),
            0.
        );
    }
}
