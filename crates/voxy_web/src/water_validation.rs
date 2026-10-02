//! Browser event-loop proof for immutable GPU world-water plans.
use super::browser::{error, yield_browser};
use physics_voxel::{WaterBudget, WaterPlan, WaterStates};
use voxy_gpu::{PendingWaterPlan, WaterTransferProgram};
use voxy_world::{
    BlockStateId, CommitError, EditSource, EditTxn, ResourceKey, VoxelPos, VoxelWrite,
};
use wasm_bindgen::JsValue;

pub(crate) async fn validate(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<u32, JsValue> {
    let program = WaterTransferProgram::new(device).await.map_err(error)?;
    let mut scene = voxy_runtime::build_bootstrap_scene(42, 0).map_err(error)?;
    let mut levels = [BlockStateId::AIR; 8];
    for (i, level) in levels.iter_mut().enumerate() {
        *level = scene
            .world
            .registry()
            .find(&ResourceKey::parse(format!("voxy:water_{}", i + 1)).map_err(error)?)
            .ok_or_else(|| error("missing water state"))?;
    }
    let states = WaterStates(levels);
    let budget = WaterBudget::default();
    let mut active = vec![VoxelPos {
        x: 16,
        y: 18,
        z: 16,
    }];
    let mut stale = program
        .begin_plan_world(
            queue,
            &scene.world,
            scene.world.registry(),
            states,
            &active,
            EditSource::Simulation,
            budget,
        )
        .map_err(error)?;
    scene
        .world
        .commit(EditTxn {
            source: EditSource::Simulation,
            expected: vec![],
            writes: vec![VoxelWrite {
                pos: VoxelPos { x: 0, y: 20, z: 0 },
                block: levels[0],
            }],
        })
        .map_err(error)?;
    let WaterPlan::Transaction { edit, .. } = read(&mut stale).await? else {
        return Err(error("expected stale water transfer"));
    };
    if !matches!(
        scene.world.commit(edit),
        Err(CommitError::RevisionConflict { .. })
    ) {
        return Err(error(
            "stale browser GPU water transaction was not rejected",
        ));
    }
    let mut commits = 0;
    for _ in 0..16 {
        let expected = physics_voxel::step_water(
            &scene.world,
            scene.world.registry(),
            states,
            &active,
            EditSource::Simulation,
            budget,
        )
        .map_err(error)?;
        let mut pending = program
            .begin_plan_world(
                queue,
                &scene.world,
                scene.world.registry(),
                states,
                &active,
                EditSource::Simulation,
                budget,
            )
            .map_err(error)?;
        let actual = read(&mut pending).await?;
        if actual != expected {
            return Err(error("browser GPU/CPU water transactions differ"));
        }
        if pending.try_plan().is_ok() {
            return Err(error("browser water result consumed twice"));
        }
        match actual {
            WaterPlan::Settled => active.clear(),
            WaterPlan::Transaction { edit, next_active } => {
                scene.world.commit(edit).map_err(error)?;
                active = next_active.into_vec();
                commits += 1;
            }
        }
    }
    if commits == 0 {
        return Err(error("browser water never moved"));
    }
    Ok(commits)
}
async fn read(pending: &mut PendingWaterPlan) -> Result<WaterPlan, JsValue> {
    let deadline = js_sys::Date::now() + 30_000.0;
    loop {
        if let Some(plan) = pending.try_plan().map_err(error)? {
            return Ok(plan);
        }
        if js_sys::Date::now() >= deadline {
            return Err(error("browser water readback timed out"));
        }
        yield_browser().await?;
    }
}
