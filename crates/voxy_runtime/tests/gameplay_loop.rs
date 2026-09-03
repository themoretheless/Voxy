use voxy_runtime::{
    CharacterConfig, CharacterInput, CharacterState, ProjectileConfig, ProjectileOutcome,
    VehicleConfig, VehicleInput, VehicleState, WaterBudget, WaterPlan, WaterStates,
    build_bootstrap_scene, plan_impact_explosion, spawn_projectile, step_character,
    step_projectile, step_vehicle, step_water,
};
use voxy_world::{
    AnchoredAabb, DestructionPlan, EditSource, Explosion, RayOrigin, ResourceKey, VoxelPos,
};

const DT: f64 = 1.0 / 60.0;

#[test]
#[allow(clippy::too_many_lines)]
fn canonical_gameplay_loop_composes_character_water_projectile_explosion_and_vehicle() {
    let scene = build_bootstrap_scene(0x56_4f_58_59, 1).unwrap();
    let mut world = scene.world;
    let registry = world.registry_handle();

    let mut character = CharacterState {
        body: AnchoredAabb {
            anchor: VoxelPos {
                x: 16,
                y: 20,
                z: 16,
            },
            min: [-0.35, 0.0, -0.35],
            max: [0.35, 1.8, 0.35],
        },
        velocity: [0.0; 3],
        grounded: false,
    };
    for _ in 0..240 {
        step_character(
            &world,
            &registry,
            &mut character,
            CharacterInput {
                planar_velocity: [2.0, 0.0],
                jump_pressed: false,
            },
            DT,
            CharacterConfig::default(),
        )
        .unwrap();
    }
    assert!(character.grounded);
    assert!(character.body.anchor.x > 16);

    let water_states = WaterStates(std::array::from_fn(|index| {
        registry
            .find(&ResourceKey::parse(format!("voxy:water_{}", index + 1)).unwrap())
            .unwrap()
    }));
    let water_source = VoxelPos {
        x: 12,
        y: 18,
        z: 12,
    };
    let WaterPlan::Transaction { edit, next_active } = step_water(
        &world,
        &registry,
        water_states,
        &[water_source],
        EditSource::Simulation,
        WaterBudget::default(),
    )
    .unwrap() else {
        panic!("bootstrap water must move");
    };
    assert!(!next_active.is_empty());
    let water_receipt = world.commit(edit).unwrap();
    assert!(!water_receipt.chunks.is_empty());

    let mut projectile = spawn_projectile(
        RayOrigin {
            voxel: VoxelPos {
                x: 16,
                y: 25,
                z: 16,
            },
            offset: [0.5; 3],
        },
        [0.0, -1.0, 0.0],
        30.0,
    )
    .unwrap();
    let hit = (0..120)
        .find_map(|_| {
            match step_projectile(&world, &mut projectile, DT, ProjectileConfig::default()).unwrap()
            {
                ProjectileOutcome::Flying => None,
                ProjectileOutcome::Impact(hit) => Some(hit),
                outcome => panic!("unexpected projectile outcome: {outcome:?}"),
            }
        })
        .expect("projectile must hit terrain or water");
    let DestructionPlan::Transaction(explosion) = plan_impact_explosion(
        &world,
        &registry,
        EditSource::Player(1),
        hit,
        Explosion {
            radius: 1,
            power: 100,
            attenuation_per_squared_voxel: 8,
            ..Explosion::default()
        },
    )
    .unwrap() else {
        panic!("impact must destroy at least its hit voxel");
    };
    let explosion_receipt = world.commit(explosion).unwrap();
    assert!(!explosion_receipt.inverse.writes.is_empty());

    let mut vehicle = VehicleState {
        chassis: character,
        heading: 0.0,
        longitudinal_speed: 0.0,
    };
    let start_x = vehicle.chassis.body.anchor.x;
    let start_z = vehicle.chassis.body.anchor.z;
    let mut peak_speed = 0.0_f64;
    for _ in 0..120 {
        step_vehicle(
            &world,
            &registry,
            &mut vehicle,
            VehicleInput {
                throttle: 1.0,
                steering: 0.0,
                brake: false,
            },
            DT,
            VehicleConfig::default(),
        )
        .unwrap();
        peak_speed = peak_speed.max(vehicle.longitudinal_speed);
    }
    assert!(peak_speed > 0.1);
    assert!(vehicle.chassis.body.anchor.x != start_x || vehicle.chassis.body.anchor.z != start_z);
}
