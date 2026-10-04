//! Outward conversion of conditional origin/angular error to world point error.
use super::*;
impl RootRigidApproximation {
    /// Encloses |scale|*(origin_error + rotation_chord*|point|) + world_radius.
    /// Error metadata and supplied world evaluation radius must already be valid
    /// bounds. This covers envelope arithmetic, not point extraction or collision
    /// normals. The stored point is interpreted exactly.
    pub fn enclosed_world_point_error_bound(
        &self,
        point: DVec3,
        scale: f64,
        world_radius: f64,
    ) -> Result<f64, AnimationError> {
        self.enclosed_world_point_box_error_bound(
            point.to_array().map(|v| [v, v]),
            scale,
            world_radius,
        )
    }
    /// Same envelope for all points in an outward coordinate box.
    pub fn enclosed_world_point_box_error_bound(
        &self,
        point: [[f64; 2]; 3],
        scale: f64,
        world_radius: f64,
    ) -> Result<f64, AnimationError> {
        if point
            .into_iter()
            .any(|v| !v[0].is_finite() || !v[1].is_finite() || v[0] > v[1])
            || !scale.is_finite()
            || [
                self.origin_error_bound,
                self.angular_error_bound,
                world_radius,
            ]
            .into_iter()
            .any(|v| !v.is_finite() || v < 0.)
        {
            return Err(AnimationError::NumericalOverflow);
        }
        if scale == 0. {
            return Ok(world_radius);
        }
        let chord = if self.angular_error_bound == 0. {
            Scalar::exact(0.)
        } else if self.angular_error_bound >= std::f64::consts::PI {
            // This branch also covers the tiny interval between stored PI and
            // real pi: the chord is always at most 2.
            Scalar::exact(2.)
        } else {
            let angle = Scalar::exact(self.angular_error_bound);
            let x = angle.div_positive(2.)?.square()?;
            // sin(z)/z: for z^2 <= 2.5, each successive absolute term
            // decreases (first ratio <= 2.5/6). The ninth term bounds remainder.
            if x.1 > 2.5 {
                return Err(AnimationError::NumericalOverflow);
            }
            let sine_over_z = series(x, Scalar::exact(1.), |n| (2 * n) * (2 * n + 1))?;
            Scalar::exact(angle.mul(sine_over_z)?.1.min(2.))
        };
        let radius = if point == [[0., 0.]; 3] || self.angular_error_bound == 0. {
            Scalar::exact(0.)
        } else {
            let mut squared = Scalar::exact(0.);
            for component in point {
                squared = squared.add(Scalar(component[0], component[1]).square()?)?;
            }
            let upper = squared.1.sqrt().next_up();
            if !upper.is_finite() {
                return Err(AnimationError::NumericalOverflow);
            }
            Scalar::exact(upper)
        };
        if self.origin_error_bound == 0. && (radius.1 == 0. || chord.1 == 0.) {
            return Ok(world_radius);
        }
        Ok(Scalar::exact(self.origin_error_bound)
            .add(chord.mul(radius)?)?
            .mul(Scalar::exact(scale.abs()))?
            .add(Scalar::exact(world_radius))?
            .1)
    }
}

impl RootRigidEnclosure {
    /// Encloses inverse similarity coordinates of an exact sum of stored vectors.
    /// Includes vector summation, enclosed origin, inverse rotation and signed
    /// scale division. Scale zero is singular and rejects. The frame represents
    /// a normalized real rotation, as required by its private enclosure owner.
    pub fn inverse_similarity_point_sum_bounds(
        &self,
        points: &[DVec3],
        scale: f64,
    ) -> Result<[[f64; 2]; 3], AnimationError> {
        if !scale.is_finite() || scale == 0. || points.iter().any(|p| !p.is_finite()) {
            return Err(AnimationError::NumericalOverflow);
        }
        let mut sum = [Scalar::exact(0.); 3];
        for point in points {
            for i in 0..3 {
                sum[i] = sum[i].add(Scalar::exact(point[i]))?;
            }
        }
        let (translation, rotation) = self.vectors();
        for i in 0..3 {
            sum[i] = sum[i].sub(translation[i])?;
        }
        let inverse = [
            Scalar(-rotation[0].1, -rotation[0].0),
            Scalar(-rotation[1].1, -rotation[1].0),
            Scalar(-rotation[2].1, -rotation[2].0),
            rotation[3],
        ];
        let mut mapped = rotate(inverse, sum)?;
        for component in &mut mapped {
            *component = component.div_positive(scale.abs())?;
            if scale < 0. {
                *component = Scalar(-component.1, -component.0);
            }
        }
        Ok(mapped.map(Scalar::array))
    }
}

impl RootRigidPath {
    /// Whole-span world point-speed caps per normalized span, for canonical
    /// ordered constant spatial fields. Prefixes are recomputed outward once.
    /// For constant (v,w), point velocity w cross x + v rotates under w, so its
    /// Euclidean magnitude is constant; bounding its initial value suffices.
    /// Stored point boxes and signed world scale are included. Larger angular
    /// increments require subdivision, matching the outward integrator contract.
    pub fn enclosed_screw_point_speed_bounds(
        &self,
        points: &[[[f64; 2]; 3]],
        scale: f64,
        max_spans: usize,
    ) -> Result<Vec<f64>, AnimationError> {
        self.prepare_screw_enclosures(max_spans)?
            .point_speed_bounds(points, scale)
    }
}
impl RootScrewEnclosurePath<'_> {
    /// Uses the prepared canonical prefixes for whole-span point speed caps.
    pub fn point_speed_bounds(
        &self,
        points: &[[[f64; 2]; 3]],
        scale: f64,
    ) -> Result<Vec<f64>, AnimationError> {
        if !scale.is_finite()
            || points.is_empty()
            || points
                .iter()
                .flatten()
                .any(|v| !v[0].is_finite() || !v[1].is_finite() || v[0] > v[1])
        {
            return Err(AnimationError::NumericalOverflow);
        }
        let mut bounds = Vec::with_capacity(self.path.spans.len());
        for (index, span) in self.path.spans.iter().enumerate() {
            let prefix = self.prefixes[index];
            let (twist, _) = span
                .screw
                .ok_or(AnimationError::RootRotationTransitionUnsupported)?;
            let (translation, rotation) = prefix.vectors();
            let omega = twist.angular.to_array().map(Scalar::exact);
            let linear = twist.linear.to_array().map(Scalar::exact);
            let h = Scalar::exact(span.end()).sub(Scalar::exact(span.start()))?;
            if h.0 <= 0. {
                return Err(AnimationError::RootRigidBudget);
            }
            let mut cap = 0_f64;
            for point in points {
                let mut position = rotate(rotation, point.map(|v| Scalar(v[0], v[1])))?;
                for i in 0..3 {
                    position[i] = position[i].add(translation[i])?;
                }
                let mut velocity = cross(omega, position)?;
                let mut squared = Scalar::exact(0.);
                for i in 0..3 {
                    velocity[i] = velocity[i].add(linear[i])?;
                    squared = squared.add(velocity[i].square()?)?;
                }
                let magnitude = squared.1.sqrt().next_up();
                if !magnitude.is_finite() {
                    return Err(AnimationError::NumericalOverflow);
                }
                let speed = Scalar::exact(magnitude)
                    .mul(h)?
                    .mul(Scalar::exact(scale.abs()))?
                    .1;
                cap = cap.max(speed);
            }
            // Preserve a truly stationary stored field without an invented
            // subnormal speed, after input/interval validation above.
            if twist.linear == DVec3::ZERO && twist.angular == DVec3::ZERO || scale == 0. {
                cap = 0.;
            }
            bounds.push(cap);
        }
        Ok(bounds)
    }
}

impl RootRigidEnclosure {
    /// Uniform rounding-only error for publishing any real coordinate in this
    /// world box as IEEE-754 round-to-nearest f32. Does not cover earlier point
    /// evaluation arithmetic. A caller may supply a whole-tick world enclosure.
    pub fn enclosed_f32_publication_error(
        world: [[f64;2];3],
    ) -> Result<([f64;3],f64),AnimationError> {
        let mut axes = [0.;3];
        let mut radius = Scalar::exact(0.);
        for axis in 0..3 {
            let [lo,hi] = world[axis];
            let magnitude = lo.abs().max(hi.abs());
            if !lo.is_finite() || !hi.is_finite() || lo > hi
                || magnitude > f64::from(f32::MAX) {
                return Err(AnimationError::NumericalOverflow);
            }
            axes[axis] = if lo == hi {
                let delta = Scalar::exact(lo).sub(Scalar::exact(f64::from(lo as f32)))?;
                delta.0.abs().max(delta.1.abs())
            } else {
                let mut upper = magnitude as f32;
                if f64::from(upper) < magnitude { upper = upper.next_up(); }
                // One full adjacent spacing bounds nearest rounding across all
                // smaller magnitudes, including subnormals and binade changes.
                if upper == f32::MAX {
                    f64::from(upper)-f64::from(upper.next_down())
                } else {
                    f64::from(upper.next_up())-f64::from(upper)
                }
            };
            radius = radius.add(Scalar::exact(axes[axis]))?;
        }
        Ok((axes,radius.1))
    }

    /// Bounds the discrepancy between the exact enclosed similarity image and
    /// a supplied, already evaluated world point (including f32 publication).
    /// Returns world-axis absolute bounds and an outward L1 radius covering the
    /// Euclidean discrepancy. This certifies this point/pose only, not all times
    /// in a trajectory or upstream curve compilation.
    pub fn enclosed_point_evaluation_error(
        &self, point: [[f64;2];3], scale: f64, evaluated: DVec3,
    ) -> Result<([f64;3],f64),AnimationError> {
        if !evaluated.is_finite() { return Err(AnimationError::NumericalOverflow); }
        let image = self.similarity_point_box_bounds(point,scale)?;
        let mut axes = [0.;3];
        let mut radius = Scalar::exact(0.);
        for axis in 0..3 {
            let delta = Scalar(image[axis][0],image[axis][1])
                .sub(Scalar::exact(evaluated[axis]))?;
            axes[axis] = delta.0.abs().max(delta.1.abs());
            radius = radius.add(Scalar::exact(axes[axis]))?;
        }
        Ok((axes,radius.1))
    }

    /// Outward image of every point in a coordinate box under this rigid frame.
    pub fn transform_point_box_bounds(
        &self,
        point: [[f64; 2]; 3],
    ) -> Result<[[f64; 2]; 3], AnimationError> {
        self.similarity_point_box_bounds(point, 1.)
    }
    /// Outward image under frame translation + scale*frame rotation*point.
    /// Includes signed scale; the rotation is the normalized real frame model.
    pub fn similarity_point_box_bounds(
        &self,
        point: [[f64; 2]; 3],
        scale: f64,
    ) -> Result<[[f64; 2]; 3], AnimationError> {
        if !scale.is_finite()
            || point
                .into_iter()
                .any(|v| !v[0].is_finite() || !v[1].is_finite() || v[0] > v[1])
        {
            return Err(AnimationError::NumericalOverflow);
        }
        let (translation, rotation) = self.vectors();
        let mut mapped = rotate(rotation, point.map(|v| Scalar(v[0], v[1])))?;
        for i in 0..3 {
            mapped[i] = mapped[i].mul(Scalar::exact(scale))?.add(translation[i])?;
        }
        Ok(mapped.map(Scalar::array))
    }
}
