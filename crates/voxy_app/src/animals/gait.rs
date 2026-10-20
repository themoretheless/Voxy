//! Grounded foot targets and analytic two-link sagittal IK.
use super::*;
#[derive(Clone, Debug)]
pub struct Gait {
    planted: [Vec3; 4],
    targets: [Vec3; 4],
    stance: [bool; 4],
    initialized: bool,
}
pub fn knee(cat: bool, leg: usize) -> Vec3 {
    let a = asset::profile(cat);
    let hip = leg_pivots(cat)[leg];
    Vec3::new(
        hip.x,
        a.shoulder_height * if leg < 2 { 0.52 } else { 0.57 },
        hip.z
            + if leg < 2 {
                -a.shoulder_height * 0.025
            } else {
                a.trunk_length * 0.075
            },
    )
}
pub fn foot(cat: bool, leg: usize) -> Vec3 {
    let a = asset::profile(cat);
    let hip = leg_pivots(cat)[leg];
    Vec3::new(
        hip.x,
        a.paw_half_width * 0.72,
        hip.z + a.paw_half_width * 0.35,
    )
}
impl Gait {
    pub fn new() -> Self {
        Self {
            planted: [Vec3::ZERO; 4],
            targets: [Vec3::ZERO; 4],
            stance: [true; 4],
            initialized: false,
        }
    }
    pub fn update(
        &mut self,
        world: &World,
        cat: bool,
        root: Mat4,
        phase: f32,
        speed: f32,
        grounded: bool,
    ) -> Result<(), String> {
        let a = asset::profile(cat);
        let inverse = root.inverse();
        for leg in 0..4 {
            let rest = foot(cat, leg);
            if !self.initialized || !grounded {
                self.planted[leg] = root.transform_point3(rest);
                self.targets[leg] = rest;
                self.stance[leg] = true;
                continue;
            }
            let offset = if speed > 1.6 {
                [0., 0.5, 0.5, 0.]
            } else {
                [0., 0.5, 0.75, 0.25]
            };
            let cycle = (phase / std::f32::consts::TAU + offset[leg]).rem_euclid(1.);
            let duty = if speed > 1.6 { 0.52 } else { 0.67 };
            let stance = cycle < duty || speed < 0.04;
            let stride = (speed * 0.30).min(a.trunk_length * 0.55);
            let mut landing = root.transform_point3(rest + Vec3::Z * stride * 0.48);
            // Sweep a small sole through terrain, independent of the broad body collider.
            use physics::CollisionWorld;
            let floor = landing.floor();
            let origin = physics::Origin {
                x: floor.x as i64,
                y: floor.y as i64,
                z: floor.z as i64,
            };
            let local = landing - floor;
            let start = local.y + a.shoulder_height * 0.65;
            let drop = a.shoulder_height * 1.5;
            let hit = collision::AnimalWorld { world, bodies: &[] }.sweep_aabb(
                physics::AnchoredAabb {
                    anchor: origin,
                    min: [
                        f64::from(local.x - 0.005),
                        f64::from(start),
                        f64::from(local.z - 0.005),
                    ],
                    max: [
                        f64::from(local.x + 0.005),
                        f64::from(start + 0.01),
                        f64::from(local.z + 0.005),
                    ],
                },
                [0., -f64::from(drop), 0.],
                4096,
            )?;
            if hit.obstacle.is_some() {
                landing.y = floor.y + start - drop * hit.fraction as f32 + rest.y;
            }
            if stance {
                if !self.stance[leg] || speed < 0.04 {
                    self.planted[leg] = landing;
                }
                let local = inverse.transform_point3(self.planted[leg]);
                if local.distance(rest) > a.shoulder_height * 0.68 {
                    self.planted[leg] = landing;
                }
                self.targets[leg] = inverse.transform_point3(self.planted[leg]);
            } else {
                let t = (cycle - duty) / (1. - duty);
                let t = t * t * (3. - 2. * t);
                let mut target = self.planted[leg].lerp(landing, t);
                target.y += (std::f32::consts::PI * t).sin() * a.shoulder_height * 0.16;
                self.targets[leg] = inverse.transform_point3(target);
            }
            self.stance[leg] = stance;
        }
        self.initialized = true;
        Ok(())
    }
    pub fn matrices(&self, cat: bool, root: Mat4) -> [Mat4; fur::BONES] {
        let mut result = [root; fur::BONES];
        for leg in 0..4 {
            let hip = leg_pivots(cat)[leg];
            let knee = knee(cat, leg);
            let rest = foot(cat, leg);
            let target = if self.initialized {
                self.targets[leg]
            } else {
                rest
            };
            let (upper, lower) = solve(hip, knee, rest, target);
            result[leg + 1] = root
                * Mat4::from_translation(hip)
                * Mat4::from_rotation_x(upper)
                * Mat4::from_translation(-hip);
            result[leg + 5] = result[leg + 1]
                * Mat4::from_translation(knee)
                * Mat4::from_rotation_x(lower - upper)
                * Mat4::from_translation(-knee);
        }
        result
    }
}
fn solve(hip: Vec3, knee: Vec3, foot: Vec3, target: Vec3) -> (f32, f32) {
    let a = knee - hip;
    let b = foot - knee;
    let target = Vec3::new(0., target.y - hip.y, target.z - hip.z);
    let l1 = a.length();
    let l2 = b.length();
    let r = target
        .length()
        .clamp((l1 - l2).abs() + 0.00001, l1 + l2 - 0.00001);
    let angle = target.z.atan2(-target.y);
    let rest1 = a.z.atan2(-a.y);
    let rest2 = b.z.atan2(-b.y);
    let sign = if rest1 > rest2 { 1. } else { -1. };
    let alpha = ((l1 * l1 + r * r - l2 * l2) / (2. * l1 * r))
        .clamp(-1., 1.)
        .acos();
    let beta = ((l2 * l2 + r * r - l1 * l1) / (2. * l2 * r))
        .clamp(-1., 1.)
        .acos();
    (
        rest1 - (angle + sign * alpha),
        rest2 - (angle - sign * beta),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ik_reaches_supported_targets() {
        let hip = Vec3::new(0., 0.5, 0.);
        let knee = Vec3::new(0., 0.27, 0.04);
        let foot = Vec3::new(0., 0.02, 0.);
        for z in [-0.10, 0., 0.10] {
            let target = Vec3::new(0., 0.08, z);
            let (u, l) = solve(hip, knee, foot, target);
            let upper = Mat4::from_translation(hip)
                * Mat4::from_rotation_x(u)
                * Mat4::from_translation(-hip);
            let lower = upper
                * Mat4::from_translation(knee)
                * Mat4::from_rotation_x(l - u)
                * Mat4::from_translation(-knee);
            assert!(lower.transform_point3(foot).distance(target) < 0.0001);
        }
    }
    #[test]
    fn stationary_soles_match_terrain_and_stance_does_not_slide() {
        let mut scene = voxy_runtime::build_procedural_scene(42, 1).unwrap();
        prepare_check_scene(&mut scene).unwrap();
        for cat in [false, true] {
            let mut gait = Gait::new();
            let root = Mat4::from_translation(Vec3::new(16.0, 13.0, 16.0));
            gait.update(&scene.world, cat, root, 0.0, 0.0, true)
                .unwrap();
            gait.update(&scene.world, cat, root, 0.0, 0.0, true)
                .unwrap();
            let pose = gait.matrices(cat, root);
            for leg in 0..4 {
                let sole = pose[leg + 5].transform_point3(foot(cat, leg));
                assert!((sole.y - 13.0 - foot(cat, leg).y).abs() < 0.001);
            }
            let before = pose[5].transform_point3(foot(cat, 0));
            let moved = Mat4::from_translation(Vec3::new(16.0, 13.0, 16.005));
            gait.update(&scene.world, cat, moved, 0.10, 0.1, true)
                .unwrap();
            let after = gait.matrices(cat, moved)[5].transform_point3(foot(cat, 0));
            assert!(
                before.distance(after) < 0.001,
                "stance foot must remain planted"
            );
        }
    }
}
