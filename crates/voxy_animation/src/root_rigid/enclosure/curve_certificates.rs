//! Source screw-point discrepancy certificates and runtime pose evaluation error bounds.
use super::*;

impl RootRigidCurve {
    /// Exact source spatial field relative to a fixed source reference.
    /// Includes every loop prefix and both channels' key domains. The returned
    /// hull covers the whole interval, without a derivative or axis constraint.
    /// STEP impulses reject; clamp completion tails have zero velocity.
    pub fn source_delta_twist_enclosure(
        &self,
        reference: f64,
        times: [f64; 2],
        axes: [bool; 3],
        limit: usize,
    ) -> Result<Option<RootRigidTwistEnclosure>, AnimationError> {
        if !reference.is_finite() || reference < 0. || reference > times[0] {
            return Err(AnimationError::InvalidSampleTime);
        }
        let cuts = self.source_partition_cuts(times, limit)?;
        let Some(initial) = self.source_sample_enclosure(reference, axes)? else {
            return Ok(None);
        };
        let inverse = initial.inverse()?;
        let zero = RootRigidTwist {
            linear: DVec3::ZERO,
            angular: DVec3::ZERO,
        }
        .enclosure()?;
        let mut result: Option<RootRigidTwistEnclosure> = None;
        // A point domain retains its source derivative; wall retiming supplies
        // the zero clock rate for paused playback.
        let cells = if times[0] == times[1] {
            vec![times]
        } else {
            cuts.windows(2).map(|cell| [cell[0], cell[1]]).collect()
        };
        for cell in cells {
            let field = if self.0.playback == Playback::Clamp && cell[0] >= self.0.duration {
                zero
            } else {
                let phases = crate::root_clock::root_segment_phases(
                    cell[0],
                    cell[1],
                    self.0.duration as f32,
                    self.0.playback,
                )?;
                let Some(local) = self.source_phase_twist_enclosure(phases, axes)? else {
                    return Ok(None);
                };
                let clock = crate::enclose_root_cycle_phase(
                    cell[0],
                    self.0.duration as f32,
                    self.0.playback,
                )?;
                let Some(prefix) = self.source_cycle_prefix_enclosure(clock.cycle(), axes)? else {
                    return Ok(None);
                };
                local.transformed(&inverse.compose(&prefix)?, 1.)?
            };
            result = Some(match result {
                Some(previous) => previous.hull(&field),
                None => field,
            });
        }
        Ok(result)
    }

    fn source_partition_cuts(
        &self,
        times: [f64; 2],
        limit: usize,
    ) -> Result<Vec<f64>, AnimationError> {
        if times.into_iter().any(|v| !v.is_finite() || v < 0.)
            || times[1] < times[0]
            || limit == 0
            || limit > MAX_ROOT_ROTATION_SPANS
        {
            return Err(AnimationError::InvalidSampleTime);
        }
        let first =
            crate::enclose_root_cycle_phase(times[0], self.0.duration as f32, self.0.playback)?;
        let last =
            crate::enclose_root_cycle_phase(times[1], self.0.duration as f32, self.0.playback)?;
        if last.cycle() - first.cycle() > limit as u64 {
            return Err(AnimationError::RootRigidBudget);
        }
        let mut keys =
            self.0
                .translation
                .cuts(0., self.0.duration, crate::MAX_ROOT_ROTATION_CACHE_KEYS)?;
        keys.extend(self.0.rotation.phase_key_times());
        keys.push(self.0.duration);
        keys.sort_by(f64::total_cmp);
        keys.dedup();
        let mut cuts = vec![times[0], times[1]];
        for cycle in first.cycle()..=last.cycle() {
            for &key in &keys {
                let absolute = cycle as f64 * self.0.duration + key;
                if absolute < times[0] || absolute > times[1] {
                    continue;
                }
                if self.0.playback == Playback::Loop {
                    let clock = crate::enclose_root_cycle_phase(
                        absolute,
                        self.0.duration as f32,
                        Playback::Loop,
                    )?;
                    let expected = if key == self.0.duration {
                        (cycle + 1, 0.)
                    } else {
                        (cycle, key)
                    };
                    if (clock.cycle(), clock.phase()) != expected
                        || clock.exact_phase_bounds() != [clock.phase(); 2]
                    {
                        return Err(AnimationError::RootRigidBudget);
                    }
                }
                cuts.push(absolute);
                if cuts.len() > 2 * (limit + 1) {
                    return Err(AnimationError::RootRigidBudget);
                }
            }
        }
        cuts.sort_by(f64::total_cmp);
        cuts.dedup();
        if cuts.len() - 1 > limit {
            return Err(AnimationError::RootRigidBudget);
        }
        Ok(cuts)
    }

    /// Uniform source-motion cells split at both channels' keys and loop seams.
    /// Nonrepresentable authored boundaries reject rather than moving an event.
    pub fn source_delta_partition(
        &self,
        reference: f64,
        times: [f64; 2],
        axes: [bool; 3],
        limit: usize,
    ) -> Result<Option<Vec<([f64; 2], RootRigidEnclosure)>>, AnimationError> {
        if !reference.is_finite() || reference < 0. || reference > times[0] {
            return Err(AnimationError::InvalidSampleTime);
        }
        let cuts = self.source_partition_cuts(times, limit)?;
        let mut result = Vec::with_capacity(cuts.len() - 1);
        for cell in cuts.windows(2) {
            let times = [cell[0], cell[1]];
            let Some(enclosure) = self.source_delta_interval_enclosure(reference, times, axes)?
            else {
                return Ok(None);
            };
            result.push((times, enclosure));
        }
        Ok(Some(result))
    }

    /// Uniform geometric point discrepancy against a canonical screw reference.
    /// Covers source compilation and loop factors, but not runtime evaluation,
    /// world mapping, f32 publication or a velocity-blended fade of two clips.
    /// Independent hull subtraction is conservative and may require refinement.
    pub fn source_screw_point_error(
        &self,
        times: [f64; 2],
        axes: [bool; 3],
        point: [[f64; 2]; 3],
        candidate: &RootScrewEnclosurePath<'_>,
        limit: usize,
    ) -> Result<Option<([f64; 3], f64)>, AnimationError> {
        if candidate.path().duration() != times[1] - times[0] {
            return Err(AnimationError::InvalidSampleTime);
        }
        self.source_screw_point_error_refined(times, axes, point, candidate, limit, 1)
    }

    /// Synchronized source/candidate hull comparison with bounded subdivision.
    /// Canonical candidate time is mapped affinely over the exact source duration.
    pub fn source_screw_point_error_refined(
        &self,
        times: [f64; 2],
        axes: [bool; 3],
        point: [[f64; 2]; 3],
        candidate: &RootScrewEnclosurePath<'_>,
        limit: usize,
        subdivisions: usize,
    ) -> Result<Option<([f64; 3], f64)>, AnimationError> {
        if subdivisions == 0
            || subdivisions > MAX_ROOT_ROTATION_SPANS
            || candidate.path().duration() != times[1] - times[0]
        {
            return Err(AnimationError::InvalidSampleTime);
        }
        let Some(partition) = self.source_delta_partition(times[0], times, axes, limit)? else {
            return Ok(None);
        };
        if partition
            .len()
            .checked_mul(subdivisions)
            .is_none_or(|n| n > MAX_ROOT_ROTATION_SPANS)
        {
            return Err(AnimationError::RootRigidBudget);
        }
        let mut error = [0_f64; 3];
        let duration = Scalar::exact(times[1]).sub(Scalar::exact(times[0]))?;
        let mut compare =
            |source: RootRigidEnclosure, cell: [f64; 2]| -> Result<(), AnimationError> {
                let source_box = source.transform_point_box_bounds(point)?;
                let candidate_times = if times[0] == times[1] {
                    [0., 0.]
                } else {
                    let mapped = Scalar(cell[0], cell[1])
                        .sub(Scalar::exact(times[0]))?
                        .div_interval_positive(duration)?
                        .mul(Scalar::exact(candidate.path().duration()))?;
                    [mapped.0.max(0.), mapped.1.min(candidate.path().duration())]
                };
                let candidate_box = candidate.point_box_bounds_between(candidate_times, point)?;
                for axis in 0..3 {
                    let delta = Scalar(source_box[axis][0], source_box[axis][1])
                        .sub(Scalar(candidate_box[axis][0], candidate_box[axis][1]))?;
                    error[axis] = error[axis].max(delta.0.abs().max(delta.1.abs()));
                }
                Ok(())
            };
        if partition.is_empty() {
            compare(RootRigidEnclosure::IDENTITY, times)?;
        }
        for (cell, _) in partition {
            let mut start = cell[0];
            for index in 1..=subdivisions {
                let end = if index == subdivisions {
                    cell[1]
                } else {
                    cell[0] + (cell[1] - cell[0]) * (index as f64 / subdivisions as f64)
                };
                if end <= start {
                    return Err(AnimationError::RootRigidBudget);
                }
                let Some(source) =
                    self.source_delta_interval_enclosure(times[0], [start, end], axes)?
                else {
                    return Ok(None);
                };
                compare(source, [start, end])?;
                start = end;
            }
        }
        let mut radius = Scalar::exact(0.);
        for axis in error {
            radius = radius.add(Scalar::exact(axis))?;
        }
        Ok(Some((error, radius.1)))
    }

    /// Refines uniform geometric discrepancy until tolerance is proved.
    /// Budget counts all source comparison cells across refinement attempts.
    pub fn certify_source_screw_point_error<'a>(
        &'a self,
        times: [f64; 2],
        axes: [bool; 3],
        point: [[f64; 2]; 3],
        candidate: &'a RootScrewEnclosurePath<'a>,
        tolerance: f64,
        max_cells: usize,
    ) -> Result<Option<RootSourceScrewPointCertificate<'a>>, AnimationError> {
        if !tolerance.is_finite()
            || tolerance < 0.
            || max_cells == 0
            || max_cells > MAX_ROOT_ROTATION_SPANS
        {
            return Err(AnimationError::InvalidSampleTime);
        }
        let Some(partition) = self.source_delta_partition(times[0], times, axes, max_cells)? else {
            return Ok(None);
        };
        let count = partition.len().max(1);
        let mut subdivisions = 1_usize;
        let mut evaluated_cells = 0_usize;
        loop {
            let work = count
                .checked_mul(subdivisions)
                .ok_or(AnimationError::RootRigidBudget)?;
            evaluated_cells = evaluated_cells
                .checked_add(work)
                .ok_or(AnimationError::RootRigidBudget)?;
            if evaluated_cells > max_cells {
                return Err(AnimationError::RootRigidBudget);
            }
            let Some((axis_error, radius)) = self.source_screw_point_error_refined(
                times,
                axes,
                point,
                candidate,
                max_cells,
                subdivisions,
            )?
            else {
                return Ok(None);
            };
            if radius <= tolerance {
                return Ok(Some(RootSourceScrewPointCertificate {
                    source: self,
                    candidate,
                    times,
                    extraction_axes: axes,
                    point,
                    axis_error,
                    radius,
                    subdivisions,
                    evaluated_cells,
                }));
            }
            subdivisions = subdivisions
                .checked_mul(2)
                .ok_or(AnimationError::RootRigidBudget)?;
        }
    }

    /// Uniform source-to-runtime image error for phase() then translation+q*point.
    /// One continuous shared local-key interval; loop prefix evaluation is separate.
    pub fn phase_point_evaluation_error_bounds(
        &self,
        times: [f64; 2],
        axes: [bool; 3],
        point: [[f64; 2]; 3],
    ) -> Result<Option<[f64; 3]>, AnimationError> {
        let Some(source) = self.source_phase_interval_enclosure(times, axes)? else {
            return Ok(None);
        };
        let position = self
            .0
            .translation
            .source_position_bounds(times[0], times[1])?;
        let position_error = self
            .0
            .translation
            .interval_evaluation_error_bounds(times[0], times[1])?;
        let Some(rotation_error) = self
            .0
            .rotation
            .cubic_relative_interval_evaluation_error_bounds(times[0], times[1])?
        else {
            return Ok(None);
        };
        Ok(Some(compilation::root_phase_point_error(
            position,
            position_error,
            source.rotation_bounds(),
            rotation_error,
            self.0.origin,
            self.0.bind,
            axes,
            point,
        )?))
    }

    /// Fixed runtime rigid loop-prefix discrepancy against its exact source.
    pub fn cycle_prefix_evaluation_error_bounds(
        &self,
        cycles: u64,
        axes: [bool; 3],
    ) -> Result<Option<([f64; 3], [f64; 4])>, AnimationError> {
        let Some(source) = self.source_cycle_prefix_enclosure(cycles, axes)? else {
            return Ok(None);
        };
        let evaluated = power(self.phase(self.0.duration, axes)?, cycles)?;
        let mut translation = [0.; 3];
        let mut rotation = [0.; 4];
        for i in 0..3 {
            let delta = Scalar(source.translation[i][0], source.translation[i][1])
                .sub(Scalar::exact(evaluated.translation[i]))?;
            translation[i] = delta.0.abs().max(delta.1.abs());
        }
        for i in 0..4 {
            let delta = Scalar(source.rotation[i][0], source.rotation[i][1])
                .sub(Scalar::exact(evaluated.rotation.to_array()[i]))?;
            rotation[i] = delta.0.abs().max(delta.1.abs());
        }
        Ok(Some((translation, rotation)))
    }

    /// Uniform sample() then translation+rotation*point error at an explicit cycle.
    /// Wall-clock phase and world/f32 publication remain separate.
    pub fn cycle_point_evaluation_error_bounds(
        &self,
        cycles: u64,
        times: [f64; 2],
        axes: [bool; 3],
        point: [[f64; 2]; 3],
    ) -> Result<Option<[f64; 3]>, AnimationError> {
        let Some(prefix) = self.source_cycle_prefix_enclosure(cycles, axes)? else {
            return Ok(None);
        };
        let Some((prefix_t_error, prefix_q_error)) =
            self.cycle_prefix_evaluation_error_bounds(cycles, axes)?
        else {
            return Ok(None);
        };
        let Some(local) = self.source_phase_interval_enclosure(times, axes)? else {
            return Ok(None);
        };
        let Some(local_t_error) =
            self.phase_point_evaluation_error_bounds(times, axes, [[0.; 2]; 3])?
        else {
            return Ok(None);
        };
        let Some(local_q_error) = self
            .0
            .rotation
            .cubic_relative_interval_evaluation_error_bounds(times[0], times[1])?
        else {
            return Ok(None);
        };
        Ok(Some(compilation::composed_rigid_point_error(
            &prefix,
            prefix_t_error,
            prefix_q_error,
            &local,
            local_t_error,
            local_q_error,
            point,
        )?))
    }

    /// Uniform source-to-published point discrepancy for an explicit loop cell.
    /// Matches sample point evaluation, then actual_scale*(actual_frame.rotation*p)
    /// +actual_frame.translation, then direct f32 cast. Fixed source frames only.
    pub fn cycle_world_point_evaluation_error_bounds(
        &self,
        cycles: u64,
        times: [f64; 2],
        axes: [bool; 3],
        point: [[f64; 2]; 3],
        source_frame: &RootRigidEnclosure,
        source_scale: RootUniformScaleEnclosure,
        actual_frame: RootRigidTransform,
        actual_scale: f64,
    ) -> Result<Option<([f64; 3], f64)>, AnimationError> {
        let Some(prefix) = self.source_cycle_prefix_enclosure(cycles, axes)? else {
            return Ok(None);
        };
        let Some(local) = self.source_phase_interval_enclosure(times, axes)? else {
            return Ok(None);
        };
        let source_point = prefix.compose(&local)?.transform_point_box_bounds(point)?;
        let Some(point_error) =
            self.cycle_point_evaluation_error_bounds(cycles, times, axes, point)?
        else {
            return Ok(None);
        };
        let (before_cast, actual_box) = compilation::mapped_point_runtime_error(
            source_point,
            point_error,
            source_frame,
            source_scale,
            actual_frame,
            actual_scale,
        )?;
        let (publication, _) = RootRigidEnclosure::enclosed_f32_publication_error(actual_box)?;
        let mut axes = [0.; 3];
        let mut radius = Scalar::exact(0.);
        for i in 0..3 {
            axes[i] = Scalar::exact(before_cast[i])
                .add(Scalar::exact(publication[i]))?
                .1;
            radius = radius.add(Scalar::exact(axes[i]))?;
        }
        Ok(Some((axes, radius.1)))
    }

    /// Uniform sample/map/f32 error on an absolute wall-clock interval.
    /// Splits channel keys/loops and refines unresolved normalization domains.
    /// Seam endpoints also certify the actual next-cycle sampling branch.
    pub fn world_point_evaluation_error_bounds(
        &self,
        times: [f64; 2],
        axes: [bool; 3],
        point: [[f64; 2]; 3],
        source_frame: &RootRigidEnclosure,
        source_scale: RootUniformScaleEnclosure,
        actual_frame: RootRigidTransform,
        actual_scale: f64,
        max_cells: usize,
    ) -> Result<Option<([f64; 3], f64)>, AnimationError> {
        let Some(partition) = self.source_delta_partition(times[0], times, axes, max_cells)? else {
            return Ok(None);
        };
        let mut pending: Vec<[f64; 2]> = partition.into_iter().map(|(cell, _)| cell).collect();
        if pending.is_empty() {
            pending.push(times);
        }
        let mut result = [0_f64; 3];
        let mut visited = 0_usize;
        let mut merge = |error: [f64; 3]| {
            for i in 0..3 {
                result[i] = result[i].max(error[i]);
            }
        };
        while let Some(cell) = pending.pop() {
            visited += 1;
            if visited > max_cells {
                return Err(AnimationError::RootRigidBudget);
            }
            let clock =
                crate::enclose_root_cycle_phase(cell[0], self.0.duration as f32, self.0.playback)?;
            let phases = crate::root_clock::root_segment_phases(
                cell[0],
                cell[1],
                self.0.duration as f32,
                self.0.playback,
            )?;
            match self.cycle_world_point_evaluation_error_bounds(
                clock.cycle(),
                phases,
                axes,
                point,
                source_frame,
                source_scale,
                actual_frame,
                actual_scale,
            ) {
                Ok(Some((error, _))) => merge(error),
                Ok(None) => return Ok(None),
                Err(AnimationError::InvalidRootRotationCurve) => {
                    let mid = cell[0] + (cell[1] - cell[0]) * 0.5;
                    if mid <= cell[0] || mid >= cell[1] {
                        return Err(AnimationError::RootRigidBudget);
                    }
                    pending.push([mid, cell[1]]);
                    pending.push([cell[0], mid]);
                    continue;
                }
                Err(error) => return Err(error),
            }
            let end_clock =
                crate::enclose_root_cycle_phase(cell[1], self.0.duration as f32, self.0.playback)?;
            if end_clock.cycle_index() != clock.cycle_index() {
                visited += 1;
                if visited > max_cells {
                    return Err(AnimationError::RootRigidBudget);
                }
                let Some((error, _)) = self.cycle_world_point_evaluation_error_bounds(
                    end_clock.cycle_index(),
                    [end_clock.phase(); 2],
                    axes,
                    point,
                    source_frame,
                    source_scale,
                    actual_frame,
                    actual_scale,
                )?
                else {
                    return Ok(None);
                };
                merge(error);
            }
        }
        let mut radius = Scalar::exact(0.);
        for error in result {
            radius = radius.add(Scalar::exact(error))?;
        }
        Ok(Some((result, radius.1)))
    }
}

/// Borrowed uniform geometric source/reference discrepancy proof.
/// Runtime pose evaluation and world publication errors are separate obligations.
#[derive(Debug)]
pub struct RootSourceScrewPointCertificate<'a> {
    source: &'a RootRigidCurve,
    candidate: &'a RootScrewEnclosurePath<'a>,
    times: [f64; 2],
    extraction_axes: [bool; 3],
    point: [[f64; 2]; 3],
    axis_error: [f64; 3],
    radius: f64,
    subdivisions: usize,
    evaluated_cells: usize,
}
impl<'a> RootSourceScrewPointCertificate<'a> {
    pub fn source(&self) -> &RootRigidCurve {
        self.source
    }
    pub fn candidate(&self) -> &RootScrewEnclosurePath<'a> {
        self.candidate
    }
    pub fn times(&self) -> [f64; 2] {
        self.times
    }
    pub fn extraction_axes(&self) -> [bool; 3] {
        self.extraction_axes
    }
    pub fn point_bounds(&self) -> [[f64; 2]; 3] {
        self.point
    }
    pub fn axis_error_bounds(&self) -> [f64; 3] {
        self.axis_error
    }
    pub fn radius(&self) -> f64 {
        self.radius
    }
    pub fn subdivisions(&self) -> usize {
        self.subdivisions
    }
    pub fn evaluated_cells(&self) -> usize {
        self.evaluated_cells
    }
}

impl RootSourceScrewPointCertificate<'_> {
    /// Geometric discrepancy plus direct f32 rounding in a fixed common frame.
    /// Does not include runtime quaternion/matrix evaluation before conversion.
    pub fn mapped_f32_geometric_error(
        &self,
        frame: &RootRigidEnclosure,
        scale: RootUniformScaleEnclosure,
    ) -> Result<([f64; 3], f64), AnimationError> {
        let (_, rotation) = frame.vectors();
        let local = self.axis_error.map(|error| Scalar(-error, error));
        let mut difference = rotate(rotation, local)?;
        for axis in &mut difference {
            *axis = axis.mul(scale.value)?;
        }
        let candidate = self.candidate.whole_path_point_box_bounds(self.point)?;
        let world = frame.similarity_point_box_bounds_enclosed(candidate, scale)?;
        let (publication, _) = RootRigidEnclosure::enclosed_f32_publication_error(world)?;
        let mut axes = [0.; 3];
        let mut radius = Scalar::exact(0.);
        for axis in 0..3 {
            axes[axis] = Scalar::exact(difference[axis].0.abs().max(difference[axis].1.abs()))
                .add(Scalar::exact(publication[axis]))?
                .1;
            radius = radius.add(Scalar::exact(axes[axis]))?;
        }
        Ok((axes, radius.1))
    }
}

impl RootRigidEnclosure {
    /// Uniform rounding cap for glam 0.33.7 DQuat*DVec3, then scale and offset.
    /// Inputs are stored f64 values within these boxes. The reference is the exact
    /// homogeneous quaternion polynomial, not a separately normalized rotation.
    /// Quaternion compilation/normalization errors must be bounded separately.
    pub fn stored_similarity_evaluation_error(
        q: [[f64; 2]; 4],
        point: [[f64; 2]; 3],
        scale: [f64; 2],
        offset: [[f64; 2]; 3],
    ) -> Result<[f64; 3], AnimationError> {
        compilation::stored_similarity_evaluation_error(q, point, scale, offset)
    }
}
