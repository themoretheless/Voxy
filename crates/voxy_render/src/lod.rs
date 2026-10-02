//! Visual LOD policy, independent of allocation residency and scene/physics owners.
/// Levels are ordered from finest to coarsest, including exact base geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LodLevel {
    /// Geometric deviation in object-space units. A strict pixel-error guarantee
    /// requires a conservative bound against the original surface, not merely a
    /// simplifier's cost or an inflated switching metric. Validation here checks
    /// finiteness/order only; it does not certify supplied geometry or attributes.
    pub object_error: f64,
    pub index_count: u32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LodPolicy {
    /// Geometric screen-error target, conditional on conservative input errors.
    /// Does not bound normal, UV, color, material or deformation differences.
    pub target_pixels: f64,
    /// Relative dead band in [0, 1); refinement tolerates target * (1 + hysteresis).
    pub hysteresis: f64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LodError {
    InvalidLevels,
    InvalidProjection,
    InvalidPolicy,
    InvalidPreviousLevel,
}
impl LodPolicy {
    /// Pixel scale for an orthographic view with square pixels.
    /// # Errors
    /// Rejects nonpositive/nonfinite viewport height or vertical world span.
    pub fn orthographic_pixel_scale(
        pixel_height: f64,
        vertical_span: f64,
    ) -> Result<f64, LodError> {
        if !pixel_height.is_finite()
            || pixel_height <= 0.0
            || !vertical_span.is_finite()
            || vertical_span <= 0.0
        {
            return Err(LodError::InvalidProjection);
        }
        let scale = pixel_height / vertical_span;
        if !scale.is_finite() {
            return Err(LodError::InvalidProjection);
        }
        Ok(scale)
    }

    /// Conservative perspective pixel scale for square pixels. Bounds must cover
    /// the full source/approximation domain in view space: minimum positive depth
    /// and maximum distance from the optical axis in the XY plane. The radial
    /// term includes screen displacement caused by depth error off-axis.
    /// None requests exact base geometry when the domain crosses the near plane.
    /// # Errors
    /// Rejects invalid bounds, viewport, field of view, near plane or overflow.
    pub fn perspective_pixel_scale(
        pixel_height: f64,
        vertical_fov: f64,
        nearest_depth: f64,
        radial_bound: f64,
        near_plane: f64,
    ) -> Result<Option<f64>, LodError> {
        if !pixel_height.is_finite()
            || pixel_height <= 0.0
            || !vertical_fov.is_finite()
            || !(0.0..std::f64::consts::PI).contains(&vertical_fov)
            || vertical_fov == 0.0
            || !nearest_depth.is_finite()
            || !radial_bound.is_finite()
            || radial_bound < 0.0
            || !near_plane.is_finite()
            || near_plane <= 0.0
        {
            return Err(LodError::InvalidProjection);
        }
        if nearest_depth <= near_plane {
            return Ok(None);
        }
        let scale = pixel_height / (2.0 * (vertical_fov * 0.5).tan()) / nearest_depth
            * 1.0_f64.hypot(radial_bound / nearest_depth);
        if !scale.is_finite() {
            return Err(LodError::InvalidProjection);
        }
        Ok(Some(scale))
    }

    /// Choose the coarsest eligible representation. The caller supplies a conservative
    /// world-scale bound and pixels per world unit for the object's nearest visible
    /// depth; orthographic views supply a depth-independent value. Near-plane-crossing
    /// objects should retain base geometry until a finite bound can be established.
    /// Selection never claims that resident bytes have been released.
    ///
    /// # Errors
    /// Rejects nonfinite/negative inputs, invalid triangle counts, unordered levels,
    /// missing exact base geometry and an out-of-range previous representation.
    pub fn select(
        self,
        levels: &[LodLevel],
        world_scale: f64,
        pixels_per_world_unit: f64,
        previous: Option<usize>,
    ) -> Result<usize, LodError> {
        if !self.target_pixels.is_finite()
            || self.target_pixels <= 0.0
            || !(self.target_pixels * (1.0 + self.hysteresis)).is_finite()
            || !self.hysteresis.is_finite()
            || !(0.0..1.0).contains(&self.hysteresis)
        {
            return Err(LodError::InvalidPolicy);
        }
        if !world_scale.is_finite()
            || world_scale < 0.0
            || !pixels_per_world_unit.is_finite()
            || pixels_per_world_unit < 0.0
        {
            return Err(LodError::InvalidProjection);
        }
        if levels.is_empty()
            || levels[0].object_error != 0.0
            || levels.iter().any(|level| {
                !level.object_error.is_finite()
                    || level.object_error < 0.0
                    || level.index_count == 0
                    || !level.index_count.is_multiple_of(3)
            })
            || levels.windows(2).any(|pair| {
                pair[1].object_error < pair[0].object_error
                    || pair[1].index_count > pair[0].index_count
            })
        {
            return Err(LodError::InvalidLevels);
        }
        if previous.is_some_and(|index| index >= levels.len()) {
            return Err(LodError::InvalidPreviousLevel);
        }
        let projection = world_scale * pixels_per_world_unit;
        if !projection.is_finite() {
            return Err(LodError::InvalidProjection);
        }
        let error = |index: usize| levels[index].object_error * projection;
        let limit = previous.map_or(self.target_pixels, |_| {
            self.target_pixels * (1.0 - self.hysteresis)
        });
        let candidate = levels
            .iter()
            .enumerate()
            .rfind(|(index, _)| error(*index) <= limit)
            .map_or(0, |(index, _)| index);
        if let Some(old) = previous
            && candidate < old
            && error(old) <= self.target_pixels * (1.0 + self.hysteresis)
        {
            return Ok(old);
        }
        Ok(candidate)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    const LEVELS: [LodLevel; 3] = [
        LodLevel {
            object_error: 0.0,
            index_count: 300,
        },
        LodLevel {
            object_error: 0.01,
            index_count: 150,
        },
        LodLevel {
            object_error: 0.1,
            index_count: 30,
        },
    ];
    const POLICY: LodPolicy = LodPolicy {
        target_pixels: 2.0,
        hysteresis: 0.2,
    };
    #[test]
    fn screen_error_and_scale_control_quality() {
        assert_eq!(POLICY.select(&LEVELS, 1.0, 10.0, None), Ok(2));
        assert_eq!(POLICY.select(&LEVELS, 1.0, 100.0, None), Ok(1));
        assert_eq!(POLICY.select(&LEVELS, 3.0, 100.0, None), Ok(0));
    }
    #[test]
    fn transitions_have_a_dead_band() {
        assert_eq!(POLICY.select(&LEVELS, 1.0, 19.0, Some(1)), Ok(1));
        assert_eq!(POLICY.select(&LEVELS, 1.0, 15.0, Some(1)), Ok(2));
        assert_eq!(POLICY.select(&LEVELS, 1.0, 23.0, Some(2)), Ok(2));
        assert_eq!(POLICY.select(&LEVELS, 1.0, 25.0, Some(2)), Ok(1));
    }
    #[test]
    fn invalid_inputs_never_produce_a_draw_index() {
        assert_eq!(
            POLICY.select(&[], 1.0, 1.0, None),
            Err(LodError::InvalidLevels)
        );
        assert_eq!(
            POLICY.select(&LEVELS, f64::NAN, 1.0, None),
            Err(LodError::InvalidProjection)
        );
        assert_eq!(
            POLICY.select(&LEVELS, 1.0, 1.0, Some(3)),
            Err(LodError::InvalidPreviousLevel)
        );
        assert_eq!(
            POLICY.select(&LEVELS, f64::MAX, 2.0, None),
            Err(LodError::InvalidProjection)
        );
        let mut broken = LEVELS;
        broken[1].index_count = 301;
        assert_eq!(
            POLICY.select(&broken, 1.0, 1.0, None),
            Err(LodError::InvalidLevels)
        );
    }
}

/// Immutable index variants referencing one shared vertex domain.
#[derive(Debug)]
pub struct LodIndexSet {
    vertex_count: usize,
    index_bytes: u64,
    indices: Vec<Vec<u32>>,
    levels: Vec<LodLevel>,
}
impl LodIndexSet {
    /// Validates index domains and derives counts from actual triangle data.
    /// Input order is finest to coarsest; the base error must be zero.
    /// This owns CPU indices only and makes no GPU residency claim.
    /// # Errors
    /// Rejects empty vertices/levels, invalid errors/count ordering, non-triangle
    /// lists, out-of-domain indices and counts that cannot fit u32.
    pub fn new(vertex_count: usize, variants: Vec<(f64, Vec<u32>)>) -> Result<Self, LodError> {
        if vertex_count == 0
            || variants
                .iter()
                .any(|(_, indices)| indices.iter().any(|index| *index as usize >= vertex_count))
        {
            return Err(LodError::InvalidLevels);
        }
        let levels = variants
            .iter()
            .map(|(object_error, indices)| {
                Ok(LodLevel {
                    object_error: *object_error,
                    index_count: u32::try_from(indices.len())
                        .map_err(|_| LodError::InvalidLevels)?,
                })
            })
            .collect::<Result<Vec<_>, LodError>>()?;
        LodPolicy {
            target_pixels: 1.0,
            hysteresis: 0.0,
        }
        .select(&levels, 1.0, 1.0, None)?;
        let index_bytes = levels.iter().try_fold(0_u64, |total, level| {
            total
                .checked_add(u64::from(level.index_count) * 4)
                .ok_or(LodError::InvalidLevels)
        })?;
        Ok(Self {
            vertex_count,
            index_bytes,
            indices: variants.into_iter().map(|(_, indices)| indices).collect(),
            levels,
        })
    }
    #[must_use]
    pub fn levels(&self) -> &[LodLevel] {
        &self.levels
    }
    /// Vertex domain required when binding these variants to geometry.
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.vertex_count
    }
    /// Logical u32 index bytes for all variants, including the exact base.
    /// This excludes vertex buffers, Vec capacity, GPU alignment and driver data.
    /// Selecting a coarser level does not change this resident-all-variants cost.
    #[must_use]
    pub fn index_bytes(&self) -> u64 {
        self.index_bytes
    }
    #[must_use]
    pub fn indices(&self, level: usize) -> Option<&[u32]> {
        self.indices.get(level).map(Vec::as_slice)
    }
}

#[cfg(test)]
mod index_set_tests {
    use super::*;
    #[test]
    fn counts_come_from_indices_and_selected_variants_share_vertex_domain() {
        let set = LodIndexSet::new(
            4,
            vec![(0.0, vec![0, 1, 2, 0, 2, 3]), (0.25, vec![0, 1, 2])],
        )
        .unwrap();
        let chosen = LodPolicy {
            target_pixels: 1.0,
            hysteresis: 0.0,
        }
        .select(set.levels(), 1.0, 2.0, None)
        .unwrap();
        assert_eq!(chosen, 1);
        assert_eq!(set.vertex_count(), 4);
        assert_eq!(set.index_bytes(), 36);
        assert_eq!(
            set.levels()[chosen].index_count as usize,
            set.indices(chosen).unwrap().len()
        );
        assert!(set.indices(2).is_none());
    }
    #[test]
    fn invalid_variant_cannot_enter_the_shared_vertex_domain() {
        assert!(LodIndexSet::new(3, vec![(0.0, vec![0, 1, 3])]).is_err());
        assert!(LodIndexSet::new(3, vec![(0.0, vec![0, 1])]).is_err());
        assert!(
            LodIndexSet::new(3, vec![(0.0, vec![0, 1, 2]), (1.0, vec![0, 1, 2, 0, 1, 2])]).is_err()
        );
        assert!(LodIndexSet::new(3, vec![(f64::NAN, vec![0, 1, 2])]).is_err());
    }
}

#[cfg(test)]
mod projection_tests {
    use super::*;
    #[test]
    fn projection_accounts_for_off_axis_depth_and_near_plane() {
        let center =
            LodPolicy::perspective_pixel_scale(1000.0, std::f64::consts::FRAC_PI_2, 10.0, 0.0, 0.1)
                .unwrap()
                .unwrap();
        assert!((center - 50.0).abs() < 1e-10);
        let off_axis = LodPolicy::perspective_pixel_scale(
            1000.0,
            std::f64::consts::FRAC_PI_2,
            10.0,
            10.0,
            0.1,
        )
        .unwrap()
        .unwrap();
        assert!(off_axis > center);
        assert!(
            LodPolicy::perspective_pixel_scale(1000.0, 1.0, 0.1, 1.0, 0.1)
                .unwrap()
                .is_none()
        );
        assert!(LodPolicy::orthographic_pixel_scale(1000.0, 20.0).is_ok());
        assert!(LodPolicy::orthographic_pixel_scale(1000.0, 0.0).is_err());
    }
}
