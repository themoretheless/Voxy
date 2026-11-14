use super::ArchiveInstance;
use glam::{DVec2, DVec3, DVec4, Mat4, Vec3, Vec4};
use std::ops::Range;

#[derive(Clone, Copy)]
pub(super) struct PlantBounds {
    center: DVec3,
    half_extent: DVec3,
    transform_magnitude: DVec3,
}

pub(super) fn wind_bounds(positions: impl Iterator<Item = Vec3>) -> Option<(DVec3, DVec3)> {
    let (mut min, mut max) = (DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY));
    for p in positions {
        min = min.min(p.as_dvec3());
        max = max.max(p.as_dvec3());
    }
    let magnitude = min.abs().max(max.abs());
    if !magnitude.is_finite() || magnitude.max_element() > 1e6 {
        return None;
    }
    let height = max.y.max(0.);
    let reach = magnitude.x.hypot(magnitude.z);
    // The shader's canopy top is at least 0.5. Gust and sine magnitudes are
    // at most one. These envelopes include bend, limb sway and leaf flutter.
    let bend = 0.1 * height * height + 0.007;
    let sway = 0.06 * reach * height + 0.014;
    let margin = DVec3::new(bend, sway, bend) + magnitude * 1e-6;
    Some((min - margin, max + margin))
}

impl PlantBounds {
    pub(super) fn transformed(
        local: Option<(DVec3, DVec3)>,
        instance: &ArchiveInstance,
    ) -> Option<Self> {
        let (min, max) = local?;
        let world = Mat4::from_cols_array_2d(&instance.world);
        if !world.is_finite()
            || world.row(3) != glam::Vec4::W
            || world.to_cols_array().iter().any(|v| v.abs() > 1e12)
        {
            return None;
        }
        let world = world.as_dmat4();
        let local_center = (min + max) * 0.5;
        let local_extent = (max - min) * 0.5;
        let absolute_product = |point: DVec3| {
            world.x_axis.truncate().abs() * point.x
                + world.y_axis.truncate().abs() * point.y
                + world.z_axis.truncate().abs() * point.z
        };
        Some(Self {
            center: world.transform_point3(local_center),
            half_extent: absolute_product(local_extent),
            transform_magnitude: absolute_product(min.abs().max(max.abs()))
                + world.w_axis.truncate().abs(),
        })
    }
}

pub(super) struct ClipVolume {
    planes: [(DVec4, DVec4); 6],
}

impl ClipVolume {
    pub(super) fn new(matrix: Mat4) -> Self {
        let matrix = matrix.as_dmat4();
        let [x, y, z, w] = [0, 1, 2, 3].map(|row| matrix.row(row));
        Self {
            planes: [
                (w + x, w.abs() + x.abs()),
                (w - x, w.abs() + x.abs()),
                (w + y, w.abs() + y.abs()),
                (w - y, w.abs() + y.abs()),
                (z, z.abs()),
                (w - z, w.abs() + z.abs()),
            ],
        }
    }

    /// Conservative bands in shadow XY for the orthographic camera volume.
    /// Include both the raster inverse and the inverse used by depth receivers.
    pub(super) fn shadow_receivers(camera: Mat4, shadow: Mat4, size: u32) -> Option<Self> {
        if camera.row(3) != Vec4::W || shadow.row(3) != Vec4::W || size == 0 {
            return None;
        }
        let inverse = camera.as_dmat4().inverse();
        let shader_inverse = camera.inverse().as_dmat4();
        let shadow = shadow.as_dmat4();
        let projected = shadow * inverse;
        let receiver = shadow * shader_inverse;
        let xy = |value: DVec4| DVec2::new(value.x, value.y);
        let center = |matrix: glam::DMat4| xy(matrix.w_axis + matrix.z_axis * 0.5);
        let axes = |matrix: glam::DMat4| {
            [
                xy(matrix.x_axis),
                xy(matrix.y_axis),
                xy(matrix.z_axis) * 0.5,
            ]
        };
        let raster_axes = axes(projected);
        let receiver_axes = axes(receiver);
        // Restrict the proof to the bounded camera matrices used by the voxel
        // renderer. A degenerate or poorly conditioned projection keeps all casters.
        if [inverse, shader_inverse, shadow, projected, receiver]
            .iter()
            .any(|matrix| !matrix.is_finite())
            || inverse
                .to_cols_array()
                .iter()
                .any(|value| value.abs() > 4096.)
            || shadow.to_cols_array().iter().any(|value| value.abs() > 16.)
            || (camera.as_dmat4() * inverse - glam::DMat4::IDENTITY)
                .to_cols_array()
                .iter()
                .any(|value| value.abs() > 1e-8)
        {
            return None;
        }
        let mut planes = [(DVec4::ZERO, DVec4::ZERO); 6];
        for (index, axis) in raster_axes.into_iter().enumerate() {
            if axis.length_squared() <= 1e-20 {
                continue;
            }
            let normal = DVec2::new(-axis.y, axis.x).normalize();
            for (side, normal) in [normal, -normal].into_iter().enumerate() {
                let support = |matrix, axes: [DVec2; 3]| {
                    normal.dot(center(matrix))
                        + axes.iter().map(|axis| normal.dot(*axis).abs()).sum::<f64>()
                };
                // The largest receiver kernel spans ±2 texels with linear
                // comparison filtering. Four texels also cover edge interpolation.
                let pcf = normal.abs().element_sum() * 8. / f64::from(size);
                let axis_row = shadow.row(0) * normal.x + shadow.row(1) * normal.y;
                let rounding_scale =
                    shadow.row(0).abs() * normal.x.abs() + shadow.row(1).abs() * normal.y.abs();
                let world_magnitude = inverse.x_axis.abs()
                    + inverse.y_axis.abs()
                    + inverse.z_axis.abs()
                    + inverse.w_axis.abs();
                let rounding =
                    rounding_scale.dot(world_magnitude) * (256. * f64::from(f32::EPSILON)) + 0.001;
                let upper = support(projected, raster_axes).max(support(receiver, receiver_axes))
                    + pcf
                    + rounding;
                planes[index * 2 + side] = (DVec4::W * upper - axis_row, rounding_scale);
            }
        }
        Some(Self { planes })
    }

    fn intersects(&self, bounds: PlantBounds) -> bool {
        !self.planes.iter().any(|&(plane, magnitude)| {
            let upper = plane.dot(bounds.center.extend(1.))
                + plane.truncate().abs().dot(bounds.half_extent);
            let error_scale = magnitude.dot(bounds.transform_magnitude.extend(1.));
            // Covers roundoff in both matrix-vector orderings used by WGSL,
            // including cancellation before evaluation of a clip plane.
            let roundoff = error_scale * (64. * f64::from(f32::EPSILON)) + 1e-20;
            error_scale < f64::from(f32::MAX) / 128. && upper < -roundoff
        })
    }

    pub(super) fn visible_ranges(&self, bounds: &[Option<PlantBounds>]) -> Vec<Range<u32>> {
        self.visible_ranges_with(bounds, None)
    }

    pub(super) fn visible_ranges_with(
        &self,
        bounds: &[Option<PlantBounds>],
        additional: Option<&Self>,
    ) -> Vec<Range<u32>> {
        let mut ranges: Vec<Range<u32>> = Vec::new();
        for (index, bounds) in bounds.iter().enumerate() {
            if bounds.is_none_or(|bounds| {
                self.intersects(bounds) && additional.is_none_or(|volume| volume.intersects(bounds))
            }) {
                let index = index as u32;
                if let Some(last) = ranges.last_mut().filter(|last| last.end == index) {
                    last.end += 1;
                } else {
                    ranges.push(index..index + 1);
                }
            }
        }
        ranges
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receiver_footprint_keeps_off_camera_casters_and_filter_margin() {
        let shadow = Mat4::from_cols(Vec4::X, Vec4::Y, Vec4::new(1., 0., 1., 0.), Vec4::W);
        let receiver = ClipVolume::shadow_receivers(Mat4::IDENTITY, shadow, 4096).unwrap();
        let local = Some((DVec3::ZERO, DVec3::ZERO));
        let at = |x, z| {
            PlantBounds::transformed(
                local,
                &ArchiveInstance::new(Mat4::from_translation(Vec3::new(x, 0., z))),
            )
            .unwrap()
        };
        assert!(!ClipVolume::new(Mat4::IDENTITY).intersects(at(1.4, 0.5)));
        assert!(receiver.intersects(at(1.4, 0.5)));
        assert!(receiver.intersects(at(2.001, 0.)));
        assert!(!receiver.intersects(at(2.1, 0.)));
        assert!(
            ClipVolume::shadow_receivers(
                glam::camera::rh::proj::directx::perspective(1., 1., 0.1, 512.),
                shadow,
                4096,
            )
            .is_none()
        );
    }

    #[test]
    fn clip_ranges_preserve_order_edges_and_unknown_bounds() {
        let local = wind_bounds([Vec3::new(-0.1, 0., 0.4), Vec3::new(0.1, 0.1, 0.6)].into_iter());
        let at = |x| {
            PlantBounds::transformed(
                local,
                &ArchiveInstance {
                    world: Mat4::from_translation(Vec3::X * x).to_cols_array_2d(),
                    normal: [[0.; 4]; 3],
                },
            )
        };
        let bounds = [at(0.), at(3.), at(1.1), None, at(-3.)];
        assert_eq!(
            ClipVolume::new(Mat4::IDENTITY).visible_ranges(&bounds),
            [0..1, 2..4]
        );
        assert_eq!(
            ClipVolume::new(Mat4::from_translation(-3. * Vec3::X)).visible_ranges(&bounds),
            [1..2, 3..4],
        );
    }
}
