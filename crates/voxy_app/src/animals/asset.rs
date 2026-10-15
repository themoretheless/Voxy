//! Original implicit animal anatomy, tessellated in Rust. No external geometry.
use super::*;
use glam::Mat3;
use std::{collections::HashMap, sync::OnceLock};
pub struct Surface {
    pub vertices: Vec<SkinnedVertex>,
    pub indices: Vec<u32>,
    pub fur: Vec<bool>,
}
static DOG: OnceLock<Result<Surface, String>> = OnceLock::new();
static CAT: OnceLock<Result<Surface, String>> = OnceLock::new();
pub fn surface(cat: bool) -> Result<&'static Surface, String> {
    (if cat { &CAT } else { &DOG })
        .get_or_init(|| build(cat))
        .as_ref()
        .map_err(Clone::clone)
}
#[derive(Clone, Copy)]
struct Muscle {
    center: Vec3,
    radii: Vec3,
    inverse: Mat3,
}
impl Muscle {
    fn distance(&self, p: Vec3) -> f32 {
        let q = self.inverse * (p - self.center);
        let k0 = (q / self.radii).length();
        let k1 = (q / (self.radii * self.radii)).length();
        if k1 < 1e-8 {
            -self.radii.min_element()
        } else {
            k0 * (k0 - 1.0) / k1
        }
    }
}
struct Anatomy {
    muscles: Vec<Muscle>,
    smooth: f32,
    profile: Vec<[f32; 4]>,
    cuts: Vec<Muscle>,
    segments: Vec<(Vec3, Vec3, f32, f32)>,
}
impl Anatomy {
    fn ellipsoid(&mut self, center: Vec3, radii: Vec3) {
        self.muscles.push(Muscle {
            center,
            radii,
            inverse: Mat3::IDENTITY,
        });
    }
    fn limb(&mut self, a: Vec3, b: Vec3, width: f32, depth: f32) {
        self.segments
            .push((a, b, width.min(depth), width.min(depth) * 0.88));
    }
    fn field(&self, p: Vec3) -> f32 {
        let profile = &self.profile;
        let z = p.z.clamp(profile[0][0], profile[profile.len() - 1][0]);
        let segment = profile
            .partition_point(|sample| sample[0] < z)
            .saturating_sub(1)
            .min(profile.len() - 2);
        let left = profile[segment];
        let right = profile[segment + 1];
        let t = ((z - left[0]) / (right[0] - left[0])).clamp(0.0, 1.0);
        let t = t * t * (3.0 - 2.0 * t);
        let cy = left[1] + (right[1] - left[1]) * t;
        let rx = left[2] + (right[2] - left[2]) * t;
        let ry = left[3] + (right[3] - left[3]) * t;
        let radial = ((p.x / rx).powi(2) + ((p.y - cy) / ry).powi(2)).sqrt();
        let torso = (radial - 1.0) * rx.min(ry);
        let outside = (p.z - z).abs();
        let initial = if outside > 0.0 {
            torso.max(outside)
        } else {
            torso
        };
        let body = self.muscles.iter().fold(initial, |distance, muscle| {
            let b = muscle.distance(p);
            let h = (0.5 + 0.5 * (b - distance) / self.smooth).clamp(0.0, 1.0);
            b * (1.0 - h) + distance * h - self.smooth * h * (1.0 - h)
        });
        let body = self.segments.iter().fold(body, |distance, (a, b, ra, rb)| {
            let axis = *b - *a;
            let t = ((p - *a).dot(axis) / axis.length_squared()).clamp(0.0, 1.0);
            let d = (p - (*a + axis * t)).length() - (*ra + (*rb - *ra) * t);
            let smooth = if a.z < -0.25 && b.z < a.z {
                self.smooth.min(ra * 0.12)
            } else {
                self.smooth
            };
            let h = (0.5 + 0.5 * (d - distance) / smooth).clamp(0.0, 1.0);
            d * (1.0 - h) + distance * h - smooth * h * (1.0 - h)
        });
        self.cuts
            .iter()
            .fold(body, |distance, cut| distance.max(-cut.distance(p)))
    }
    fn normal(&self, p: Vec3) -> Vec3 {
        let e = 0.0003;
        Vec3::new(
            self.field(p + Vec3::X * e) - self.field(p - Vec3::X * e),
            self.field(p + Vec3::Y * e) - self.field(p - Vec3::Y * e),
            self.field(p + Vec3::Z * e) - self.field(p - Vec3::Z * e),
        )
        .normalize_or_zero()
    }
}
fn anatomy(cat: bool) -> Anatomy {
    let mut a = Anatomy {
        muscles: Vec::new(),
        smooth: if cat { 0.012 } else { 0.022 },
        profile: if cat {
            vec![
                [-0.29, 0.285, 0.016, 0.04],
                [-0.23, 0.285, 0.082, 0.10],
                [-0.12, 0.29, 0.086, 0.108],
                [0.0, 0.285, 0.079, 0.102],
                [0.14, 0.29, 0.082, 0.122],
                [0.22, 0.31, 0.06, 0.10],
                [0.255, 0.32, 0.025, 0.06],
            ]
        } else {
            vec![
                [-0.44, 0.46, 0.02, 0.05],
                [-0.33, 0.46, 0.132, 0.16],
                [-0.16, 0.45, 0.138, 0.17],
                [0.0, 0.465, 0.15, 0.19],
                [0.18, 0.485, 0.16, 0.21],
                [0.30, 0.49, 0.135, 0.22],
                [0.39, 0.52, 0.045, 0.085],
            ]
        },
        cuts: Vec::new(),
        segments: Vec::new(),
    };
    if cat {
        a.limb(
            Vec3::new(0.0, 0.32, 0.17),
            Vec3::new(0.0, 0.385, 0.245),
            0.060,
            0.064,
        );
        a.ellipsoid(Vec3::new(0.0, 0.402, 0.265), Vec3::new(0.077, 0.064, 0.072));
        a.ellipsoid(Vec3::new(0.0, 0.371, 0.292), Vec3::new(0.058, 0.037, 0.055));
        a.ellipsoid(Vec3::new(0.0, 0.350, 0.315), Vec3::new(0.037, 0.012, 0.029));
        for side in [-1.0, 1.0] {
            a.ellipsoid(
                Vec3::new(side * 0.024, 0.377, 0.326),
                Vec3::new(0.028, 0.020, 0.024),
            );
            a.cuts.push(Muscle {
                center: Vec3::new(side * 0.042, 0.414, 0.319),
                radii: Vec3::new(0.022, 0.013, 0.014),
                inverse: Mat3::IDENTITY,
            });
            for front in [false, true] {
                let x = side * 0.068;
                let hip = Vec3::new(x, 0.29, if front { 0.17 } else { -0.19 });
                let knee = Vec3::new(x, 0.18, if front { 0.19 } else { -0.12 });
                let hock = Vec3::new(x, 0.072, if front { 0.19 } else { -0.24 });
                let ankle = Vec3::new(x, 0.028, if front { 0.205 } else { -0.21 });
                a.limb(hip, knee, if front { 0.028 } else { 0.042 }, 0.031);
                a.limb(knee, hock, 0.019, 0.020);
                a.limb(hock, ankle, 0.014, 0.015);
                a.ellipsoid(
                    ankle + Vec3::new(0.0, -0.009, 0.012),
                    Vec3::new(0.025, 0.019, 0.035),
                );
                for toe in [-1.0, 0.0, 1.0] {
                    a.ellipsoid(
                        ankle + Vec3::new(toe * 0.013, -0.010, 0.034),
                        Vec3::new(0.009, 0.014, 0.013),
                    );
                }
            }
        }
        let tail = [
            Vec3::new(0.0, 0.33, -0.255),
            Vec3::new(0.0, 0.36, -0.36),
            Vec3::new(0.0, 0.40, -0.47),
            Vec3::new(0.0, 0.44, -0.57),
            Vec3::new(0.025, 0.46, -0.63),
        ];
        for (i, pair) in tail.windows(2).enumerate() {
            a.limb(
                pair[0],
                pair[1],
                0.018 - i as f32 * 0.002,
                0.018 - i as f32 * 0.002,
            );
        }
    } else {
        a.limb(
            Vec3::new(0.0, 0.53, 0.27),
            Vec3::new(0.0, 0.70, 0.445),
            0.105,
            0.115,
        );
        a.ellipsoid(Vec3::new(0.0, 0.755, 0.49), Vec3::new(0.108, 0.101, 0.122));
        a.ellipsoid(Vec3::new(0.0, 0.711, 0.62), Vec3::new(0.083, 0.063, 0.143));
        a.ellipsoid(Vec3::new(0.0, 0.665, 0.614), Vec3::new(0.073, 0.025, 0.108));
        a.ellipsoid(Vec3::new(0.0, 0.735, 0.565), Vec3::new(0.064, 0.047, 0.07));
        for side in [-1.0, 1.0] {
            a.ellipsoid(
                Vec3::new(side * 0.078, 0.795, 0.559),
                Vec3::new(0.031, 0.017, 0.034),
            );
            a.cuts.push(Muscle {
                center: Vec3::new(side * 0.074, 0.775, 0.585),
                radii: Vec3::new(0.023, 0.013, 0.015),
                inverse: Mat3::IDENTITY,
            });
            for front in [false, true] {
                let x = side * 0.118;
                let hip = Vec3::new(x, 0.49, if front { 0.29 } else { -0.30 });
                let knee = Vec3::new(x, 0.285, if front { 0.32 } else { -0.16 });
                let hock = Vec3::new(x, 0.12, if front { 0.325 } else { -0.34 });
                let ankle = Vec3::new(x, 0.038, if front { 0.35 } else { -0.31 });
                a.limb(hip, knee, if front { 0.046 } else { 0.067 }, 0.050);
                a.limb(knee, hock, 0.029, 0.030);
                a.limb(hock, ankle, 0.020, 0.022);
                a.ellipsoid(
                    ankle + Vec3::new(0.0, -0.012, 0.024),
                    Vec3::new(0.042, 0.026, 0.058),
                );
                for toe in [-1.5, -0.5, 0.5, 1.5] {
                    a.ellipsoid(
                        ankle + Vec3::new(toe * 0.018, -0.011, 0.061),
                        Vec3::new(0.012, 0.020, 0.020),
                    );
                }
            }
        }
        let tail = [
            Vec3::new(0.0, 0.53, -0.40),
            Vec3::new(0.0, 0.515, -0.54),
            Vec3::new(0.0, 0.49, -0.68),
            Vec3::new(0.015, 0.46, -0.82),
            Vec3::new(0.025, 0.44, -0.92),
        ];
        for (i, pair) in tail.windows(2).enumerate() {
            a.limb(
                pair[0],
                pair[1],
                0.035 - i as f32 * 0.007,
                0.035 - i as f32 * 0.007,
            );
        }
    }
    a
}
fn build(cat: bool) -> Result<Surface, String> {
    let anatomy = anatomy(cat);
    let step = if cat { 0.009 } else { 0.014 };
    let min = if cat {
        Vec3::new(-0.20, -0.02, -0.68)
    } else {
        Vec3::new(-0.25, -0.02, -0.98)
    };
    let max = if cat {
        Vec3::new(0.20, 0.80, 0.44)
    } else {
        Vec3::new(0.25, 1.02, 0.86)
    };
    let size = ((max - min) / step).ceil().as_uvec3() + glam::UVec3::ONE;
    let count = (size.x * size.y * size.z) as usize;
    let mut points = Vec::with_capacity(count);
    let mut values = Vec::with_capacity(count);
    for z in 0..size.z {
        for y in 0..size.y {
            for x in 0..size.x {
                let p = min + Vec3::new(x as f32, y as f32, z as f32) * step;
                points.push(p);
                values.push(anatomy.field(p));
            }
        }
    }
    let mut result = Surface {
        vertices: Vec::new(),
        indices: Vec::new(),
        fur: Vec::new(),
    };
    let mut edges = HashMap::<(usize, usize), u32>::new();
    let tetrahedra = [
        [0, 1, 3, 7],
        [0, 3, 2, 7],
        [0, 2, 6, 7],
        [0, 6, 4, 7],
        [0, 4, 5, 7],
        [0, 5, 1, 7],
    ];
    let index = |x: u32, y: u32, z: u32| (x + size.x * (y + size.y * z)) as usize;
    for z in 0..size.z - 1 {
        for y in 0..size.y - 1 {
            for x in 0..size.x - 1 {
                let cube = [
                    index(x, y, z),
                    index(x + 1, y, z),
                    index(x, y + 1, z),
                    index(x + 1, y + 1, z),
                    index(x, y, z + 1),
                    index(x + 1, y, z + 1),
                    index(x, y + 1, z + 1),
                    index(x + 1, y + 1, z + 1),
                ];
                if cube.iter().all(|&i| values[i] > 0.0) || cube.iter().all(|&i| values[i] <= 0.0) {
                    continue;
                }
                for tetra in tetrahedra {
                    let mut inside = Vec::with_capacity(4);
                    let mut outside = Vec::with_capacity(4);
                    for k in tetra {
                        let i = cube[k];
                        if values[i] <= 0.0 {
                            inside.push(i);
                        } else {
                            outside.push(i);
                        }
                    }
                    if inside.is_empty() || outside.is_empty() {
                        continue;
                    }
                    let mut crossing = |a: usize, b: usize| -> u32 {
                        let key = (a.min(b), a.max(b));
                        *edges.entry(key).or_insert_with(|| {
                            let t = values[a] / (values[a] - values[b]);
                            let p = points[a].lerp(points[b], t);
                            let n = anatomy.normal(p);
                            let color = coat(
                                cat,
                                p,
                                if cat {
                                    Vec3::new(0.30, 0.31, 0.32)
                                } else {
                                    Vec3::new(0.36, 0.18, 0.065)
                                },
                            );
                            let (joints, weights) = skin(p, cat);
                            let next = result.vertices.len() as u32;
                            result.vertices.push(SkinnedVertex {
                                position: p.to_array(),
                                normal: n.to_array(),
                                uv: encoded_color(color, 0.78),
                                joints,
                                weights,
                            });
                            result.fur.push(true);
                            next
                        })
                    };
                    let triangles: Vec<[u32; 3]> = if inside.len() == 1 {
                        vec![[
                            crossing(inside[0], outside[0]),
                            crossing(inside[0], outside[1]),
                            crossing(inside[0], outside[2]),
                        ]]
                    } else if outside.len() == 1 {
                        vec![[
                            crossing(outside[0], inside[0]),
                            crossing(outside[0], inside[1]),
                            crossing(outside[0], inside[2]),
                        ]]
                    } else {
                        let a = crossing(inside[0], outside[0]);
                        let b = crossing(inside[0], outside[1]);
                        let c = crossing(inside[1], outside[1]);
                        let d = crossing(inside[1], outside[0]);
                        vec![[a, b, c], [a, c, d]]
                    };
                    for mut triangle in triangles {
                        let p = triangle
                            .map(|i| Vec3::from_array(result.vertices[i as usize].position));
                        let n =
                            triangle.map(|i| Vec3::from_array(result.vertices[i as usize].normal));
                        if (p[1] - p[0]).cross(p[2] - p[0]).dot(n[0] + n[1] + n[2]) < 0.0 {
                            triangle.swap(1, 2);
                        }
                        result.indices.extend(triangle);
                    }
                }
            }
        }
    }
    features(&mut result, cat);
    Ok(result)
}
fn detail(s: &mut Surface, center: Vec3, radii: Vec3, color: Vec3, roughness: f32, cat: bool) {
    let start = s.vertices.len() as u32;
    for y in 0..=16 {
        for x in 0..=24 {
            let theta = std::f32::consts::PI * y as f32 / 16.0;
            let phi = std::f32::consts::TAU * x as f32 / 24.0;
            let n = Vec3::new(
                theta.sin() * phi.cos(),
                theta.cos(),
                theta.sin() * phi.sin(),
            );
            let p = center + n * radii;
            let (joints, weights) = skin(p, cat);
            s.vertices.push(SkinnedVertex {
                position: p.to_array(),
                normal: (n / radii).normalize().to_array(),
                uv: encoded_color(color, roughness),
                joints,
                weights,
            });
            s.fur.push(false);
        }
    }
    for y in 0..16 {
        for x in 0..24 {
            let a = start + y * 25 + x;
            s.indices.extend([a, a + 1, a + 25, a + 1, a + 26, a + 25]);
        }
    }
}
fn features(s: &mut Surface, cat: bool) {
    for side in [-1.0, 1.0] {
        eye_patch(s, side, cat);
        ear_flap(s, side, cat);
        let origin = if cat {
            Vec3::new(side * 0.027, 0.378, 0.346)
        } else {
            Vec3::new(side * 0.068, 0.707, 0.723)
        };
        for k in 0..if cat { 7 } else { 3 } {
            let end = origin
                + Vec3::new(
                    side * if cat { 0.075 } else { 0.035 },
                    (k as f32 - if cat { 3.0 } else { 1.0 }) * 0.005,
                    -0.010 - k as f32 * 0.002,
                );
            whisker(s, origin, end, cat);
        }
    }
    detail(
        s,
        if cat {
            Vec3::new(0.0, 0.390, 0.348)
        } else {
            Vec3::new(0.0, 0.730, 0.752)
        },
        if cat {
            Vec3::new(0.011, 0.007, 0.006)
        } else {
            Vec3::new(0.040, 0.024, 0.013)
        },
        if cat {
            Vec3::new(0.12, 0.042, 0.032)
        } else {
            Vec3::splat(0.007)
        },
        0.45,
        cat,
    );
    for side in [-1.0, 1.0] {
        detail(
            s,
            if cat {
                Vec3::new(side * 0.005, 0.391, 0.353)
            } else {
                Vec3::new(side * 0.022, 0.732, 0.764)
            },
            if cat {
                Vec3::new(0.002, 0.0012, 0.001)
            } else {
                Vec3::new(0.007, 0.004, 0.002)
            },
            Vec3::splat(0.001),
            0.68,
            cat,
        );
        // Lip commissure, following the tapered muzzle rather than a smiling line.
        let points = if cat {
            vec![
                Vec3::new(0.0, 0.378, 0.348),
                Vec3::new(side * 0.013, 0.365, 0.345),
                Vec3::new(side * 0.033, 0.365, 0.330),
            ]
        } else {
            vec![
                Vec3::new(side * 0.02, 0.684, 0.752),
                Vec3::new(side * 0.063, 0.681, 0.697),
                Vec3::new(side * 0.077, 0.684, 0.636),
            ]
        };
        for pair in points.windows(2) {
            line_tube(
                s,
                pair[0],
                pair[1],
                if cat { 0.0007 } else { 0.0015 },
                Vec3::splat(0.008),
                cat,
            );
        }
    }
}
fn eye_patch(s: &mut Surface, side: f32, cat: bool) {
    let center = if cat {
        Vec3::new(side * 0.042, 0.414, 0.320)
    } else {
        Vec3::new(side * 0.074, 0.775, 0.586)
    };
    let forward = Vec3::new(side * 0.30, 0.03, 1.0).normalize();
    let right = Vec3::Y.cross(forward).normalize();
    let up = forward.cross(right);
    let rx = if cat { 0.014 } else { 0.016 };
    let ry = if cat { 0.0065 } else { 0.0065 };
    let start = s.vertices.len() as u32;
    for row in 0..=14 {
        let r = row as f32 / 14.0;
        for k in 0..=64 {
            let angle = std::f32::consts::TAU * k as f32 / 64.0;
            let x = angle.cos() * r;
            let y = angle.sin().signum() * (1.0 - x * x) * r;
            let p = center
                + right * x * rx
                + up * (y * ry + side * x * 0.002)
                + forward * (0.004 * (1.0 - r * r));
            let radial = ((x * rx).powi(2) + (y * ry).powi(2)).sqrt();
            let iris = if cat {
                Vec3::new(0.105, 0.145, 0.035)
            } else {
                Vec3::new(0.085, 0.041, 0.012)
            };
            let spokes =
                (angle * 79.0 + radial * 1900.0).sin() * 0.12 + (angle * 137.0).sin() * 0.06;
            let pupil = if cat {
                (x * rx).abs() < 0.0025 * (1.0 - (y * 0.7).powi(2))
            } else {
                radial < 0.0045
            };
            let color = if pupil {
                Vec3::splat(0.001)
            } else if radial < if cat { 0.0115 } else { 0.010 } {
                iris * (1.0 + spokes)
            } else {
                Vec3::new(0.019, 0.016, 0.013)
            };
            s.vertices.push(SkinnedVertex {
                position: p.to_array(),
                normal: (forward + right * x * 0.14 + up * y * 0.14)
                    .normalize()
                    .to_array(),
                uv: encoded_color(color, 0.11),
                joints: [0; 4],
                weights: [65535, 0, 0, 0],
            });
            s.fur.push(false);
        }
    }
    for row in 0..14 {
        for k in 0..64 {
            let a = start + row * 65 + k;
            s.indices.extend([a, a + 65, a + 1, a + 1, a + 65, a + 66]);
        }
    }
    let mut rim = Vec::new();
    for k in 0..=64 {
        let angle = std::f32::consts::TAU * k as f32 / 64.0;
        let x = angle.cos();
        let y = angle.sin().signum() * (1.0 - x * x);
        rim.push(center + right * x * rx + up * (y * ry + side * x * 0.002) + forward * 0.0005);
    }
    for pair in rim.windows(2) {
        line_tube(
            s,
            pair[0],
            pair[1],
            if cat { 0.0015 } else { 0.002 },
            Vec3::new(0.016, 0.011, 0.008),
            cat,
        );
    }
}
fn ear_flap(s: &mut Surface, side: f32, cat: bool) {
    let (a, b, c) = if cat {
        (
            Vec3::new(side * 0.030, 0.437, 0.246),
            Vec3::new(side * 0.090, 0.434, 0.230),
            Vec3::new(side * 0.086, 0.515, 0.223),
        )
    } else {
        (
            Vec3::new(side * 0.088, 0.795, 0.484),
            Vec3::new(side * 0.141, 0.778, 0.413),
            Vec3::new(side * 0.146, 0.650, 0.463),
        )
    };
    let point = |u: f32, t: f32, back: bool| {
        let mut p = a.lerp(b, u).lerp(c, t);
        let cup = (u * (1.0 - u) * (1.0 - t)) * if cat { 0.060 } else { 0.090 };
        p.z += if back { -0.003 - cup } else { 0.003 - cup };
        p.x += side * (std::f32::consts::PI * t).sin() * if cat { 0.004 } else { 0.014 };
        p
    };
    for back in [false, true] {
        let start = s.vertices.len() as u32;
        for row in 0..=24 {
            let t = row as f32 / 24.0;
            for col in 0..=24 {
                let u = col as f32 / 24.0;
                let p = point(u, t, back);
                let du =
                    point((u + 0.001).min(1.0), t, back) - point((u - 0.001).max(0.0), t, back);
                let dv =
                    point(u, (t + 0.001).min(0.9999), back) - point(u, (t - 0.001).max(0.0), back);
                let mut n = du.cross(dv).normalize_or_zero();
                if (side > 0.0) == back {
                    n = -n;
                }
                if n.length_squared() < 0.5 {
                    n = Vec3::Z;
                }
                let edge = (u.min(1.0 - u) * (1.0 - t)).min(t);
                let color = if cat && !back && edge > 0.07 {
                    Vec3::new(0.10, 0.047, 0.038) * (0.85 + 0.15 * (u * 77.0 + t * 93.0).sin())
                } else if cat {
                    Vec3::new(0.13, 0.105, 0.080)
                } else {
                    Vec3::new(0.14, 0.068, 0.023)
                };
                s.vertices.push(SkinnedVertex {
                    position: p.to_array(),
                    normal: n.to_array(),
                    uv: encoded_color(color, 0.92),
                    joints: [0; 4],
                    weights: [65535, 0, 0, 0],
                });
                s.fur.push(back || edge < 0.05);
            }
        }
        for row in 0..24 {
            for col in 0..24 {
                let i = start + row * 25 + col;
                if (side > 0.0) == back {
                    s.indices.extend([i, i + 25, i + 1, i + 1, i + 25, i + 26]);
                } else {
                    s.indices.extend([i, i + 1, i + 25, i + 1, i + 26, i + 25]);
                }
            }
        }
    }
}
fn line_tube(s: &mut Surface, a: Vec3, b: Vec3, radius: f32, color: Vec3, cat: bool) {
    let axis = (b - a).normalize();
    let right = axis
        .cross(if axis.y.abs() > 0.95 {
            Vec3::X
        } else {
            Vec3::Y
        })
        .normalize();
    let up = axis.cross(right);
    let start = s.vertices.len() as u32;
    for point in [a, b] {
        for k in 0..6 {
            let angle = std::f32::consts::TAU * k as f32 / 6.0;
            let n = right * angle.cos() + up * angle.sin();
            let (joints, weights) = skin(point, cat);
            s.vertices.push(SkinnedVertex {
                position: (point + n * radius).to_array(),
                normal: n.to_array(),
                uv: encoded_color(color, 0.7),
                joints,
                weights,
            });
            s.fur.push(false);
        }
    }
    for k in 0..6 {
        let a = start + k;
        let b = start + (k + 1) % 6;
        s.indices.extend([a, b, a + 6, b, b + 6, a + 6]);
    }
}
fn whisker(s: &mut Surface, a: Vec3, b: Vec3, cat: bool) {
    let tangent = (b - a).normalize().cross(Vec3::Y).normalize();
    let bitangent = (b - a).normalize().cross(tangent);
    let start = s.vertices.len() as u32;
    for k in 0..=5 {
        let t = k as f32 / 5.0;
        let p = a.lerp(b, t) + Vec3::Y * (t * t * 0.007);
        for side in 0..4 {
            let phi = std::f32::consts::TAU * side as f32 / 4.0;
            let n = tangent * phi.cos() + bitangent * phi.sin();
            let (joints, weights) = skin(p, cat);
            s.vertices.push(SkinnedVertex {
                position: (p + n * (0.00012 * (1.0 - t * 0.85))).to_array(),
                normal: n.to_array(),
                uv: encoded_color(Vec3::splat(if cat { 0.30 } else { 0.045 }), 0.7),
                joints,
                weights,
            });
            s.fur.push(false);
        }
    }
    for k in 0..5 {
        for side in 0..4 {
            let a = start + k * 4 + side;
            let b = start + k * 4 + (side + 1) % 4;
            s.indices.extend([a, b, a + 4, b, b + 4, a + 4]);
        }
    }
}
/// UV.y <= -10000 explicitly selects packed linear RGB and roughness in the
/// native skinned pipelines. Decode in the vertex shader before interpolation.
pub fn encoded_color(color: Vec3, roughness: f32) -> [f32; 2] {
    let c = color
        .clamp(Vec3::ZERO, Vec3::ONE)
        .to_array()
        .map(|x| (x * 255.0).round() as u32);
    [
        (c[0] | c[1] << 8 | c[2] << 16) as f32,
        -10000.0 - roughness.clamp(0.04, 1.0),
    ]
}
fn coat(cat: bool, p: Vec3, base: Vec3) -> Vec3 {
    let fine = (p.x * 641.0 + p.y * 511.0 + p.z * 733.0).sin() * 0.025;
    if cat {
        let stripe =
            ((p.z * 63.0 + (p.y * 40.0).sin() * 1.2 + p.x.abs() * 21.0).sin() * 0.5 + 0.5).powi(15);
        let belly = ((0.25 - p.y) * 12.0).clamp(0.0, 1.0);
        let pigment = Vec3::new(0.115, 0.092, 0.067) * (1.0 - stripe * 0.40 + fine);
        pigment.lerp(Vec3::new(0.21, 0.18, 0.14), belly * 0.65)
    } else {
        let _ = base;
        Vec3::new(0.18, 0.082, 0.026) * (0.9 + fine + 0.1 * (p.y * 7.0).sin())
    }
}
/// Continuous weights keep the authored shoulder and knee surfaces connected.
pub fn skin(p: Vec3, cat: bool) -> ([u16; 4], [u16; 4]) {
    let hips = leg_pivots(cat);
    let leg = if p.z > 0.0 {
        if p.x < 0.0 { 0 } else { 1 }
    } else if p.x < 0.0 {
        2
    } else {
        3
    };
    let hip = hips[leg];
    let region = ((p.x.abs() - if cat { 0.020 } else { 0.038 }) / if cat { 0.035 } else { 0.055 })
        .clamp(0.0, 1.0)
        * (1.0 - (p.z - hip.z).abs() / if cat { 0.14 } else { 0.22 }).clamp(0.0, 1.0);
    let limb = ((hip.y + 0.025 - p.y) / 0.13).clamp(0.0, 1.0) * region;
    let foot = ((hip.y * 0.52 - p.y) / 0.10).clamp(0.0, 1.0);
    // The raised feline tail occupies the posterior, high region.
    let tail = if p.z < if cat { -0.24 } else { -0.51 } && p.y > if cat { 0.34 } else { 0.30 } {
        ((-p.z - if cat { 0.24 } else { 0.51 }) / 0.12).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let values = [
        (1.0 - limb) * (1.0 - tail),
        limb * (1.0 - foot) * (1.0 - tail),
        limb * foot * (1.0 - tail),
        tail,
    ];
    let mut weights = values.map(|w| (w * 65535.0).floor() as u16);
    let sum: u32 = weights.iter().map(|&w| u32::from(w)).sum();
    weights[0] += (65535 - sum) as u16;
    ([0, 1 + leg as u16, 5 + leg as u16, 9], weights)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ear_winding_agrees_with_surface_normals() {
        for cat in [false, true] {
            for side in [-1.0, 1.0] {
                let mut surface = Surface {
                    vertices: Vec::new(),
                    indices: Vec::new(),
                    fur: Vec::new(),
                };
                ear_flap(&mut surface, side, cat);
                for face in surface.indices.chunks_exact(3) {
                    let p = std::array::from_fn::<_, 3, _>(|k| {
                        Vec3::from_array(surface.vertices[face[k] as usize].position)
                    });
                    let normal = face
                        .iter()
                        .map(|&i| Vec3::from_array(surface.vertices[i as usize].normal))
                        .sum::<Vec3>();
                    assert!(
                        (p[1] - p[0]).cross(p[2] - p[0]).dot(normal) >= -1e-10,
                        "ear normals face away from the visible geometry"
                    );
                }
            }
        }
    }
}
