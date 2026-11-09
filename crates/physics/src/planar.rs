//! Small planar contact solver. Coordinates and velocities must be finite; radii positive.
//! Callers own integration and must bound travel per substep relative to the smallest radius.

/// Two-dimensional vector in application-defined units.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}
impl Vec2 {
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    #[must_use]
    pub fn dot(self, rhs: Self) -> f64 {
        self.x * rhs.x + self.y * rhs.y
    }
    #[must_use]
    pub fn length(self) -> f64 {
        self.dot(self).sqrt()
    }
    #[must_use]
    pub const fn perpendicular(self) -> Self {
        Self::new(-self.y, self.x)
    }
}
impl std::ops::Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}
impl std::ops::Sub for Vec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}
impl std::ops::Mul<f64> for Vec2 {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Ball {
    pub position: Vec2,
    pub velocity: Vec2,
    pub radius: f64,
}

/// Rounded segment, optionally rotating about `a`. A circle is a zero-length capsule.
#[derive(Clone, Copy, Debug)]
pub struct Capsule {
    pub a: Vec2,
    pub b: Vec2,
    pub radius: f64,
    pub linear_velocity: Vec2,
    pub angular_velocity: f64,
}
impl Capsule {
    #[must_use]
    pub const fn fixed(a: Vec2, b: Vec2, radius: f64) -> Self {
        Self {
            a,
            b,
            radius,
            linear_velocity: Vec2::new(0.0, 0.0),
            angular_velocity: 0.0,
        }
    }
}

/// Resolves penetration and normal relative velocity against an infinite-mass capsule.
/// Returns impact speed (zero for separating/resting contacts). Restitution is clamped to [0,1].
/// No friction or spin is modeled. This is a discrete contact, not a swept collision query.
pub fn resolve_contact(ball: &mut Ball, obstacle: Capsule, restitution: f64) -> f64 {
    let segment = obstacle.b - obstacle.a;
    let length_squared = segment.dot(segment);
    let t = if length_squared > 1e-16 {
        ((ball.position - obstacle.a).dot(segment) / length_squared).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let closest = obstacle.a + segment * t;
    let offset = ball.position - closest;
    let distance = offset.length();
    let radius = ball.radius + obstacle.radius;
    if distance > radius {
        return 0.0;
    }
    let normal = if distance > 1e-12 {
        offset * (1.0 / distance)
    } else if length_squared > 1e-16 {
        segment.perpendicular() * (1.0 / length_squared.sqrt())
    } else {
        Vec2::new(0.0, 1.0)
    };
    ball.position = ball.position + normal * (radius - distance + 1e-9);
    let surface_point = closest + normal * obstacle.radius;
    let surface_velocity = obstacle.linear_velocity
        + (surface_point - obstacle.a).perpendicular() * obstacle.angular_velocity;
    let approach = (ball.velocity - surface_velocity).dot(normal);
    if approach >= 0.0 {
        return 0.0;
    }
    ball.velocity = ball.velocity - normal * ((1.0 + restitution.clamp(0.0, 1.0)) * approach);
    -approach
}

/// Computes the 2D normal and tangential contact impulse `[j_n, j_t]` for relative contact
/// velocity `rel_vel_nt = [v_n, v_t]`, effective inverse mass `inv_mass_sum`, restitution $e \in [0, 1]$,
/// and Coulomb friction cone $|j_t| \le \mu j_n$.
#[must_use]
pub fn planar_2d_impulse_with_coulomb_friction(
    rel_vel_nt: [f64; 2],
    inv_mass_sum: f64,
    restitution: f64,
    mu_friction: f64,
) -> Option<[f64; 2]> {
    if !rel_vel_nt[0].is_finite()
        || !rel_vel_nt[1].is_finite()
        || !inv_mass_sum.is_finite()
        || !restitution.is_finite()
        || !mu_friction.is_finite()
        || inv_mass_sum <= 0.0
        || !(0.0..=1.0).contains(&restitution)
        || mu_friction < 0.0
    {
        return None;
    }
    if rel_vel_nt[0] >= 0.0 {
        return Some([0.0, 0.0]);
    }
    let j_n = -(1.0 + restitution) * rel_vel_nt[0] / inv_mass_sum;
    let max_jt = mu_friction * j_n;
    let j_t_stick = -rel_vel_nt[1] / inv_mass_sum;
    let j_t = j_t_stick.clamp(-max_jt, max_jt);
    Some([j_n, j_t])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn planar_2d_impulse_clamps_tangential_impulse_to_coulomb_cone() {
        let imp = planar_2d_impulse_with_coulomb_friction([-4.0, 10.0], 0.5, 0.5, 0.25).unwrap();
        assert!((imp[0] - 12.0).abs() < 1e-12);
        assert!((imp[1] - (-3.0)).abs() < 1e-12);
    }
    #[test]
    fn wall_reflects_normal_and_preserves_tangent() {
        let mut ball = Ball {
            position: Vec2::new(0.1, 2.0),
            velocity: Vec2::new(-10.0, 3.0),
            radius: 0.2,
        };
        let hit = resolve_contact(
            &mut ball,
            Capsule::fixed(Vec2::new(0.0, 0.0), Vec2::new(0.0, 4.0), 0.1),
            0.8,
        );
        assert!((hit - 10.0).abs() < 1e-9);
        assert!((ball.velocity.x - 8.0).abs() < 1e-9);
        assert!((ball.velocity.y - 3.0).abs() < 1e-9);
        assert!(ball.position.x >= 0.3);
    }
    #[test]
    fn rotating_flipper_transfers_surface_velocity() {
        let mut ball = Ball {
            position: Vec2::new(2.0, 0.25),
            velocity: Vec2::default(),
            radius: 0.2,
        };
        let mut bat = Capsule::fixed(Vec2::default(), Vec2::new(3.0, 0.0), 0.1);
        bat.angular_velocity = 5.0;
        resolve_contact(&mut ball, bat, 0.5);
        assert!((ball.velocity.y - 15.0).abs() < 1e-9);
    }
    #[test]
    fn separating_overlap_is_corrected_without_second_impulse() {
        let mut ball = Ball {
            position: Vec2::new(0.0, 0.1),
            velocity: Vec2::new(0.0, 2.0),
            radius: 0.2,
        };
        assert_eq!(
            resolve_contact(
                &mut ball,
                Capsule::fixed(Vec2::default(), Vec2::default(), 0.5),
                1.0
            ),
            0.0
        );
        assert!(ball.position.y > 0.7);
        assert_eq!(ball.velocity.y, 2.0);
    }
    #[test]
    fn endpoint_contact_has_radial_normal() {
        let mut ball = Ball {
            position: Vec2::new(1.2, 0.2),
            velocity: Vec2::new(-2.0, -2.0),
            radius: 0.3,
        };
        resolve_contact(
            &mut ball,
            Capsule::fixed(Vec2::default(), Vec2::new(1.0, 0.0), 0.1),
            1.0,
        );
        assert!((ball.velocity.x - 2.0).abs() < 1e-9);
        assert!((ball.velocity.y - 2.0).abs() < 1e-9);
    }
}
