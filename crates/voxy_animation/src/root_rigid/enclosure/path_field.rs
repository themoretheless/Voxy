//! Complete spatial field domains, including limits at continuous key boundaries.
use super::*;
impl RootRigidPath {
    /// Encloses every velocity value over checked stored clip times. Adjacent
    /// continuous key limits are both included, and gaps contribute zero.
    /// No finite derivative bound across keys is implied. Pose STEP events reject.
    pub fn spatial_twist_enclosure_between(
        &self,
        times: [f64; 2],
    ) -> Result<Option<RootRigidTwistEnclosure>, AnimationError> {
        if times.into_iter().any(|v| !v.is_finite())
            || times[1] < times[0]
            || times[0] < 0.
            || times[1] > self.duration()
        {
            return Err(AnimationError::InvalidSampleTime);
        }
        let zero = RootRigidTwist {
            linear: DVec3::ZERO,
            angular: DVec3::ZERO,
        }
        .enclosure()?;
        let mut result: Option<RootRigidTwistEnclosure> = None;
        let first = self.spans().partition_point(|span| span.end() < times[0]);
        let mut previous = if first == 0 {
            0.
        } else {
            self.spans()[first - 1].end()
        };
        let mut stationary = false;
        for span in &self.spans()[first..] {
            if span.start() > previous && times[0] <= span.start() && times[1] >= previous {
                stationary = true;
            }
            if span.start() > times[1] {
                break;
            }
            previous = previous.max(span.end());
            if span.is_step() {
                return Err(AnimationError::RootRotationTransitionUnsupported);
            }
            let Some(field) = span.spatial_twist_enclosure_at_times([
                times[0].max(span.start()),
                times[1].min(span.end()),
            ])?
            else {
                return Ok(None);
            };
            result = Some(result.map_or(field, |old| old.hull(&field)));
        }
        if previous < self.duration() && times[0] <= self.duration() && times[1] >= previous {
            stationary = true;
        }
        if stationary {
            result = Some(result.map_or(zero, |old| old.hull(&zero)));
        }
        Ok(Some(result.unwrap_or(zero)))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn whole_key_domain_contains_velocity_jump_without_losing_normal_constraint() {
        let twist = |x| RootRigidTwist {
            linear: DVec3::X * x,
            angular: DVec3::Y,
        };
        let path = RootRigidPath::from_twists(&[(twist(1.), 0.3), (twist(-3.), 0.7)], 2).unwrap();
        let field = path
            .spatial_twist_enclosure_between([0.3_f64.next_down(), 0.3_f64.next_up()])
            .unwrap()
            .unwrap();
        assert_eq!(field.linear_bounds()[0], [-3., 1.]);
        assert_eq!(field.coordinate_velocity_range(1), Some([0., 0.]));
        let point = path
            .spatial_twist_enclosure_between([0.3, 0.3])
            .unwrap()
            .unwrap();
        assert_eq!(point.linear_bounds()[0], [-3., 1.]);
        assert!(path.spatial_twist_enclosure_between([0.8, 0.2]).is_err());
        let empty = RootRigidPath::from_twists(&[], 0).unwrap();
        assert_eq!(
            empty
                .spatial_twist_enclosure_between([0., 0.])
                .unwrap()
                .unwrap()
                .linear_bounds(),
            [[0., 0.]; 3]
        );
    }
}

#[cfg(test)]
mod gap_tests {
    use super::*;
    #[test]
    fn stationary_gaps_and_tilted_neighbor_are_not_hidden_by_key_hull() {
        let twist = |linear| RootRigidTwist {
            linear,
            angular: DVec3::ZERO,
        };
        let mut path = RootRigidPath::from_twists(
            &[
                (twist(DVec3::X), 0.2),
                (twist(DVec3::ZERO), 0.3),
                (twist(DVec3::X * 3.), 0.5),
            ],
            3,
        )
        .unwrap();
        // Removing a zero-motion span preserves geometry and creates a real gap.
        path.spans.remove(1);
        assert_eq!(
            path.spatial_twist_enclosure_between([0.3, 0.4])
                .unwrap()
                .unwrap()
                .linear_bounds(),
            [[0., 0.]; 3]
        );
        assert_eq!(
            path.spatial_twist_enclosure_between([0.2, 0.5])
                .unwrap()
                .unwrap()
                .linear_bounds()[0],
            [0., 3.]
        );
        let tilted = RootRigidPath::from_twists(
            &[
                (
                    RootRigidTwist {
                        linear: DVec3::X,
                        angular: DVec3::Y,
                    },
                    0.3,
                ),
                (
                    RootRigidTwist {
                        linear: DVec3::X,
                        angular: DVec3::new(1e-300, 1., 0.),
                    },
                    0.7,
                ),
            ],
            2,
        )
        .unwrap();
        assert!(tilted
            .spatial_twist_enclosure_between([0.2, 0.4])
            .unwrap()
            .unwrap()
            .coordinate_velocity_range(1)
            .is_none());
    }
}
