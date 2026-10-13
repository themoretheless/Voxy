//! Affine cuts from witnessed intersections of a proposed joint root/free path.
//! Samples find cuts only; the complete continuous guard still admits motion.
use super::*;

fn point(points: &[V], segment: usize, fraction: f64) -> V {
    add(
        mul(points[segment], 1. - fraction),
        mul(points[segment + 1], fraction),
    )
}
fn add_cut(
    constraints: &mut Vec<Constraint>,
    rods: &[HairRod],
    start: &[HairRod],
    pair: &StrandResponse,
    time: f64,
    radius: f64,
    scales: [f64; 2],
) -> Result<(), &'static str> {
    if !time.is_finite() || time <= 0. || time > 1. {
        return Err("invalid swept contact time");
    }
    if scales.iter().any(|s| !s.is_finite() || *s <= 0. || *s > 1.) {
        return Err("invalid swept witness scale");
    }
    let (a, i, s) = pair.a;
    let (b, j, t) = pair.b;
    let old = dot(
        sub(point(&start[a].x, i, s), point(&start[b].x, j, t)),
        pair.normal,
    );
    let staged = dot(
        sub(point(&rods[a].x, i, s), point(&rods[b].x, j, t)),
        pair.normal,
    );
    // At tau: (1-tau)*old + tau*(staged + J*scaled_free) >= diameter.
    // Staged includes the exact prescribed-root endpoint contribution;
    // pinned DOFs are removed only from J, never from this affine bound.
    // Keep the physical space-time row, including its time Jacobian.
    // Dividing by a very early witness time changes the force multiplier and
    // turns a physical residual into an endpoint residual with another unit
    // scale. FMA avoids cancellation in the affine prescribed-root term.
    let bound = time.mul_add(old - staged, 2. * radius - old);
    let ea = entries(a, i, s, mul(pair.normal, time * scales[0]), &rods[a]);
    let eb = entries(b, j, t, mul(pair.normal, -time * scales[1]), &rods[b]);
    add_constraint(constraints, [ea[0], ea[1], eb[0], eb[1]], bound)?
        .ok_or("swept contact has no movable endpoint")?;
    Ok(())
}

pub(super) struct Cut {
    pair: StrandResponse,
    time: f64,
    scales: [f64; 2],
}

pub(super) fn witnessed_merit(
    rods: &[HairRod],
    start: &[HairRod],
    cuts: &[Cut],
    radius: f64,
    signed_plane: bool,
) -> f64 {
    cuts.iter()
        .map(|cut| {
            let (a, i, s) = cut.pair.a;
            let (b, j, t) = cut.pair.b;
            let at = |r: usize, segment: usize, fraction: f64| {
                let old = point(&start[r].x, segment, fraction);
                add(
                    old,
                    mul(sub(point(&rods[r].x, segment, fraction), old), cut.time),
                )
            };
            let delta = sub(at(a, i, s), at(b, j, t));
            let distance = if signed_plane {
                dot(delta, cut.pair.normal)
            } else {
                len(delta)
            };
            (2. * radius - distance - 1e-10).max(0.).powi(2)
        })
        .sum()
}

#[cfg(test)]
pub(super) fn trace_witness_penetrations(
    rods: &[HairRod],
    start: &[HairRod],
    cuts: &[Cut],
    radius: f64,
) {
    for cut in cuts {
        let (a, i, s) = cut.pair.a;
        let (b, j, t) = cut.pair.b;
        let old_delta = sub(point(&start[a].x, i, s), point(&start[b].x, j, t));
        let end_delta = sub(point(&rods[a].x, i, s), point(&rods[b].x, j, t));
        let at = |r: usize, segment: usize, fraction: f64| {
            let old = point(&start[r].x, segment, fraction);
            add(
                old,
                mul(sub(point(&rods[r].x, segment, fraction), old), cut.time),
            )
        };
        let delta = sub(at(a, i, s), at(b, j, t));
        let distance = len(delta);
        let excess = (2. * radius - distance - 1e-10).max(0.);
        if excess == 0. {
            continue;
        }
        let cosine = if distance > 0. {
            Some(dot(delta, cut.pair.normal) / distance)
        } else {
            None
        };
        let cosine = cosine
            .filter(|v| v.is_finite())
            .map_or_else(|| "null".to_owned(), |v| v.to_string());
        eprintln!(
            "RESTORATION WITNESS {{\"a\":[{a},{i},{s}],\"b\":[{b},{j},{t}],\"time\":{},\"distance_m\":{distance},\"excess_m\":{excess},\"merit\":{},\"old_gap_m\":{},\"end_gap_m\":{},\"normal\":{:?},\"normal_cosine\":{cosine}}}",
            cut.time,
            excess * excess,
            len(old_delta) - 2. * radius,
            len(end_delta) - 2. * radius,
            cut.pair.normal
        );
    }
}

#[cfg(test)]
pub(super) fn witnessed_orientation(
    rods: &[HairRod],
    start: &[HairRod],
    cuts: &[Cut],
) -> (usize, f64) {
    let mut opposite = 0;
    let mut minimum = 1f64;
    for cut in cuts {
        let at = |r: usize, i, fraction| {
            let old = point(&start[r].x, i, fraction);
            add(old, mul(sub(point(&rods[r].x, i, fraction), old), cut.time))
        };
        let (a, i, s) = cut.pair.a;
        let (b, j, t) = cut.pair.b;
        let delta = sub(at(a, i, s), at(b, j, t));
        let distance = len(delta);
        if distance > 1e-12 {
            let cosine = dot(mul(delta, 1. / distance), cut.pair.normal);
            minimum = minimum.min(cosine);
            opposite += usize::from(cosine < 0.);
        }
    }
    (opposite, minimum)
}

#[test]
fn witnessed_merit_detects_interior_crossing_with_clear_endpoints() {
    let a = HairRod::new(
        vec![[-0.001, -0.001, 0.], [0., -0.001, 0.], [0.001, -0.001, 0.]],
        Default::default(),
    )
    .unwrap();
    let b = HairRod::new(
        vec![[0., 0., -0.001], [0., 0., 0.], [0., 0., 0.001]],
        Default::default(),
    )
    .unwrap();
    let start = vec![a, b];
    let mut end = start.clone();
    for x in &mut end[0].x {
        x[1] += 0.002;
    }
    let mut cuts = vec![Cut {
        pair: StrandResponse {
            a: (0, 1, 0.),
            b: (1, 1, 0.),
            normal: [1., 0., 0.],
            impulse: 0.,
        },
        time: 0.5,
        scales: [1., 1.],
    }];
    assert!(witnessed_merit(&end, &start, &cuts, 1e-4, false) > 0.);
    cuts[0].time = 0.;
    assert_eq!(witnessed_merit(&end, &start, &cuts, 1e-4, false), 0.);
    cuts[0].time = 1.;
    assert_eq!(witnessed_merit(&end, &start, &cuts, 1e-4, false), 0.);
}

// Different barycentric witnesses of one segment pair constrain different
// affine points. Replacing by segment identity alone can reopen an earlier
// collision. Retain witnesses for this Newton admission transaction only.
fn retain_cut(cuts: &mut Vec<Cut>, pair: StrandResponse, time: f64, scales: [f64; 2]) -> bool {
    if let Some(old) = cuts.iter_mut().find(|old| {
        old.time == time
            && old.pair.a == pair.a
            && old.pair.b == pair.b
            && old.pair.normal == pair.normal
    }) {
        let changed = old.scales != scales;
        old.scales = scales;
        return changed;
    }
    cuts.push(Cut { pair, time, scales });
    true
}

pub(super) fn activate(
    constraints: &mut Vec<Constraint>,
    rods: &[HairRod],
    start: &[HairRod],
    increment: &PositionIncrement,
    groups: &mut Vec<StrandResponse>,
    base: &[StrandResponse],
    cuts: &mut Vec<Cut>,
    radius: f64,
) -> Result<bool, &'static str> {
    activate_with_workers(
        constraints,
        rods,
        start,
        increment,
        groups,
        base,
        cuts,
        radius,
        None,
    )
}

pub(super) fn activate_with_workers(
    constraints: &mut Vec<Constraint>,
    rods: &[HairRod],
    start: &[HairRod],
    increment: &PositionIncrement,
    groups: &mut Vec<StrandResponse>,
    base: &[StrandResponse],
    cuts: &mut Vec<Cut>,
    radius: f64,
    workers: Option<usize>,
) -> Result<bool, &'static str> {
    let admitted = origin_path_admitted(rods, start, radius);
    activate_from_origin(
        constraints,
        rods,
        start,
        increment,
        groups,
        base,
        cuts,
        radius,
        workers,
        admitted,
    )
}

pub(super) fn origin_path_admitted(rods: &[HairRod], start: &[HairRod], radius: f64) -> bool {
    // Origin-preserving elastic linearization belongs to the feasible phase.
    // A safe strand path alone does not certify the body's contact rows. Keep
    // first-approach restoration while any original physical row is violated.
    for rod in rods {
        for contact in &rod.contacts {
            let p = point(&rod.x, contact.segment, contact.fraction);
            let gap = contact.physical_gap(p);
            if !gap.is_finite() || gap < -1e-10 {
                return false;
            }
        }
    }
    let end: Vec<_> = rods.iter().map(|rod| rod.x.clone()).collect();
    super::strand_fraction(start, &end, radius).is_ok_and(|fraction| fraction == 1.)
}

fn origin_normal(
    rods: &[HairRod],
    start: &[HairRod],
    pair: &StrandResponse,
    time: f64,
    radius: f64,
    admitted: bool,
    fallback: V,
) -> V {
    if !admitted {
        return fallback;
    }
    let (a, i, u) = pair.a;
    let (b, j, v) = pair.b;
    let old = sub(point(&start[a].x, i, u), point(&start[b].x, j, v));
    let current = sub(point(&rods[a].x, i, u), point(&rods[b].x, j, v));
    let separation = add(old, mul(sub(current, old), time));
    // Retain an existing supporting plane when it already admits the frozen
    // origin. Changing such a row rotates a valid local feasible region and
    // discards its first-approach information without repairing an exclusion.
    if dot(separation, fallback) >= 2. * radius {
        return fallback;
    }
    let distance = len(separation);
    // This affine feature belongs to the frozen safe trajectory. Its radial
    // supporting plane admits zero correction; a normal at a different time
    // or feature can wrongly exclude that same certified trajectory.
    if distance < 2. * radius {
        return fallback;
    }
    let radial = mul(separation, 1. / distance);
    let tangent = sub(fallback, mul(radial, dot(fallback, radial)));
    let tangent_length = len(tangent);
    if tangent_length == 0. {
        return radial;
    }
    // Project the first-approach normal onto the spherical cap that supports
    // this origin feature. This is the smallest angular change admitting zero
    // correction, rather than replacing the entire approach direction.
    let cosine = (2. * radius / distance + 32. * f64::EPSILON).min(1.);
    let sine = (1. - cosine * cosine).max(0.).sqrt();
    let normal = unit(add(
        mul(radial, cosine),
        mul(tangent, sine / tangent_length),
    ));
    // Rounded arithmetic must never turn a certified origin into an excluded
    // one. Radial is the conservative fallback at a degenerate cap boundary.
    if dot(separation, normal) >= 2. * radius {
        normal
    } else {
        radial
    }
}

#[cfg(test)]
std::thread_local! {
    static DIAGNOSTIC_CURRENT_NORMALS:std::cell::Cell<bool>=const {std::cell::Cell::new(false)};
}

#[cfg(test)]
pub(in crate::hair) fn with_current_native_normals<T>(enabled: bool, run: impl FnOnce() -> T) -> T {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            DIAGNOSTIC_CURRENT_NORMALS.with(|mode| mode.set(self.0));
        }
    }
    let _reset = Reset(DIAGNOSTIC_CURRENT_NORMALS.with(|mode| mode.replace(enabled)));
    run()
}

pub(super) fn activate_from_origin(
    constraints: &mut Vec<Constraint>,
    rods: &[HairRod],
    start: &[HairRod],
    increment: &PositionIncrement,
    groups: &mut Vec<StrandResponse>,
    base: &[StrandResponse],
    cuts: &mut Vec<Cut>,
    radius: f64,
    workers: Option<usize>,
    origin_admitted: bool,
) -> Result<bool, &'static str> {
    #[cfg(test)]
    let current_normals = DIAGNOSTIC_CURRENT_NORMALS.with(std::cell::Cell::get);
    #[cfg(not(test))]
    let current_normals = false;
    activate_with_normal_mode(
        constraints,
        rods,
        start,
        increment,
        groups,
        base,
        cuts,
        radius,
        workers,
        origin_admitted,
        current_normals,
    )
}

pub(super) fn activate_current_normals(
    constraints: &mut Vec<Constraint>,
    rods: &[HairRod],
    start: &[HairRod],
    increment: &PositionIncrement,
    groups: &mut Vec<StrandResponse>,
    base: &[StrandResponse],
    cuts: &mut Vec<Cut>,
    radius: f64,
) -> Result<bool, &'static str> {
    activate_with_normal_mode(
        constraints,
        rods,
        start,
        increment,
        groups,
        base,
        cuts,
        radius,
        None,
        origin_path_admitted(rods, start, radius),
        true,
    )
}

fn activate_with_normal_mode(
    constraints: &mut Vec<Constraint>,
    rods: &[HairRod],
    start: &[HairRod],
    increment: &PositionIncrement,
    groups: &mut Vec<StrandResponse>,
    base: &[StrandResponse],
    cuts: &mut Vec<Cut>,
    radius: f64,
    workers: Option<usize>,
    origin_admitted: bool,
    current_normals: bool,
) -> Result<bool, &'static str> {
    let mut motions = Vec::new();
    let mut ids = Vec::new();
    let (_, scales) = trust_components(rods, increment, groups)?;
    for (r, rod) in rods.iter().enumerate() {
        for i in 0..rod.x.len() - 1 {
            ids.push((r, i));
            motions.push(CapsuleMotion {
                start: [start[r].x[i], start[r].x[i + 1]],
                end: [
                    add(rod.x[i], mul(increment.linear[r][i], scales[r])),
                    add(rod.x[i + 1], mul(increment.linear[r][i + 1], scales[r])),
                ],
                radius,
            });
        }
    }
    let pairs = swept_capsule_pairs(&motions, 1e-10)?;
    // Keep small queries serial. Full-groom captures qualify canonical row
    // and first-error ordering with parallel read-only witness searches.
    let workers = workers.unwrap_or_else(|| {
        if pairs.len() >= 1024 {
            std::thread::available_parallelism()
                .map_or(1, usize::from)
                .min(8)
        } else {
            1
        }
    });
    let witnesses = ordered_query_results(&pairs, workers, |&(a, b)| {
        let (ra, ia) = ids[a];
        let (rb, ib) = ids[b];
        if ra == rb && ia.abs_diff(ib) <= 2 {
            return Ok(None);
        }
        let mut worst: Option<(f64, f64, StrandResponse, f64, f64)> = None;
        let mut times: Vec<_> = (1..=32).map(|step| step as f64 / 32.).collect();
        let regions = if ia == 0 && ib == 0 {
            vec![
                (trim(motions[a]), motions[b]),
                (motions[a], trim(motions[b])),
            ]
        } else {
            vec![(motions[a], motions[b])]
        };
        let mut may_intersect = false;
        for (ma, mb) in regions {
            if !matches!(
                sweep_capsules(
                    ma,
                    mb,
                    CapsuleSweepOptions {
                        tolerance_m: 5e-11,
                        ..Default::default()
                    }
                )?,
                CapsuleSweep::Clear
            ) {
                may_intersect = true;
                let time = localized_minimum_time(ma, mb);
                if time > 0. {
                    times.push(time);
                }
            }
        }
        // Clear is a full continuous certificate, not an endpoint/sample
        // estimate. Only unresolved regions need counterexample sampling.
        if !may_intersect {
            return Ok(None);
        }
        times.sort_by(f64::total_cmp);
        times.dedup();
        for time in times {
            let at = |m: CapsuleMotion| {
                std::array::from_fn::<_, 2, _>(|p| {
                    add(m.start[p], mul(sub(m.end[p], m.start[p]), time))
                })
            };
            let aa = at(motions[a]);
            let bb = at(motions[b]);
            let regions = if ia == 0 && ib == 0 {
                vec![
                    (
                        trim(CapsuleMotion {
                            start: aa,
                            end: aa,
                            radius,
                        })
                        .start,
                        bb,
                        0.2,
                        0.,
                    ),
                    (
                        aa,
                        trim(CapsuleMotion {
                            start: bb,
                            end: bb,
                            radius,
                        })
                        .start,
                        0.,
                        0.2,
                    ),
                ]
            } else {
                vec![(aa, bb, 0., 0.)]
            };
            for (aa, bb, offset_a, offset_b) in regions {
                let (s, t, p, q) = super::super::super::segment_pair(aa[0], aa[1], bb[0], bb[1]);
                let delta = sub(p, q);
                let distance = len(delta);
                if distance >= 2. * radius - 1e-10 {
                    continue;
                }
                let s = offset_a + (1. - offset_a) * s;
                let t = offset_b + (1. - offset_b) * t;
                let normal = if distance > 1e-12 {
                    mul(delta, 1. / distance)
                } else {
                    let old = sub(point(&start[ra].x, ia, s), point(&start[rb].x, ib, t));
                    if len(old) <= 1e-12 {
                        return Err("swept contact has no separating direction");
                    }
                    unit(old)
                };
                if worst.as_ref().is_none_or(|w| distance < w.0) {
                    worst = Some((
                        distance,
                        time,
                        StrandResponse {
                            a: (ra, ia, s),
                            b: (rb, ib, t),
                            normal,
                            impulse: 0.,
                        },
                        offset_a,
                        offset_b,
                    ));
                }
            }
        }
        if let Some((_, time, mut pair, offset_a, offset_b)) = worst {
            if current_normals {
                return Ok(Some((pair, time, [scales[ra], scales[rb]])));
            }
            let ma = if offset_a > 0. {
                trim(motions[a])
            } else {
                motions[a]
            };
            let mb = if offset_b > 0. {
                trim(motions[b])
            } else {
                motions[b]
            };
            let first = match sweep_capsules(
                ma,
                mb,
                CapsuleSweepOptions {
                    tolerance_m: 5e-11,
                    ..Default::default()
                },
            )? {
                CapsuleSweep::Clear => {
                    return Err("sampled intersection contradicts capsule sweep");
                }
                CapsuleSweep::InitialContact { .. } => 0.,
                CapsuleSweep::Approach { fraction, .. }
                | CapsuleSweep::IterationLimit { fraction, .. } => fraction,
            };
            let at = |m: CapsuleMotion| {
                std::array::from_fn::<_, 2, _>(|p| {
                    add(m.start[p], mul(sub(m.end[p], m.start[p]), first))
                })
            };
            let aa = at(ma);
            let bb = at(mb);
            let (_, _, p, q) = super::super::super::segment_pair(aa[0], aa[1], bb[0], bb[1]);
            let delta = sub(p, q);
            let distance = len(delta);
            if distance <= 1e-12 {
                return Err("first swept contact has no separating direction");
            }
            // The side comes from first approach, but the barycentric
            // feature belongs to the witnessed collision. Reusing the first
            // feature would constrain a different point after sliding.
            pair.normal = origin_normal(
                rods,
                start,
                &pair,
                time,
                radius,
                origin_admitted,
                mul(delta, 1. / distance),
            );
            return Ok(Some((pair, time, [scales[ra], scales[rb]])));
        }
        Ok(None)
    })?;
    // Geometry is read-only in workers. Publish cuts canonically only after
    // every query succeeds, then rebuild the same physical constraint owner.
    // A new Newton proposal can change the trust clock even when its path
    // introduces no new geometric witness. Old rows then need rebuilding too.
    let clock_changes = cuts
        .iter()
        .filter(|cut| cut.scales != [scales[cut.pair.a.0], scales[cut.pair.b.0]])
        .count();
    let previous_cut_count = cuts.len();
    let mut changed = clock_changes != 0;
    for (pair, time, scales) in witnesses {
        changed |= retain_cut(cuts, pair, time, scales);
    }
    if std::env::var_os("VOXY_HAIR_SWEEP_REFINEMENT_TRACE").is_some() {
        let minimum_scale = scales.iter().copied().fold(1., f64::min);
        let minimum_time = cuts.iter().map(|cut| cut.time).fold(1., f64::min);
        eprintln!(
            "HAIR SWEPT CUT UPDATE added={} clock_changes={} cuts={} minimum_scale={minimum_scale:e} minimum_time={minimum_time:e} changed={changed}",
            cuts.len() - previous_cut_count,
            clock_changes,
            cuts.len()
        );
    }
    if changed {
        // Rebuild from the canonical base and all witnessed affine planes.
        // add_constraint retains its canonical coalescing and strongest bound.
        *groups = base.to_vec();
        groups.extend(cuts.iter().map(|cut| cut.pair.clone()));
        let (_, scales) = trust_components(rods, increment, groups)?;
        // Base endpoint planes and swept witnesses must evaluate the same
        // staged + scaled-free trajectory. Reassemble through the canonical
        // owner so mobility, diagonal and aliases use the scaled Jacobian too.
        *constraints = position_constraints_on_clock(rods, base, radius, Some(&scales))?.0;
        for cut in cuts {
            // Contact connectivity may change the common angular clock.
            // Relinearize every retained feature on that same current path;
            // obsolete clock paths are not extra physical constraints.
            cut.scales = [scales[cut.pair.a.0], scales[cut.pair.b.0]];
            let _ = add_cut(
                constraints,
                rods,
                start,
                &cut.pair,
                cut.time,
                radius,
                cut.scales,
            )?;
        }
    }
    Ok(changed)
}

// Counterexample locator only. Closest-feature changes can create multiple
// minima, so this never replaces the final continuous admission query.
fn localized_minimum_time(a: CapsuleMotion, b: CapsuleMotion) -> f64 {
    let distance = |time: f64| {
        let at = |m: CapsuleMotion| {
            std::array::from_fn::<_, 2, _>(|p| {
                add(m.start[p], mul(sub(m.end[p], m.start[p]), time))
            })
        };
        let aa = at(a);
        let bb = at(b);
        let (_, _, p, q) = super::super::super::segment_pair(aa[0], aa[1], bb[0], bb[1]);
        len(sub(p, q))
    };
    let ratio = (5f64.sqrt() - 1.) * 0.5;
    let (mut lo, mut hi) = (0., 1.);
    let mut left = hi - ratio * (hi - lo);
    let mut right = lo + ratio * (hi - lo);
    let mut dl = distance(left);
    let mut dr = distance(right);
    for _ in 0..64 {
        if dl <= dr {
            hi = right;
            right = left;
            dr = dl;
            left = hi - ratio * (hi - lo);
            dl = distance(left);
        } else {
            lo = left;
            left = right;
            dl = dr;
            right = lo + ratio * (hi - lo);
            dr = distance(right);
        }
    }
    if dl <= dr { left } else { right }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn safe_strand_path_does_not_enable_elastic_origin_planes_inside_body() {
        let mut rod = HairRod::new(
            vec![[0., 0., 0.], [0., 0., 0.01], [0., 0., 0.02]],
            Default::default(),
        )
        .unwrap();
        let row = rod.record_contact(
            1,
            1.,
            [1., 0., 0.],
            [2e-10, 0., 0.02],
            ContactSource::Mesh(0),
        );
        let rods = vec![rod];
        let end = vec![rods[0].x.clone()];
        assert_eq!(
            super::super::strand_fraction(&rods, &end, 40e-6).unwrap(),
            1.
        );
        assert!(
            !origin_path_admitted(&rods, &rods, 40e-6),
            "body restoration must retain its original side rule"
        );
        let mut admitted = rods.clone();
        admitted[0].contacts[row].target[0] = 5e-11;
        assert!(
            origin_path_admitted(&admitted, &admitted, 40e-6),
            "the existing physical admission tolerance is unchanged"
        );
    }
    #[test]
    fn certified_origin_preserves_an_already_supporting_first_approach_plane() {
        let make = |x| {
            HairRod::new(
                vec![[x, 0., 0.], [x, 0.01, 0.], [x, 0.02, 0.]],
                Default::default(),
            )
            .unwrap()
        };
        let rods = vec![make(0.001), make(0.)];
        // Oblique, unit length and deliberately distinct from radial +X.
        let fallback = [0.6, 0.8, 0.];
        let pair = StrandResponse {
            a: (0, 1, 0.5),
            b: (1, 1, 0.5),
            normal: fallback,
            impulse: 0.,
        };
        let result = origin_normal(&rods, &rods, &pair, 1., 40e-6, true, fallback);
        assert_eq!(
            result, fallback,
            "an origin-admitting plane must retain its exact normal"
        );
        let mut rows = Vec::new();
        add_cut(&mut rows, &rods, &rods, &pair, 1., 40e-6, [1., 1.]).unwrap();
        assert!(rows[0].bound < 0.);
    }
    #[test]
    fn captured_safe_origin_is_not_excluded_by_a_different_feature_normal() {
        // Actual first refined row134, coupled frame2, rods431/447. The
        // preceding normal at another feature demanded1.642um of correction
        // even though this affine witness is81.850um apart (diameter80um).
        let segments = [
            [
                [
                    0.10064733785552496,
                    0.6631849208501187,
                    0.005164870202436705,
                ],
                [0.10037345683065663, 0.650348954561776, 0.004972200232476973],
            ],
            [
                [
                    0.10069802918164983,
                    0.6616111924273493,
                    0.005092859540689342,
                ],
                [
                    0.10035319600727302,
                    0.6466279058458493,
                    0.005173598654484514,
                ],
            ],
        ];
        let rods: Vec<_> = segments
            .iter()
            .map(|p| {
                HairRod::new(
                    vec![sub(p[0], [0., 0., 0.01]), p[0], p[1]],
                    Default::default(),
                )
                .unwrap()
            })
            .collect();
        let mut pair = StrandResponse {
            a: (0, 1, 0.2985886501529072),
            b: (1, 1, 0.14989811198187503),
            normal: [-0.978639289460203, 0.016039870255973965, -0.204958199853045],
            impulse: 0.,
        };
        let mut old_rows = Vec::new();
        add_cut(&mut old_rows, &rods, &rods, &pair, 1., 40e-6, [1., 1.]).unwrap();
        assert!(
            old_rows[0].bound > 1.6e-6,
            "fixture must reproduce origin exclusion"
        );
        let fallback = pair.normal;
        assert_eq!(
            origin_normal(&rods, &rods, &pair, 1., 40e-6, false, fallback),
            fallback,
            "uncertified origins must retain first-side restoration"
        );
        pair.normal = origin_normal(&rods, &rods, &pair, 1., 40e-6, true, fallback);
        let mut rows = Vec::new();
        add_cut(&mut rows, &rods, &rods, &pair, 1., 40e-6, [1., 1.]).unwrap();
        assert!(
            rows[0].bound <= 0.,
            "safe feature must admit zero correction"
        );
        let separation = sub(
            point(&rods[0].x, 1, pair.a.2),
            point(&rods[1].x, 1, pair.b.2),
        );
        assert!(
            dot(pair.normal, fallback) > dot(unit(separation), fallback),
            "repair must retain more first-approach direction than full radial replacement"
        );
        assert!((len(pair.normal) - 1.).abs() < 1e-14);
        assert_eq!(
            rows[0]
                .entries
                .iter()
                .filter(|e| e.gradient != [0.; 3])
                .count(),
            4
        );
    }
    #[test]
    fn retained_clear_witness_refreshes_when_only_the_trust_clock_changes() {
        let make = |x| {
            HairRod::new(
                vec![[x, 0., 0.], [x, 0.01, 0.], [x, 0.02, 0.]],
                crate::hair::HairMaterial::default(),
            )
            .unwrap()
        };
        let rods = vec![make(0.), make(0.001)];
        let pair = StrandResponse {
            a: (0, 1, 0.5),
            b: (1, 1, 0.5),
            normal: [-1., 0., 0.],
            impulse: 0.,
        };
        let base = vec![pair.clone()];
        let mut cuts = Vec::new();
        retain_cut(&mut cuts, pair.clone(), 1., [1., 1.]);
        let mut rows = position_constraints(&rods, &base, 40e-6).unwrap().0;
        add_cut(&mut rows, &rods, &rods, &pair, 1., 40e-6, [1., 1.]).unwrap();
        assert_eq!(
            rows.len(),
            1,
            "endpoint witness and base plane are exact aliases"
        );
        let old_diagonal = rows[0].diagonal;
        let mut free = PositionIncrement {
            linear: vec![vec![[0.; 3]; 3]; 2],
            angular: vec![vec![[0.; 3]; 2]; 2],
        };
        free.angular[0][1] = [0.7, 0., 0.];
        let mut groups = vec![pair];
        assert!(
            activate(
                &mut rows,
                &rods,
                &rods,
                &free,
                &mut groups,
                &base,
                &mut cuts,
                40e-6
            )
            .unwrap()
        );
        assert_eq!(
            cuts.len(),
            1,
            "clear proposal must not create another witness"
        );
        assert_eq!(cuts[0].scales, [0.5, 0.5]);
        assert_eq!(
            rows.len(),
            1,
            "clock refresh must preserve exact base/witness aliasing"
        );
        assert!((rows[0].diagonal - old_diagonal * 0.25).abs() < 1e-10 * old_diagonal);
        assert!(
            !activate(
                &mut rows,
                &rods,
                &rods,
                &free,
                &mut groups,
                &base,
                &mut cuts,
                40e-6
            )
            .unwrap(),
            "unchanged clock must settle rather than force endless refinement"
        );
    }
    #[test]
    fn base_endpoint_rows_use_the_same_restricted_path_as_swept_cuts() {
        let make = |x| {
            HairRod::new(
                vec![[x, 0., 0.], [x, 0.01, 0.], [x, 0.02, 0.]],
                crate::hair::HairMaterial::default(),
            )
            .unwrap()
        };
        let rods = vec![make(0.), make(60e-6)];
        let pair = StrandResponse {
            a: (0, 1, 0.5),
            b: (1, 1, 0.5),
            normal: [-1., 0., 0.],
            impulse: 0.,
        };
        let mut free = PositionIncrement {
            linear: vec![vec![[0.; 3]; 3]; 2],
            angular: vec![vec![[0.; 3]; 2]; 2],
        };
        free.linear[0][1] = [-20e-6, 0., 0.];
        free.linear[0][2] = [-20e-6, 0., 0.];
        let base = position_constraints(&rods, std::slice::from_ref(&pair), 40e-6)
            .unwrap()
            .0;
        assert!((base[0].speed(&free) - base[0].bound).abs() < 1e-18);
        let scales = [0.25, 0.25];
        let rows = position_constraints_on_clock(&rods, &[pair], 40e-6, Some(&scales))
            .unwrap()
            .0;
        let actual_gap = 60e-6 + 20e-6 * scales[0] - 80e-6;
        assert!(rows[0].speed(&free) < rows[0].bound);
        assert!((rows[0].speed(&free) - rows[0].bound - actual_gap).abs() < 1e-18);
        assert!(
            (rows[0].diagonal - base[0].diagonal * 0.25f64.powi(2)).abs()
                < 1e-10 * base[0].diagonal
        );
        assert!(position_constraints_on_clock(&rods, &[], 40e-6, Some(&[0., 1.])).is_err());
    }
    #[test]
    fn captured_blocking_pair_locator_retains_short_intersection() {
        let a = CapsuleMotion {
            start: [
                [
                    -0.10571503383239862,
                    0.5353139616408988,
                    0.017056329341362284,
                ],
                [
                    -0.10806919706770535,
                    0.5299472118400107,
                    0.023043577335470203,
                ],
            ],
            end: [
                [
                    -0.10577149379928885,
                    0.5351591661098012,
                    0.01705744597213609,
                ],
                [
                    -0.10812570262070621,
                    0.5297880864893493,
                    0.02304079376955749,
                ],
            ],
            radius: 4e-5,
        };
        let b = CapsuleMotion {
            start: [
                [
                    -0.10800291278035071,
                    0.5311111101854139,
                    0.022075529636794787,
                ],
                [-0.1083593551281725, 0.5185706833323395, 0.03356630216649008],
            ],
            end: [
                [
                    -0.10806758277419319,
                    0.5309456355605204,
                    0.022085131983851477,
                ],
                [
                    -0.10842155264723373,
                    0.5183892560338552,
                    0.03355856313709558,
                ],
            ],
            radius: 4e-5,
        };
        let distance = |time: f64| {
            let at = |m: CapsuleMotion| {
                std::array::from_fn::<_, 2, _>(|i| {
                    add(m.start[i], mul(sub(m.end[i], m.start[i]), time))
                })
            };
            let aa = at(a);
            let bb = at(b);
            let (_, _, p, q) = super::super::super::super::segment_pair(aa[0], aa[1], bb[0], bb[1]);
            len(sub(p, q))
        };
        let located = localized_minimum_time(a, b);
        let sampled = (0..=4096)
            .map(|i| {
                let t = i as f64 / 4096.;
                (t, distance(t))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap();
        eprintln!(
            "BLOCKING PAIR located_time={located} located_distance={} sampled={sampled:?}",
            distance(located)
        );
        eprintln!(
            "BLOCKING PAIR certificates line={} planes={:?} sweep={:?} admitted_fraction={:?}",
            super::super::line_certificate::clear(a, b, 512),
            super::super::interval_planes_clear(a, b, 32768),
            sweep_capsules(
                a,
                b,
                CapsuleSweepOptions {
                    tolerance_m: 5e-11,
                    ..Default::default()
                }
            ),
            super::super::pair_fraction(
                a,
                b,
                CapsuleSweepOptions {
                    tolerance_m: 5e-11,
                    ..Default::default()
                }
            )
        );
        assert!(
            distance(located) <= sampled.1 + 1e-14,
            "locator missed a shorter interior feature"
        );
        assert!(
            super::super::line_certificate::clear(a, b, 512),
            "finite endpoint feature must certify the complete captured path"
        );
        assert_eq!(
            super::super::pair_fraction(
                a,
                b,
                CapsuleSweepOptions {
                    tolerance_m: 5e-11,
                    ..Default::default()
                }
            )
            .unwrap(),
            1.
        );
    }
    #[test]
    fn early_witness_residual_and_gradient_remain_in_physical_metres() {
        let make = |x| {
            HairRod::new(
                vec![[x, 0., 0.], [x, 0.01, 0.], [x, 0.02, 0.]],
                crate::hair::HairMaterial::default(),
            )
            .unwrap()
        };
        let start = vec![make(0.), make(80e-6 + 3e-10)];
        let mut staged = start.clone();
        staged[0].x[0][0] += 1e-5;
        staged[0].x[1][0] += 4e-5;
        let pair = StrandResponse {
            a: (0, 0, 0.5),
            b: (1, 0, 0.5),
            normal: [-1., 0., 0.],
            impulse: 0.,
        };
        let scales = [0.25, 0.5];
        let mut free = PositionIncrement {
            linear: vec![vec![[0.; 3]; 3]; 2],
            angular: vec![vec![[0.; 3]; 2]; 2],
        };
        free.linear[0][1] = [2e-5, 0., 0.];
        free.linear[1][1] = [-1e-5, 0., 0.];
        for time in [1e-8, 0.25, 0.5, 1.] {
            let mut rows = Vec::new();
            add_cut(&mut rows, &staged, &start, &pair, time, 40e-6, scales).unwrap();
            let at = |r: usize| {
                add(
                    mul(point(&start[r].x, 0, 0.5), 1. - time),
                    mul(
                        add(
                            point(&staged[r].x, 0, 0.5),
                            mul(free.linear[r][1], 0.5 * scales[r]),
                        ),
                        time,
                    ),
                )
            };
            let actual = dot(sub(at(0), at(1)), pair.normal) - 80e-6;
            assert!((rows[0].speed(&free) - rows[0].bound - actual).abs() < 1e-18);
            for entry in rows[0].entries.iter().filter(|entry| entry.point != 0) {
                let sign = if entry.rod == 0 { -1. } else { 1. };
                assert_eq!(
                    entry.gradient,
                    [sign * time * scales[entry.rod] * 0.5, 0., 0.]
                );
            }
            assert!(
                rows[0]
                    .entries
                    .iter()
                    .all(|e| e.point != 0 || e.gradient == [0.; 3])
            );
        }
    }
    #[test]
    fn angular_trust_collision_is_found_on_the_path_that_will_be_admitted() {
        let make = |x| {
            HairRod::new(
                vec![[x, 0., 0.], [x, 0.01, 0.], [x, 0.02, 0.]],
                crate::hair::HairMaterial::default(),
            )
            .unwrap()
        };
        let start = vec![make(0.), make(80e-6 + 5e-10)];
        let mut staged = start.clone();
        for rod in &mut staged {
            rod.x[0][0] += 0.004;
        }
        let mut free = PositionIncrement {
            linear: vec![vec![[0.; 3]; 3]; 2],
            angular: vec![vec![[0.; 3]; 2]; 2],
        };
        for points in &mut free.linear {
            for p in &mut points[1..] {
                p[0] = 0.004;
            }
        }
        free.angular[1][1] = [0., 0., 0.7];
        let full: Vec<Vec<V>> = staged
            .iter()
            .enumerate()
            .map(|(r, rod)| {
                rod.x
                    .iter()
                    .enumerate()
                    .map(|(p, x)| add(*x, free.linear[r][p]))
                    .collect()
            })
            .collect();
        assert_eq!(
            strand_fraction(&start, &full, 40e-6).unwrap(),
            1.,
            "full rigid transport is clear"
        );
        let base = vec![StrandResponse {
            a: (0, 1, 0.5),
            b: (1, 1, 0.5),
            normal: [-1., 0., 0.],
            impulse: 0.,
        }];
        let (_, scales) = trust_components(&staged, &free, &base).unwrap();
        let restricted: Vec<Vec<V>> = staged
            .iter()
            .enumerate()
            .map(|(r, rod)| {
                rod.x
                    .iter()
                    .enumerate()
                    .map(|(p, x)| add(*x, mul(free.linear[r][p], scales[r])))
                    .collect()
            })
            .collect();
        assert!(strand_fraction(&start, &restricted, 40e-6).unwrap() < 1.);
        let mut constraints = Vec::new();
        let mut groups = base.clone();
        let mut cuts = Vec::new();
        assert!(
            activate(
                &mut constraints,
                &staged,
                &start,
                &free,
                &mut groups,
                &base,
                &mut cuts,
                40e-6
            )
            .unwrap()
        );
        assert!(
            constraints.iter().any(|c| c.speed(&free) < c.bound - 1e-10),
            "scaled witness must reject the actual restricted trajectory"
        );
        for cut in &cuts {
            let mut row = Vec::new();
            add_cut(
                &mut row, &staged, &start, &cut.pair, cut.time, 40e-6, cut.scales,
            )
            .unwrap();
            let (a, i, s) = cut.pair.a;
            let (b, j, t) = cut.pair.b;
            let at = |r: usize, segment: usize, fraction: f64| {
                add(
                    mul(point(&start[r].x, segment, fraction), 1. - cut.time),
                    mul(point(&restricted[r], segment, fraction), cut.time),
                )
            };
            let actual = dot(sub(at(a, i, s), at(b, j, t)), cut.pair.normal) - 80e-6;
            assert!(
                ((row[0].speed(&free) - row[0].bound) - actual).abs() < 1e-15,
                "affine load must include the same clocks and prescribed roots as geometry"
            );
        }
    }
    #[test]
    fn shifted_witness_cannot_reopen_a_previously_constrained_crossing() {
        let make = |x| {
            HairRod::new(
                vec![[x, 0., 0.], [x, 0.01, 0.], [x, 0.02, 0.]],
                crate::hair::HairMaterial::default(),
            )
            .unwrap()
        };
        let rods = vec![make(0.), make(0.001)];
        let old = StrandResponse {
            a: (0, 1, 0.1),
            b: (1, 1, 0.1),
            normal: [-1., 0., 0.],
            impulse: 0.,
        };
        let mut new = old.clone();
        new.a.2 = 0.9;
        new.b.2 = 0.9;
        let mut free = PositionIncrement {
            linear: vec![vec![[0.; 3]; 3]; 2],
            angular: vec![vec![[0.; 3]; 2]; 2],
        };
        free.linear[0][1][0] = 0.002;
        free.linear[0][2][0] = -0.002;
        let mut replacement = Vec::new();
        add_cut(&mut replacement, &rods, &rods, &new, 1., 40e-6, [1., 1.]).unwrap();
        assert!(replacement[0].speed(&free) >= replacement[0].bound);
        let old_end = dot(
            sub(
                add(
                    point(&rods[0].x, 1, 0.1),
                    add(mul(free.linear[0][1], 0.9), mul(free.linear[0][2], 0.1)),
                ),
                point(&rods[1].x, 1, 0.1),
            ),
            old.normal,
        );
        assert!(
            old_end < 0.,
            "replacement admits crossing the old contact side"
        );
        let mut cuts = Vec::new();
        assert!(retain_cut(&mut cuts, old.clone(), 1., [1., 1.]));
        assert!(retain_cut(&mut cuts, new, 1., [1., 1.]));
        assert!(!retain_cut(&mut cuts, old, 1., [1., 1.]));
        let mut retained = Vec::new();
        for cut in &cuts {
            add_cut(
                &mut retained,
                &rods,
                &rods,
                &cut.pair,
                cut.time,
                40e-6,
                cut.scales,
            )
            .unwrap();
        }
        assert!(retained.iter().any(|c| c.speed(&free) < c.bound - 1e-10));
        assert!(retained.iter().all(|c| {
            c.entries
                .iter()
                .all(|e| e.point != 0 || e.gradient == [0.; 3])
        }));
    }
    #[test]
    fn localized_witness_finds_captured_penetration_between_coarse_samples() {
        let a = CapsuleMotion {
            radius: 40e-6,
            start: [
                [
                    -0.05320794855818693,
                    0.8135695376889923,
                    0.038037514531762524,
                ],
                [
                    -0.05448365126943721,
                    0.8122917787067718,
                    0.03580298606584595,
                ],
            ],
            end: [
                [
                    -0.053139853296977235,
                    0.8137719008823939,
                    0.03808365478321574,
                ],
                [
                    -0.054390995942093075,
                    0.812423006562546,
                    0.03587609922408957,
                ],
            ],
        };
        let b = CapsuleMotion {
            radius: 40e-6,
            start: [
                [
                    -0.05364587941295736,
                    0.8132718340966427,
                    0.03860107685139109,
                ],
                [
                    -0.05451064581400578,
                    0.8123308423697174,
                    0.035916939966533064,
                ],
            ],
            end: [
                [
                    -0.05340466461219224,
                    0.812612857539943,
                    0.038785437810288384,
                ],
                [
                    -0.05425001225440255,
                    0.8116082804903335,
                    0.036117569023514415,
                ],
            ],
        };
        let gap = |time| {
            let at = |m: CapsuleMotion| {
                std::array::from_fn::<_, 2, _>(|p| {
                    add(m.start[p], mul(sub(m.end[p], m.start[p]), time))
                })
            };
            let aa = at(a);
            let bb = at(b);
            let (_, _, p, q) = super::super::super::super::segment_pair(aa[0], aa[1], bb[0], bb[1]);
            len(sub(p, q)) - a.radius - b.radius
        };
        for step in 0..=32 {
            assert!(gap(step as f64 / 32.) >= -1e-10);
        }
        let time = localized_minimum_time(a, b);
        assert!(time > 0. && time < 1. / 32.);
        assert!(gap(time) < -1e-7);
    }
    #[test]
    fn swept_cut_keeps_the_first_contact_side_after_candidate_crosses() {
        let make = |x| {
            HairRod::new(
                vec![[x, 0., 0.], [x, 0.01, 0.], [x, 0.02, 0.]],
                crate::hair::HairMaterial::default(),
            )
            .unwrap()
        };
        let start = vec![make(0.), make(0.001)];
        let mut staged = start.clone();
        staged[0].x[0][0] = 0.0033;
        let mut free = PositionIncrement {
            linear: vec![vec![[0.; 3]; 3]; 2],
            angular: vec![vec![[0.; 3]; 2]; 2],
        };
        for p in 1..3 {
            free.linear[0][p][0] = 0.0033;
        }
        let mut constraints = Vec::new();
        let mut groups = Vec::new();
        let mut cuts = Vec::new();
        assert!(
            activate(
                &mut constraints,
                &staged,
                &start,
                &free,
                &mut groups,
                &[],
                &mut cuts,
                40e-6
            )
            .unwrap()
        );
        assert!(groups.iter().all(|pair| pair.normal[0] < -0.99));
        let previous_count = cuts.len();
        for p in 1..3 {
            free.linear[0][p][0] = 0.0034;
        }
        assert!(
            activate(
                &mut constraints,
                &staged,
                &start,
                &free,
                &mut groups,
                &[],
                &mut cuts,
                40e-6
            )
            .unwrap()
        );
        assert!(
            cuts.len() > previous_count,
            "new witnesses must preserve prior planes"
        );
        assert_eq!(groups.len(), cuts.len());
        assert!(constraints.iter().any(|c| c.speed(&free) < c.bound - 1e-10));
        assert!(constraints.iter().all(|c| {
            c.entries
                .iter()
                .all(|e| e.point != 0 || e.gradient == [0.; 3])
        }));
    }
    #[test]
    fn affine_cut_includes_prescribed_root_and_time_interpolation() {
        let make = |x| {
            HairRod::new(
                vec![[x, 0., 0.], [x, 0.01, 0.], [x, 0.02, 0.]],
                crate::hair::HairMaterial::default(),
            )
            .unwrap()
        };
        let start = vec![make(0.), make(0.001)];
        let mut staged = start.clone();
        staged[0].x[0][0] = 0.001;
        let pair = StrandResponse {
            a: (0, 0, 0.5),
            b: (1, 0, 0.5),
            normal: [-1., 0., 0.],
            impulse: 0.,
        };
        let mut constraints = Vec::new();
        add_cut(
            &mut constraints,
            &staged,
            &start,
            &pair,
            0.5,
            40e-6,
            [1., 1.],
        )
        .unwrap();
        let mut free = PositionIncrement {
            linear: vec![vec![[0.; 3]; 3]; 2],
            angular: vec![vec![[0.; 3]; 2]; 2],
        };
        free.linear[0][1][0] = 0.001;
        let old = dot(
            sub(point(&start[0].x, 0, 0.5), point(&start[1].x, 0, 0.5)),
            pair.normal,
        );
        let end = dot(
            sub(
                add(point(&staged[0].x, 0, 0.5), mul(free.linear[0][1], 0.5)),
                point(&staged[1].x, 0, 0.5),
            ),
            pair.normal,
        );
        let actual = 0.5 * old + 0.5 * end - 80e-6;
        assert!(((constraints[0].speed(&free) - constraints[0].bound) - actual).abs() < 1e-15);
        assert!(
            constraints[0]
                .entries
                .iter()
                .all(|e| e.point != 0 || e.gradient == [0.; 3])
        );
    }
}

#[cfg(test)]
pub(super) fn activate_reference(
    constraints: &mut Vec<Constraint>,
    rods: &[HairRod],
    start: &[HairRod],
    increment: &PositionIncrement,
    groups: &mut Vec<StrandResponse>,
    base: &[StrandResponse],
    cuts: &mut Vec<Cut>,
    radius: f64,
) -> Result<bool, &'static str> {
    let origin_admitted = origin_path_admitted(rods, start, radius);
    let mut motions = Vec::new();
    let mut ids = Vec::new();
    let (_, scales) = trust_components(rods, increment, groups)?;
    for (r, rod) in rods.iter().enumerate() {
        for i in 0..rod.x.len() - 1 {
            ids.push((r, i));
            motions.push(CapsuleMotion {
                start: [start[r].x[i], start[r].x[i + 1]],
                end: [
                    add(rod.x[i], mul(increment.linear[r][i], scales[r])),
                    add(rod.x[i + 1], mul(increment.linear[r][i + 1], scales[r])),
                ],
                radius,
            });
        }
    }
    let mut changed = cuts
        .iter()
        .any(|cut| cut.scales != [scales[cut.pair.a.0], scales[cut.pair.b.0]]);
    for (a, b) in swept_capsule_pairs(&motions, 1e-10)? {
        let (ra, ia) = ids[a];
        let (rb, ib) = ids[b];
        if ra == rb && ia.abs_diff(ib) <= 2 {
            continue;
        }
        let mut worst: Option<(f64, f64, StrandResponse, f64, f64)> = None;
        let mut times: Vec<_> = (1..=32).map(|step| step as f64 / 32.).collect();
        let regions = if ia == 0 && ib == 0 {
            vec![
                (trim(motions[a]), motions[b]),
                (motions[a], trim(motions[b])),
            ]
        } else {
            vec![(motions[a], motions[b])]
        };
        for (ma, mb) in regions {
            if !matches!(
                sweep_capsules(
                    ma,
                    mb,
                    CapsuleSweepOptions {
                        tolerance_m: 5e-11,
                        ..Default::default()
                    }
                )?,
                CapsuleSweep::Clear
            ) {
                let time = localized_minimum_time(ma, mb);
                if time > 0. {
                    times.push(time);
                }
            }
        }
        times.sort_by(f64::total_cmp);
        times.dedup();
        for time in times {
            let at = |m: CapsuleMotion| {
                std::array::from_fn::<_, 2, _>(|p| {
                    add(m.start[p], mul(sub(m.end[p], m.start[p]), time))
                })
            };
            let aa = at(motions[a]);
            let bb = at(motions[b]);
            let regions = if ia == 0 && ib == 0 {
                vec![
                    (
                        trim(CapsuleMotion {
                            start: aa,
                            end: aa,
                            radius,
                        })
                        .start,
                        bb,
                        0.2,
                        0.,
                    ),
                    (
                        aa,
                        trim(CapsuleMotion {
                            start: bb,
                            end: bb,
                            radius,
                        })
                        .start,
                        0.,
                        0.2,
                    ),
                ]
            } else {
                vec![(aa, bb, 0., 0.)]
            };
            for (aa, bb, offset_a, offset_b) in regions {
                let (s, t, p, q) = super::super::super::segment_pair(aa[0], aa[1], bb[0], bb[1]);
                let delta = sub(p, q);
                let distance = len(delta);
                if distance >= 2. * radius - 1e-10 {
                    continue;
                }
                let s = offset_a + (1. - offset_a) * s;
                let t = offset_b + (1. - offset_b) * t;
                let normal = if distance > 1e-12 {
                    mul(delta, 1. / distance)
                } else {
                    let old = sub(point(&start[ra].x, ia, s), point(&start[rb].x, ib, t));
                    if len(old) <= 1e-12 {
                        return Err("swept contact has no separating direction");
                    }
                    unit(old)
                };
                if worst.as_ref().is_none_or(|w| distance < w.0) {
                    worst = Some((
                        distance,
                        time,
                        StrandResponse {
                            a: (ra, ia, s),
                            b: (rb, ib, t),
                            normal,
                            impulse: 0.,
                        },
                        offset_a,
                        offset_b,
                    ));
                }
            }
        }
        if let Some((_, time, mut pair, offset_a, offset_b)) = worst {
            let ma = if offset_a > 0. {
                trim(motions[a])
            } else {
                motions[a]
            };
            let mb = if offset_b > 0. {
                trim(motions[b])
            } else {
                motions[b]
            };
            let first = match sweep_capsules(
                ma,
                mb,
                CapsuleSweepOptions {
                    tolerance_m: 5e-11,
                    ..Default::default()
                },
            )? {
                CapsuleSweep::Clear => {
                    return Err("sampled intersection contradicts capsule sweep");
                }
                CapsuleSweep::InitialContact { .. } => 0.,
                CapsuleSweep::Approach { fraction, .. }
                | CapsuleSweep::IterationLimit { fraction, .. } => fraction,
            };
            let at = |m: CapsuleMotion| {
                std::array::from_fn::<_, 2, _>(|p| {
                    add(m.start[p], mul(sub(m.end[p], m.start[p]), first))
                })
            };
            let aa = at(ma);
            let bb = at(mb);
            let (_, _, p, q) = super::super::super::segment_pair(aa[0], aa[1], bb[0], bb[1]);
            let delta = sub(p, q);
            let distance = len(delta);
            if distance <= 1e-12 {
                return Err("first swept contact has no separating direction");
            }
            // The side comes from first approach, but the barycentric
            // feature belongs to the witnessed collision. Reusing the first
            // feature would constrain a different point after sliding.
            pair.normal = origin_normal(
                rods,
                start,
                &pair,
                time,
                radius,
                origin_admitted,
                mul(delta, 1. / distance),
            );
            changed |= retain_cut(cuts, pair, time, [scales[ra], scales[rb]]);
        }
    }
    if changed {
        // Rebuild from the canonical base and all witnessed affine planes.
        // add_constraint retains its canonical coalescing and strongest bound.
        *groups = base.to_vec();
        groups.extend(cuts.iter().map(|cut| cut.pair.clone()));
        let (_, scales) = trust_components(rods, increment, groups)?;
        *constraints = position_constraints_on_clock(rods, base, radius, Some(&scales))?.0;
        for cut in cuts {
            // Contact connectivity may change the common angular clock.
            // Relinearize every retained feature on that same current path;
            // obsolete clock paths are not extra physical constraints.
            cut.scales = [scales[cut.pair.a.0], scales[cut.pair.b.0]];
            let _ = add_cut(
                constraints,
                rods,
                start,
                &cut.pair,
                cut.time,
                radius,
                cut.scales,
            )?;
        }
    }
    Ok(changed)
}
