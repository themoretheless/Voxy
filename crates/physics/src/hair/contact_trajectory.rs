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
    trajectory_contact_after_nonclear_sweep(capsule, triangle, options, closed)
}

// The batch query has already validated this exact motion and evaluated its
// sweep. Never use this entry point for a broad-phase overlap alone.
pub(super) fn trajectory_contact_after_nonclear_sweep(
    capsule: CapsuleMotion,
    triangle: TriangleMotion,
    options: CapsuleSweepOptions,
    closed: bool,
) -> Result<Option<TrajectoryContact>, &'static str> {
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
    let (time, mut gap, p, q) = best;
    if time <= 0. {
        return Err("initial trajectory penetration cannot be corrected by endpoint motion");
    }
    // Match closest_points' interpolation exactly. Endpoint witnesses must
    // keep their topological identity when published on adjacent segments.
    let points: [V; 2] = std::array::from_fn(|i| {
        add(
            capsule.start[i],
            mul(sub(capsule.end[i], capsule.start[i]), time),
        )
    });
    let direction = sub(points[1], points[0]);
    let squared = dot(direction, direction);
    let fraction = if p == points[0] {
        0.
    } else if p == points[1] {
        1.
    } else if squared > 0. {
        (dot(sub(p, points[0]), direction) / squared).clamp(0., 1.)
    } else {
        0.
    };
    let distance = len(sub(p, q));
    let oriented_face_normal = if closed {
        let vertices: [V; 3] = std::array::from_fn(|i| {
            add(
                triangle.start[i],
                mul(sub(triangle.end[i], triangle.start[i]), time),
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
        if matches!(
            // Classify from the query point. Reclassifying its rounded
            // surface projection can turn an edge into a face and replace
            // the edge metric with the smaller distance to the face plane.
            super::super::closest_triangle_feature(p, &face).1,
            super::super::ClosestFeature::Face
        ) {
            Some(n)
        } else {
            None
        }
    } else {
        None
    };
    let normal = if let Some(n) = oriented_face_normal {
        // Unsigned discovery may select a point just behind a closed face.
        // Its physical row must differentiate the signed outward branch,
        // including nonzero separation, just like endpoint mesh geometry.
        gap = dot(sub(p, q), n) - capsule.radius;
        n
    } else if distance <= 1e-12 {
        return Err(if closed {
            "trajectory edge crossing needs oriented feature activation"
        } else {
            "trajectory surface crossing needs oriented contact activation"
        });
    } else {
        mul(sub(p, q), 1. / distance)
    };
    // Anchor the endpoint linearization at the candidate itself. Expanding
    // q + n*r - (1-t)*old and dividing by t loses the tiny residual through
    // cancellation of world coordinates. The local residual form retains
    // the discovered gap without changing its normal or endpoint Jacobian.
    let endpoint = add(
        mul(capsule.end[0], 1. - fraction),
        mul(capsule.end[1], fraction),
    );
    let target = sub(endpoint, mul(normal, gap / time));
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
    #[test]
    fn captured_interior_penetration_generates_effective_endpoint_constraint() {
        // Binary64 geometry captured from the full groom at rod 9, segment 19.
        // Keep this regression self-contained so ordinary test runs exercise it.
        let values: [f64; 32] = [
            -0.10430728662467607,
            0.5338032657791152,
            -0.051681571706334216,
            -0.10314318741700262,
            0.5092151503238094,
            -0.06804549049180765,
            -0.1044070294389939,
            0.5338818581579452,
            -0.05190695914890549,
            -0.1032849720564701,
            0.5093398225337193,
            -0.06834283946726445,
            -0.097025066614151,
            0.5243496298789978,
            -0.0603339821100235,
            -0.10848993062973022,
            0.5205245018005371,
            -0.05877222865819931,
            -0.10236699134111404,
            0.5275492072105408,
            -0.05627758428454399,
            -0.09693187847733498,
            0.5242734253406525,
            -0.06077761296182871,
            -0.10839556157588959,
            0.5204505175352097,
            -0.059224497992545366,
            -0.10227613802999258,
            0.5274754464626312,
            -0.0567256142385304,
            4e-05,
            1e-10,
        ];
        let p = |i: usize| [values[i], values[i + 1], values[i + 2]];
        let capsule = CapsuleMotion {
            start: [p(0), p(3)],
            end: [p(6), p(9)],
            radius: values[30],
        };
        let triangle = TriangleMotion {
            start: [p(12), p(15), p(18)],
            end: [p(21), p(24), p(27)],
        };
        for closed in [false, true] {
            let witness = trajectory_contact_after_nonclear_sweep(
                capsule,
                triangle,
                CapsuleSweepOptions {
                    tolerance_m: values[31],
                    ..Default::default()
                },
                closed,
            )
            .unwrap();
            eprintln!(
                "CAPTURED INTERIOR ACTIVATION closed={closed} witness={witness:?} endpoint_gap={:?}",
                witness.map(|w| w.endpoint_gap(capsule.end))
            );
            let witness = witness
                .expect("exactly proven interior violation must produce a trajectory constraint");
            assert!(witness.time > 0. && witness.time < 1.);
            assert!(
                witness.endpoint_gap(capsule.end) < -values[31],
                "published endpoint row must retain the discovered physical violation"
            );
        }
    }
    #[test]
    fn moving_endpoint_witness_keeps_adjacent_segment_identity() {
        // Frozen rod355/segment17: a sub-ULP trial used to change the endpoint
        // fraction from 1 to 0.9999999999999998, double-counting its merit on
        // adjacent segments after mesh_motion canonicalized only exact ends.
        let start = [
            [
                0.06610006941656807,
                0.5629035967858032,
                -0.04080877476566326,
            ],
            [
                0.06721148923222399,
                0.5521830670396227,
                -0.04430061978701083,
            ],
        ];
        let triangle = TriangleMotion {
            start: [
                [
                    0.06893501058220863,
                    0.5549008548259735,
                    -0.040359847247600555,
                ],
                [
                    0.06924695521593094,
                    0.5477399528026581,
                    -0.04907388798892498,
                ],
                [
                    0.06404399126768112,
                    0.5535735785961151,
                    -0.04350353591144085,
                ],
            ],
            end: [
                [
                    0.06908527202904224,
                    0.5547468513250351,
                    -0.04111059941351414,
                ],
                [0.0694073811173439, 0.547576442360878, -0.049811783246695995],
                [
                    0.06419733911752701,
                    0.5534168034791946,
                    -0.04425608832389116,
                ],
            ],
        };
        for end in [
            [
                [0.0663130843200608, 0.5624728499108836, -0.04137759931645231],
                [
                    0.06753421349474432,
                    0.5518720774621811,
                    -0.04518424406802109,
                ],
            ],
            [
                [
                    0.06631308432006078,
                    0.5624728499108836,
                    -0.04137759931645229,
                ],
                [
                    0.06753421349474431,
                    0.5518720774621811,
                    -0.04518424406802117,
                ],
            ],
        ] {
            let capsule = CapsuleMotion {
                start,
                end,
                radius: 40e-6,
            };
            let witness = trajectory_contact_oriented(capsule, triangle, Default::default(), true)
                .unwrap()
                .unwrap();
            assert!(witness.time > 0.02 && witness.time < 0.03);
            assert_eq!(witness.fraction, 1., "endpoint changed segment identity");
            assert!((witness.endpoint_gap(end) - witness.gap).abs() < 1e-14);
        }
    }
    #[test]
    fn rounded_edge_projection_does_not_select_face_plane_metric() {
        let vertices = [
            [0.09425839595496655, 0.472492553293705, -0.07590783201158047],
            [
                0.09931213222444057,
                0.4624982252717018,
                -0.07562194764614105,
            ],
            [
                0.09137955494225025,
                0.4575122892856598,
                -0.07773331552743912,
            ],
        ];
        let end = [
            [0.0984715757252889, 0.4703294935312507, -0.07520405043636896],
            [
                0.09760007508564893,
                0.4541119440432464,
                -0.07672627344947909,
            ],
        ];
        let normal = unit(cross(
            sub(vertices[1], vertices[0]),
            sub(vertices[2], vertices[0]),
        ));
        let capsule = CapsuleMotion {
            start: end.map(|p| add(p, mul(normal, 0.001))),
            end,
            radius: 40e-6,
        };
        let triangle = TriangleMotion {
            start: vertices,
            end: vertices,
        };
        let witness = trajectory_contact_oriented(capsule, triangle, Default::default(), true)
            .unwrap()
            .unwrap();
        assert_eq!(witness.time, 1.);
        let p = add(
            mul(end[0], 1. - witness.fraction),
            mul(end[1], witness.fraction),
        );
        let face = Triangle {
            ids: [0, 1, 2],
            p: vertices,
            previous_p: vertices,
            velocity: [[0.; 3]; 3],
            normal,
            min: [0.; 3],
            max: [0.; 3],
        };
        let (q, feature) = super::super::super::closest_triangle_feature(p, &face);
        assert!(matches!(
            feature,
            super::super::super::ClosestFeature::Edge(0, 1)
        ));
        let expected = len(sub(p, q)) - capsule.radius;
        assert!(
            (witness.gap - expected).abs() < 1e-14,
            "edge witness used face-plane distance: {} vs {expected}",
            witness.gap
        );
        assert!((witness.endpoint_gap(end) - expected).abs() < 1e-14);
    }
    #[test]
    fn nonzero_backside_face_witness_uses_closed_outward_branch() {
        for reversed in [false, true] {
            let mut vertices = [[-1., -1., 0.], [1., -1., 0.], [0., 1., 0.]];
            if reversed {
                vertices.swap(1, 2);
            }
            let sign = if reversed { -1. } else { 1. };
            let triangle = TriangleMotion {
                start: vertices,
                end: vertices,
            };
            let capsule = CapsuleMotion {
                start: [[-0.1, 0., -0.1 * sign], [0.1, 0., -0.1 * sign]],
                end: [[-0.1, 0., -20e-6 * sign], [0.1, 0., -20e-6 * sign]],
                radius: 40e-6,
            };
            let unsigned = trajectory_contact(capsule, triangle, Default::default())
                .unwrap()
                .unwrap();
            let signed = trajectory_contact_oriented(capsule, triangle, Default::default(), true)
                .unwrap()
                .unwrap();
            assert_eq!(signed.time, 1.);
            assert_eq!(signed.normal, [0., 0., sign]);
            assert!(dot(unsigned.normal, [0., 0., -sign]) > 1. - 1e-12);
            assert!((signed.gap + 60e-6).abs() < 1e-15);
            assert!((signed.endpoint_gap(capsule.end) - signed.gap).abs() < 1e-15);
            let corrected = capsule.end.map(|p| add(p, mul(signed.normal, 60e-6)));
            assert!(signed.endpoint_gap(corrected).abs() < 1e-15);
        }
    }
    use crate::hair::{ContactSource, HairMaterial, HairRod};
    #[test]
    fn oblique_face_crossing_plateau_keeps_witness_under_ulp_motion() {
        let vertices = [[0.13, -0.27, 0.19], [1.17, 0.31, 0.41], [0.23, 1.29, 0.73]];
        let normal = unit(cross(
            sub(vertices[1], vertices[0]),
            sub(vertices[2], vertices[0]),
        ));
        let center = mul(add(add(vertices[0], vertices[1]), vertices[2]), 1. / 3.);
        let triangle = TriangleMotion {
            start: vertices,
            end: vertices,
        };
        let capsule = CapsuleMotion {
            start: [-0.0065, 0.0065].map(|offset| add(center, mul(normal, offset + 0.0105))),
            end: [-0.0065, 0.0065].map(|offset| add(center, mul(normal, offset - 0.0105))),
            radius: 40e-6,
        };
        let reference = trajectory_contact_oriented(capsule, triangle, Default::default(), true)
            .unwrap()
            .unwrap();
        assert_eq!(reference.gap, -capsule.radius);
        for shift in [-1e-15, 1e-15] {
            let mut moved = capsule;
            for point in &mut moved.end {
                point[0] += shift;
            }
            let contact = trajectory_contact_oriented(moved, triangle, Default::default(), true)
                .unwrap()
                .unwrap();
            assert_eq!(
                contact.time, reference.time,
                "roundoff selected another point of the crossing plateau"
            );
            assert!((contact.fraction - reference.fraction).abs() < 1e-12);
            assert!(len(sub(contact.normal, reference.normal)) < 1e-12);
            assert_eq!(contact.gap, -capsule.radius);
        }
    }
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
