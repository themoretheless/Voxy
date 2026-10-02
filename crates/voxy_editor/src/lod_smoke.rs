//! Native LOD acceptance reads instance selection only after presented frames.
use crate::{
    App,
    gpu_model::{DeferredUpload, ModelGeometry},
};
use winit::keyboard::KeyCode;

#[derive(Debug, Default)]
pub(super) struct Smoke {
    phase: u8,
    since: u64,
    restore_budget: Option<u64>,
}

impl App {
    fn advance_lod_smoke(&mut self, phase: u8) {
        if let Some(smoke) = &mut self.lod_smoke {
            smoke.phase = phase;
            smoke.since = self.frames;
        }
    }

    pub(super) fn lod_acceptance(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let Some((phase, since)) = self
            .lod_smoke
            .as_ref()
            .map(|smoke| (smoke.phase, smoke.since))
        else {
            return Ok(false);
        };
        if self.frames < since + 3 {
            return Ok(false);
        }
        let styles = self
            .frame_styles
            .as_ref()
            .ok_or("missing presented styles")?;
        if styles.camera.distance.to_bits() != self.camera.distance.to_bits()
            || styles.materials.len() != self.extraction.instances().len()
            || self
                .extraction
                .instances()
                .iter()
                .any(|instance| !styles.materials.contains_key(&instance.owner))
        {
            return Err("presented frame styles disagree with extracted owners/camera".into());
        }
        println!(
            "LOD NATIVE SNAPSHOT PASS frames={} owners={}",
            self.frames,
            styles.materials.len()
        );
        let split = self.split_views();
        let graphics = self.graphics.as_mut().ok_or("missing graphics")?;
        let Some((id, model)) = graphics
            .models
            .iter()
            .find(|(_, model)| matches!(model.geometry, ModelGeometry::Lod { .. }))
        else {
            return Ok(false);
        };
        let ModelGeometry::Lod { bundle, .. } = &model.geometry else {
            unreachable!()
        };
        let asset = self.catalog.snapshot(id).ok_or("missing CPU model")?;
        if !model.geometry_residency_valid(asset.value()) {
            return Err("LOD resident bytes differ from preflight".into());
        }
        let selected: Vec<_> = self
            .extraction
            .instances()
            .iter()
            .filter(|instance| instance.component.asset == *id)
            .filter_map(|instance| {
                graphics
                    .lod_history
                    .get(&(0, instance.owner))
                    .and_then(voxy_render::SceneLodHistory::previous)
            })
            .collect();
        if selected.is_empty() {
            return Ok(false);
        }
        match phase {
            0 => {
                if selected.contains(&0) {
                    return Err("far view did not select coarse LOD".into());
                }
                println!(
                    "LOD NATIVE FAR PASS frames={} levels={selected:?} bytes={}",
                    self.frames,
                    model.geometry_bytes()
                );
                self.camera.distance = 0.02;
                self.advance_lod_smoke(1);
                Ok(false)
            }
            1 => {
                if selected.iter().any(|level| *level != 0) {
                    return Err("near view did not refine to base".into());
                }
                let before = std::ptr::from_ref(bundle.level(0).ok_or("missing base")?);
                let id = id.clone();
                let old_limit = graphics.residency_cache.geometry_budget;
                // Exercise the ordinary retry/publication path, with the old
                // model still resident, rather than an isolated upload call.
                graphics.residency_errors.insert(
                    id.clone(),
                    DeferredUpload::new(
                        &asset,
                        &graphics.residency_cache,
                        "acceptance retry".into(),
                    ),
                );
                // Changing the admission state unblocks this staged retry.
                graphics.residency_cache.geometry_budget = 0;
                self.synchronize_gpu_residency();
                let graphics = self.graphics.as_mut().ok_or("missing graphics")?;
                let retained = graphics.models.get(&id).ok_or("old model was removed")?;
                let rejected = graphics
                    .residency_errors
                    .get(&id)
                    .is_some_and(|error| error.message.contains("GPU geometry budget exceeded"));
                let unchanged = std::ptr::from_ref(retained.geometry.base()) == before;
                graphics.residency_cache.geometry_budget = old_limit;
                if !rejected || !unchanged {
                    return Err("failed admission changed old geometry".into());
                }
                println!(
                    "LOD NATIVE NEAR AND LAST GOOD PASS frames={} levels={selected:?}",
                    self.frames
                );
                self.advance_lod_smoke(2);
                Ok(false)
            }
            2 => {
                if graphics.residency_errors.contains_key(id) {
                    return Err("LOD admission did not recover after restoring budget".into());
                }
                println!("LOD NATIVE BUDGET RECOVERY PASS frames={}", self.frames);
                println!(
                    "LOD NATIVE PRESENTED AFTER FAILURE PASS frames={}",
                    self.frames
                );
                self.edit_key(KeyCode::F4)?;
                let mut far = self.camera.clone();
                far.distance = 1000.;
                self.secondary_camera = Some(far);
                self.advance_lod_smoke(3);
                Ok(false)
            }
            3..=5 => {
                let owners: Vec<_> = self
                    .extraction
                    .instances()
                    .iter()
                    .filter(|instance| instance.component.asset == *id)
                    .map(|instance| instance.owner)
                    .collect();
                let levels = [0, 1].map(|view| {
                    owners
                        .iter()
                        .map(|owner| {
                            graphics
                                .lod_history
                                .get(&(view, *owner))
                                .and_then(voxy_render::SceneLodHistory::previous)
                        })
                        .collect::<Vec<_>>()
                });
                let near = usize::from(phase == 5);
                let far = 1 - near;
                if !split
                    || levels[near].iter().any(|level| *level != Some(0))
                    || levels[far]
                        .iter()
                        .any(|level| level.is_none_or(|level| level == 0))
                {
                    return Err(format!("split LOD histories disagree: {levels:?}").into());
                }
                if (phase >= 4) != self.msaa4 {
                    return Err("split LOD sample mode disagrees with acceptance phase".into());
                }
                println!(
                    "LOD NATIVE SPLIT PASS frames={} phase={phase} msaa4={} levels={levels:?} bytes={}",
                    self.frames,
                    self.msaa4,
                    model.geometry_bytes()
                );
                match phase {
                    3 => self.edit_key(KeyCode::F10)?,
                    4 => {
                        self.camera.distance = 1000.;
                        self.secondary_camera
                            .as_mut()
                            .ok_or("missing secondary camera")?
                            .distance = 0.02;
                    }
                    _ => {
                        self.edit_key(KeyCode::F10)?;
                        self.edit_key(KeyCode::F4)?;
                        self.camera.distance = 0.02;
                    }
                }
                self.advance_lod_smoke(phase + 1);
                Ok(false)
            }
            6 => {
                if split || self.msaa4 || selected.iter().any(|level| *level != 0) {
                    return Err("single-view restoration after split LOD failed".into());
                }
                if graphics.lod_history.keys().any(|(view, _)| *view != 0) {
                    return Err("removed view retained LOD history".into());
                }
                println!("LOD NATIVE SINGLE RESTORED PASS frames={}", self.frames);
                let required = self.required_gpu_assets();
                if required.len() > 1 {
                    if required.iter().any(|id| {
                        self.graphics
                            .as_ref()
                            .is_none_or(|graphics| !graphics.models.contains_key(id))
                    }) {
                        return Ok(false);
                    }
                    self.camera.distance = 1000.;
                    self.advance_lod_smoke(7);
                    return Ok(false);
                }
                self.lod_smoke = None;
                Ok(true)
            }
            7..=9 => {
                let required = self
                    .extraction
                    .instances()
                    .iter()
                    .map(|instance| instance.component.asset.clone())
                    .collect::<std::collections::BTreeSet<_>>();
                if required.len() != 2 || graphics.models.len() != 2 {
                    return Err("competition acceptance requires exactly two model assets".into());
                }
                let mut optional_costs = Vec::new();
                let mut resident_assets = 0;
                for asset in &required {
                    let model = graphics
                        .models
                        .get(asset)
                        .ok_or("missing competing model")?;
                    let ModelGeometry::Lod { bundle, .. } = &model.geometry else {
                        return Err("competition requires certified LOD for both assets".into());
                    };
                    let snapshot = self
                        .catalog
                        .snapshot(asset)
                        .ok_or("missing competing CPU model")?;
                    if !model.geometry_residency_valid(snapshot.value()) {
                        return Err("competing model residency bytes disagree".into());
                    }
                    let optional = bundle.allocation_bytes()
                        - bundle
                            .level(0)
                            .ok_or("missing competing base")?
                            .allocation_bytes();
                    resident_assets += usize::from(optional != 0);
                    optional_costs.push(optional);
                    let levels: Vec<_> = self
                        .extraction
                        .instances()
                        .iter()
                        .filter(|instance| instance.component.asset == *asset)
                        .map(|instance| {
                            graphics
                                .lod_history
                                .get(&(0, instance.owner))
                                .and_then(voxy_render::SceneLodHistory::previous)
                        })
                        .collect();
                    if levels.is_empty()
                        || levels.iter().any(|level| {
                            if optional == 0 {
                                *level != Some(0)
                            } else {
                                level.is_none_or(|level| level == 0)
                            }
                        })
                    {
                        return Err(format!(
                            "competing asset fallback disagrees: {} {levels:?}",
                            asset.0
                        )
                        .into());
                    }
                }
                let live = graphics.geometry_bytes();
                let budget = graphics.residency_cache.geometry_budget;
                if live != graphics.residency_cache.geometry_live || live > budget {
                    return Err(format!(
                        "competition ledger exceeds budget: live={live} budget={budget}"
                    )
                    .into());
                }
                if phase == 7 {
                    if resident_assets != 2 || !graphics.lod_pending.is_empty() {
                        return Err("both coarse levels must load before contraction".into());
                    }
                    let mandatory = live - optional_costs.iter().sum::<u64>();
                    let one_level = *optional_costs.iter().max().ok_or("missing optional cost")?;
                    self.lod_smoke
                        .as_mut()
                        .ok_or("missing smoke")?
                        .restore_budget = Some(budget);
                    graphics.residency_cache.geometry_budget = mandatory + one_level;
                    println!(
                        "LOD NATIVE COMPETITION CONTRACT frames={} mandatory={mandatory} optional={optional_costs:?} budget={}",
                        self.frames,
                        mandatory + one_level
                    );
                    self.advance_lod_smoke(8);
                    Ok(false)
                } else if phase == 8 {
                    if resident_assets != 1 || graphics.lod_pending.len() != 1 {
                        return Err("contracted budget did not defer exactly one asset".into());
                    }
                    println!(
                        "LOD NATIVE COMPETITION FALLBACK PASS frames={} live={live} budget={budget} pending={:?}",
                        self.frames, graphics.lod_pending
                    );
                    graphics.residency_cache.geometry_budget = self
                        .lod_smoke
                        .as_ref()
                        .and_then(|smoke| smoke.restore_budget)
                        .ok_or("missing original budget")?;
                    self.advance_lod_smoke(9);
                    Ok(false)
                } else {
                    if resident_assets != 2 || !graphics.lod_pending.is_empty() {
                        return Err("competing LOD did not recover after budget restoration".into());
                    }
                    println!(
                        "LOD NATIVE COMPETITION RECOVERY PASS frames={} live={live} budget={budget}",
                        self.frames
                    );
                    self.lod_smoke = None;
                    Ok(true)
                }
            }
            _ => Err("invalid LOD acceptance phase".into()),
        }
    }
}
