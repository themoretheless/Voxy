//! Optional bounded per-rod step observation; disabled in ordinary simulation.
use super::math::{Q, V};
use super::{HairContactDiagnostic, HairRod, HairSystem};
#[derive(Clone, Debug)]
pub struct HairPhaseDiagnostic {
    pub phase: &'static str,
    pub substep: usize,
    pub iteration: usize,
    pub rod: usize,
    pub positions: Vec<V>,
    pub velocities: Vec<V>,
    pub orientations: Vec<Q>,
    pub contacts: Vec<HairContactDiagnostic>,
    /// Signed axial strain of every segment, measured against its rest length.
    pub relative_strains: Vec<f64>,
}
/// Per-pair friction inputs and impulses in deterministic application order.
#[derive(Clone, Debug)]
pub struct HairFrictionDiagnostic {
    pub substep: usize,
    pub a: (usize, usize, f64),
    pub b: (usize, usize, f64),
    pub normal: V,
    pub position_impulse: f64,
    pub relative_velocity: V,
    pub normal_speed: f64,
    pub tangent_speed: f64,
    pub mobility_a: f64,
    pub mobility_b: f64,
    pub friction: f64,
    pub normal_impulse: f64,
    pub tangent_impulse: f64,
    pub applied_impulse: V,
}
impl HairPhaseDiagnostic {
    pub(super) fn capture(
        phase: &'static str,
        substep: usize,
        iteration: usize,
        index: usize,
        rod: &HairRod,
    ) -> Self {
        Self {
            phase,
            substep,
            iteration,
            rod: index,
            positions: rod.x.clone(),
            velocities: rod.velocity.clone(),
            orientations: rod.q.clone(),
            contacts: rod.contact_diagnostics(),
            relative_strains: rod.x.windows(2).zip(rod.rest_lengths()).map(|(p, length)| {
                super::math::len(super::math::sub(p[1], p[0])) / length - 1.
            }).collect(),
        }
    }
}
impl HairSystem {
    pub fn set_trace_rod(&mut self, index: Option<usize>) -> Result<(), &'static str> {
        if index.is_some_and(|index| index >= self.rods.len()) {
            return Err("hair trace rod out of range");
        }
        self.trace_rod = index;
        self.last_trace.clear();
        self.last_friction_trace.clear();
        self.last_projection_trace.clear();
        Ok(())
    }
    pub fn friction_trace(&self) -> &[HairFrictionDiagnostic] {
        &self.last_friction_trace
    }
    pub fn step_trace(&self) -> &[HairPhaseDiagnostic] {
        &self.last_trace
    }
    pub(super) fn record_phase(&mut self, phase: &'static str, substep: usize, iteration: usize) {
        if let Some(index) = self.trace_rod {
            self.last_trace.push(HairPhaseDiagnostic::capture(
                phase,
                substep,
                iteration,
                index,
                &self.rods[index],
            ));
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::hair::RootPose;
    #[test]
    fn rejected_candidate_preserves_complete_state_and_next_step() {
        let rod=HairRod::new(vec![[0.,0.,0.],[0.,0.01,0.],[0.,0.02,0.]],Default::default()).unwrap();
        let mut hair=HairSystem::new(vec![rod]).unwrap();
        hair.self_collision=false;
        hair.set_trace_rod(Some(0)).unwrap();
        let before=format!("{hair:?}");
        let mut reference=hair.clone();
        let roots=[RootPose {position:[0.;3],rotation:[0.,0.,0.,1.]}];
        let mut observed=false;
        let result=hair.step_validated(1./240.,&roots,[0.,-9.81,0.],[0.;3],&[],None,|candidate| {
            observed=true;
            assert!(!candidate.step_trace().is_empty());
            Err("application rejected candidate")
        });
        assert!(observed);
        assert_eq!(result,Err("application rejected candidate"));
        assert_eq!(format!("{hair:?}"),before);
        hair.step(1./240.,&roots,[0.,-9.81,0.],[0.;3],&[]).unwrap();
        reference.step(1./240.,&roots,[0.,-9.81,0.],[0.;3],&[]).unwrap();
        assert_eq!(format!("{hair:?}"),format!("{reference:?}"));
    }
    #[test]
    fn parallel_structural_observation_is_complete_and_preserves_motion() {
        let rods: Vec<_> = (0..8)
            .map(|index| {
                let x = index as f64 * 0.01;
                HairRod::new(
                    vec![[x, 0., 0.], [x, 0.01, 0.], [x, 0.02, 0.]],
                    Default::default(),
                )
                .unwrap()
            })
            .collect();
        let roots: Vec<_> = rods
            .iter()
            .map(|rod| RootPose {
                position: rod.x[0],
                rotation: [0., 0., 0., 1.],
            })
            .collect();
        let mut plain = HairSystem::new(rods).unwrap();
        plain.workers = 2;
        plain.substeps = 1;
        plain.iterations = 4;
        plain.joint_contact_positions = true;
        let mut traced = plain.clone();
        traced.set_trace_rod(Some(5)).unwrap();
        for _ in 0..2 {
            plain
                .step(1. / 240., &roots, [0., 0., -9.81], [0.; 3], &[])
                .unwrap();
            traced
                .step(1. / 240., &roots, [0., 0., -9.81], [0.; 3], &[])
                .unwrap();
            for (a, b) in plain.rods.iter().zip(&traced.rods) {
                assert_eq!(a.x, b.x);
                assert_eq!(a.q, b.q);
                assert_eq!(a.velocity, b.velocity);
                assert_eq!(a.omega, b.omega);
            }
            assert!(plain.step_trace().is_empty());
            assert!(plain.contact_projection_trace().is_empty());
            let structural: Vec<_> = traced
                .step_trace()
                .iter()
                .filter(|entry| entry.phase == "structural")
                .collect();
            assert_eq!(structural.len(), plain.iterations);
            assert_eq!(traced.contact_projection_trace().len(), plain.iterations);
            assert!(
                traced
                    .contact_projection_trace()
                    .iter()
                    .all(|record| record.before.len() == 1 && record.before[0].rod == 5)
            );
            assert_eq!(
                structural
                    .iter()
                    .map(|entry| entry.iteration)
                    .collect::<Vec<_>>(),
                vec![0, 1, 2, 3]
            );
            assert!(
                structural
                    .iter()
                    .all(|entry| entry.rod == 5 && entry.substep == 0)
            );
        }
    }
    #[test]
    fn tracing_preserves_native_motion_and_validates_selection() {
        let rod = HairRod::new(
            vec![[0., 0., 0.], [0., 0.01, 0.], [0., 0.02, 0.]],
            Default::default(),
        )
        .unwrap();
        let mut plain = HairSystem::new(vec![rod]).unwrap();
        plain.substeps = 1;
        plain.iterations = 2;
        plain.self_collision = false;
        let mut traced = plain.clone();
        traced.set_trace_rod(Some(0)).unwrap();
        assert!(traced.set_trace_rod(Some(1)).is_err());
        let roots = [RootPose {
            position: [0.; 3],
            rotation: [0., 0., 0., 1.],
        }];
        for _ in 0..2 {
            plain
                .step(1. / 240., &roots, [0., -9.81, 0.], [0.; 3], &[])
                .unwrap();
            traced
                .step(1. / 240., &roots, [0., -9.81, 0.], [0.; 3], &[])
                .unwrap();
            assert_eq!(plain.rods[0].x, traced.rods[0].x);
            assert_eq!(plain.rods[0].q, traced.rods[0].q);
            assert_eq!(plain.rods[0].velocity, traced.rods[0].velocity);
            assert!(plain.step_trace().is_empty());
            assert_eq!(traced.step_trace().first().unwrap().phase, "predicted");
            assert_eq!(traced.step_trace().last().unwrap().phase, "finished");
            assert!(
                traced
                    .step_trace()
                    .iter()
                    .any(|entry| entry.phase == "structural")
            );
        }
        traced.set_trace_rod(None).unwrap();
        assert!(traced.step_trace().is_empty());
    }
}
