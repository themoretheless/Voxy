//! Read-only snapshots of the selected guide's contact-connected component.
use super::{HairPhaseDiagnostic, HairRod, HairSystem, contact::StrandResponse, math::V};

#[derive(Clone, Debug)]
pub struct HairProjectionPairDiagnostic {
    pub pair_index: usize,
    pub a: (usize, usize, f64),
    pub b: (usize, usize, f64),
    pub normal: V,
    pub position_impulse: f64,
}

#[derive(Clone, Debug)]
pub struct HairContactProjectionDiagnostic {
    pub substep: usize,
    pub structural_iteration: usize,
    pub iteration: usize,
    pub rod: usize,
    pub before: Vec<HairPhaseDiagnostic>,
    pub after: Vec<HairPhaseDiagnostic>,
    pub pairs: Vec<HairProjectionPairDiagnostic>,
}

pub(super) struct HairProjectionTrace<'a> {
    pub rod: usize,
    pub substep: usize,
    pub structural_iteration: usize,
    pub output: &'a mut Vec<HairContactProjectionDiagnostic>,
}

impl HairContactProjectionDiagnostic {
    pub(super) fn begin(
        trace: &HairProjectionTrace<'_>,
        iteration: usize,
        rods: &[HairRod],
        pairs: &[StrandResponse],
    ) -> Self {
        let mut connected = vec![false; rods.len()];
        connected[trace.rod] = true;
        loop {
            let mut changed = false;
            for pair in pairs {
                if connected[pair.a.0] != connected[pair.b.0] {
                    connected[pair.a.0] = true;
                    connected[pair.b.0] = true;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        let before = rods
            .iter()
            .enumerate()
            .filter(|(index, _)| connected[*index])
            .map(|(index, rod)| {
                HairPhaseDiagnostic::capture(
                    "contact-query",
                    trace.substep,
                    trace.structural_iteration,
                    index,
                    rod,
                )
            })
            .collect();
        let pairs = pairs
            .iter()
            .enumerate()
            .filter(|(_, pair)| connected[pair.a.0])
            .map(|(index, pair)| HairProjectionPairDiagnostic {
                pair_index: index,
                a: pair.a,
                b: pair.b,
                normal: pair.normal,
                position_impulse: pair.impulse,
            })
            .collect();
        Self {
            substep: trace.substep,
            structural_iteration: trace.structural_iteration,
            iteration,
            rod: trace.rod,
            before,
            after: Vec::new(),
            pairs,
        }
    }
    pub(super) fn record_reactions(&mut self, pairs: &[StrandResponse]) {
        for pair in &mut self.pairs {
            pair.position_impulse = pairs[pair.pair_index].impulse;
        }
    }
    pub(super) fn complete(&mut self, rods: &[HairRod]) {
        self.after = self
            .before
            .iter()
            .map(|state| {
                HairPhaseDiagnostic::capture(
                    "contact-increment",
                    self.substep,
                    self.structural_iteration,
                    state.rod,
                    &rods[state.rod],
                )
            })
            .collect();
    }
}
impl HairSystem {
    pub fn contact_projection_trace(&self) -> &[HairContactProjectionDiagnostic] {
        &self.last_projection_trace
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn observing_real_paired_contacts_preserves_parallel_step_bitwise() {
        let rods: Vec<_> = (0..8)
            .map(|index| {
                let x = if index == 1 {
                    60e-6
                } else {
                    index as f64 * 0.01
                };
                HairRod::new(
                    vec![[x, 0., 0.], [x, 0.01, 0.], [x, 0.02, 0.]],
                    Default::default(),
                )
                .unwrap()
            })
            .collect();
        let roots: Vec<_> = rods
            .iter()
            .map(|rod| super::super::RootPose {
                position: rod.x[0],
                rotation: [0., 0., 0., 1.],
            })
            .collect();
        let mut plain = HairSystem::new(rods).unwrap();
        plain.workers = 2;
        plain.iterations = 4;
        plain.substeps = 1;
        plain.joint_contact_positions = true;
        let mut traced = plain.clone();
        traced.set_trace_rod(Some(0)).unwrap();
        plain
            .step(1. / 240., &roots, [0., -9.81, 0.], [0.; 3], &[])
            .unwrap();
        traced
            .step(1. / 240., &roots, [0., -9.81, 0.], [0.; 3], &[])
            .unwrap();
        for (a, b) in plain.rods.iter().zip(&traced.rods) {
            assert_eq!(a.x, b.x);
            assert_eq!(a.q, b.q);
            assert_eq!(a.velocity, b.velocity);
            assert_eq!(a.omega, b.omega);
        }
        assert!(plain.contact_projection_trace().is_empty());
        assert!(
            traced
                .contact_projection_trace()
                .iter()
                .any(|record| record.before.len() == 2 && !record.pairs.is_empty())
        );
        if std::env::var_os("VOXY_HAIR_EMIT_COMPONENT_TRACE").is_some() {
            for record in traced.contact_projection_trace() {
                eprintln!("HAIR PROJECTION TRACE frame=1 native {record:?}");
                eprintln!("HAIR PROJECTION TRACE frame=1 external {record:?}");
            }
        }
    }
    #[test]
    fn snapshot_captures_transitive_contact_component_in_stable_rod_order() {
        let rods: Vec<_> = (0..5)
            .map(|i| {
                HairRod::new(
                    vec![
                        [i as f64, 0., 0.],
                        [i as f64, 0.01, 0.],
                        [i as f64, 0.02, 0.],
                    ],
                    Default::default(),
                )
                .unwrap()
            })
            .collect();
        let pair = |a, b| StrandResponse {
            a: (a, 1, 0.5),
            b: (b, 1, 0.5),
            normal: [1., 0., 0.],
            impulse: 0.,
        };
        let mut pairs = vec![pair(1, 2), pair(3, 4), pair(0, 1)];
        let mut output = Vec::new();
        let trace = HairProjectionTrace {
            rod: 0,
            substep: 1,
            structural_iteration: 4,
            output: &mut output,
        };
        let mut snapshot = HairContactProjectionDiagnostic::begin(&trace, 3, &rods, &pairs);
        assert_eq!(
            snapshot
                .before
                .iter()
                .map(|state| state.rod)
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert_eq!(
            snapshot
                .pairs
                .iter()
                .map(|pair| pair.pair_index)
                .collect::<Vec<_>>(),
            vec![0, 2]
        );
        pairs[0].impulse = 1e-12;
        pairs[2].impulse = 2e-12;
        snapshot.record_reactions(&pairs);
        snapshot.complete(&rods);
        assert_eq!(
            snapshot
                .pairs
                .iter()
                .map(|pair| pair.position_impulse)
                .collect::<Vec<_>>(),
            vec![1e-12, 2e-12]
        );
        assert_eq!(
            snapshot
                .after
                .iter()
                .map(|state| state.rod)
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert_eq!(snapshot.before[0].positions, snapshot.after[0].positions);
    }
}
