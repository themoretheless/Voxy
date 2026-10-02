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
}
