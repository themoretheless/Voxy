//! Conservative advancement for linearly moving capsule centreline endpoints.
//! Query only: contact-safe integration and root ownership remain with HairSystem.
use super::segment_pair;
use crate::hair::math::*;

#[derive(Clone, Copy, Debug)]
pub struct CapsuleMotion {
    pub start: [V; 2],
    pub end: [V; 2],
    pub radius: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct CapsuleSweepOptions {
    pub tolerance_m: f64,
    pub max_iterations: usize,
}
impl Default for CapsuleSweepOptions {
    fn default() -> Self {
        Self {
            tolerance_m: 1e-10,
            max_iterations: 1024,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CapsuleSweep {
    Clear,
    /// Already in the contact band; the caller must reconcile the active set.
    InitialContact {
        gap_m: f64,
    },
    /// A conservative safe fraction, not an exact time-of-impact estimate.
    Approach {
        fraction: f64,
        gap_m: f64,
        iterations: usize,
    },
    /// No absence-of-collision claim: retain the last conservative fraction.
    IterationLimit {
        fraction: f64,
        gap_m: f64,
        iterations: usize,
    },
}

/// Conservative candidate pairs for linear endpoint motion. Bounds include
/// both endpoint states and the contact band; no spatial-grid overflow may
/// silently discard a segment. Anatomical/adjacency exclusions belong to the
/// caller, not this geometry query.
pub fn swept_capsule_pairs(
    motions: &[CapsuleMotion],
    tolerance_m: f64,
) -> Result<Vec<(usize, usize)>, &'static str> {
    if !tolerance_m.is_finite() || tolerance_m <= 0. {
        return Err("invalid swept capsule bounds tolerance");
    }
    let mut bounds = Vec::with_capacity(motions.len());
    for motion in motions {
        if !motion.radius.is_finite()
            || motion.radius <= 0.
            || motion
                .start
                .iter()
                .chain(&motion.end)
                .any(|point| !finite(*point))
        {
            return Err("invalid swept capsule bounds");
        }
        let scale = motion
            .start
            .iter()
            .chain(&motion.end)
            .flatten()
            .map(|value| value.abs())
            .fold(motion.radius, f64::max);
        let roundoff = 64. * f64::EPSILON * scale;
        if tolerance_m < roundoff {
            return Err(
                "swept capsule tolerance is below coordinate precision; recenter the query",
            );
        }
        let margin = motion.radius + tolerance_m + roundoff;
        let min: V = std::array::from_fn(|axis| {
            motion
                .start
                .iter()
                .chain(&motion.end)
                .map(|point| point[axis])
                .fold(f64::INFINITY, f64::min)
                - margin
        });
        let max: V = std::array::from_fn(|axis| {
            motion
                .start
                .iter()
                .chain(&motion.end)
                .map(|point| point[axis])
                .fold(f64::NEG_INFINITY, f64::max)
                + margin
        });
        if !finite(min) || !finite(max) {
            return Err("swept capsule bounds overflow");
        }
        bounds.push((min, max));
    }
    // Sweep along the greatest scene extent, then reject disjoint bounds on
    // all axes. Final pair sorting makes results independent of sweep order.
    let extent: V = std::array::from_fn(|axis| {
        bounds
            .iter()
            .map(|b| b.1[axis])
            .fold(f64::NEG_INFINITY, f64::max)
            - bounds
                .iter()
                .map(|b| b.0[axis])
                .fold(f64::INFINITY, f64::min)
    });
    let axis = (0..3)
        .max_by(|a, b| extent[*a].total_cmp(&extent[*b]))
        .unwrap();
    let mut order: Vec<_> = (0..motions.len()).collect();
    order.sort_unstable_by(|a, b| {
        bounds[*a].0[axis]
            .total_cmp(&bounds[*b].0[axis])
            .then(a.cmp(b))
    });
    let mut pairs = Vec::new();
    for (slot, &a) in order.iter().enumerate() {
        for &b in &order[slot + 1..] {
            if bounds[b].0[axis] > bounds[a].1[axis] {
                break;
            }
            if (0..3).all(|axis| {
                bounds[a].0[axis] <= bounds[b].1[axis] && bounds[b].0[axis] <= bounds[a].1[axis]
            }) {
                pairs.push((a.min(b), a.max(b)));
            }
        }
    }
    pairs.sort_unstable();
    Ok(pairs)
}

/// Run continuous queries only for swept candidates, retaining every outcome
/// that fails to prove separation, including initial contacts and limits.
pub fn swept_capsule_contacts(
    motions: &[CapsuleMotion],
    options: CapsuleSweepOptions,
) -> Result<Vec<((usize, usize), CapsuleSweep)>, &'static str> {
    if options.max_iterations == 0 {
        return Err("invalid capsule sweep iteration budget");
    }
    let pairs = swept_capsule_pairs(motions, options.tolerance_m)?;
    let mut contacts = Vec::new();
    for (a, b) in pairs {
        let outcome = sweep_capsules(motions[a], motions[b], options)?;
        if outcome != CapsuleSweep::Clear {
            contacts.push(((a, b), outcome));
        }
    }
    Ok(contacts)
}

fn gap(a: &CapsuleMotion, b: &CapsuleMotion, t: f64) -> Result<f64, &'static str> {
    let endpoints = |motion: &CapsuleMotion| {
        std::array::from_fn::<_, 2, _>(|i| {
            add(motion.start[i], mul(sub(motion.end[i], motion.start[i]), t))
        })
    };
    let aa = endpoints(a);
    let bb = endpoints(b);
    let (_, _, p, q) = segment_pair(aa[0], aa[1], bb[0], bb[1]);
    let value = len(sub(p, q)) - a.radius - b.radius;
    if value.is_finite() {
        Ok(value)
    } else {
        Err("capsule sweep distance overflow")
    }
}

pub fn sweep_capsules(
    a: CapsuleMotion,
    b: CapsuleMotion,
    options: CapsuleSweepOptions,
) -> Result<CapsuleSweep, &'static str> {
    if !options.tolerance_m.is_finite()
        || options.tolerance_m <= 0.
        || options.max_iterations == 0
        || [&a, &b].iter().any(|motion| {
            !motion.radius.is_finite()
                || motion.radius <= 0.
                || motion
                    .start
                    .iter()
                    .chain(&motion.end)
                    .any(|point| !finite(*point))
        })
    {
        return Err("invalid capsule sweep");
    }
    let coordinate_scale = [&a, &b]
        .iter()
        .flat_map(|motion| motion.start.iter().chain(&motion.end))
        .flatten()
        .map(|value| value.abs())
        .fold(a.radius + b.radius, f64::max);
    let roundoff = 64. * f64::EPSILON * coordinate_scale;
    if !coordinate_scale.is_finite() || options.tolerance_m < roundoff {
        return Err("capsule sweep tolerance is below coordinate precision; recenter the query");
    }
    let velocity = |motion: &CapsuleMotion| {
        std::array::from_fn::<_, 2, _>(|i| sub(motion.end[i], motion.start[i]))
    };
    let va = velocity(&a);
    let vb = velocity(&b);
    // Relative endpoint velocities bound every convex barycentric relative
    // velocity, and hence the change in minimum centreline distance. Common
    // rigid translation cancels instead of unnecessarily reducing the step.
    let speed = va
        .iter()
        .flat_map(|va| vb.iter().map(move |vb| len(sub(*va, *vb))))
        .fold(0., f64::max)
        * (1. + 16. * f64::EPSILON);
    if !speed.is_finite() {
        return Err("capsule sweep motion overflow");
    }
    let mut fraction = 0.;
    let mut current = gap(&a, &b, fraction)?;
    if current <= options.tolerance_m {
        return Ok(CapsuleSweep::InitialContact { gap_m: current });
    }
    if speed == 0. {
        return Ok(CapsuleSweep::Clear);
    }
    for iteration in 0..options.max_iterations {
        if current <= 2. * options.tolerance_m {
            return Ok(CapsuleSweep::Approach {
                fraction,
                gap_m: current,
                iterations: iteration,
            });
        }
        let clearance = (current - options.tolerance_m - roundoff).max(0.);
        let remaining = 1. - fraction;
        if clearance > speed * remaining {
            return Ok(CapsuleSweep::Clear);
        }
        // Strict safety margin keeps advancement inside the Lipschitz bound.
        let next = (fraction + 0.9 * clearance / speed).min(1.);
        if next <= fraction {
            return Ok(CapsuleSweep::IterationLimit {
                fraction,
                gap_m: current,
                iterations: iteration,
            });
        }
        fraction = next;
        current = gap(&a, &b, fraction)?;
        if current < 0. {
            return Err("capsule sweep lost conservative clearance");
        }
    }
    Ok(CapsuleSweep::IterationLimit {
        fraction,
        gap_m: current,
        iterations: options.max_iterations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn vertical(x: f64) -> [V; 2] {
        [[x, 0., 0.], [x, 1., 0.]]
    }
    #[test]
    fn swept_bounds_find_crossing_with_clear_endpoint_states() {
        let a = CapsuleMotion {
            start: [[-1., -0.2, 0.], [-1., 0.2, 0.]],
            end: [[1., -0.2, 0.], [1., 0.2, 0.]],
            radius: 0.01,
        };
        let b = CapsuleMotion {
            start: [[0., 0., -0.2], [0., 0., 0.2]],
            end: [[0., 0., -0.2], [0., 0., 0.2]],
            radius: 0.01,
        };
        assert!(gap(&a, &b, 0.).unwrap() > 0.);
        assert!(gap(&a, &b, 1.).unwrap() > 0.);
        let pairs = swept_capsule_pairs(&[a, b], 1e-10).unwrap();
        assert_eq!(pairs, vec![(0, 1)]);
        assert!(matches!(
            sweep_capsules(a, b, CapsuleSweepOptions::default()).unwrap(),
            CapsuleSweep::Approach { .. }
        ));
    }

    #[test]
    fn swept_candidates_cover_brute_force_continuous_queries() {
        let mut seed = 19u64;
        let mut value = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            (seed >> 11) as f64 / (1u64 << 53) as f64 * 0.1 - 0.05
        };
        let motions: Vec<_> = (0..64)
            .map(|_| CapsuleMotion {
                start: std::array::from_fn(|_| std::array::from_fn(|_| value())),
                end: std::array::from_fn(|_| std::array::from_fn(|_| value())),
                radius: 0.001,
            })
            .collect();
        let pairs = swept_capsule_pairs(&motions, 1e-10).unwrap();
        assert!(pairs.windows(2).all(|p| p[0] < p[1]));
        let mut nonclear = 0;
        for a in 0..motions.len() {
            for b in a + 1..motions.len() {
                let query =
                    sweep_capsules(motions[a], motions[b], CapsuleSweepOptions::default()).unwrap();
                if query != CapsuleSweep::Clear {
                    nonclear += 1;
                    assert!(
                        pairs.binary_search(&(a, b)).is_ok(),
                        "missing swept pair {a}/{b}: {query:?}"
                    );
                }
            }
        }
        assert!(nonclear > 0);
        // Independent time samples also check pairs whose conservative query
        // terminates at its iteration budget.
        for a in 0..motions.len() {
            for b in a + 1..motions.len() {
                for step in 0..=16 {
                    if gap(&motions[a], &motions[b], step as f64 / 16.).unwrap() <= 1e-10 {
                        assert!(pairs.binary_search(&(a, b)).is_ok());
                    }
                }
            }
        }
    }

    #[test]
    fn swept_bounds_never_silently_drop_large_motion_or_invalid_input() {
        let a = CapsuleMotion {
            start: [[-100., 0., 0.]; 2],
            end: [[100., 0., 0.]; 2],
            radius: 0.01,
        };
        let b = CapsuleMotion {
            start: [[0., 0., 0.]; 2],
            end: [[0., 0., 0.]; 2],
            radius: 0.01,
        };
        assert_eq!(swept_capsule_pairs(&[a, b], 1e-10).unwrap(), vec![(0, 1)]);
        assert_eq!(swept_capsule_pairs(&[], 1e-10).unwrap(), vec![]);
        assert!(swept_capsule_pairs(&[], 0.).is_err());
        let mut invalid = a;
        invalid.end[0][0] = f64::NAN;
        assert!(swept_capsule_pairs(&[invalid], 1e-10).is_err());
        let mut imprecise = a;
        imprecise.start = [[1e12, 0., 0.]; 2];
        imprecise.end = imprecise.start;
        assert!(swept_capsule_pairs(&[imprecise], 1e-10).is_err());
    }

    fn approaching(a: CapsuleMotion, b: CapsuleMotion, expected: f64) {
        let result = sweep_capsules(a, b, Default::default()).unwrap();
        let CapsuleSweep::Approach {
            fraction, gap_m, ..
        } = result
        else {
            panic!("unexpected sweep result {result:?}");
        };
        assert!(fraction <= expected && expected - fraction < 1e-9);
        assert!(gap_m >= 0. && gap_m <= 2e-10);
    }
    #[test]
    fn detects_crossing_between_two_separated_end_states() {
        approaching(
            CapsuleMotion {
                start: vertical(-1.),
                end: vertical(1.),
                radius: 0.1,
            },
            CapsuleMotion {
                start: vertical(1.),
                end: vertical(-1.),
                radius: 0.1,
            },
            0.45,
        );
    }
    #[test]
    fn common_translation_and_separating_motion_remain_clear() {
        let a = CapsuleMotion {
            start: vertical(0.),
            end: vertical(2.),
            radius: 0.1,
        };
        let b = CapsuleMotion {
            start: vertical(1.),
            end: vertical(3.),
            radius: 0.1,
        };
        assert_eq!(
            sweep_capsules(a, b, Default::default()).unwrap(),
            CapsuleSweep::Clear
        );
        let a = CapsuleMotion {
            start: vertical(0.),
            end: vertical(-1.),
            radius: 0.1,
        };
        let b = CapsuleMotion {
            start: vertical(1.),
            end: vertical(2.),
            radius: 0.1,
        };
        assert_eq!(
            sweep_capsules(a, b, Default::default()).unwrap(),
            CapsuleSweep::Clear
        );
    }
    #[test]
    fn stationary_sphere_against_a_moving_segment() {
        let a = CapsuleMotion {
            start: [[0.; 3]; 2],
            end: [[0.; 3]; 2],
            radius: 0.1,
        };
        let b = CapsuleMotion {
            start: vertical(-1.),
            end: vertical(1.),
            radius: 0.1,
        };
        approaching(a, b, 0.4);
    }
    #[test]
    fn initial_contact_and_iteration_limit_are_not_reported_clear() {
        let a = CapsuleMotion {
            start: vertical(0.),
            end: vertical(0.),
            radius: 0.1,
        };
        let b = CapsuleMotion {
            start: vertical(0.1),
            end: vertical(1.),
            radius: 0.1,
        };
        assert!(
            matches!(sweep_capsules(a,b,Default::default()).unwrap(),CapsuleSweep::InitialContact {gap_m} if gap_m<0.)
        );
        let a = CapsuleMotion {
            start: vertical(-1.),
            end: vertical(1.),
            radius: 0.1,
        };
        let b = CapsuleMotion {
            start: vertical(1.),
            end: vertical(-1.),
            radius: 0.1,
        };
        assert!(
            matches!(sweep_capsules(a,b,CapsuleSweepOptions {max_iterations:1,..Default::default()}).unwrap(),CapsuleSweep::IterationLimit {fraction,..} if fraction<0.45)
        );
    }
    #[test]
    fn rejects_invalid_or_unresolvable_precision_requests() {
        let a = CapsuleMotion {
            start: vertical(0.),
            end: vertical(0.),
            radius: 0.1,
        };
        let mut b = a;
        b.radius = f64::NAN;
        assert!(sweep_capsules(a, b, Default::default()).is_err());
        assert!(
            sweep_capsules(
                a,
                a,
                CapsuleSweepOptions {
                    tolerance_m: 0.,
                    ..Default::default()
                }
            )
            .is_err()
        );
        let a = CapsuleMotion {
            start: vertical(1e9),
            end: vertical(1e9 + 1.),
            radius: 0.1,
        };
        assert!(sweep_capsules(a, a, Default::default()).is_err());
    }
    #[test]
    fn captured_structural_transition_is_limited_before_deep_overlap() {
        let cases = [
            (
                [
                    [
                        -0.03130735138549047,
                        0.5808372517149203,
                        -0.06271887781146256,
                    ],
                    [
                        -0.03021624882492256,
                        0.5650200779489079,
                        -0.06352827961383717,
                    ],
                    [
                        -0.03144986940101922,
                        0.5800976615039692,
                        -0.06270301749142872,
                    ],
                    [
                        -0.03024322094596822,
                        0.5647407714073533,
                        -0.06360855728977184,
                    ],
                ],
                [
                    [
                        -0.03146610318783463,
                        0.5790630302583674,
                        -0.06260679420661501,
                    ],
                    [
                        -0.03036507714302969,
                        0.5632492527038871,
                        -0.06346785790049413,
                    ],
                    [
                        -0.03158162875651323,
                        0.5783205126265673,
                        -0.06254987032162926,
                    ],
                    [
                        -0.030344212446803135,
                        0.5629677555669672,
                        -0.06348353996343947,
                    ],
                ],
            ),
            (
                [
                    [
                        -0.03021624882492256,
                        0.5650200779489079,
                        -0.06352827961383717,
                    ],
                    [
                        -0.02921132883070826,
                        0.5491888757727258,
                        -0.06415755555233814,
                    ],
                    [
                        -0.03144986940101922,
                        0.5800976615039692,
                        -0.06270301749142872,
                    ],
                    [
                        -0.03024322094596822,
                        0.5647407714073533,
                        -0.06360855728977184,
                    ],
                ],
                [
                    [
                        -0.03036507714302969,
                        0.5632492527038871,
                        -0.06346785790049413,
                    ],
                    [
                        -0.029346065849085376,
                        0.5474212370537173,
                        -0.06415002614156959,
                    ],
                    [
                        -0.03158162875651323,
                        0.5783205126265673,
                        -0.06254987032162926,
                    ],
                    [
                        -0.030344212446803135,
                        0.5629677555669672,
                        -0.06348353996343947,
                    ],
                ],
            ),
            (
                [
                    [
                        -0.03021624882492256,
                        0.5650200779489079,
                        -0.06352827961383717,
                    ],
                    [
                        -0.02921132883070826,
                        0.5491888757727258,
                        -0.06415755555233814,
                    ],
                    [
                        -0.03024322094596822,
                        0.5647407714073533,
                        -0.06360855728977184,
                    ],
                    [
                        -0.029119417575689272,
                        0.5493664265271927,
                        -0.06429729325158404,
                    ],
                ],
                [
                    [
                        -0.03036507714302969,
                        0.5632492527038871,
                        -0.06346785790049413,
                    ],
                    [
                        -0.029346065849085376,
                        0.5474212370537173,
                        -0.06415002614156959,
                    ],
                    [
                        -0.030344212446803135,
                        0.5629677555669672,
                        -0.06348353996343947,
                    ],
                    [
                        -0.029188207767564416,
                        0.5475973936346811,
                        -0.06420614833809467,
                    ],
                ],
            ),
        ];
        for (index, (old, new)) in cases.into_iter().enumerate() {
            let a = CapsuleMotion {
                start: [old[0], old[1]],
                end: [new[0], new[1]],
                radius: 40e-6,
            };
            let b = CapsuleMotion {
                start: [old[2], old[3]],
                end: [new[2], new[3]],
                radius: 40e-6,
            };
            assert!(gap(&a, &b, 0.).unwrap() > 0.);
            assert!(gap(&a, &b, 1.).unwrap() < -75e-6);
            assert_eq!(swept_capsule_pairs(&[a, b], 1e-10).unwrap(), vec![(0, 1)]);
            let events = swept_capsule_contacts(&[a, b], Default::default()).unwrap();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].0, (0, 1));
            let result = sweep_capsules(a, b, Default::default()).unwrap();
            assert_eq!(events[0].1, result);
            let CapsuleSweep::Approach {
                fraction,
                gap_m,
                iterations,
            } = result
            else {
                panic!("captured sweep {index}: {result:?}");
            };
            assert!(fraction > 0. && fraction < 1. && gap_m >= 0.);
            for sample in 0..=64 {
                assert!(gap(&a, &b, fraction * sample as f64 / 64.).unwrap() >= -1e-10);
            }
            eprintln!(
                "CAPTURED CAPSULE SWEEP case={index} safe_fraction={fraction} gap_m={gap_m:e} iterations={iterations}"
            );
        }
    }
}
