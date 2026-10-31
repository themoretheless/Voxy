//! Pure listener/source geometry; no scene ownership or device coupling.
use crate::AudioError;

/// Equal-power stereo gains for a mono source duplicated into both PCM channels.
/// Units are caller-defined but consistent. Inside `near` distance attenuation is
/// one; beyond `far` it is zero; between them it falls linearly. Front/back and
/// elevation share the same stereo projection (no HRTF or occlusion).
/// # Errors
/// Rejects nonfinite geometry, zero listener right vector, or invalid ranges.
pub fn spatial_gains(
    listener: [f32; 3],
    right: [f32; 3],
    source: [f32; 3],
    near: f32,
    far: f32,
) -> Result<[f32; 2], AudioError> {
    if listener
        .iter()
        .chain(&right)
        .chain(&source)
        .any(|x| !x.is_finite())
        || !near.is_finite()
        || !far.is_finite()
        || near < 0.0
        || far <= near
    {
        return Err(AudioError::InvalidSpatial);
    }
    let delta = std::array::from_fn::<_, 3, _>(|i| f64::from(source[i]) - f64::from(listener[i]));
    let distance = delta.iter().map(|x| x * x).sum::<f64>().sqrt();
    let right_length = right
        .iter()
        .map(|x| f64::from(*x).powi(2))
        .sum::<f64>()
        .sqrt();
    if right_length == 0.0 {
        return Err(AudioError::InvalidSpatial);
    }
    let pan = if distance == 0.0 {
        0.0
    } else {
        (delta
            .iter()
            .zip(right)
            .map(|(d, r)| d * f64::from(r))
            .sum::<f64>()
            / (distance * right_length))
            .clamp(-1.0, 1.0)
    };
    let attenuation = ((f64::from(far) - distance) / f64::from(far - near)).clamp(0.0, 1.0);
    #[allow(clippy::cast_possible_truncation)] // Normalized gains within [0, 1].
    Ok([
        (((1.0 - pan) * 0.5).sqrt() * attenuation) as f32,
        (((1.0 + pan) * 0.5).sqrt() * attenuation) as f32,
    ])
}

/// Estimates room reverberation decay time $RT_{60}$ (in seconds) using the Eyring–Norris
/// acoustic formula (falling back to Sabine in the low-absorption limit):
///
/// $$RT_{60} = \frac{0.161 \cdot V}{-S \ln(1 - \bar{\alpha})}$$
///
/// # Errors
///
/// Returns [`AudioError::InvalidSpatial`] if `volume_m3 <= 0`, `surface_area_m2 <= 0`, or
/// `mean_absorption` is outside `(0.0, 1.0)`.
pub fn sabine_eyring_rt60(
    volume_m3: f32,
    surface_area_m2: f32,
    mean_absorption: f32,
) -> Result<f32, AudioError> {
    if !volume_m3.is_finite()
        || !surface_area_m2.is_finite()
        || !mean_absorption.is_finite()
        || volume_m3 <= 0.0
        || surface_area_m2 <= 0.0
        || mean_absorption <= 0.0
        || mean_absorption >= 1.0
    {
        return Err(AudioError::InvalidSpatial);
    }
    let neg_ln = -(1.0 - f64::from(mean_absorption)).ln();
    let denom = f64::from(surface_area_m2) * neg_ln.max(1e-6);
    #[allow(clippy::cast_possible_truncation)]
    Ok((0.161 * f64::from(volume_m3) / denom).max(0.0) as f32)
}

/// Computes acoustic direct-path transmission gain (`[0, 1]`) between `listener` and `source`
/// by marching a 3D Amanatides–Woo voxel DDA segment and accumulating `db_loss_per_solid_voxel`
/// for each solid voxel along the line of sight.
///
/// # Errors
///
/// Returns [`AudioError::InvalidSpatial`] if coordinates or `db_loss_per_solid_voxel` (< 0) are invalid.
pub fn trace_voxel_occlusion_gain<F>(
    listener: [f32; 3],
    source: [f32; 3],
    db_loss_per_solid_voxel: f32,
    max_steps: u32,
    mut is_solid: F,
) -> Result<f32, AudioError>
where
    F: FnMut([i32; 3]) -> bool,
{
    if listener.iter().chain(&source).any(|x| !x.is_finite())
        || !db_loss_per_solid_voxel.is_finite()
        || db_loss_per_solid_voxel < 0.0
    {
        return Err(AudioError::InvalidSpatial);
    }
    let dx = f64::from(source[0]) - f64::from(listener[0]);
    let dy = f64::from(source[1]) - f64::from(listener[1]);
    let dz = f64::from(source[2]) - f64::from(listener[2]);
    let dist = (dx * dx + dy * dy + dz * dz).sqrt();
    if dist < 1e-6 {
        return Ok(1.0);
    }
    let mut current = [
        listener[0].floor() as i32,
        listener[1].floor() as i32,
        listener[2].floor() as i32,
    ];
    let target = [
        source[0].floor() as i32,
        source[1].floor() as i32,
        source[2].floor() as i32,
    ];
    let step = [
        if dx >= 0.0 { 1 } else { -1 },
        if dy >= 0.0 { 1 } else { -1 },
        if dz >= 0.0 { 1 } else { -1 },
    ];
    let dir = [dx / dist, dy / dist, dz / dist];
    let t_delta = [
        if dir[0].abs() > 1e-12 {
            (1.0 / dir[0]).abs()
        } else {
            f64::INFINITY
        },
        if dir[1].abs() > 1e-12 {
            (1.0 / dir[1]).abs()
        } else {
            f64::INFINITY
        },
        if dir[2].abs() > 1e-12 {
            (1.0 / dir[2]).abs()
        } else {
            f64::INFINITY
        },
    ];
    let init_t_max = |pos: f32, c: i32, s: i32, td: f64| {
        if !td.is_finite() {
            f64::INFINITY
        } else if s > 0 {
            (f64::from(c) + 1.0 - f64::from(pos)) * td
        } else {
            (f64::from(pos) - f64::from(c)) * td
        }
    };
    let mut t_max = [
        init_t_max(listener[0], current[0], step[0], t_delta[0]),
        init_t_max(listener[1], current[1], step[1], t_delta[1]),
        init_t_max(listener[2], current[2], step[2], t_delta[2]),
    ];

    let mut occluded_voxels = 0_u32;
    for _ in 0..max_steps {
        if current == target {
            break;
        }
        if is_solid(current) {
            occluded_voxels += 1;
        }
        if t_max[0] <= t_max[1] && t_max[0] <= t_max[2] {
            if t_max[0] > dist {
                break;
            }
            current[0] += step[0];
            t_max[0] += t_delta[0];
        } else if t_max[1] <= t_max[2] {
            if t_max[1] > dist {
                break;
            }
            current[1] += step[1];
            t_max[1] += t_delta[1];
        } else {
            if t_max[2] > dist {
                break;
            }
            current[2] += step[2];
            t_max[2] += t_delta[2];
        }
    }
    let total_db_loss = f64::from(occluded_voxels) * f64::from(db_loss_per_solid_voxel);
    #[allow(clippy::cast_possible_truncation)]
    Ok(10.0_f64.powf(-total_db_loss / 20.0).clamp(0.0, 1.0) as f32)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn orientation_distance_and_invalid_geometry() {
        let origin = [0.0; 3];
        let right = [1.0, 0.0, 0.0];
        let center = spatial_gains(origin, right, origin, 1.0, 5.0).unwrap();
        assert!((center[0].powi(2) + center[1].powi(2) - 1.0).abs() < 1e-6);
        let left = spatial_gains(origin, right, [-1.0, 0.0, 0.0], 1.0, 5.0).unwrap();
        assert!((left[0] - 1.0).abs() < 1e-6 && left[1].abs() < 1e-6);
        let rotated = spatial_gains(origin, [0.0, 0.0, 2.0], [0.0, 0.0, 3.0], 1.0, 5.0).unwrap();
        assert!(rotated[0].abs() < 1e-6 && (rotated[1] - 0.5).abs() < 1e-6);
        assert!(
            spatial_gains(origin, right, [0.0, 8.0, 0.0], 1.0, 5.0)
                .unwrap()
                .iter()
                .all(|x| *x == 0.0)
        );
        assert_eq!(
            spatial_gains(origin, origin, origin, 0.0, 1.0),
            Err(AudioError::InvalidSpatial)
        );
        assert_eq!(
            spatial_gains(origin, right, origin, 1.0, 1.0),
            Err(AudioError::InvalidSpatial)
        );
        assert!(spatial_gains(origin, right, [f32::NAN; 3], 0.0, 1.0).is_err());
    }

    #[test]
    fn rt60_and_voxel_occlusion_attenuate_predictably() {
        let rt = sabine_eyring_rt60(100.0, 140.0, 0.25).unwrap();
        assert!(rt > 0.35 && rt < 0.45);
        assert!(sabine_eyring_rt60(0.0, 100.0, 0.25).is_err());

        let clear =
            trace_voxel_occlusion_gain([0.5, 0.5, 0.5], [4.5, 0.5, 0.5], 6.0, 32, |_| false)
                .unwrap();
        assert!((clear - 1.0).abs() < 1e-6);

        let wall = trace_voxel_occlusion_gain(
            [0.5, 0.5, 0.5],
            [4.5, 0.5, 0.5],
            6.0,
            32,
            |pos| pos[0] == 2,
        )
        .unwrap();
        // 1 solid voxel * 6 dB loss => ~0.501187 linear gain
        assert!((wall - 0.501_187).abs() < 1e-3);
    }
}
