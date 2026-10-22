//! Rivers: a water ribbon along a centre-line spline whose speed follows the bed slope.
//! Steep segments become rapids and, beyond `falls_slope`, waterfalls.

use glam::{Vec2, Vec3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RiverNode {
    /// Water surface at the centre line.
    pub position: Vec3,
    pub width: f32,
    /// Depth of the channel below the surface (for absorption and buoyancy).
    pub depth: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct River {
    pub nodes: Vec<RiverNode>,
    /// Surface speed (m/s) on level ground.
    pub speed: f32,
    /// Drop per horizontal metre beyond which the water free-falls.
    pub falls_slope: f32,
}

/// One point of the resampled centre line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RiverSample {
    pub position: Vec3,
    /// Unit flow direction (3D, follows the drop).
    pub tangent: Vec3,
    /// Horizontal unit vector to the right bank.
    pub right: Vec3,
    pub width: f32,
    pub depth: f32,
    /// Surface speed (m/s).
    pub speed: f32,
    /// Distance along the centre line (m).
    pub along: f32,
    /// 0 calm, 1 white water.
    pub foam: f32,
    pub falls: bool,
}

/// Water state at a point inside the river.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RiverPoint {
    pub level: f32,
    pub velocity: Vec3,
    pub depth: f32,
    pub foam: f32,
    pub falls: bool,
}

impl River {
    pub fn new(nodes: Vec<RiverNode>, speed: f32) -> Self {
        Self {
            nodes,
            speed,
            falls_slope: 1.2,
        }
    }

    /// Catmull-Rom resampling of the centre line every `step` metres (or finer).
    pub fn samples(&self, step: f32) -> Vec<RiverSample> {
        let n = self.nodes.len();
        if n < 2 {
            return Vec::new();
        }
        let node = |i: isize| self.nodes[i.clamp(0, n as isize - 1) as usize];
        let steep = |i: isize| {
            if i < 0 || i as usize >= n - 1 {
                return false;
            }
            let (a, b) = (node(i).position, node(i + 1).position);
            (a.y - b.y) / Vec2::new(b.x - a.x, b.z - a.z).length().max(1e-4) > self.falls_slope
        };
        let mut points: Vec<(Vec3, f32, f32)> = Vec::new();
        for i in 0..n - 1 {
            // a spline through a short drop between long reaches overshoots the lip and
            // folds the ribbon: drops and their neighbours stay straight
            let straight = steep(i as isize - 1) || steep(i as isize) || steep(i as isize + 1);
            let (p0, p1, p2, p3) = (
                node(i as isize - 1),
                node(i as isize),
                node(i as isize + 1),
                node(i as isize + 2),
            );
            let length = p1.position.distance(p2.position);
            let pieces = (length / step).ceil().max(1.0) as usize;
            for j in 0..pieces {
                let t = j as f32 / pieces as f32;
                let (t2, t3) = (t * t, t * t * t);
                let catmull = |a: Vec3, b: Vec3, c: Vec3, d: Vec3| {
                    0.5 * (2.0 * b
                        + (c - a) * t
                        + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2
                        + (3.0 * b - a - 3.0 * c + d) * t3)
                };
                // the drop stays monotonic: interpolate height linearly between nodes
                let mut p = if straight {
                    p1.position.lerp(p2.position, t)
                } else {
                    catmull(p0.position, p1.position, p2.position, p3.position)
                };
                p.y = p1.position.y + (p2.position.y - p1.position.y) * t;
                points.push((
                    p,
                    p1.width + (p2.width - p1.width) * t,
                    p1.depth + (p2.depth - p1.depth) * t,
                ));
            }
        }
        let last = self.nodes[n - 1];
        points.push((last.position, last.width, last.depth));
        let mut along = 0.0;
        // churned white water below a fall fades over this distance (m)
        let mut since_falls = f32::MAX;
        let mut out = Vec::with_capacity(points.len());
        for i in 0..points.len() {
            let prev = points[i.saturating_sub(1)].0;
            let next = points[(i + 1).min(points.len() - 1)].0;
            let delta = next - prev;
            let horizontal = Vec2::new(delta.x, delta.z).length().max(1e-4);
            let slope = (-delta.y / horizontal).max(0.0);
            let tangent = delta.normalize_or(Vec3::Z);
            let right = Vec3::new(-tangent.z, 0.0, tangent.x).normalize_or(Vec3::X);
            if i > 0 {
                along += points[i].0.distance(points[i - 1].0);
            }
            let falls = slope > self.falls_slope;
            if falls {
                since_falls = 0.0;
            } else if i > 0 && since_falls < f32::MAX {
                since_falls += points[i].0.distance(points[i - 1].0);
            }
            let plunge = if since_falls < f32::MAX {
                (-since_falls / 6.0).exp()
            } else {
                0.0
            };
            out.push(RiverSample {
                position: points[i].0,
                tangent,
                right,
                width: points[i].1,
                depth: points[i].2,
                speed: if falls {
                    (2.0 * 9.81 * horizontal.max(0.5)).sqrt().max(self.speed)
                } else {
                    self.speed * (1.0 + 6.0 * slope).min(5.0)
                },
                along,
                foam: if falls {
                    1.0
                } else {
                    (slope * 6.0).clamp(0.0, 1.0).max(plunge)
                },
                falls,
            });
        }
        out
    }

    /// Water at `point` if it lies over the channel (nearest centre-line segment within
    /// half the width). Waterfall sheets are not "inside": their water is airborne.
    pub fn sample(&self, point: Vec3, samples: &[RiverSample]) -> Option<RiverPoint> {
        let p = Vec2::new(point.x, point.z);
        let mut best: Option<(f32, RiverPoint)> = None;
        for pair in samples.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if a.falls || b.falls {
                continue;
            }
            let (a2, b2) = (
                Vec2::new(a.position.x, a.position.z),
                Vec2::new(b.position.x, b.position.z),
            );
            let ab = b2 - a2;
            let t = ((p - a2).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
            let distance = p.distance(a2 + ab * t);
            let half = (a.width + (b.width - a.width) * t) * 0.5;
            if distance > half || best.as_ref().is_some_and(|(d, _)| *d <= distance) {
                continue;
            }
            let lerp = |x: f32, y: f32| x + (y - x) * t;
            let level = lerp(a.position.y, b.position.y);
            // faster in the middle of the channel than at the banks
            let profile = 1.0 - (distance / half.max(1e-3)).powi(2) * 0.6;
            let velocity = a.tangent.lerp(b.tangent, t).normalize_or(Vec3::Z)
                * lerp(a.speed, b.speed)
                * profile;
            best = Some((
                distance,
                RiverPoint {
                    level,
                    velocity,
                    depth: lerp(a.depth, b.depth),
                    foam: lerp(a.foam, b.foam),
                    falls: false,
                },
            ));
        }
        best.map(|(_, p)| p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn river() -> River {
        let node = |x: f32, y: f32, z: f32| RiverNode {
            position: Vec3::new(x, y, z),
            width: 4.0,
            depth: 1.0,
        };
        River::new(
            vec![
                node(0., 5., 0.),
                node(0., 4.9, 20.),
                node(0., 4.7, 30.),
                node(0., 1.0, 31.),
                node(0., 0.9, 50.),
            ],
            1.0,
        )
    }

    #[test]
    fn slope_speeds_up_and_cliff_becomes_falls() {
        let r = river();
        let s = r.samples(0.5);
        assert!(s.iter().any(|s| s.falls));
        let flat = r.sample(Vec3::new(0.5, 5.0, 8.0), &s).unwrap();
        assert!(flat.velocity.z > 0.5 && flat.velocity.z < 1.5);
        assert!((flat.level - 4.96).abs() < 0.05, "{}", flat.level);
        let steep = r.sample(Vec3::new(0.0, 4.8, 25.0), &s).unwrap();
        assert!(steep.velocity.length() > flat.velocity.length());
        let plunge = r.sample(Vec3::new(0.0, 1.0, 33.0), &s).unwrap();
        assert!(
            plunge.foam > 0.5 && flat.foam < 0.1,
            "{} {}",
            plunge.foam,
            flat.foam
        );
    }

    #[test]
    fn outside_the_banks_is_dry() {
        let r = river();
        let s = r.samples(0.5);
        assert!(r.sample(Vec3::new(3.0, 5.0, 8.0), &s).is_none());
        assert!(r.sample(Vec3::new(0.0, 5.0, 60.0), &s).is_none());
    }
}
