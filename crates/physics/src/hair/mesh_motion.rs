//! Whole staged-step admission for time-aligned mesh/guide trajectories.
use super::*;

fn guide_motions(rods: &[HairRod], radius: f64) -> (Vec<CapsuleMotion>, Vec<(usize, usize)>) {
    let mut motions = Vec::new();
    let mut ids = Vec::new();
    for (r, rod) in rods.iter().enumerate() {
        for i in 0..rod.lengths.len() {
            let mut start = [rod.old_x[i], rod.old_x[i + 1]];
            let mut end = [rod.x[i], rod.x[i + 1]];
            // Match the prescribed follicle corner of static mesh contacts.
            if i == 0 {
                start[0] = add(mul(start[0], 0.85), mul(start[1], 0.15));
                end[0] = add(mul(end[0], 0.85), mul(end[1], 0.15));
            }
            motions.push(CapsuleMotion { start, end, radius });
            ids.push((r, i));
        }
    }
    (motions, ids)
}

pub(super) fn refresh(
    rods: &mut [HairRod],
    meshes: &[TriangleMesh],
    radius: f64,
) -> Result<(), &'static str> {
    let (motions, ids) = guide_motions(rods, radius);
    for (mesh_index, mesh) in meshes.iter().enumerate() {
        for (i, witness) in mesh.trajectory_constraints(&motions, Default::default())? {
            let (r, mut segment) = ids[i];
            let mut fraction = if segment == 0 {
                0.15 + 0.85 * witness.fraction
            } else {
                witness.fraction
            };
            if fraction == 1. && segment + 1 < rods[r].lengths.len() {
                segment += 1;
                fraction = 0.;
            }
            if rods[r].contacts.iter().any(|row| {
                row.source == ContactSource::Mesh(mesh_index)
                    && row.segment == segment
                    && row.fraction == fraction
                    && row.trajectory_time == Some(witness.time)
                    && dot(row.normal, witness.normal) > 1. - 1e-12
                    && dot(sub(row.target, witness.target), witness.normal).abs() <= 1e-12
            }) {
                continue;
            }
            rods[r].contacts.push(RodContact {
                segment,
                fraction,
                normal: witness.normal,
                target: witness.target,
                source: ContactSource::Mesh(mesh_index),
                surface_velocity: [0.; 3],
                metric_scale: witness.time,
                trajectory_time: Some(witness.time),
            });
        }
    }
    Ok(())
}

pub(super) fn admit(
    rods: &[HairRod],
    meshes: &[TriangleMesh],
    radius: f64,
) -> Result<(), &'static str> {
    let (motions, ids) = guide_motions(rods, radius);
    for (mesh_index, mesh) in meshes.iter().enumerate() {
        let contacts = mesh.swept_capsule_contacts(&motions, Default::default())?;
        if let Some((i, face, query)) = contacts.first() {
            if let Some(path) = std::env::var_os("VOXY_HAIR_MESH_SWEEP_REJECT_EXPORT") {
                if let Some(triangle) = mesh.face_motion(*face) {
                    let motion = motions[*i];
                    let payload = format!(
                        "{{\"rod\":{},\"segment\":{},\"radius\":{:?},\"capsule_start\":{:?},\"capsule_end\":{:?},\"triangle_start\":{:?},\"triangle_end\":{:?}}}",
                        ids[*i].0,
                        ids[*i].1,
                        motion.radius,
                        motion.start,
                        motion.end,
                        triangle.start,
                        triangle.end
                    );
                    if let Err(error) = std::fs::write(path, payload) {
                        eprintln!("HAIR MESH SWEEP EXPORT ERROR: {error}");
                    }
                }
            }
            if std::env::var_os("VOXY_HAIR_MESH_SWEEP_TRACE").is_some() {
                if let Some(triangle) = mesh.face_motion(*face) {
                    match contact::trajectory_contact(motions[*i], triangle, Default::default()) {
                        Ok(Some(witness)) => eprintln!(
                            "HAIR TRAJECTORY CONTACT WITNESS time={} fraction={} gap={} endpoint_metric_gap={}",
                            witness.time,
                            witness.fraction,
                            witness.gap,
                            witness.endpoint_gap(motions[*i].end)
                        ),
                        Ok(None) => eprintln!(
                            "HAIR TRAJECTORY CONTACT WITNESS no discovered penetration; continuous admission remains unknown"
                        ),
                        Err(error) => eprintln!("HAIR TRAJECTORY CONTACT WITNESS ERROR: {error}"),
                    }
                }
                eprintln!(
                    "HAIR MESH SWEEP REJECT rod={} segment={} mesh={mesh_index} face={face:?} query={query:?}",
                    ids[*i].0, ids[*i].1
                );
            }
            return Err(match query {
                CapsuleSweep::InitialContact { .. } => {
                    "continuous mesh motion needs initial contact activation"
                }
                CapsuleSweep::Approach { .. } => {
                    "continuous mesh motion crosses collider contact band"
                }
                CapsuleSweep::IterationLimit { .. } => {
                    "continuous mesh motion reached query iteration limit"
                }
                CapsuleSweep::Clear => unreachable!("batch only contains non-clear queries"),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn continuous_mode_admits_imported_overlaps_before_starting_its_clock() {
        let rods:Vec<_>=[-1.,1.].into_iter().map(|side|HairRod::new(
            vec![[side*0.5e-3,0.,0.],[side*0.25e-3,0.,0.1],[0.,0.,0.2]],HairMaterial::default()).unwrap()).collect();
        let rest_lengths:Vec<_>=rods.iter().map(|rod|rod.lengths.clone()).collect();
        let roots:Vec<_>=rods.iter().map(|rod|RootPose {position:rod.x[0],rotation:[0.,0.,0.,1.]}).collect();
        let mut system=HairSystem::new(rods).unwrap();
        system.substeps=1;system.iterations=4;system.joint_contact_positions=true;
        system.continuous_mesh_admission=true;system.trace_rod=Some(0);
        let initial=contact::refresh_strand_responses(&mut system.rods,system.contact_radius,&[]);
        assert!(!contact::strand_geometry_admitted(&system.rods,&initial,system.contact_radius).unwrap());
        let vertices=[[-1.,-1.,-1.],[-1.,1.,-1.],[-1.,0.,1.]];
        let mut mesh=TriangleMesh::new(&vertices,&[[0,1,2]]).unwrap();
        mesh.refit_with_timestep(&vertices.map(|p|add(p,[0.1,0.,0.])),1./240.).unwrap();
        system.step(1./240.,&roots,[0.;3],[0.;3],&[mesh]).unwrap();
        assert!(system.swept_strand_initialized);
        assert!(system.last_trace.iter().any(|trace|trace.phase=="initial-contact-admission"));
        contact::admit_staged_strands(&system.rods,system.contact_radius).unwrap();
        for ((rod,root),lengths) in system.rods.iter().zip(&roots).zip(&rest_lengths) {
            assert_eq!(rod.x[0],root.position);
            assert_eq!(&rod.lengths,lengths,"initial admission cannot rewrite stress-free material lengths");
            assert!(rod.max_relative_stretch()<0.05);
        }
    }
    #[test]
    fn past_contact_does_not_impose_an_endpoint_velocity_constraint() {
        let mut system = system();
        let rod = &mut system.rods[0];
        rod.velocity[1] = [0., 1., 0.];
        let row = rod.record_point_contact(1, [0., 1., 0.], rod.x[1], ContactSource::Mesh(0));
        rod.contacts[row].surface_velocity = [0., 10., 0.];
        rod.contacts[row].metric_scale = 0.25;
        rod.contacts[row].trajectory_time = Some(0.25);
        contact::stabilize_contact_velocities(&mut system.rods, &[], 1. / 240., 40e-6).unwrap();
        assert_eq!(system.rods[0].velocity[1], [0., 1., 0.]);
        system.rods[0].contacts[row].trajectory_time = None;
        contact::stabilize_contact_velocities(&mut system.rods, &[], 1. / 240., 40e-6).unwrap();
        assert!(system.rods[0].velocity[1][1] >= 10. - 1e-8);
    }
    fn system() -> HairSystem {
        let rod = HairRod::new(
            vec![[0., 0., 0.], [0., 0.01, 0.], [0., 0.02, 0.]],
            HairMaterial::default(),
        )
        .unwrap();
        let mut system = HairSystem::new(vec![rod]).unwrap();
        system.substeps = 1;
        system.iterations = 4;
        system.self_collision = false;
        system
    }
    fn slab(dt: f64) -> TriangleMesh {
        let points = [
            [-1., -1., -1.01],
            [1., -1., -1.01],
            [1., 1., -1.01],
            [-1., 1., -1.01],
            [-1., -1., -0.99],
            [1., -1., -0.99],
            [1., 1., -0.99],
            [-1., 1., -0.99],
        ];
        let faces = [
            [0, 2, 1],
            [0, 3, 2],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ];
        let mut mesh = TriangleMesh::new(&points, &faces).unwrap();
        mesh.enable_closed_feature_normals().unwrap();
        mesh.refit_with_timestep(&points.map(|p| add(p, [0., 0., 2.])), dt)
            .unwrap();
        mesh
    }
    #[test]
    fn moving_closed_body_cannot_tunnel_through_a_published_step() {
        let dt = 1. / 240.;
        let mesh = slab(dt);
        let roots = [RootPose {
            position: [0.; 3],
            rotation: [0., 0., 0., 1.],
        }];
        let mut discrete = system();
        discrete
            .step(dt, &roots, [0.; 3], [0.; 3], &[mesh.clone()])
            .unwrap();
        assert!(discrete.rods[0].max_relative_stretch() < 1e-10);
        let mut continuous = system();
        continuous.continuous_mesh_admission = true;
        continuous.trace_rod = Some(0);
        let before = format!("{continuous:?}");
        assert_eq!(
            continuous.step(dt, &roots, [0.; 3], [0.; 3], &[mesh]),
            Err("continuous mesh motion crosses collider contact band")
        );
        assert_eq!(
            format!("{continuous:?}"),
            before,
            "pose, history, contact caches and diagnostics must all roll back"
        );
        continuous.step(dt, &roots, [0.; 3], [0.; 3], &[]).unwrap();
        assert_eq!(continuous.rods[0].x[0], roots[0].position);
    }
    #[test]
    fn clear_step_preserves_physics_bitwise_and_rebase_keeps_admission() {
        let dt = 1. / 240.;
        let roots = [RootPose {
            position: [0.; 3],
            rotation: [0., 0., 0., 1.],
        }];
        let mut mesh = slab(dt);
        mesh.refit(&mesh.replay_geometry().0).unwrap();
        let mut discrete = system();
        let mut continuous = discrete.clone();
        continuous.continuous_mesh_admission = true;
        discrete
            .step(dt, &roots, [0., -9.81, 0.], [0.; 3], &[mesh.clone()])
            .unwrap();
        continuous
            .step(dt, &roots, [0., -9.81, 0.], [0.; 3], &[mesh])
            .unwrap();
        assert_eq!(
            format!("{:?}", discrete.rods),
            format!("{:?}", continuous.rods)
        );
        assert!(
            continuous
                .rebased(
                    continuous
                        .rods
                        .iter()
                        .map(|rod| rod.rest_x.clone())
                        .collect()
                )
                .unwrap()
                .continuous_mesh_admission
        );
    }
}
