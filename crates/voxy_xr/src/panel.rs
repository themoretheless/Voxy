//! Controller aim picking for compositor panels, independent of graphics API.
use crate::XrRuntimeError;

/// Coordinates use a top-left UI origin; distance is in reference-space metres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuadHit {
    pub uv: [f32; 2],
    pub distance: f32,
}

/// Intersects the controller's local -Z aim ray with the front of a quad panel.
/// Locate the aim space relative to the panel's reference space at the pending
/// frame's predicted time. Sync actions first and supply their aim activity.
/// Missing tracking, a back-facing/parallel ray or an out-of-panel hit returns
/// None. Do not use a previous hit after tracking loss to drive UI selection.
/// # Errors
/// Rejects malformed valid poses, panel geometry and nonpositive/nonfinite range.
pub fn hit_test_quad(
    aim_active: bool,
    aim: &openxr::SpaceLocation,
    panel_pose: openxr::Posef,
    panel_size: openxr::Extent2Df,
    max_distance: f32,
) -> Result<Option<QuadHit>, XrRuntimeError> {
    crate::session::validate_quad(panel_pose, panel_size, openxr::EyeVisibility::BOTH)?;
    if !max_distance.is_finite() || max_distance <= 0.0 {
        return Err(XrRuntimeError::InvalidQuad);
    }
    let valid =
        openxr::SpaceLocationFlags::POSITION_VALID | openxr::SpaceLocationFlags::ORIENTATION_VALID;
    if !aim_active || !aim.location_flags.contains(valid) {
        return Ok(None);
    }
    crate::session::validate_quad(aim.pose, panel_size, openxr::EyeVisibility::BOTH)?;
    let point = |p: openxr::Vector3f| [f64::from(p.x), f64::from(p.y), f64::from(p.z)];
    let origin = point(aim.pose.position);
    let centre = point(panel_pose.position);
    let offset = std::array::from_fn(|i| origin[i] - centre[i]);
    let q = panel_pose.orientation;
    let inverse = openxr::Quaternionf {
        x: -q.x,
        y: -q.y,
        z: -q.z,
        w: q.w,
    };
    let local_origin = rotate(inverse, offset);
    let local_direction = rotate(inverse, rotate(aim.pose.orientation, [0.0, 0.0, -1.0]));
    if local_direction[2] >= -1.0e-8 {
        return Ok(None);
    }
    let distance = -local_origin[2] / local_direction[2];
    if distance < 0.0 || distance > f64::from(max_distance) {
        return Ok(None);
    }
    let x = local_origin[0] + distance * local_direction[0];
    let y = local_origin[1] + distance * local_direction[1];
    let u = x / f64::from(panel_size.width) + 0.5;
    let v = 0.5 - y / f64::from(panel_size.height);
    if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
        return Ok(None);
    }
    Ok(Some(QuadHit {
        uv: [u as f32, v as f32],
        distance: distance as f32,
    }))
}

fn rotate(q: openxr::Quaternionf, v: [f64; 3]) -> [f64; 3] {
    // Validation permits normal floating-point unit-quaternion roundoff.
    // Normalize here so the transformed direction remains a metre-length ray.
    let norm = (f64::from(q.x).powi(2)
        + f64::from(q.y).powi(2)
        + f64::from(q.z).powi(2)
        + f64::from(q.w).powi(2))
    .sqrt();
    let qv = [
        f64::from(q.x) / norm,
        f64::from(q.y) / norm,
        f64::from(q.z) / norm,
    ];
    let cross = |a: [f64; 3], b: [f64; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let t = cross(qv, v).map(|x| 2.0 * x);
    let c = cross(qv, t);
    std::array::from_fn(|i| v[i] + (f64::from(q.w) / norm) * t[i] + c[i])
}

#[cfg(test)]
mod tests {
    use super::*;
    fn location() -> openxr::SpaceLocation {
        openxr::SpaceLocation {
            location_flags: openxr::SpaceLocationFlags::POSITION_VALID
                | openxr::SpaceLocationFlags::ORIENTATION_VALID,
            pose: openxr::Posef::IDENTITY,
        }
    }
    fn panel() -> openxr::Posef {
        let mut pose = openxr::Posef::IDENTITY;
        pose.position.z = -2.0;
        pose
    }
    fn size() -> openxr::Extent2Df {
        openxr::Extent2Df {
            width: 2.0,
            height: 1.0,
        }
    }
    #[test]
    fn centre_edges_range_and_tracking() {
        let mut aim = location();
        assert_eq!(
            hit_test_quad(true, &aim, panel(), size(), 3.0).unwrap(),
            Some(QuadHit {
                uv: [0.5, 0.5],
                distance: 2.0
            })
        );
        aim.pose.position.x = -1.0;
        aim.pose.position.y = 0.5;
        assert_eq!(
            hit_test_quad(true, &aim, panel(), size(), 2.0)
                .unwrap()
                .unwrap()
                .uv,
            [0.0, 0.0]
        );
        aim.pose.position.x = -1.01;
        assert!(
            hit_test_quad(true, &aim, panel(), size(), 3.0)
                .unwrap()
                .is_none()
        );
        assert!(
            hit_test_quad(true, &location(), panel(), size(), 1.9)
                .unwrap()
                .is_none()
        );
        assert!(
            hit_test_quad(false, &location(), panel(), size(), 3.0)
                .unwrap()
                .is_none()
        );
        aim.location_flags = openxr::SpaceLocationFlags::POSITION_VALID;
        assert!(
            hit_test_quad(true, &aim, panel(), size(), 3.0)
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn translated_rotated_reference_space() {
        let q = openxr::Quaternionf {
            x: 0.0,
            y: std::f32::consts::FRAC_1_SQRT_2,
            z: 0.0,
            w: std::f32::consts::FRAC_1_SQRT_2,
        };
        let mut aim = location();
        aim.pose.orientation = q;
        aim.pose.position = openxr::Vector3f {
            x: 5.0,
            y: 3.0,
            z: 7.0,
        };
        let mut pose = aim.pose;
        pose.position.x -= 2.0;
        let hit = hit_test_quad(true, &aim, pose, size(), 3.0)
            .unwrap()
            .unwrap();
        assert!((hit.distance - 2.0).abs() < 1.0e-5);
        assert!((hit.uv[0] - 0.5).abs() < 1.0e-5);
        assert!((hit.uv[1] - 0.5).abs() < 1.0e-5);
    }
    #[test]
    fn rotated_panel_and_back_face() {
        let mut pose = panel();
        pose.orientation = openxr::Quaternionf {
            x: 0.0,
            y: 0.0,
            z: 1.0,
            w: 0.0,
        };
        let mut aim = location();
        aim.pose.position.x = 0.5;
        aim.pose.position.y = 0.25;
        assert_eq!(
            hit_test_quad(true, &aim, pose, size(), 3.0)
                .unwrap()
                .unwrap()
                .uv,
            [0.25, 0.75]
        );
        pose.orientation = openxr::Quaternionf {
            x: 0.0,
            y: 1.0,
            z: 0.0,
            w: 0.0,
        };
        assert!(
            hit_test_quad(true, &aim, pose, size(), 3.0)
                .unwrap()
                .is_none()
        );
        assert!(hit_test_quad(true, &aim, panel(), size(), f32::NAN).is_err());
        aim.pose.orientation.w = 0.0;
        assert!(hit_test_quad(true, &aim, panel(), size(), 3.0).is_err());
    }
}
