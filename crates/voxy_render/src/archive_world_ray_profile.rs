//! Optional last-source-frame stage counters. Read only after the output fence.
use crate::{ComputeMemoryBudget, ComputeStorage, RaySceneError};

#[derive(Debug)]
pub(super) struct WorldRayProfile {
    pub(super) set: wgpu::QuerySet,
    resolve: ComputeStorage,
    readback: ComputeStorage,
    regions: u32,
    stages: u32,
    source_stages: u32,
    consumer_cache: bool,
    consumer_split: bool,
    consumer_sorted: bool,
    tiles: u32,
    native_hint: bool,
    native_witness: bool,
}
fn consumer_resolve_offset(
    regions: u32,
    stages: u32,
    tiles: u32,
    consumer: bool,
    region: u32,
) -> Option<u32> {
    if !consumer || tiles == 0 || region >= regions {
        return None;
    }
    let tile = region % tiles;
    if tile + 1 == tiles {
        return None;
    }
    Some(regions * stages * 2 + (region / tiles * (tiles - 1) + tile) * 2)
}
impl WorldRayProfile {
    #[allow(clippy::fn_params_excessive_bools)]
    pub(super) fn new(
        device: &wgpu::Device,
        regions: usize,
        stages: u32,
        native_hint: bool,
        native_witness: bool,
        consumer_cache: bool,
        consumer_split: bool,
        consumer_sorted: bool,
        tiles: usize,
    ) -> Result<Self, RaySceneError> {
        if (consumer_split || consumer_sorted) && !consumer_cache {
            return Err(RaySceneError::Unsupported);
        }
        let source_stages = stages;
        let stages = stages
            + if consumer_cache {
                if consumer_sorted {
                    5
                } else if consumer_split {
                    3
                } else {
                    1
                }
            } else {
                0
            }
            + 1;
        let tiles = u32::try_from(tiles).map_err(|_| RaySceneError::Capacity)?;
        if tiles == 0 {
            return Err(RaySceneError::Capacity);
        }
        let regions = u32::try_from(regions).map_err(|_| RaySceneError::Capacity)?;
        let count = regions
            .checked_mul(stages * 2)
            .ok_or(RaySceneError::Capacity)?;
        if regions % tiles != 0 {
            return Err(RaySceneError::Capacity);
        }
        let extra = if consumer_cache {
            (regions - regions / tiles) * 2
        } else {
            0
        };
        let count = count.checked_add(extra).ok_or(RaySceneError::Capacity)?;
        if count == 0 || count > wgpu::QUERY_SET_MAX_QUERIES {
            return Err(RaySceneError::Capacity);
        }
        if !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            return Err(RaySceneError::Unsupported);
        }
        let bytes = u64::from(count) * 8;
        let [resolve, readback] = ComputeMemoryBudget::for_device(device)
            .allocate_buffers([
                voxy_gpu_core::ManagedBufferDescriptor {
                    label: "world stage counter resolve",
                    size: bytes.div_ceil(256) * 256,
                    contents: None,
                    usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                },
                voxy_gpu_core::ManagedBufferDescriptor {
                    label: "world stage counter snapshot",
                    size: bytes,
                    contents: None,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                },
            ])
            .map_err(|_| RaySceneError::Capacity)?;
        let set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("last complete source frame stages"),
            ty: wgpu::QueryType::Timestamp,
            count,
        });
        Ok(Self {
            set,
            resolve,
            readback,
            regions,
            stages,
            source_stages,
            consumer_cache,
            consumer_split,
            consumer_sorted,
            tiles,
            native_hint,
            native_witness,
        })
    }
    pub(super) fn stage_count(&self) -> u32 {
        self.stages
    }
    pub(super) fn consumer_stage_count(&self) -> u32 {
        if self.consumer_cache {
            if self.consumer_sorted {
                5
            } else if self.consumer_split {
                3
            } else {
                1
            }
        } else {
            0
        }
    }
    pub(super) fn resolve_query(&self, region: usize) -> Option<u32> {
        consumer_resolve_offset(
            self.regions,
            self.stages,
            self.tiles,
            self.consumer_cache,
            region as u32,
        )
    }
    pub(super) fn snapshot(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        tiles: usize,
        kinds: usize,
        encoded_frames: u64,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        if encoded_frames == 0 {
            return Err("no source-query frame has been encoded".into());
        }
        device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })?;
        if self.regions as usize != tiles * kinds || self.tiles as usize != tiles {
            return Err("profile region ownership changed".into());
        }
        let source_words = self.regions * self.stages * 2;
        let count = source_words
            + if self.consumer_cache {
                (self.regions - self.regions / self.tiles) * 2
            } else {
                0
            };
        let bytes = u64::from(count) * 8;
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.resolve_query_set(&self.set, 0..count, &self.resolve, 0);
        encoder.copy_buffer_to_buffer(&self.resolve, 0, &self.readback, 0, bytes);
        let submission = queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        self.readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
        device.poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })?;
        rx.recv()??;
        let mapped = self.readback.slice(..).get_mapped_range()?;
        let ticks: Vec<u64> = mapped
            .chunks_exact(8)
            .map(|v| u64::from_le_bytes(v.try_into().unwrap()))
            .collect();
        drop(mapped);
        self.readback.unmap();
        let mut totals = vec![vec![0f64; self.stages as usize]; kinds];
        let mut records = Vec::new();
        for (region, words) in ticks[..source_words as usize]
            .chunks_exact(self.stages as usize * 2)
            .enumerate()
        {
            let mut ms = vec![0f64; self.stages as usize];
            for stage in 0..self.stages as usize {
                if words[stage * 2] == 0 || words[stage * 2 + 1] <= words[stage * 2] {
                    return Err(
                        format!("invalid source timestamp region={region} stage={stage}").into(),
                    );
                }
                ms[stage] = (words[stage * 2 + 1] - words[stage * 2]) as f64
                    * f64::from(queue.get_timestamp_period())
                    / 1e6;
                totals[region / tiles][stage] += ms[stage];
            }
            records.push(ms);
        }
        #[allow(clippy::possible_missing_else)]
        let stage_names = {
            let mut names = vec!["producer"];
            if self.consumer_sorted {
                names.extend([
                    "consumer_project",
                    "consumer_prefix",
                    "consumer_scatter",
                    "consumer_traverse",
                    "consumer_colour",
                ]);
            } else if self.consumer_split {
                names.extend(["consumer_project", "consumer_traverse", "consumer_colour"]);
            } else if self.consumer_cache {
                names.push("consumer_fallback");
            }
            if self.native_hint {
                names.push(if self.native_witness {
                    "hardware_source_witness"
                } else {
                    "hardware_hint"
                });
            }
            names.push("certificate");
            if self.source_stages - u32::from(self.native_hint) == 5 {
                names.push("admission");
            }
            names.extend(["winner", "candidates", "fallback"]);
            names
        };
        if stage_names.len() != self.stages as usize {
            return Err("stage label/count mismatch".into());
        }
        let mut resolve_samples = vec![Vec::<f64>::new(); kinds];
        if self.consumer_cache {
            for kind in 0..kinds {
                for tile in 0..tiles - 1 {
                    let at = source_words as usize + 2 * (kind * (tiles - 1) + tile);
                    let a = ticks[at];
                    let b = ticks[at + 1];
                    if a == 0 || b <= a {
                        return Err(format!(
                            "invalid consumer resolve timestamp kind={kind} tile={tile}"
                        )
                        .into());
                    }
                    resolve_samples[kind]
                        .push((b - a) as f64 * f64::from(queue.get_timestamp_period()) / 1e6);
                }
            }
        }
        let resolve_totals: Vec<f64> = resolve_samples.iter().map(|v| v.iter().sum()).collect();
        Ok(
            serde_json::json!({"scope":"last fully completed encoded source-query frame","encoded_source_frame":encoded_frames,"consumer_components_profiled":self.consumer_cache,"consumer_split_diagnostic":self.consumer_split,"consumer_sorted":self.consumer_sorted,"source_region_tick_words":source_words,"consumer_resolve_sampled_tiles_per_kind":if self.consumer_cache {tiles-1}else{0},"consumer_resolve_excluded_tile_indices":if self.consumer_cache {vec![tiles-1]}else{vec![]},"consumer_resolve_samples_ms":resolve_samples,"consumer_resolve_sample_sum_ms_per_kind":resolve_totals,"stages":stage_names,"ray_kinds":if kinds==2 {vec!["GI","SSR"]}else{vec!["GI"]},"tiles_per_kind":tiles,"sum_stage_ms_per_kind":totals,"region_stage_ms":records,"raw_ticks":ticks,"intervals_may_overlap":true,"exclusive_frame_cost":false,"frame_fps_measured":false}),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn query_ranges_cover_each_slot_once_and_preserve_last_tile_global_timer() {
        for tiles in [1, 2, 5, 7, 12, 72] {
            for kinds in [1, 2] {
                for source in [4, 5, 6] {
                    for cache in [false, true] {
                        for split in [false, true] {
                            if split && !cache {
                                continue;
                            }
                            let stages =
                                source + if cache { if split { 3 } else { 1 } } else { 0 } + 1;
                            let regions = tiles * kinds;
                            let mut used = std::collections::BTreeSet::new();
                            for region in 0..regions {
                                for slot in region * stages * 2..(region + 1) * stages * 2 {
                                    assert!(used.insert(slot));
                                }
                                let offset =
                                    consumer_resolve_offset(regions, stages, tiles, cache, region);
                                assert_eq!(offset.is_some(), cache && region % tiles + 1 < tiles);
                                if let Some(at) = offset {
                                    assert!(used.insert(at));
                                    assert!(used.insert(at + 1));
                                }
                            }
                            let count = regions * stages * 2
                                + if cache { kinds * (tiles - 1) * 2 } else { 0 };
                            assert_eq!(used, (0..count).collect());
                            assert!(
                                consumer_resolve_offset(regions, stages, tiles, cache, regions)
                                    .is_none()
                            );
                        }
                    }
                }
            }
        }
    }
}
