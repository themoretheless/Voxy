//! Intersections of zero-valued contact windows in common fixed-tick coordinates.
use super::{FootContactKey, contact_weight};
use voxy_animation::AnimationPhaseInterval;

#[derive(Clone, Copy)]
pub(super) enum SourceTravel<'a> {
    Clip {
        keys: &'a [FootContactKey],
        phase: AnimationPhaseInterval,
        active_fraction: f64,
    },
    Frozen {
        weight: f32,
        active_fraction: f64,
    },
}
fn spend(budget: &mut voxy_gameplay::SupportQueryBudget) -> Result<(), String> {
    budget
        .charge_work(1)
        .map_err(|_| "contact event budget exceeded".into())
}
fn zero_windows(
    keys: &[FootContactKey],
    phase: AnimationPhaseInterval,
    fraction: f64,
    budget: &mut voxy_gameplay::SupportQueryBudget,
) -> Result<Vec<[f64; 2]>, String> {
    if !phase.start.is_finite()
        || !phase.end.is_finite()
        || !(0. ..=1.).contains(&phase.start)
        || phase.end < phase.start
        || !fraction.is_finite()
        || !(0. ..=1.).contains(&fraction)
    {
        return Err("invalid contact event interval".into());
    }
    if fraction == 0. || keys.is_empty() {
        return Ok(vec![]);
    }
    let mut base: Vec<[f64; 2]> = Vec::new();
    for (index, key) in keys.iter().enumerate() {
        spend(budget)?;
        if key.weight == 0. {
            let end = keys
                .get(index + 1)
                .filter(|next| next.weight == 0.)
                .map_or(key.phase, |next| next.phase);
            base.push([f64::from(key.phase), f64::from(end)]);
        }
    }
    if base.is_empty() {
        return Ok(vec![]);
    }
    if keys.iter().all(|key| key.weight == 0.) {
        return Ok(vec![[0., fraction]]);
    }
    if phase.end == phase.start {
        return Ok(if contact_weight(keys, Some(phase.start))? == 0. {
            vec![[0., fraction]]
        } else {
            vec![]
        });
    }
    let cycles = if phase.looping {
        phase.end.floor() + 1.
    } else {
        1.
    };
    if cycles > budget.remaining() as f64 {
        return Err("contact event budget exceeded".into());
    }
    let mut windows = Vec::new();
    for cycle in 0..cycles as usize {
        for &[start, end] in &base {
            spend(budget)?;
            let offset = cycle as f64;
            let lo = (start + offset).max(phase.start);
            let hi = (end + offset).min(phase.end);
            if hi >= lo {
                windows.push([
                    (lo - phase.start) / (phase.end - phase.start) * fraction,
                    (hi - phase.start) / (phase.end - phase.start) * fraction,
                ]);
            }
        }
    }
    windows.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    Ok(windows)
}
pub(super) fn mixed_swing(
    target: &[FootContactKey],
    target_phase: AnimationPhaseInterval,
    source: Option<SourceTravel<'_>>,
    budget: &mut voxy_gameplay::SupportQueryBudget,
) -> Result<bool, String> {
    let Some(source) = source else {
        // The ordinary single-clip case needs no loop enumeration.
        budget
            .charge_work(target.len())
            .map_err(|_| "contact event budget exceeded")?;
        return Ok(super::crossed_swing(target, Some(target_phase)));
    };
    let target = zero_windows(target, target_phase, 1., budget)?;
    let (source, fraction) = match source {
        SourceTravel::Clip {
            keys,
            phase,
            active_fraction,
        } => (
            zero_windows(keys, phase, active_fraction, budget)?,
            active_fraction,
        ),
        SourceTravel::Frozen {
            weight,
            active_fraction,
        } => {
            if !weight.is_finite()
                || !(0. ..=1.).contains(&weight)
                || !active_fraction.is_finite()
                || !(0. ..=1.).contains(&active_fraction)
            {
                return Err("invalid frozen contact source".into());
            }
            (
                if weight == 0. {
                    vec![[0., active_fraction]]
                } else {
                    vec![]
                },
                active_fraction,
            )
        }
    };
    if target
        .iter()
        .any(|window| window[1] >= fraction && window[1] > 0.)
    {
        return Ok(true); // Source no longer contributes after fade completion.
    }
    let (mut a, mut b) = (0, 0);
    while a < target.len() && b < source.len() {
        spend(budget)?;
        let lo = target[a][0].max(source[b][0]);
        let hi = target[a][1].min(source[b][1]);
        if hi >= lo && hi > 0. {
            return Ok(true);
        }
        if target[a][1] < source[b][1] {
            a += 1;
        } else {
            b += 1;
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn mixed_swing(
        target: &[FootContactKey],
        phase: AnimationPhaseInterval,
        source: Option<SourceTravel<'_>>,
        limit: usize,
    ) -> Result<bool, String> {
        let mut budget =
            voxy_gameplay::SupportQueryBudget::new(limit).map_err(|e| e.to_string())?;
        super::mixed_swing(target, phase, source, &mut budget)
    }

    fn curve(zero: f32) -> Vec<FootContactKey> {
        vec![
            FootContactKey {
                phase: 0.,
                weight: 1.,
            },
            FootContactKey {
                phase: zero,
                weight: 0.,
            },
            FootContactKey {
                phase: 1.,
                weight: 1.,
            },
        ]
    }
    fn phase(start: f64, end: f64) -> AnimationPhaseInterval {
        AnimationPhaseInterval {
            start,
            end,
            looping: true,
        }
    }
    #[test]
    fn mixed_zero_events_require_both_sources_and_include_completion_tail() {
        let target = curve(0.25);
        let matching = curve(0.25);
        let different = curve(0.75);
        let end_zero = vec![
            FootContactKey {
                phase: 0.,
                weight: 1.,
            },
            FootContactKey {
                phase: 1.,
                weight: 0.,
            },
        ];
        assert!(
            mixed_swing(
                &end_zero,
                phase(0., 1.),
                Some(SourceTravel::Frozen {
                    weight: 1.,
                    active_fraction: 1.
                }),
                100
            )
            .unwrap()
        );
        let travel = |keys| SourceTravel::Clip {
            keys,
            phase: phase(0., 1.),
            active_fraction: 1.,
        };
        assert!(mixed_swing(&target, phase(0., 1.), Some(travel(&matching)), 100).unwrap());
        assert!(!mixed_swing(&target, phase(0., 1.), Some(travel(&different)), 100).unwrap());
        assert!(
            !mixed_swing(
                &target,
                phase(0., 1.),
                Some(SourceTravel::Frozen {
                    weight: 1.,
                    active_fraction: 1.
                }),
                100
            )
            .unwrap()
        );
        assert!(
            mixed_swing(
                &target,
                phase(0., 1.),
                Some(SourceTravel::Frozen {
                    weight: 0.,
                    active_fraction: 1.
                }),
                100
            )
            .unwrap()
        );
        assert!(
            mixed_swing(
                &target,
                phase(0., 1.),
                Some(SourceTravel::Frozen {
                    weight: 1.,
                    active_fraction: 0.2
                }),
                100
            )
            .unwrap()
        );
        assert!(
            !mixed_swing(
                &target,
                phase(0., 1.),
                Some(SourceTravel::Frozen {
                    weight: 1.,
                    active_fraction: 0.5
                }),
                100
            )
            .unwrap()
        );
    }
    #[test]
    fn mixed_loop_events_are_bounded_and_start_only_events_do_not_release() {
        let keys = curve(0.25);
        let source = SourceTravel::Clip {
            keys: &keys,
            phase: phase(0.2, 2.2),
            active_fraction: 1.,
        };
        assert!(mixed_swing(&keys, phase(0.2, 2.2), Some(source), 100).unwrap());
        assert!(mixed_swing(&keys, phase(0., 1e20), Some(source), 100).is_err());
        assert!(mixed_swing(&keys, phase(0., 1.), Some(source), 0).is_err());
        assert!(
            !mixed_swing(
                &keys,
                phase(0.25, 0.5),
                Some(SourceTravel::Frozen {
                    weight: 0.,
                    active_fraction: 1.
                }),
                100
            )
            .unwrap()
        );
    }
    #[test]
    fn contacts_share_the_existing_tick_budget_with_each_other_and_support_work() {
        let keys = curve(0.25);
        let source = SourceTravel::Clip {
            keys: &keys,
            phase: phase(0., 1.),
            active_fraction: 1.,
        };
        let mut budget = voxy_gameplay::SupportQueryBudget::new(12).unwrap();
        assert!(super::mixed_swing(&keys, phase(0., 1.), Some(source), &mut budget).unwrap());
        assert!(super::mixed_swing(&keys, phase(0., 1.), Some(source), &mut budget).is_err());
        let mut budget = voxy_gameplay::SupportQueryBudget::new(8).unwrap();
        budget.charge_work(5).unwrap();
        assert!(super::mixed_swing(&keys, phase(0., 1.), None, &mut budget).unwrap());
        assert_eq!(budget.remaining(), 0);
        assert!(budget.charge_work(1).is_err());
        assert_eq!(budget.remaining(), 0);
    }
}
