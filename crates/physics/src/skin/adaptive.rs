use super::{Attachment, ContactScene, Point, Skin, SolverConfig, StepReport};

impl Skin {
    /// Advance an entire interval atomically, bisecting failed nonlinear solves.
    /// Attachment targets follow their original linear trajectory throughout retries.
    /// For moving obstacles use `step_adaptive_with_contacts`.
    /// The report sums accepted iterations, retains the maximum accepted residual,
    /// minimum area ratio, and final substep energy.
    pub fn step_adaptive(
        &mut self,
        dt: f64,
        acceleration: Point,
        forces: &[Point],
        attachments: &[Attachment],
        config: SolverConfig,
        max_depth: usize,
    ) -> Result<StepReport, &'static str> {
        self.step_adaptive_with_contacts(dt,acceleration,forces,attachments,&ContactScene::default(),config,max_depth)
    }

    /// Atomically advance with moving analytic contacts. Every retry shifts the
    /// original obstacle and attachment trajectories to its own start time.
    /// Failed intervals never publish a partially integrated skin state.
    pub fn step_adaptive_with_contacts(
        &mut self,dt:f64,acceleration:Point,forces:&[Point],
        attachments:&[Attachment],contacts:&ContactScene,
        config:SolverConfig,max_depth:usize,
    )->Result<StepReport,&'static str> {
        let (staged,report)=self.prepare_adaptive_with_contacts(dt,acceleration,forces,attachments,contacts,config,max_depth)?;
        *self=staged;
        Ok(report)
    }
    /// Return an admitted candidate without changing this skin.
    pub fn prepare_adaptive_with_contacts(
        &self,dt:f64,acceleration:Point,forces:&[Point],attachments:&[Attachment],
        contacts:&ContactScene,config:SolverConfig,max_depth:usize,
    )->Result<(Self,StepReport),&'static str> {
        if max_depth > 12 {
            return Err("invalid skin subdivision depth");
        }
        let mut staged = self.clone();
        let report = advance(
            &mut staged,
            dt,
            0.0,
            acceleration,
            forces,
            attachments,
            contacts,
            config,
            max_depth,
        )?;
        Ok((staged,report))
    }
}

fn advance(
    skin: &mut Skin,
    dt: f64,
    elapsed: f64,
    acceleration: Point,
    forces: &[Point],
    attachments: &[Attachment],
    contacts: &ContactScene,
    config: SolverConfig,
    depth: usize,
) -> Result<StepReport, &'static str> {
    let shifted = if elapsed==0. {std::borrow::Cow::Borrowed(attachments)} else {std::borrow::Cow::Owned(attachments
        .iter()
        .map(|a| Attachment {
            target: std::array::from_fn(|k| a.target[k] + elapsed * a.velocity[k]),
            ..*a
        })
        .collect::<Vec<_>>())};
    // The first attempt uses the original start geometry. Empty contact scenes
    // need no storage on any retry. Only moving retry intervals own a copy.
    let shifted_contacts=if elapsed==0. || (contacts.spheres.is_empty() && contacts.planes.is_empty()) {
        std::borrow::Cow::Borrowed(contacts)
    } else {
    let mut shifted_contacts=contacts.clone();
    for sphere in &mut shifted_contacts.spheres {
        sphere.center=std::array::from_fn(|k|sphere.center[k]+elapsed*sphere.velocity[k]);
    }
    for plane in &mut shifted_contacts.planes {
        plane.offset+=elapsed*plane.normal.iter().zip(plane.velocity).map(|(n,v)|n*v).sum::<f64>();
    }
    std::borrow::Cow::Owned(shifted_contacts)
    };
    match skin.step_with_contacts(dt, acceleration, forces, &shifted, &shifted_contacts, config) {
        Ok(report) => Ok(report),
        Err(error)
            if depth > 0
                && matches!(
                    error,
                    "skin Newton did not converge"
                        | "skin line search failed"
                        | "skin swept contact or element collapse"
                        | "skin contact overlap"
                ) =>
        {
            let first = advance(
                skin,
                dt * 0.5,
                elapsed,
                acceleration,
                forces,
                attachments,
                contacts,
                config,
                depth - 1,
            )?;
            let second = advance(
                skin,
                dt * 0.5,
                elapsed + dt * 0.5,
                acceleration,
                forces,
                attachments,
                contacts,
                config,
                depth - 1,
            )?;
            Ok(StepReport {
                iterations: first.iterations + second.iterations,
                residual: first.residual.max(second.residual),
                energy: second.energy,
                min_area_ratio: first.min_area_ratio.min(second.min_area_ratio),
            })
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod moving_contact_tests {
    use super::*;
    use crate::skin::{ContactPlane,ContactSphere,SkinMaterial,patch};

    #[test]
    fn subinterval_uses_original_moving_sphere_and_plane_trajectory() {
        let mut actual=patch(2,2,0.01,SkinMaterial::default()).unwrap();
        let mut expected=actual.clone();
        let mut frozen=actual.clone();
        let scene=ContactScene {
            spheres:vec![ContactSphere {center:[0.005,0.005,-0.012],radius:0.0005,velocity:[0.,0.,0.01]}],
            planes:vec![ContactPlane {normal:[0.,0.,1.],offset:-0.009,velocity:[0.02,0.,0.01]}],
            distance:0.02,stiffness:1.,
        };
        let elapsed=0.125;
        let shifted=ContactScene {
            spheres:vec![ContactSphere {center:[0.005,0.005,-0.01075],..scene.spheres[0]}],
            planes:vec![ContactPlane {offset:-0.00775,..scene.planes[0]}],
            ..scene.clone()
        };
        let forces=vec![[0.;3];actual.positions().len()];
        let config=SolverConfig::default();
        advance(&mut actual,0.001,elapsed,[0.;3],&forces,&[],&scene,config,0).unwrap();
        expected.step_with_contacts(0.001,[0.;3],&forces,&[],&shifted,config).unwrap();
        frozen.step_with_contacts(0.001,[0.;3],&forces,&[],&scene,config).unwrap();
        // Independently written decimal endpoints may differ by input ulps.
        for (a,b) in actual.positions().iter().flatten().zip(expected.positions().iter().flatten()) {assert!((a-b).abs()<1e-18);}
        for (a,b) in actual.velocities().iter().flatten().zip(expected.velocities().iter().flatten()) {assert!((a-b).abs()<1e-15);}
        assert_ne!(actual.positions(),frozen.positions(),"moving contact must affect the solution");
    }

    #[test]
    fn rejected_contact_interval_preserves_skin_state() {
        let mut skin=patch(2,2,0.01,SkinMaterial::default()).unwrap();
        let positions=skin.positions().to_vec();let velocities=skin.velocities().to_vec();
        let scene=ContactScene {planes:vec![ContactPlane {normal:[0.,0.,1.],offset:0.01,velocity:[0.;3]}],..Default::default()};
        let forces=vec![[0.;3];positions.len()];
        assert!(skin.step_adaptive_with_contacts(0.001,[0.;3],&forces,&[],&scene,SolverConfig::default(),6).is_err());
        assert_eq!(skin.positions(),positions);assert_eq!(skin.velocities(),velocities);
    }
    #[test]
    fn later_moving_contact_failure_rolls_back_accepted_first_half() {
        let mut skin=Skin::new(vec![[0.,0.,0.],[0.02,0.,0.],[0.,0.02,0.]],
            vec![[0,1,2]],&[0],SkinMaterial::default(),vec![[1.,0.,0.]]).unwrap();
        let original=skin.clone();
        let scene=ContactScene {planes:vec![ContactPlane {
            normal:[0.,0.,1.],offset:-0.003,velocity:[0.,0.,0.4],
        }],..Default::default()};
        let forces=vec![[0.;3];3];
        let mut half=skin.clone();
        half.step_adaptive_with_contacts(0.005,[0.,0.,1.],&forces,&[],&scene,SolverConfig::default(),6).unwrap();
        assert_ne!(half.positions(),original.positions(),"first half must actually advance");
        assert!(skin.step_adaptive_with_contacts(0.01,[0.,0.,1.],&forces,&[],&scene,SolverConfig::default(),6).is_err());
        assert_eq!(skin.positions(),original.positions());
        assert_eq!(skin.velocities(),original.velocities());
        assert_eq!(skin.stored_energy().unwrap(),original.stored_energy().unwrap());
    }

}
