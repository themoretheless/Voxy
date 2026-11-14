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
    // Normalized gains within [0, 1].
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
    Ok(10.0_f64.powf(-total_db_loss / 20.0).clamp(0.0, 1.0) as f32)
}

/// Computes the ISO 9613-1 atmospheric sound absorption coefficient $\alpha$ in $\text{dB/m}$
/// at 1 atm standard pressure from acoustic `frequency_hz`, ambient `temp_celsius`, and
/// `relative_humidity_pct` $\in [0, 100]$.
#[must_use]
pub fn iso9613_air_absorption_db_per_m(
    frequency_hz: f32,
    temp_celsius: f32,
    relative_humidity_pct: f32,
) -> Option<f32> {
    if !frequency_hz.is_finite()
        || frequency_hz <= 0.0
        || !temp_celsius.is_finite()
        || !(-60.0..=80.0).contains(&temp_celsius)
        || !relative_humidity_pct.is_finite()
        || !(0.0..=100.0).contains(&relative_humidity_pct)
    {
        return None;
    }
    let f = f64::from(frequency_hz.clamp(20.0, 100_000.0));
    let t_k = f64::from(temp_celsius) + 273.15;
    let t_ref = 293.15_f64;
    let t_triple = 273.16_f64;
    let rel_t = t_k / t_ref;
    let h_rel = f64::from(relative_humidity_pct.clamp(1.0, 100.0));
    let p_sat_ratio = 10.0_f64.powf(-6.8346 * (t_triple / t_k).powf(1.261) + 4.6151);
    let h = h_rel * p_sat_ratio;
    let fr_o = 24.0 + 4.04e4 * h * ((0.02 + h) / (0.391 + h));
    let fr_n =
        rel_t.powf(-0.5) * (9.0 + 280.0 * h * (-4.170 * (rel_t.powf(-1.0 / 3.0) - 1.0)).exp());
    let term_classical = 1.84e-11 * rel_t.sqrt();
    let term_o = 0.01275 * (-2239.1 / t_k).exp() / (fr_o + (f * f) / fr_o);
    let term_n = 0.1068 * (-3352.0 / t_k).exp() / (fr_n + (f * f) / fr_n);
    let alpha_db_m = 8.686 * f * f * (term_classical + rel_t.powf(-2.5) * (term_o + term_n));
    let out = alpha_db_m.max(0.0) as f32;
    out.is_finite().then_some(out)
}

/// Computes normalized 2nd-order Butterworth low-pass biquad filter coefficients
/// `[b0, b1, b2, a1, a2]` (with $a_0 = 1$) for head-shadow and obstacle diffraction filtering.
#[must_use]
pub fn biquad_lowpass_butterworth_coeffs(cutoff_hz: f32, sample_rate_hz: f32) -> Option<[f32; 5]> {
    if !cutoff_hz.is_finite()
        || !sample_rate_hz.is_finite()
        || cutoff_hz <= 0.0
        || sample_rate_hz <= 2.0 * cutoff_hz
    {
        return None;
    }
    let omega = 2.0 * std::f64::consts::PI * f64::from(cutoff_hz) / f64::from(sample_rate_hz);
    let (sin_w, cos_w) = omega.sin_cos();
    let alpha = sin_w / std::f64::consts::SQRT_2;
    let a0 = 1.0 + alpha;
    if a0.abs() <= 1e-12 {
        return None;
    }
    let b1 = 1.0 - cos_w;
    let b0 = 0.5 * b1;
    let b2 = b0;
    let a1 = -2.0 * cos_w;
    let a2 = 1.0 - alpha;
    Some([
        (b0 / a0) as f32,
        (b1 / a0) as f32,
        (b2 / a0) as f32,
        (a1 / a0) as f32,
        (a2 / a0) as f32,
    ])
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

        let wall = trace_voxel_occlusion_gain([0.5, 0.5, 0.5], [4.5, 0.5, 0.5], 6.0, 32, |pos| {
            pos[0] == 2
        })
        .unwrap();
        // 1 solid voxel * 6 dB loss => ~0.501187 linear gain
        assert!((wall - 0.501_187).abs() < 1e-3);
        let low_f = iso9613_air_absorption_db_per_m(500.0, 20.0, 50.0).unwrap();
        let high_f = iso9613_air_absorption_db_per_m(8000.0, 20.0, 50.0).unwrap();
        assert!(high_f > low_f && low_f > 0.0);
        let coeffs = biquad_lowpass_butterworth_coeffs(1000.0, 48_000.0).unwrap();
        let dc_gain = (coeffs[0] + coeffs[1] + coeffs[2]) / (1.0 + coeffs[3] + coeffs[4]);
        assert!((dc_gain - 1.0).abs() < 1e-5);
        assert!(biquad_lowpass_butterworth_coeffs(25_000.0, 48_000.0).is_none());
    }
}
