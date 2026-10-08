use physics::hair::{HairMaterial, HairRod, HairSystem, RootPose, TriangleMesh};
fn root() -> RootPose {
    RootPose {
        position: [0.; 3],
        rotation: [0., 0., 0., 1.],
    }
}
fn straight(length: f64, n: usize, material: HairMaterial) -> HairRod {
    HairRod::new(
        (0..=n)
            .map(|i| [length * i as f64 / n as f64, 0., 0.])
            .collect(),
        material,
    )
    .unwrap()
}
fn system(rod: HairRod) -> HairSystem {
    let mut s = HairSystem::new(vec![rod]).unwrap();
    s.self_collision = false;
    s.iterations = 32;
    s
}
#[test]
fn joint_contact_failure_rolls_back_pose_and_future_dynamics() {
    let rod=HairRod::new(vec![[0.,0.,0.],[0.,0.01,0.],[0.,0.02,0.]],HairMaterial::default()).unwrap();
    let mut actual=system(rod);actual.iterations=1;actual.substeps=1;actual.joint_contact_velocities=true;
    let mut control=actual.clone();let dt=1./240.;
    let points=[[0.,-1.,-1.],[0.,1.,-1.],[0.,0.,1.]];
    let mut walls=Vec::new();
    for (shift,face) in [(-dt,[0,1,2]),(dt,[0,2,1])] {
        let previous=points.map(|mut point| {point[0]=shift;point});
        let mut wall=TriangleMesh::new(&previous,&[face]).unwrap();
        wall.refit_with_timestep(&points,dt).unwrap();walls.push(wall);
    }
    assert!(actual.step(dt,&[root()],[0.;3],[0.;3],&walls).is_err(),"opposing surface velocities must reject the staged step");
    assert_eq!(actual.rods()[0].positions(),control.rods()[0].positions());
    assert_eq!(actual.rods()[0].orientations(),control.rods()[0].orientations());
    actual.joint_contact_velocities=false;control.joint_contact_velocities=false;
    for _ in 0..3 {
        actual.step(dt,&[root()],[0.,-9.81,0.],[0.;3],&[]).unwrap();
        control.step(dt,&[root()],[0.,-9.81,0.],[0.;3],&[]).unwrap();
        assert_eq!(actual.rods()[0].positions(),control.rods()[0].positions());
        assert_eq!(actual.rods()[0].orientations(),control.rods()[0].orientations());
    }
}
#[test]
fn constitutive_units_and_rest_equilibrium() {
    let material = HairMaterial::default();
    let mut s = system(straight(0.2, 16, material));
    let mass = material.density * material.area() * 0.2;
    assert!((s.rods()[0].mass() / mass - 1.).abs() < 1e-12);
    let original = s.rods()[0].positions().to_vec();
    for _ in 0..120 {
        s.step(1. / 120., &[root()], [0.; 3], [0.; 3], &[]).unwrap();
    }
    let error = s.rods()[0]
        .positions()
        .iter()
        .zip(original)
        .map(|(a, b)| {
            a.iter()
                .zip(b)
                .map(|(x, y)| (x - y).powi(2))
                .sum::<f64>()
                .sqrt()
        })
        .fold(0., f64::max);
    assert!(error < 1e-8, "rest drift {error}");
}
#[test]
fn gravity_sag_is_material_dependent_and_lengths_remain_bounded() {
    let mut soft = system(straight(0.10, 12, HairMaterial::default()));
    let mut stiff = system(straight(
        0.10,
        12,
        HairMaterial {
            young_modulus: 4e11,
            ..Default::default()
        },
    ));
    for _ in 0..240 {
        for s in [&mut soft, &mut stiff] {
            s.step(1. / 120., &[root()], [0., -9.81, 0.], [0.; 3], &[])
                .unwrap();
        }
    }
    let sy = soft.rods()[0].positions()[12][1];
    let hy = stiff.rods()[0].positions()[12][1];
    println!(
        "soft sag={sy}, stiff sag={hy}, stretches={} / {}",
        soft.rods()[0].max_relative_stretch(),
        stiff.rods()[0].max_relative_stretch()
    );
    assert!(sy < -0.025 && sy < hy * 1.5, "sag soft {sy}, stiff {hy}");
    assert!(soft.rods()[0].max_relative_stretch() < 0.02);
}
#[test]
fn rigid_rotation_of_root_rotates_follicle_frame_and_free_end_has_inertia() {
    let mut s = system(straight(0.2, 16, HairMaterial::default()));
    let angle = 0.3f64;
    let r = RootPose {
        position: [0., 0.01, 0.],
        rotation: [0., 0., (angle / 2.).sin(), (angle / 2.).cos()],
    };
    s.step(1. / 120., &[r], [0.; 3], [0.; 3], &[]).unwrap();
    assert_eq!(s.rods()[0].positions()[0], r.position);
    let tip = s.rods()[0].positions()[16];
    assert!(tip[1] < 0.03, "tip teleported with root {tip:?}");
    let q = s.rods()[0].orientations()[0];
    assert!(q.iter().all(|v| v.is_finite()));
    assert!(s.rods()[0].max_relative_stretch() < 0.05);
}
#[test]
fn segment_contact_detects_plane_between_particles() {
    // Floor normal +Y; middle segment crosses it despite both nodes lying away from contact.
    let mesh = TriangleMesh::new(
        &[[-1., 0., -1.], [1., 0., -1.], [1., 0., 1.], [-1., 0., 1.]],
        &[[0, 2, 1], [0, 3, 2]],
    )
    .unwrap();
    let rod = HairRod::new(
        vec![
            [0., 0.08, 0.],
            [0.015, 0.04, 0.],
            [0.025, -0.01, 0.],
            [0.045, -0.04, 0.],
        ],
        HairMaterial::default(),
    )
    .unwrap();
    let mut s = system(rod);
    s.contact_radius = 0.001;
    s.iterations = 64;
    let r = RootPose {
        position: [0., 0.08, 0.],
        ..root()
    };
    for _ in 0..60 {
        s.step(
            1. / 120.,
            &[r],
            [0., -9.81, 0.],
            [0.; 3],
            std::slice::from_ref(&mesh),
        )
        .unwrap();
    }
    let min = s.rods()[0]
        .positions()
        .iter()
        .map(|p| p[1])
        .fold(f64::INFINITY, f64::min);
    println!(
        "floor minimum {min}, stretch {}",
        s.rods()[0].max_relative_stretch()
    );
    assert!(min >= 0.0009, "penetration {min}");
    assert!(s.rods()[0].max_relative_stretch() < 0.02);
}
#[test]
fn invalid_step_does_not_change_positions_or_frames() {
    let mut s = system(straight(0.1, 8, HairMaterial::default()));
    let p = s.rods()[0].positions().to_vec();
    let q = s.rods()[0].orientations().to_vec();
    assert!(s.step(f64::NAN, &[root()], [0.; 3], [0.; 3], &[]).is_err());
    assert_eq!(p, s.rods()[0].positions());
    assert_eq!(q, s.rods()[0].orientations());
    assert!(HairRod::new(vec![[0.; 3]; 3], HairMaterial::default()).is_err());
}
#[test]
fn timestep_refinement_preserves_gravity_response() {
    let mut a = system(straight(0.1, 12, HairMaterial::default()));
    let mut b = a.clone();
    b.substeps = 8;
    for _ in 0..120 {
        a.step(1. / 120., &[root()], [0., -9.81, 0.], [0.; 3], &[])
            .unwrap();
        b.step(1. / 120., &[root()], [0., -9.81, 0.], [0.; 3], &[])
            .unwrap();
    }
    let pa = a.rods()[0].positions()[12];
    let pb = b.rods()[0].positions()[12];
    let delta = pa
        .iter()
        .zip(pb)
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f64>()
        .sqrt();
    println!("timestep refinement tip error={delta}, {pa:?} / {pb:?}");
    assert!(delta < 0.01, "timestep dependence {delta}");
}

#[test]
fn crossing_segments_separate_without_shared_particles() {
    let material = HairMaterial::default();
    let a = HairRod::new(
        vec![
            [-0.1, 0., 0.],
            [-0.05, 0., 0.],
            [0.05, 0., 0.],
            [0.1, 0., 0.],
        ],
        material,
    )
    .unwrap();
    let b = HairRod::new(
        vec![
            [0., 0., -0.1],
            [0., 0., -0.05],
            [0., 0., 0.05],
            [0., 0., 0.1],
        ],
        material,
    )
    .unwrap();
    let mut s = HairSystem::new(vec![a, b]).unwrap();
    s.contact_radius = 0.001;
    s.iterations = 32;
    let roots = [
        RootPose {
            position: [-0.1, 0., 0.],
            ..root()
        },
        RootPose {
            position: [0., 0., -0.1],
            ..root()
        },
    ];
    for _ in 0..10 {
        s.step(1. / 120., &roots, [0.; 3], [0.; 3], &[]).unwrap();
    }
    let a = s.rods()[0].positions();
    let b = s.rods()[1].positions();
    let a_mid = std::array::from_fn::<_, 3, _>(|i| (a[1][i] + a[2][i]) * 0.5);
    let b_mid = std::array::from_fn::<_, 3, _>(|i| (b[1][i] + b[2][i]) * 0.5);
    let distance = a_mid
        .iter()
        .zip(b_mid)
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f64>()
        .sqrt();
    println!("crossed capsules separation={distance}");
    assert!(distance > 0.0015);
}

#[test]
fn small_deflection_cantilever_matches_beam_law() {
    let material = HairMaterial {
        damping: 12.,
        air_drag: 0.,
        ..Default::default()
    };
    let length = 0.02;
    let mut s = system(straight(length, 24, material));
    s.iterations = 64;
    for _ in 0..360 {
        s.step(1. / 120., &[root()], [0., -9.81, 0.], [0.; 3], &[])
            .unwrap();
    }
    // First segment frame is clamped; compare effective free length.
    let free = length * (23. / 24.);
    let predicted = material.density * material.area() * 9.81 * free.powi(4)
        / (8. * material.bending_rigidity());
    let actual = -s.rods()[0].positions()[24][1];
    println!(
        "cantilever: measured={actual}, beam={predicted}, ratio={}, stretch={}, tipframe={:?}",
        actual / predicted,
        s.rods()[0].max_relative_stretch(),
        s.rods()[0].orientations().last()
    );
    assert!((actual / predicted - 1.).abs() < 0.30);
}

#[test]
fn parallel_guides_match_serial_solver_with_surface_contacts() {
    let mut serial = HairSystem::new(
        (0..40)
            .map(|i| {
                HairRod::new(
                    (0..=6)
                        .map(|j| [j as f64 * 0.01, i as f64 * 0.004 + 0.01, 0.])
                        .collect(),
                    HairMaterial::default(),
                )
                .unwrap()
            })
            .collect(),
    )
    .unwrap();
    serial.workers = 1;
    let roots: Vec<_> = serial
        .rods()
        .iter()
        .map(|rod| RootPose {
            position: rod.positions()[0],
            ..root()
        })
        .collect();
    let floor = TriangleMesh::new(
        &[[-1., 0., -1.], [1., 0., -1.], [1., 0., 1.], [-1., 0., 1.]],
        &[[0, 2, 1], [0, 3, 2]],
    )
    .unwrap();
    let initial = serial;
    for self_collision in [false, true] {
        for iterations in [1, 3, 4, 5, 6, 9] {
            let mut serial = initial.clone();
            serial.self_collision = self_collision;
            serial.iterations = iterations;
            let mut parallel = serial.clone();
            parallel.workers = 4;
            parallel.profiling = true;
            for _ in 0..4 {
                for system in [&mut serial, &mut parallel] {
                    system
                        .step(
                            1. / 120.,
                            &roots,
                            [0., -9.81, 0.],
                            [0.; 3],
                            std::slice::from_ref(&floor),
                        )
                        .unwrap();
                }
            }
            assert!(parallel.last_profile.structural_ms > 0.);
            assert!(parallel.last_profile.mesh_contacts_ms > 0.);
            assert_eq!(parallel.last_profile.self_contacts_ms > 0., self_collision);
            assert_eq!(serial.last_profile.structural_ms, 0.);
            for (a, b) in serial.rods().iter().zip(parallel.rods()) {
                assert_eq!(a.positions(), b.positions());
                assert_eq!(a.orientations(), b.orientations());
            }
        }
    }
}

#[test]
fn axial_root_rotation_transmits_material_twist_without_bending_straight_fiber() {
    let material = HairMaterial {
        damping: 8.0,
        air_drag: 0.0,
        ..Default::default()
    };
    let rod = straight(0.10, 12, material);
    let rest = rod.orientations()[0];
    let mut system = system(rod);
    let angle = 0.4f64;
    let rotation = [(angle / 2.).sin(), 0., 0., (angle / 2.).cos()];
    let target = [
        rotation[0] * rest[3],
        rotation[3] * rest[1],
        rotation[0] * rest[1],
        rotation[3] * rest[3],
    ];
    let pose = RootPose { rotation, ..root() };
    for _ in 0..240 {
        system
            .step(1. / 120., &[pose], [0.; 3], [0.; 3], &[])
            .unwrap();
    }
    let tip_frame = system.rods()[0].orientations().last().unwrap();
    let cosine = tip_frame
        .iter()
        .zip(target)
        .map(|(a, b)| a * b)
        .sum::<f64>()
        .abs()
        .min(1.0);
    let error = 2.0 * cosine.acos();
    let tip = system.rods()[0].positions()[12];
    println!("axial twist equilibrium error={error} rad, tip={tip:?}");
    assert!(error < 0.02);
    assert!((tip[0] - 0.10).abs() < 1e-6 && tip[1].abs() < 1e-6 && tip[2].abs() < 1e-6);
}

#[test]
fn surface_refit_invalidates_clearance_from_previous_step() {
    let vertices = [[-1., 0., -1.], [1., 0., -1.], [1., 0., 1.], [-1., 0., 1.]];
    let mut mesh = TriangleMesh::new(&vertices, &[[0, 2, 1], [0, 3, 2]]).unwrap();
    let rod = HairRod::new(
        vec![[0., 0.03, 0.], [0.01, 0.03, 0.], [0.02, 0.03, 0.]],
        HairMaterial::default(),
    )
    .unwrap();
    let mut s = system(rod);
    let root = RootPose {
        position: [0., 0.03, 0.],
        ..root()
    };
    s.step(
        1. / 120.,
        &[root],
        [0.; 3],
        [0.; 3],
        std::slice::from_ref(&mesh),
    )
    .unwrap();
    let raised = vertices.map(|p| [p[0], p[1] + 0.032, p[2]]);
    mesh.refit(&raised).unwrap();
    s.step(
        1. / 120.,
        &[root],
        [0.; 3],
        [0.; 3],
        std::slice::from_ref(&mesh),
    )
    .unwrap();
    assert!(
        s.rods()[0].positions()[2][1] > 0.031,
        "refitted surface was ignored"
    );
}

#[test]
fn linear_system_capture_preserves_dynamic_state_and_next_step() {
    let mut actual = system(straight(0.2, 20, HairMaterial::default()));
    actual.step(1./120., &[root()], [0.,-9.81,0.], [0.;3], &[]).unwrap();
    let mut reference = actual.clone();
    let positions = actual.rods()[0].positions().to_vec();
    let orientations = actual.rods()[0].orientations().to_vec();
    let assembled = actual.rods()[0].linear_system(1./240.).unwrap();
    assert_eq!(assembled.band_width, 9);
    assert_eq!(assembled.rhs.len(), 126);
    assert_eq!(assembled.matrix.len(), 126*9);
    assert_eq!(assembled.active, 6..123);
    assert!(assembled.matrix.iter().chain(&assembled.rhs).all(|v| v.is_finite()));
    assert!(assembled.active.clone().all(|i| assembled.matrix[i*9] > 0.));
    assert_eq!(actual.rods()[0].positions(), positions);
    assert_eq!(actual.rods()[0].orientations(), orientations);
    for dt in [0., -1., f64::NAN, f64::INFINITY, 0.1] {
        assert!(actual.rods()[0].linear_system(dt).is_err());
    }
    for system in [&mut actual, &mut reference] {
        system.step(1./120., &[root()], [0.,-9.81,0.], [0.;3], &[]).unwrap();
    }
    assert_eq!(actual.rods()[0].positions(), reference.rods()[0].positions());
    assert_eq!(actual.rods()[0].orientations(), reference.rods()[0].orientations());
}

struct NativeBatch { calls: usize, corrupt: Option<usize>, fail_after: Option<usize> }
impl physics::hair::HairLinearSolver for NativeBatch {
    fn solve(&mut self,systems:&[physics::hair::HairLinearSystem])->Result<Vec<Vec<f64>>, &'static str> {
        self.calls+=1;
        if self.fail_after==Some(self.calls) {return Err("injected accelerator failure");}
        let mut corrections=systems.iter().map(|s|s.solve_native()).collect::<Result<Vec<_>,_>>()?;
        match self.corrupt {
            Some(0)=>{corrections.pop();},
            Some(1)=>{corrections[0][6]=f64::NAN;},
            Some(2)=>{corrections[0][0]=1.;},
            Some(3)=>{corrections[0][6]+=1.;},
            _=>{}
        }
        Ok(corrections)
    }
}
fn accelerator_fixture()->(HairSystem,Vec<RootPose>,TriangleMesh) {
    let curves:Vec<_>=(0..8).map(|rod|(0..=8).map(|point|[rod as f64*0.001,0.04,point as f64*0.01]).collect::<Vec<_>>()).collect();
    let roots=curves.iter().map(|curve|RootPose {position:curve[0],rotation:[0.,0.,0.,1.]}).collect();
    let rods=curves.into_iter().map(|curve|HairRod::new(curve,HairMaterial::default()).unwrap()).collect();
    let floor=TriangleMesh::new(&[[-1.,0.,-1.],[1.,0.,-1.],[1.,0.,1.],[-1.,0.,1.]],&[[0,2,1],[0,3,2]]).unwrap();
    (HairSystem::new(rods).unwrap(),roots,floor)
}
#[test]
fn external_batches_preserve_native_contact_order_and_dynamic_state() {
    let (initial,roots,floor)=accelerator_fixture();
    for workers in [1,4] {for iterations in [1,4,5,9] {
        let mut native=initial.clone();native.workers=workers;native.iterations=iterations;
        let mut external=native.clone();let mut solver=NativeBatch {calls:0,corrupt:None,fail_after:None};
        for _ in 0..4 {
            native.step(1./120.,&roots,[0.,-9.81,0.],[0.;3],std::slice::from_ref(&floor)).unwrap();
            external.step_with_solver(1./120.,&roots,[0.,-9.81,0.],[0.;3],std::slice::from_ref(&floor),&mut solver).unwrap();
            for (a,b) in native.rods().iter().zip(external.rods()) {
                assert_eq!(a.positions(),b.positions());assert_eq!(a.orientations(),b.orientations());
            }
        }
        assert_eq!(solver.calls,4*initial.substeps*iterations);
    }}
}
#[test]
fn external_failures_rollback_all_substeps_and_hidden_state() {
    let (initial,roots,floor)=accelerator_fixture();
    for corruption in [None,Some(0),Some(1),Some(2),Some(3)] {
        let mut actual=initial.clone();let mut reference=initial.clone();
        let mut solver=NativeBatch {calls:0,corrupt:corruption,fail_after:if corruption.is_none() {Some(3)} else {None}};
        assert!(actual.step_with_solver(1./120.,&roots,[0.,-9.81,0.],[0.;3],std::slice::from_ref(&floor),&mut solver).is_err());
        for (a,b) in actual.rods().iter().zip(initial.rods()) {
            assert_eq!(a.positions(),b.positions());assert_eq!(a.orientations(),b.orientations());
        }
        for system in [&mut actual,&mut reference] {system.step(1./120.,&roots,[0.,-9.81,0.],[0.;3],std::slice::from_ref(&floor)).unwrap();}
        for (a,b) in actual.rods().iter().zip(reference.rods()) {
            assert_eq!(a.positions(),b.positions());assert_eq!(a.orientations(),b.orientations());
        }
    }
}

#[test]
fn native_numerical_failure_rolls_back_pose_and_next_step_state() {
    let mut candidate=system(straight(0.1,4,HairMaterial::default()));
    candidate.iterations=1;candidate.substeps=2;
    let mut reference=candidate.clone();
    let extreme=RootPose {position:[1e308;3],..root()};
    assert!(candidate.step(1./120.,&[extreme],[0.;3],[0.;3],&[]).is_err());
    assert_eq!(candidate.rods()[0].positions(),reference.rods()[0].positions());
    assert_eq!(candidate.rods()[0].orientations(),reference.rods()[0].orientations());
    for _ in 0..3 {
        candidate.step(1./120.,&[root()],[0.,-9.81,0.],[0.;3],&[]).unwrap();
        reference.step(1./120.,&[root()],[0.,-9.81,0.],[0.;3],&[]).unwrap();
        assert_eq!(candidate.rods()[0].positions(),reference.rods()[0].positions());
        assert_eq!(candidate.rods()[0].orientations(),reference.rods()[0].orientations());
    }
}

#[test]
fn terminal_contact_accelerator_failure_rolls_back_reconciled_state() {
    let (mut initial,roots,floor)=accelerator_fixture();
    initial.iterations=1;initial.substeps=2;initial.terminal_contact_iterations=2;
    let mut actual=initial.clone();let mut reference=initial.clone();
    let mut failure=NativeBatch {calls:0,corrupt:None,fail_after:Some(2)};
    assert!(actual.step_with_solver(1./120.,&roots,[0.,-9.81,0.],[0.;3],std::slice::from_ref(&floor),&mut failure).is_err());
    for (a,b) in actual.rods().iter().zip(reference.rods()) {
        assert_eq!(a.positions(),b.positions());assert_eq!(a.orientations(),b.orientations());
    }
    let mut good=NativeBatch {calls:0,corrupt:None,fail_after:None};
    for _ in 0..3 {
        actual.step_with_solver(1./120.,&roots,[0.,-9.81,0.],[0.;3],std::slice::from_ref(&floor),&mut good).unwrap();
        reference.step(1./120.,&roots,[0.,-9.81,0.],[0.;3],std::slice::from_ref(&floor)).unwrap();
        for (a,b) in actual.rods().iter().zip(reference.rods()) {
            assert_eq!(a.positions(),b.positions());assert_eq!(a.orientations(),b.orientations());
        }
    }
    assert_eq!(good.calls,18);
}

#[test]
fn incompatible_joint_position_contacts_rollback_pose_and_future_dynamics() {
    let rod=HairRod::new(vec![[0.,0.,0.],[0.,0.01,0.],[0.,0.02,0.]],HairMaterial::default()).unwrap();
    let mut actual=system(rod);actual.iterations=1;actual.substeps=1;
    actual.joint_contact_positions=true;let mut control=actual.clone();let dt=1./240.;
    let points=[[0.,-1.,-1.],[0.,1.,-1.],[0.,0.,1.]];
    let walls=[TriangleMesh::new(&points,&[[0,1,2]]).unwrap(),TriangleMesh::new(&points,&[[0,2,1]]).unwrap()];
    assert!(actual.step(dt,&[root()],[0.;3],[0.;3],&walls).is_err());
    assert_eq!(actual.rods()[0].positions(),control.rods()[0].positions());
    assert_eq!(actual.rods()[0].orientations(),control.rods()[0].orientations());
    for _ in 0..3 {
        actual.step(dt,&[root()],[0.,-9.81,0.],[0.;3],&[]).unwrap();
        control.step(dt,&[root()],[0.,-9.81,0.],[0.;3],&[]).unwrap();
        assert_eq!(actual.rods()[0].positions(),control.rods()[0].positions());
        assert_eq!(actual.rods()[0].orientations(),control.rods()[0].orientations());
    }
}

#[test]
fn joint_position_contacts_preserve_worker_and_accelerator_equivalence() {
    let (mut initial,roots,floor)=accelerator_fixture();
    initial.iterations=3;initial.substeps=2;initial.contact_radius=0.0006;
    initial.joint_contact_positions=true;
    let mut serial=initial.clone();let mut parallel=initial.clone();parallel.workers=4;
    let mut external=parallel.clone();let mut solver=NativeBatch {calls:0,corrupt:None,fail_after:None};
    for _ in 0..3 {
        serial.step(1./120.,&roots,[0.,-9.81,0.],[0.;3],std::slice::from_ref(&floor)).unwrap();
        parallel.step(1./120.,&roots,[0.,-9.81,0.],[0.;3],std::slice::from_ref(&floor)).unwrap();
        external.step_with_solver(1./120.,&roots,[0.,-9.81,0.],[0.;3],std::slice::from_ref(&floor),&mut solver).unwrap();
        for ((a,b),c) in serial.rods().iter().zip(parallel.rods()).zip(external.rods()) {
            assert_eq!(a.positions(),b.positions());assert_eq!(a.orientations(),b.orientations());
            assert_eq!(a.positions(),c.positions());assert_eq!(a.orientations(),c.orientations());
        }
    }
    assert_eq!(solver.calls,18);
}
