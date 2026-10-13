//! Offline contact candidates. Thresholds classify authored samples, not physics.
use crate::AnimationError;

pub const MAX_CONTACT_BAKE_SAMPLES: usize = 4095;

/// Caller-selected distance and velocity units, matching the sampled heights.
#[derive(Clone, Copy, Debug)]
pub struct ContactBakeSettings {
    pub height_threshold: f64,
    pub vertical_speed_threshold: f64,
}
#[derive(Clone, Debug)]
pub struct BakedFootContacts {
    /// Minimum observed height; this is not a discovered physical support plane.
    pub minimum_height: f64,
    /// Uniform samples on [0,period), without a repeated endpoint.
    pub contact: Vec<bool>,
    pub vertical_speed: Vec<f64>,
}

/// Bakes periodic candidate gates using height and central vertical speed.
/// The caller samples equally spaced heights along its chosen up axis. A final
/// curve key at phase1 must repeat gate0. No first-sample zero speed is invented:
/// both seam neighbors participate. Samples can alias motion between them, and
/// thresholds require author review. No curve is installed in a live owner.
/// # Errors
/// Rejects invalid units/settings, fewer than3 or more than4095 samples, and
/// nonfinite heights, differences or unrepresentable sample intervals/speeds.
pub fn bake_periodic_foot_contacts(
    heights: &[f64],
    period: f64,
    settings: ContactBakeSettings,
) -> Result<BakedFootContacts, AnimationError> {
    if !(3..=MAX_CONTACT_BAKE_SAMPLES).contains(&heights.len())
        || !period.is_finite()
        || period <= 0.
        || !settings.height_threshold.is_finite()
        || settings.height_threshold < 0.
        || !settings.vertical_speed_threshold.is_finite()
        || settings.vertical_speed_threshold < 0.
        || heights.iter().any(|h| !h.is_finite())
    {
        return Err(AnimationError::InvalidContactBake);
    }
    let dt = period / heights.len() as f64;
    let span = 2. * dt;
    if !span.is_finite() || span <= 0. {
        return Err(AnimationError::InvalidContactBake);
    }
    let minimum_height = heights.iter().copied().fold(f64::INFINITY, f64::min);
    let mut contact = Vec::with_capacity(heights.len());
    let mut vertical_speed = Vec::with_capacity(heights.len());
    for (i, &height) in heights.iter().enumerate() {
        let previous = heights[(i + heights.len() - 1) % heights.len()];
        let next = heights[(i + 1) % heights.len()];
        let speed = (next - previous) / span;
        let clearance = height - minimum_height;
        if !speed.is_finite() || !clearance.is_finite() {
            return Err(AnimationError::InvalidContactBake);
        }
        contact.push(
            clearance <= settings.height_threshold
                && speed.abs() <= settings.vertical_speed_threshold,
        );
        vertical_speed.push(speed);
    }
    Ok(BakedFootContacts {
        minimum_height,
        contact,
        vertical_speed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn height_and_speed_gates_use_periodic_neighbors_and_explicit_units() {
        let heights = [0., 0.01, 0.03, 0.1, 0.2, 0.1, 0.03, 0.01];
        let settings = ContactBakeSettings {
            height_threshold: 0.04,
            vertical_speed_threshold: 0.31,
        };
        let baked = bake_periodic_foot_contacts(&heights, 0.8, settings).unwrap();
        assert_eq!(
            baked.contact,
            [true, true, false, false, false, false, false, true]
        );
        assert!((baked.vertical_speed[2] - 0.45).abs() < 1e-12);
        let translated: Vec<_> = heights.iter().map(|h| h + 2.).collect();
        assert_eq!(
            bake_periodic_foot_contacts(&translated, 0.8, settings)
                .unwrap()
                .contact,
            baked.contact
        );
        assert_eq!(
            bake_periodic_foot_contacts(
                &heights,
                0.4,
                ContactBakeSettings {
                    vertical_speed_threshold: 0.62,
                    ..settings
                }
            )
            .unwrap()
            .contact,
            baked.contact
        );
        let seam = bake_periodic_foot_contacts(
            &[0., 0.3, 0.2, 0.1],
            4.,
            ContactBakeSettings {
                height_threshold: 0.01,
                vertical_speed_threshold: 0.05,
            },
        )
        .unwrap();
        assert!(!seam.contact[0]);
        assert!((seam.vertical_speed[0] - 0.1).abs() < 1e-12);
        assert!(
            bake_periodic_foot_contacts(&[3.; 8], 1., settings)
                .unwrap()
                .contact
                .iter()
                .all(|v| *v)
        );
    }
    #[test]
    fn malformed_bakes_are_rejected_without_partial_gates() {
        let settings = ContactBakeSettings {
            height_threshold: 0.06,
            vertical_speed_threshold: 0.3,
        };
        for period in [0., -1., f64::NAN, f64::INFINITY, f64::from_bits(1)] {
            assert!(bake_periodic_foot_contacts(&[0.; 8], period, settings).is_err());
        }
        for heights in [
            vec![0.; 2],
            vec![0.; MAX_CONTACT_BAKE_SAMPLES + 1],
            vec![0., f64::NAN, 0.],
            vec![-f64::MAX, f64::MAX, 0.],
        ] {
            assert!(bake_periodic_foot_contacts(&heights, 1., settings).is_err());
        }
        assert!(
            bake_periodic_foot_contacts(
                &[0.; 8],
                1.,
                ContactBakeSettings {
                    height_threshold: -1.,
                    ..settings
                }
            )
            .is_err()
        );
        assert!(
            bake_periodic_foot_contacts(
                &[0.; 8],
                1.,
                ContactBakeSettings {
                    vertical_speed_threshold: f64::NAN,
                    ..settings
                }
            )
            .is_err()
        );
    }
}
