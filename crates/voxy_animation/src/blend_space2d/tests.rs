use super::*;
use crate::{BlendSample, BlendSpace1D, Joint, JointTrack, Playback, Transform, Vec3Key};
use glam::{Mat4, Vec3};

fn rig() -> Skeleton {
    Skeleton::new(vec![
        Joint {
            name: Arc::from("root"),
            parent: None,
            bind_local: Transform::IDENTITY,
            inverse_bind: Mat4::IDENTITY,
        },
        Joint {
            name: Arc::from("child"),
            parent: Some(0),
            bind_local: Transform::IDENTITY,
            inverse_bind: Mat4::IDENTITY,
        },
    ])
    .unwrap()
}

fn clip(rig: &Skeleton, duration: f32, value: Vec3) -> Arc<AnimationClip> {
    let mut tracks = vec![JointTrack::default(); 2];
    tracks[1].translations = vec![
        Vec3Key {
            time: 0.0,
            value: Vec3::ZERO,
        },
        Vec3Key {
            time: duration,
            value,
        },
    ];
    Arc::new(AnimationClip::new("c", duration, Playback::Loop, tracks, rig).unwrap())
}

fn sample(rig: &Skeleton, x: f32, y: f32, duration: f32) -> BlendSample2D {
    BlendSample2D {
        parameter: Vec2::new(x, y),
        clip: clip(rig, duration, Vec3::new(x, y, 0.0)),
    }
}

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[test]
fn delaunay_has_empty_circumcircles_and_covers_hull() {
    for seed in 1..=40u64 {
        let mut rng = Lcg(seed);
        let count = 3 + (seed as usize % 12);
        let points: Vec<DVec2> = (0..count)
            .map(|_| DVec2::new(rng.next() * 10.0, rng.next() * 10.0))
            .collect();
        let tris = delaunay(&points);
        assert!(!tris.is_empty(), "seed {seed}");
        let mut area = 0.0;
        for &[a, b, c] in &tris {
            assert!(
                cross(points[a], points[b], points[c]) > 0.0,
                "ccw, seed {seed}"
            );
            area += cross(points[a], points[b], points[c]) * 0.5;
            for (i, &p) in points.iter().enumerate() {
                if i != a && i != b && i != c {
                    assert!(
                        !circumcircle_contains(points[a], points[b], points[c], p),
                        "seed {seed}: point {i} inside circumcircle of {a},{b},{c}"
                    );
                }
            }
        }
        let hull = convex_hull(&points);
        let hull_area: f64 = (0..hull.len())
            .map(|k| points[hull[k]].perp_dot(points[hull[(k + 1) % hull.len()]]))
            .sum::<f64>()
            * 0.5;
        assert!(
            (area - hull_area).abs() < 1e-9 * hull_area.max(1.0),
            "seed {seed}"
        );
    }
}

#[test]
fn weights_are_barycentric_partition_of_unity_and_exact_at_samples() {
    let rig = rig();
    let space = BlendSpace2D::new(
        vec![
            sample(&rig, 0.0, 0.0, 1.0),
            sample(&rig, 2.0, 0.0, 0.5),
            sample(&rig, 0.0, 2.0, 0.8),
            sample(&rig, 2.0, 2.0, 0.4),
        ],
        &rig,
    )
    .unwrap();
    assert_eq!(space.triangles().len(), 2);
    assert_eq!(space.hull().len(), 4);
    for (i, s) in space.samples().iter().enumerate() {
        let w = space.weights(s.parameter).unwrap();
        let own: f32 = w
            .indices
            .iter()
            .zip(w.weights)
            .filter(|(j, _)| **j == i)
            .map(|(_, w)| w)
            .sum();
        assert!((own - 1.0).abs() < 1e-6, "sample {i}: {w:?}");
    }
    let mut rng = Lcg(7);
    for _ in 0..200 {
        let p = Vec2::new(rng.next() as f32 * 2.0, rng.next() as f32 * 2.0);
        let w = space.weights(p).unwrap();
        let sum: f32 = w.weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5);
        assert!(w.weights.iter().all(|&x| x >= 0.0));
        let reconstructed: Vec2 = w
            .indices
            .iter()
            .zip(w.weights)
            .map(|(&i, w)| space.samples()[i].parameter * w)
            .sum();
        assert!((reconstructed - p).length() < 1e-4);
    }
    assert!(space.weights(Vec2::NAN).is_err());
}

#[test]
fn outside_points_clamp_to_nearest_hull_edge() {
    let rig = rig();
    let space = BlendSpace2D::new(
        vec![
            sample(&rig, 0.0, 0.0, 1.0),
            sample(&rig, 2.0, 0.0, 1.0),
            sample(&rig, 0.0, 2.0, 1.0),
        ],
        &rig,
    )
    .unwrap();
    let w = space.weights(Vec2::new(1.0, -5.0)).unwrap();
    assert_eq!(w.weights[2], 0.0);
    let projected: Vec2 = w
        .indices
        .iter()
        .zip(w.weights)
        .map(|(&i, w)| space.samples()[i].parameter * w)
        .sum();
    assert!((projected - Vec2::new(1.0, 0.0)).length() < 1e-6);
    let far = space.weights(Vec2::new(-9.0, -9.0)).unwrap();
    assert!((far.weights[0] - 1.0).abs() < 1e-6 || (far.weights[1] - 1.0).abs() < 1e-6);
}

#[test]
fn collinear_y_samples_reduce_to_blend_space_1d() {
    let rig = rig();
    let a = clip(&rig, 1.0, Vec3::X);
    let b = clip(&rig, 0.5, Vec3::X * 3.0);
    let c = clip(&rig, 0.25, Vec3::X * 9.0);
    let one = BlendSpace1D::new(
        vec![
            BlendSample {
                parameter: 1.0,
                clip: a.clone(),
            },
            BlendSample {
                parameter: 3.0,
                clip: b.clone(),
            },
            BlendSample {
                parameter: 9.0,
                clip: c.clone(),
            },
        ],
        &rig,
    )
    .unwrap();
    // A degenerate third row keeps the hull two-dimensional while the query
    // line y=0 stays on an edge of every triangle.
    let two = BlendSpace2D::new(
        vec![
            BlendSample2D {
                parameter: Vec2::new(1.0, 0.0),
                clip: a.clone(),
            },
            BlendSample2D {
                parameter: Vec2::new(3.0, 0.0),
                clip: b.clone(),
            },
            BlendSample2D {
                parameter: Vec2::new(9.0, 0.0),
                clip: c.clone(),
            },
            BlendSample2D {
                parameter: Vec2::new(5.0, 100.0),
                clip: a,
            },
        ],
        &rig,
    )
    .unwrap();
    for x in [1.0f32, 1.5, 2.9, 3.0, 6.0, 8.99, 9.0] {
        let phase = 0.37;
        let p1 = one.evaluate(&rig, x, phase).unwrap();
        let p2 = two.evaluate(&rig, Vec2::new(x, 0.0), phase).unwrap();
        let (t1, t2) = (p1.local()[1].translation, p2.local()[1].translation);
        assert!((t1 - t2).length() < 1e-4, "x={x}: {t1:?} vs {t2:?}");
        let (d1, d2) = (
            one.cycle_duration(x).unwrap(),
            two.cycle_duration(Vec2::new(x, 0.0)).unwrap(),
        );
        assert!((d1 - d2).abs() < 1e-5, "x={x}: {d1} vs {d2}");
    }
}

#[test]
fn rejects_degenerate_sets() {
    let rig = rig();
    assert!(
        BlendSpace2D::new(
            vec![sample(&rig, 0.0, 0.0, 1.0), sample(&rig, 1.0, 0.0, 1.0)],
            &rig
        )
        .is_err()
    );
    assert!(
        BlendSpace2D::new(
            vec![
                sample(&rig, 0.0, 0.0, 1.0),
                sample(&rig, 1.0, 1.0, 1.0),
                sample(&rig, 2.0, 2.0, 1.0)
            ],
            &rig
        )
        .is_err()
    );
    assert!(
        BlendSpace2D::new(
            vec![
                sample(&rig, 0.0, 0.0, 1.0),
                sample(&rig, 0.0, 0.0, 1.0),
                sample(&rig, 2.0, 1.0, 1.0)
            ],
            &rig
        )
        .is_err()
    );
}

#[test]
fn player_keeps_phase_across_parameter_change() {
    let rig = rig();
    let space = Arc::new(
        BlendSpace2D::new(
            vec![
                sample(&rig, 0.0, 0.0, 1.0),
                sample(&rig, 2.0, 0.0, 0.5),
                sample(&rig, 0.0, 2.0, 0.5),
            ],
            &rig,
        )
        .unwrap(),
    );
    let mut player = BlendSpace2DPlayer::new(space, Vec2::ZERO);
    player.advance(Vec2::ZERO, 0.25).unwrap();
    assert!((player.phase() - 0.25).abs() < 1e-9);
    player.advance(Vec2::new(2.0, 0.0), 0.25).unwrap();
    assert!((player.phase() - 0.75).abs() < 1e-9);
    assert!(player.advance(Vec2::ZERO, -0.1).is_err());
    player.pose(&rig).unwrap();
}
