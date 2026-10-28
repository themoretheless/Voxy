use super::*;
use crate::BoxCollider;
use glam::{Quat, Vec3};
use voxy_scene::{SceneGraph, Transform};
fn setup() -> (SceneGraph, voxy_scene::NodeId, FootContactInput) {
    let mut scene = SceneGraph::new(4);
    let owner = scene.spawn(None, Transform::default()).unwrap();
    scene
        .insert_component(
            owner,
            BoxCollider {
                half_extents: [2., 0.1, 2.],
            },
        )
        .unwrap();
    (
        scene,
        owner,
        FootContactInput {
            sole: DVec3::new(0.3, f64::from(0.1_f32), 0.2),
            up: DVec3::Y,
            grounded: true,
            plant: true,
        },
    )
}
fn budget() -> SupportQueryBudget {
    SupportQueryBudget::new(1024).unwrap()
}
#[test]
fn contact_holds_surface_point_through_animation_drift_and_platform_transform() {
    let (mut scene, owner, mut input) = setup();
    let settings = FootContactSettings::default();
    let world = SupportWorld::from_scene(&scene, 1).unwrap();
    let planted = FootContactState::default()
        .prepare(&world, settings, input, &mut budget())
        .unwrap();
    assert_eq!(planted.status, FootContactStatus::Planted);
    let point = planted.contact.unwrap().position;
    input.sole.x += 0.2;
    let held = planted
        .state
        .prepare(&world, settings, input, &mut budget())
        .unwrap();
    assert_eq!(held.contact.unwrap().position, point);
    let moved = Transform {
        translation: Vec3::new(0.1, 0.02, 0.),
        rotation: Quat::from_rotation_y(0.2),
        ..Default::default()
    };
    scene.set_local(owner, moved).unwrap();
    let matrix = scene.world_matrix(owner).unwrap().as_dmat4();
    let expected = matrix.transform_point3(point);
    let world = SupportWorld::from_scene(&scene, 1).unwrap();
    let held = held
        .state
        .prepare(&world, settings, input, &mut budget())
        .unwrap();
    assert!(held.contact.unwrap().position.abs_diff_eq(expected, 1e-14));
    assert_eq!(held.state.anchor(), planted.state.anchor());
}
#[test]
fn release_hysteresis_prevents_replanting_until_swing_and_landing_rearms() {
    let (scene, _, mut input) = setup();
    let settings = FootContactSettings::default();
    let world = SupportWorld::from_scene(&scene, 1).unwrap();
    let state = FootContactState::default()
        .prepare(&world, settings, input, &mut budget())
        .unwrap()
        .state;
    input.sole.x += 0.36;
    let released = state
        .prepare(&world, settings, input, &mut budget())
        .unwrap();
    assert_eq!(released.status, FootContactStatus::Released);
    input.sole.x -= 0.36;
    let blocked = released
        .state
        .prepare(&world, settings, input, &mut budget())
        .unwrap();
    assert_eq!(blocked.status, FootContactStatus::AwaitingSwing);
    input.plant = false;
    let swing = blocked
        .state
        .prepare(&world, settings, input, &mut budget())
        .unwrap();
    input.plant = true;
    let replanted = swing
        .state
        .prepare(&world, settings, input, &mut budget())
        .unwrap();
    assert_eq!(replanted.status, FootContactStatus::Planted);
    input.grounded = false;
    let airborne = replanted
        .state
        .prepare(&world, settings, input, &mut budget())
        .unwrap();
    assert!(airborne.contact.is_none());
    input.grounded = true;
    assert_eq!(
        airborne
            .state
            .prepare(&world, settings, input, &mut budget())
            .unwrap()
            .status,
        FootContactStatus::Planted
    );
}
#[test]
fn missing_or_steep_support_releases_instead_of_transferring_a_planted_foot() {
    let (mut scene, owner, input) = setup();
    let settings = FootContactSettings {
        release_distance: 10.,
        ..Default::default()
    };
    let world = SupportWorld::from_scene(&scene, 1).unwrap();
    let state = FootContactState::default()
        .prepare(&world, settings, input, &mut budget())
        .unwrap()
        .state;
    scene
        .set_local(
            owner,
            Transform {
                rotation: Quat::from_rotation_z(1.),
                ..Default::default()
            },
        )
        .unwrap();
    let tilted = SupportWorld::from_scene(&scene, 1).unwrap();
    assert_eq!(
        state
            .prepare(&tilted, settings, input, &mut budget())
            .unwrap()
            .status,
        FootContactStatus::Released
    );
    scene.remove_subtree(owner).unwrap();
    let empty = SupportWorld::from_scene(&scene, 1).unwrap();
    assert_eq!(
        state
            .prepare(&empty, settings, input, &mut budget())
            .unwrap()
            .status,
        FootContactStatus::Released
    );
}
#[test]
fn acquisition_distance_validation_and_failed_budget_leave_published_state_intact() {
    let (scene, _, mut input) = setup();
    let settings = FootContactSettings::default();
    let world = SupportWorld::from_scene(&scene, 1).unwrap();
    input.sole.y += 0.15;
    let searching = FootContactState::default()
        .prepare(&world, settings, input, &mut budget())
        .unwrap();
    assert_eq!(searching.status, FootContactStatus::Searching);
    input.sole.y -= 0.15;
    let state = searching
        .state
        .prepare(&world, settings, input, &mut budget())
        .unwrap()
        .state;
    assert_eq!(
        state.prepare(
            &world,
            settings,
            input,
            &mut SupportQueryBudget::new(0).unwrap()
        ),
        Err(PhysicsError::SweepBudget)
    );
    assert!(state.anchor().is_some());
    let invalid = FootContactSettings {
        release_distance: 0.01,
        ..settings
    };
    let mut limit = budget();
    assert_eq!(
        state.prepare(&world, invalid, input, &mut limit),
        Err(PhysicsError::InvalidMotion)
    );
    assert_eq!(limit.remaining(), 1024);
}

#[test]
fn pivot_transfer_admits_same_support_and_preserves_old_state_on_rejection() {
    let (mut scene, _, mut input) = setup();
    let world = SupportWorld::from_scene(&scene, 1).unwrap();
    let settings = FootContactSettings::default();
    let state = FootContactState::default()
        .prepare(&world, settings, input, &mut budget())
        .unwrap()
        .state;
    let saved = state;
    input.sole.z += 0.15;
    let next = state
        .prepare_pivot_transfer(&world, settings, 0.08, input, &mut budget())
        .unwrap()
        .unwrap();
    assert_eq!(next.correction, next.contact.position - input.sole);
    assert!(next.contact.position.abs_diff_eq(input.sole, 1e-12));
    assert_ne!(next.state.anchor(), state.anchor());
    input.sole.y += settings.plant_distance + 0.01;
    assert!(
        state
            .prepare_pivot_transfer(&world, settings, 0.08, input, &mut budget())
            .unwrap()
            .is_none()
    );
    input.sole.y -= settings.plant_distance + 0.01;
    input.plant = false;
    assert!(
        state
            .prepare_pivot_transfer(&world, settings, 0.08, input, &mut budget())
            .unwrap()
            .is_none()
    );
    input.plant = true;
    assert!(
        state
            .prepare_pivot_transfer(
                &world,
                settings,
                0.08,
                input,
                &mut SupportQueryBudget::new(0).unwrap()
            )
            .is_err()
    );
    let other = scene
        .spawn(
            None,
            Transform {
                translation: Vec3::new(5., 0., 0.),
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            other,
            BoxCollider {
                half_extents: [2., 0.1, 2.],
            },
        )
        .unwrap();
    let two_supports = SupportWorld::from_scene(&scene, 2).unwrap();
    input.sole.x = 5.;
    assert!(
        state
            .prepare_pivot_transfer(&two_supports, settings, 0.08, input, &mut budget())
            .unwrap()
            .is_none()
    );
    assert_eq!(state, saved);
}

#[test]
fn admitted_heel_to_toe_transfer_rolls_about_physical_surface_and_moves_with_platform() {
    use glam::DQuat;
    use voxy_animation::{FootPlant, RootRigidTransform};
    let (mut scene, owner, input) = setup();
    let world = SupportWorld::from_scene(&scene, 1).unwrap();
    let settings = FootContactSettings::default();
    let initial = FootContactState::default()
        .prepare(&world, settings, input, &mut budget())
        .unwrap();
    let heel = DVec3::new(0., -0.08, -0.04);
    let toe = DVec3::new(0., -0.08, 0.14);
    let ankle = RootRigidTransform {
        translation: initial.contact.unwrap().position - heel,
        rotation: DQuat::IDENTITY,
    };
    let plant = FootPlant::capture_pivot(ankle, heel, RootRigidTransform::IDENTITY).unwrap();
    let new_site = ankle.transform_point(toe).unwrap();
    let transferred = initial
        .state
        .prepare_pivot_transfer(
            &world,
            settings,
            0.08,
            FootContactInput {
                sole: new_site,
                ..input
            },
            &mut budget(),
        )
        .unwrap()
        .unwrap();
    let rolled_plant = plant
        .with_pivot_preserving_ankle(RootRigidTransform::IDENTITY, ankle.rotation, toe)
        .unwrap();
    let toe_contact = transferred.contact.position;
    // In the admitted flat transfer pose the proposed and projected points agree.
    assert!(toe_contact.distance(new_site) < 1e-12);
    for (step, angle) in [0., 0.15, 0.3, 0.5].into_iter().enumerate() {
        let transform = Transform {
            translation: Vec3::new(step as f32 * 0.03, 0.02, 0.),
            rotation: Quat::from_rotation_y(step as f32 * 0.1),
            ..Default::default()
        };
        scene.set_local(owner, transform).unwrap();
        let moved_world = SupportWorld::from_scene(&scene, 1).unwrap();
        let physical = moved_world
            .resolve(transferred.state.anchor().unwrap(), &mut budget())
            .unwrap()
            .unwrap();
        let support = RootRigidTransform {
            translation: transform.translation.as_dvec3(),
            rotation: transform.rotation.as_dquat().normalize(),
        };
        let rotation = support.rotation * DQuat::from_rotation_x(angle);
        let target = rolled_plant
            .target_with_rotation(support, RootRigidTransform::IDENTITY, rotation, Vec3::Z, 1.)
            .unwrap();
        let solved_toe = target.position.as_dvec3() + rotation * toe;
        assert!(solved_toe.distance(physical.position) < 1e-6);
        // Chosen toe-off sign raises the heel above the admitted plane.
        let solved_heel = target.position.as_dvec3() + rotation * heel;
        assert!((solved_heel - physical.position).dot(physical.normal) >= -1e-6);
    }
    // A point within the acquisition band may still need a visible pose correction.
    let world = SupportWorld::from_scene(&scene, 1).unwrap();
    let contact = world
        .resolve(transferred.state.anchor().unwrap(), &mut budget())
        .unwrap()
        .unwrap();
    let elevated = contact.position + DVec3::Y * 0.04;
    let candidate = transferred
        .state
        .prepare_pivot_transfer(
            &world,
            settings,
            0.08,
            FootContactInput {
                sole: elevated,
                ..input
            },
            &mut budget(),
        )
        .unwrap()
        .unwrap();
    assert!((candidate.contact.position.distance(elevated) - 0.04).abs() < 1e-12);
    assert!(
        candidate
            .correction
            .abs_diff_eq(DVec3::new(0., -0.04, 0.), 1e-12)
    );
    assert!((elevated + candidate.correction).distance(candidate.contact.position) < 1e-12);
    for limit in [0., 0.039] {
        assert!(
            transferred
                .state
                .prepare_pivot_transfer(
                    &world,
                    settings,
                    limit,
                    FootContactInput {
                        sole: elevated,
                        ..input
                    },
                    &mut budget()
                )
                .unwrap()
                .is_none()
        );
    }
    for limit in [f64::NAN, f64::INFINITY, -0.1, 1e6 + 1.] {
        let mut query_budget = budget();
        let remaining = query_budget.remaining();
        assert!(
            transferred
                .state
                .prepare_pivot_transfer(
                    &world,
                    settings,
                    limit,
                    FootContactInput {
                        sole: elevated,
                        ..input
                    },
                    &mut query_budget
                )
                .is_err()
        );
        assert_eq!(query_budget.remaining(), remaining);
    }
    assert!(
        transferred
            .state
            .prepare_pivot_transfer(
                &world,
                settings,
                0.041,
                FootContactInput {
                    sole: elevated,
                    ..input
                },
                &mut budget()
            )
            .unwrap()
            .is_some()
    );
}

#[test]
fn admitted_pivot_target_includes_projection_and_converts_actor_frame() {
    use glam::DQuat;
    use voxy_animation::RootRigidTransform;
    let (scene, _, mut input) = setup();
    let world = SupportWorld::from_scene(&scene, 1).unwrap();
    let settings = FootContactSettings::default();
    let old = FootContactState::default()
        .prepare(&world, settings, input, &mut budget())
        .unwrap();
    input.sole += DVec3::new(0., 0.04, 0.15);
    let admitted = old
        .state
        .prepare_pivot_transfer(&world, settings, 0.05, input, &mut budget())
        .unwrap()
        .unwrap();
    let saved = admitted;
    let actor = RootRigidTransform {
        translation: DVec3::new(2., 0.5, -3.),
        rotation: DQuat::from_rotation_y(0.7),
    };
    let orientation = DQuat::from_rotation_x(0.3);
    let pivot = DVec3::new(0., -0.08, 0.14);
    let target = admitted
        .ik_target(actor, orientation, pivot, Vec3::Z)
        .unwrap();
    let actual = actor.transform_point(target.position.as_dvec3()).unwrap() + orientation * pivot;
    assert!(actual.distance(admitted.contact.position) < 1e-6);
    assert!(
        (actor.rotation * target.rotation.unwrap().as_dquat())
            .dot(orientation)
            .abs()
            > 1. - 1e-6
    );
    assert_eq!(target.weight, 1.);
    assert!(
        admitted
            .ik_target(actor, orientation, DVec3::splat(f64::NAN), Vec3::Z)
            .is_err()
    );
    assert!(
        admitted
            .ik_target(actor, DQuat::from_xyzw(0., 0., 0., 2.), pivot, Vec3::Z)
            .is_err()
    );
    assert!(
        admitted
            .ik_target(actor, orientation, pivot, Vec3::splat(f32::NAN))
            .is_err()
    );
    assert_eq!(admitted, saved);
}

#[test]
fn admitted_pivot_pose_stages_contact_only_for_reachable_ik() {
    use glam::{DQuat, Mat4};
    use std::sync::Arc;
    use voxy_animation::{Joint, RootRigidTransform, Skeleton, TwoBoneChain};
    let rig = Skeleton::new(
        vec![
            ("hip", None, Vec3::new(0., 1.5, 0.)),
            ("knee", Some(0), Vec3::new(0., -0.7, 0.1)),
            ("foot", Some(1), Vec3::new(0., -0.7, -0.1)),
        ]
        .into_iter()
        .map(|(name, parent, translation)| Joint {
            name: Arc::from(name),
            parent,
            bind_local: voxy_animation::Transform {
                translation,
                ..voxy_animation::Transform::IDENTITY
            },
            inverse_bind: Mat4::IDENTITY,
        })
        .collect(),
    )
    .unwrap();
    let chain = TwoBoneChain::new(&rig, [0, 1, 2]).unwrap();
    let source = rig.bind_pose();
    let original = source.local().to_vec();
    let (scene, _, mut input) = setup();
    input.sole.x = 0.;
    input.sole.z = 0.;
    let world = SupportWorld::from_scene(&scene, 1).unwrap();
    let settings = FootContactSettings::default();
    let old = FootContactState::default()
        .prepare(&world, settings, input, &mut budget())
        .unwrap();
    input.sole.z = 0.1;
    input.sole.y += 0.04;
    let candidate = old
        .state
        .prepare_pivot_transfer(&world, settings, 0.05, input, &mut budget())
        .unwrap()
        .unwrap();
    let pivot = DVec3::new(0., -0.08, 0.14);
    let (solved, state) = candidate
        .solve_pose(
            &source,
            &chain,
            RootRigidTransform::IDENTITY,
            DQuat::IDENTITY,
            pivot,
            Vec3::Z,
        )
        .unwrap()
        .unwrap();
    let point = solved.skin_matrices(&rig).unwrap()[2]
        .transform_point3(pivot.as_vec3())
        .as_dvec3();
    assert!(point.distance(candidate.contact.position) < 2e-6);
    assert_eq!(state, candidate.state);
    let original_frame = voxy_animation::AnimatorFrame {
        properties: Default::default(),
        pose: source.clone(),
        skin_matrices: source.skin_matrices(&rig).unwrap(),
        root_motion: Vec3::new(0.1, 0., 0.2),
        root_motion_joint: 0,
        transition_weight: 0.37,
    };
    let (frame, frame_state) = candidate
        .solve_frame(
            &original_frame,
            &rig,
            &chain,
            RootRigidTransform::IDENTITY,
            DQuat::IDENTITY,
            pivot,
            Vec3::Z,
        )
        .unwrap()
        .unwrap();
    assert_eq!(frame_state, state);
    assert_eq!(frame.root_motion, original_frame.root_motion);
    assert_eq!(frame.root_motion_joint, original_frame.root_motion_joint);
    assert_eq!(frame.transition_weight, original_frame.transition_weight);
    assert_eq!(frame.skin_matrices, frame.pose.skin_matrices(&rig).unwrap());
    assert!(
        frame.skin_matrices[2]
            .transform_point3(pivot.as_vec3())
            .as_dvec3()
            .distance(candidate.contact.position)
            < 2e-6
    );
    assert_eq!(original_frame.pose.local(), original.as_slice());

    let distant_actor = RootRigidTransform {
        translation: DVec3::X * 10.,
        ..RootRigidTransform::IDENTITY
    };
    assert!(
        candidate
            .solve_pose(
                &source,
                &chain,
                distant_actor,
                DQuat::IDENTITY,
                pivot,
                Vec3::Z
            )
            .unwrap()
            .is_none()
    );
    assert!(
        candidate
            .solve_frame(
                &original_frame,
                &rig,
                &chain,
                distant_actor,
                DQuat::IDENTITY,
                pivot,
                Vec3::Z
            )
            .unwrap()
            .is_none()
    );
    assert_eq!(
        original_frame.skin_matrices,
        source.skin_matrices(&rig).unwrap()
    );
    assert_eq!(source.local(), original.as_slice());
    assert_eq!(old.state.anchor().unwrap(), old.contact.unwrap().anchor);
}
