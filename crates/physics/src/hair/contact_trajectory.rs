//! Discovery and endpoint Jacobian of a penetrating space-time surface witness.
//! Approximate discovery is never a clearance certificate; CCD admission remains
//! responsible for proving the entire corrected trajectory admissible.
use super::*;

#[derive(Clone, Copy, Debug)]
pub(in crate::hair) struct TrajectoryContact {
    pub time: f64,
    pub fraction: f64,
    pub normal: V,
    pub target: V,
    pub gap: f64,
}
impl TrajectoryContact {
    pub fn endpoint_gap(&self, points: [V; 2]) -> f64 {
        let p = add(
            mul(points[0], 1. - self.fraction),
            mul(points[1], self.fraction),
        );
        self.time * dot(sub(p, self.target), self.normal)
    }
}

pub(in crate::hair) fn trajectory_contact(
    capsule: CapsuleMotion,
    triangle: TriangleMotion,
    options: CapsuleSweepOptions,
) -> Result<Option<TrajectoryContact>, &'static str> {
    trajectory_contact_oriented(capsule, triangle, options, false)
}

pub(super) fn trajectory_contact_oriented(
    capsule: CapsuleMotion,
    triangle: TriangleMotion,
    options: CapsuleSweepOptions,
    closed: bool,
) -> Result<Option<TrajectoryContact>, &'static str> {
    validate_capsule(capsule, options)?;
    if sweep_capsule_triangle(capsule, triangle, options)? == CapsuleSweep::Clear {
        return Ok(None);
    }
    let evaluate = |t: f64| -> Result<_, &'static str> {
        let (p, q) = closest_points(capsule, triangle, t)?;
        Ok((t, len(sub(p, q)) - capsule.radius, p, q))
    };
    let samples = (0..=16)
        .map(|i| evaluate(i as f64 / 16.))
        .collect::<Result<Vec<_>, _>>()?;
    if samples[0].1 < -options.tolerance_m {
        return Err("trajectory begins below mesh clearance tolerance");
    }
    let mut best = samples[0];
    for value in &samples {
        if value.1 < best.1 {
            best = *value;
        }
    }
    let speed = capsule
        .start
        .iter()
        .zip(&capsule.end)
        .flat_map(|(a, b)| {
            triangle
                .start
                .iter()
                .zip(&triangle.end)
                .map(move |(c, d)| len(sub(sub(*b, *a), sub(*d, *c))))
        })
        .fold(0., f64::max);
    if !speed.is_finite() {
        return Err("trajectory witness motion overflow");
    }
    for i in 0..=16 {
        let (lo, hi) = if i == 0 {
            if samples[0].1 > samples[1].1 {
                continue;
            }
            (0, 1)
        } else if i == 16 {
            if samples[16].1 > samples[15].1 {
                continue;
            }
            (15, 16)
        } else {
            if samples[i].1 > samples[i - 1].1 || samples[i].1 > samples[i + 1].1 {
                continue;
            }
            (i - 1, i + 1)
        };
        let mut left = samples[lo].0;
        let mut right = samples[hi].0;
        let ratio = (5f64.sqrt() - 1.) * 0.5;
        let mut x = evaluate(right - ratio * (right - left))?;
        let mut y = evaluate(left + ratio * (right - left))?;
        for _ in 0..80 {
            for candidate in [x, y] {
                if candidate.1 < best.1 {
                    best = candidate;
                }
            }
            if (right - left) * speed <= 1e-12 {
                break;
            }
            if x.1 <= y.1 {
                right = y.0;
                y = x;
                x = evaluate(right - ratio * (right - left))?;
            } else {
                left = x.0;
                x = y;
                y = evaluate(left + ratio * (right - left))?;
            }
        }
    }
    if best.1 >= -options.tolerance_m {
        return Ok(None);
    }
    let (time, gap, p, q) = best;
    if time <= 0. {
        return Err("initial trajectory penetration cannot be corrected by endpoint motion");
    }
    let points: [V; 2] =
        std::array::from_fn(|i| add(mul(capsule.start[i], 1. - time), mul(capsule.end[i], time)));
    let direction = sub(points[1], points[0]);
    let squared = dot(direction, direction);
    let fraction = if squared > 0. {
        (dot(sub(p, points[0]), direction) / squared).clamp(0., 1.)
    } else {
        0.
    };
    let distance = len(sub(p, q));
    let normal = if distance <= 1e-12 {
        if !closed {
            return Err("trajectory surface crossing needs oriented contact activation");
        }
        let vertices: [V; 3] = std::array::from_fn(|i| {
            add(
                mul(triangle.start[i], 1. - time),
                mul(triangle.end[i], time),
            )
        });
        let n = cross(sub(vertices[1], vertices[0]), sub(vertices[2], vertices[0]));
        if !finite(n) || len(n) < 1e-14 {
            return Err("oriented trajectory crossing has a degenerate face");
        }
        let n = unit(n);
        let face = Triangle {
            ids: [0, 1, 2],
            p: vertices,
            previous_p: vertices,
            velocity: [[0.; 3]; 3],
            normal: n,
            min: [0.; 3],
            max: [0.; 3],
        };
        if !matches!(
            super::super::closest_triangle_feature(q, &face).1,
            super::super::ClosestFeature::Face
        ) {
            return Err("trajectory edge crossing needs oriented feature activation");
        }
        // The validated closed mesh owns winding. At an exact face crossing,
        // the signed-distance branch differentiates in its outward direction.
        n
    } else {
        mul(sub(p, q), 1. / distance)
    };
    let old = add(
        mul(capsule.start[0], 1. - fraction),
        mul(capsule.start[1], fraction),
    );
    let target = mul(
        sub(add(q, mul(normal, capsule.radius)), mul(old, 1. - time)),
        1. / time,
    );
    if !finite(target) || !finite(normal) {
        return Err("trajectory contact linearization overflow");
    }
    Ok(Some(TrajectoryContact {
        time,
        fraction,
        normal,
        target,
        gap,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hair::{ContactSource, HairMaterial, HairRod};
    #[test]
    fn exact_face_crossing_uses_validated_closed_winding() {
        let vertices = [[-1., -1., 0.], [1., -1., 0.], [0., 1., 0.]];
        for reversed in [false, true] {
            let vertices = if reversed {
                [vertices[0], vertices[2], vertices[1]]
            } else {
                vertices
            };
            let triangle = TriangleMotion {
                start: vertices,
                end: vertices,
            };
            let z = if reversed { -1. } else { 1. };
            let capsule = CapsuleMotion {
                start: [[-0.1, 0., z], [0.1, 0., z]],
                end: [[-0.1, 0., -z], [0.1, 0., -z]],
                radius: 40e-6,
            };
            assert!(trajectory_contact(capsule, triangle, Default::default()).is_err());
            let contact = trajectory_contact_oriented(capsule, triangle, Default::default(), true)
                .unwrap()
                .unwrap();
            assert_eq!(contact.normal, [0., 0., z]);
            assert!((contact.endpoint_gap(capsule.end) - contact.gap).abs() < 1e-12);
            let corrected = capsule.end.map(|p| add(p, mul(contact.normal, 1e-4)));
            assert!(contact.endpoint_gap(corrected) > contact.endpoint_gap(capsule.end));
            assert_ne!(
                sweep_capsule_triangle(capsule, triangle, Default::default()).unwrap(),
                CapsuleSweep::Clear
            );
        }
    }
    #[test]
    fn captured_time_contact_has_correct_jacobian_and_native_reaction() {
        let capsule = CapsuleMotion {
            start: [
                [
                    -0.09856575082434922,
                    0.5532094283493131,
                    0.001944126688601915,
                ],
                [
                    -0.10134279356150146,
                    0.5462632012035866,
                    0.008392947137751012,
                ],
            ],
            end: [
                [
                    -0.0979642690407443,
                    0.5544745766931043,
                    0.0008698362845989304,
                ],
                [
                    -0.10075827114148952,
                    0.5474753244209715,
                    0.007253672172510615,
                ],
            ],
            radius: 4e-05,
        };
        let triangle = TriangleMotion {
            start: [
                [
                    -0.10607147216796875,
                    0.5424706935882568,
                    0.010344523936510086,
                ],
                [
                    -0.0986102819442749,
                    0.5457484722137451,
                    0.010289707221090794,
                ],
                [
                    -0.09887480735778809,
                    0.5483585596084595,
                    0.007142554968595505,
                ],
            ],
            end: [
                [
                    -0.10607147216796875,
                    0.5424645841121674,
                    0.010344523936510086,
                ],
                [
                    -0.0986102819442749,
                    0.5457423627376556,
                    0.010289707221090794,
                ],
                [-0.09887480735778809, 0.54835245013237, 0.007142554968595505],
            ],
        };
        let contact = trajectory_contact(capsule, triangle, Default::default())
            .unwrap()
            .unwrap();
        assert!((0.1..0.2).contains(&contact.time));
        assert!(contact.gap < -2e-6);
        assert!((contact.endpoint_gap(capsule.end) - contact.gap).abs() < 1e-12);
        for endpoint in 0..2 {
            for axis in 0..3 {
                let epsilon = 1e-8;
                let mut a = capsule.end;
                let mut b = a;
                a[endpoint][axis] -= epsilon;
                b[endpoint][axis] += epsilon;
                let actual = (contact.endpoint_gap(b) - contact.endpoint_gap(a)) / (2. * epsilon);
                let weight = if endpoint == 0 {
                    1. - contact.fraction
                } else {
                    contact.fraction
                };
                assert!((actual - contact.time * weight * contact.normal[axis]).abs() < 1e-7);
                let actual_geometry = (trajectory_contact(
                    CapsuleMotion { end: b, ..capsule },
                    triangle,
                    Default::default(),
                )
                .unwrap()
                .unwrap()
                .gap - trajectory_contact(
                    CapsuleMotion { end: a, ..capsule },
                    triangle,
                    Default::default(),
                )
                .unwrap()
                .unwrap()
                .gap)
                    / (2. * epsilon);
                assert!(
                    (actual_geometry - contact.time * weight * contact.normal[axis]).abs() < 1e-6,
                    "space-time witness Jacobian must match re-optimized geometry"
                );
            }
        }
        // Local two-segment constitutive fixture uses the captured capsule,
        // not the entire captured 469-guide articulated state.
        let root = sub(capsule.start[0], sub(capsule.start[1], capsule.start[0]));
        let mut rod = HairRod::new(
            vec![root, capsule.start[0], capsule.start[1]],
            HairMaterial::default(),
        )
        .unwrap();
        // Carry the local prescribed root with the capsule's common translation;
        // freezing this synthetic root would introduce unrelated 18% strain.
        let root = add(root, sub(capsule.end[0], capsule.start[0]));
        rod.x[0] = root;
        rod.x[1] = capsule.end[0];
        rod.x[2] = capsule.end[1];
        let row = rod.record_contact(
            1,
            contact.fraction,
            contact.normal,
            contact.target,
            ContactSource::Mesh(0),
        );
        rod.contacts[row].metric_scale = contact.time;
        let mut rods = [rod];
        crate::hair::contact::reconcile_contact_positions(
            &mut rods,
            &mut [],
            1. / 240.,
            capsule.radius,
        )
        .unwrap();
        assert_eq!(rods[0].x[0], root);
        let after = contact.endpoint_gap([rods[0].x[1], rods[0].x[2]]);
        eprintln!(
            "CAPTURED TRAJECTORY RESPONSE time={} fraction={} before={} after={} stretch={}",
            contact.time,
            contact.fraction,
            contact.gap,
            after,
            rods[0].max_relative_stretch()
        );
        assert!(after >= -1e-10);
        let refreshed = trajectory_contact(
            CapsuleMotion {
                end: [rods[0].x[1], rods[0].x[2]],
                ..capsule
            },
            triangle,
            Default::default(),
        )
        .unwrap();
        if let Some(witness) = refreshed {
            eprintln!(
                "CAPTURED TRAJECTORY RESPONSE refreshed_time={} refreshed_gap={}",
                witness.time, witness.gap
            );
            assert!(
                witness.gap > contact.gap,
                "native reaction must decrease actual re-optimized penetration"
            );
        }
        for iteration in 0..32 {
            let motion = CapsuleMotion {
                end: [rods[0].x[1], rods[0].x[2]],
                ..capsule
            };
            let query = sweep_capsule_triangle(motion, triangle, Default::default()).unwrap();
            if query == CapsuleSweep::Clear {
                eprintln!("CAPTURED TRAJECTORY CERTIFIED iteration={iteration}");
                assert!(rods[0].max_relative_stretch() <= 0.05);
                assert_eq!(rods[0].x[0], root);
                return;
            }
            let next = trajectory_contact(motion, triangle, Default::default())
                .unwrap()
                .expect("an uncertified trajectory must still provide a correctable witness");
            eprintln!(
                "CAPTURED TRAJECTORY ITERATION iteration={iteration} time={} gap={} query={query:?}",
                next.time, next.gap
            );
            rods[0].contacts.clear();
            let row = rods[0].record_contact(
                1,
                next.fraction,
                next.normal,
                next.target,
                ContactSource::Mesh(0),
            );
            rods[0].contacts[row].metric_scale = next.time;
            crate::hair::contact::reconcile_contact_positions(
                &mut rods,
                &mut [],
                1. / 240.,
                capsule.radius,
            )
            .unwrap();
            assert_eq!(rods[0].x[0], root);
            assert!(rods[0].max_relative_stretch() <= 0.05);
        }
        panic!("coupled local contact trajectory must reach certified admission");
    }
}
