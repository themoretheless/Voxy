//! Exact diagnostic snapshots of one complete ray chunk, never FPS admission.
use super::{ComputeMemoryBudget, ComputeStorage, HIT_STRIDE, SEGMENT_STRIDE};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
#[derive(Debug)]
pub(super) struct RayCapture {
    snapshots: Vec<ComputeStorage>,
    sizes: Vec<u64>,
    camera: wgpu::Buffer,
    depth: wgpu::Texture,
    pub(super) tile: usize,
    kinds: usize,
    pixels: u64,
    stride: u32,
    wanted: u64,
    frame: AtomicU64,
    ready: AtomicBool,
    constants: Vec<(String, f64)>,
}
fn setting(name: &str, default: u64) -> Result<u64, crate::ComputeError> {
    match std::env::var(name) {
        Ok(value) => value.parse().map_err(|_| {
            crate::ComputeError::Validation(format!("{name} must be an unsigned integer"))
        }),
        Err(std::env::VarError::NotPresent) => Ok(default),
        Err(_) => Err(crate::ComputeError::Validation(format!(
            "{name} must be UTF-8"
        ))),
    }
}
impl RayCapture {
    pub(super) fn new(
        device: &wgpu::Device,
        camera: &ComputeStorage,
        depth: &wgpu::Texture,
        tiles: &[([u32; 2], u32)],
        kinds: usize,
        constants: &[(&str, f64)],
    ) -> Result<Self, crate::ComputeError> {
        let tile = usize::try_from(setting(
            "VOXY_STREET_RAY_CAPTURE_TILE",
            (tiles.len() / 2) as u64,
        )?)
        .map_err(|_| crate::ComputeError::InvalidBuffer)?;
        let wanted = setting("VOXY_STREET_RAY_CAPTURE_FRAME", 122)?;
        if !(1..=2).contains(&kinds)
            || wanted == 0
            || tile >= tiles.len()
            || !camera.usage().contains(wgpu::BufferUsages::COPY_SRC)
            || !depth.usage().contains(wgpu::TextureUsages::COPY_SRC)
        {
            return Err(crate::ComputeError::Validation(
                "capture requires a reachable positive staged frame and tile, with copyable inputs"
                    .into(),
            ));
        }
        let extent = tiles[tile].0;
        let pixels = u64::from(extent[0]) * u64::from(extent[1]);
        let stride = (depth.width() * 4).div_ceil(256) * 256;
        let mut sizes = Vec::new();
        for _ in 0..kinds {
            sizes.extend([pixels * SEGMENT_STRIDE, pixels * HIT_STRIDE, pixels * 4]);
        }
        sizes.extend([
            camera.size(),
            u64::from(stride) * u64::from(depth.height()),
            16,
        ]);
        let descriptors: Vec<_> = sizes
            .iter()
            .map(|&size| voxy_gpu_core::ManagedBufferDescriptor {
                label: "diagnostic exact ray snapshot",
                size,
                contents: None,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            })
            .collect();
        let snapshots =
            ComputeMemoryBudget::for_device(device).allocate_buffer_batch(&descriptors)?;
        Ok(Self {
            snapshots,
            sizes,
            camera: (**camera).clone(),
            depth: depth.clone(),
            tile,
            kinds,
            pixels,
            stride,
            wanted,
            frame: AtomicU64::new(0),
            ready: AtomicBool::new(false),
            constants: constants.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        })
    }
    pub(super) fn next_frame(&self) -> bool {
        self.frame.fetch_add(1, Ordering::Relaxed) + 1 == self.wanted
    }
    pub(super) fn encode_kind(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        kind: usize,
        buffers: &[ComputeStorage; 7],
    ) {
        for (slot, source) in [0usize, 1, 3].into_iter().enumerate() {
            let target = kind * 3 + slot;
            encoder.copy_buffer_to_buffer(
                &buffers[source],
                0,
                &self.snapshots[target],
                0,
                self.sizes[target],
            );
        }
    }
    pub(super) fn finish_encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        buffers: &[ComputeStorage; 7],
        offset: u32,
    ) {
        let base = self.kinds * 3;
        encoder.copy_buffer_to_buffer(&self.camera, 0, &self.snapshots[base], 0, self.sizes[base]);
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.depth,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::DepthOnly,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.snapshots[base + 1],
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.stride),
                    rows_per_image: Some(self.depth.height()),
                },
            },
            self.depth.size(),
        );
        encoder.copy_buffer_to_buffer(
            &buffers[2],
            u64::from(offset),
            &self.snapshots[base + 2],
            0,
            16,
        );
        self.ready.store(true, Ordering::Release);
    }
    pub(super) fn export(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        directory: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !self.ready.load(Ordering::Acquire) {
            return Err("requested ray capture frame was not reached".into());
        }
        std::fs::create_dir_all(directory)?;
        let mut files = Vec::new();
        let mut tile = [0u32; 4];
        for (index, source) in self.snapshots.iter().enumerate() {
            let logical = self.sizes[index];
            let mapped_size = logical.div_ceil(8) * 8;
            let [read] = ComputeMemoryBudget::for_device(device).allocate_buffers([
                voxy_gpu_core::ManagedBufferDescriptor {
                    label: "bounded diagnostic ray readback",
                    size: mapped_size,
                    contents: None,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                },
            ])?;
            let mut encoder = device.create_command_encoder(&Default::default());
            encoder.copy_buffer_to_buffer(source, 0, &read, 0, logical);
            let submission = queue.submit([encoder.finish()]);
            let (tx, rx) = std::sync::mpsc::channel();
            read.slice(..)
                .map_async(wgpu::MapMode::Read, move |result| {
                    let _ = tx.send(result);
                });
            device.poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })?;
            rx.recv()??;
            let data = read.slice(..).get_mapped_range()?;
            let base = self.kinds * 3;
            let name = if index < base {
                format!(
                    "{}.{}.bin",
                    if index / 3 == 0 { "gi" } else { "ssr" },
                    ["segments", "hits", "order"][index % 3]
                )
            } else {
                ["camera.bin", "depth.bin", "tile.bin"][index - base].to_owned()
            };
            let mut output = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(directory.join(&name))?;
            let bytes = if index == base + 1 {
                let row = self.depth.width() as usize * 4;
                for y in 0..self.depth.height() as usize {
                    output.write_all(
                        &data[y * self.stride as usize..y * self.stride as usize + row],
                    )?;
                }
                u64::from(self.depth.width()) * u64::from(self.depth.height()) * 4
            } else {
                output.write_all(&data[..logical as usize])?;
                logical
            };
            if index == base + 2 {
                for (i, value) in tile.iter_mut().enumerate() {
                    *value = u32::from_le_bytes(data[i * 4..i * 4 + 4].try_into()?);
                }
            }
            files.push(serde_json::json!({"name":name,"bytes":bytes}));
            drop(data);
            read.unmap();
        }
        if u64::from(tile[2]) * u64::from(tile[3]) != self.pixels {
            return Err("captured tile does not match complete ray chunk".into());
        }
        let metadata = serde_json::json!({"schema":"voxy-ray-chunk-v1","byte_order":"little","viewport":[self.depth.width(),self.depth.height()],"tile":tile,"tile_index":self.tile,"staged_frame":self.wanted,"ray_count_per_kind":self.pixels,"ray_kinds":if self.kinds==2 {vec!["gi","ssr"]}else{vec!["gi"]},"segment_stride":SEGMENT_STRIDE,"hit_stride":HIT_STRIDE,"camera_bytes":self.camera.size(),"constants":self.constants,"files":files,"scope":"one complete chunk from one real rendered frame; no ray subsampling","full_scene_fps_proven":false});
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join("metadata.json"))?;
        output.write_all(&serde_json::to_vec_pretty(&metadata)?)?;
        Ok(())
    }
}
