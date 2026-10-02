//! GPU numerical acceptance for rough-reflection spatial filtering.
use voxy_render::{ReflectionHit, ReflectionSpatialOptions, ReflectionSpatialPipeline};
use wgpu::util::DeviceExt;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))?;
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))?;
    println!("SPATIAL GPU: {:?}", adapter.get_info());
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let pipeline = ReflectionSpatialPipeline::new(&device)?;
    let radiance = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("noisy reflection samples"),
        size: wgpu::Extent3d {
            width: 9,
            height: 3,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let values: Vec<[f32; 4]> = (0..27)
        .map(|i| {
            if [10, 13, 16].contains(&i) {
                [0., 0., 0., 1.]
            } else {
                [2., 4., 6., 1.]
            }
        })
        .collect();
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &radiance,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        bytemuck::cast_slice(&values),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(144),
            rows_per_image: Some(3),
        },
        radiance.size(),
    );
    let initial: Vec<[f32; 8]> = (0..27)
        .map(|i| {
            [
                (i % 9) as f32 * 0.001,
                (i / 9) as f32 * 0.001,
                0.,
                1.,
                0.,
                0.,
                1.,
                if i == 13 { 0. } else { 0.5 },
            ]
        })
        .collect();
    let initial_hits: Vec<_> = (0..27)
        .map(|i| ReflectionHit {
            position_distance: [0., 0., 0., 1.],
            identity: [0, 0, 0, i],
            barycentrics_valid: [0., 0., 0., if i == 16 { 0. } else { 1. }],
        })
        .collect();
    let surface = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("controlled primary surfaces"),
        contents: bytemuck::cast_slice(&initial),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    });
    let hits = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("controlled reflected hits"),
        contents: bytemuck::cast_slice(&initial_hits),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    });
    let mut numerical_errors = Vec::new();
    for mode in 0..9 {
        let mut surfaces = initial.clone();
        let mut records = initial_hits.clone();
        for y in 0..3 {
            for x in 0..3 {
                let i = y * 9 + x;
                if i == 10 {
                    continue;
                }
                match mode {
                    1 => records[i].identity[0] = 1,
                    2 => surfaces[i][2] = 0.1,
                    3 => {
                        surfaces[i][4] = 1.;
                        surfaces[i][6] = 0.;
                    }
                    4 => surfaces[i][7] = 0.8,
                    5 => records[i].position_distance[3] = 2.,
                    6 => records[i].barycentrics_valid[3] = 0.,
                    7 => records[i].position_distance[3] = f32::NAN,
                    8 => {
                        surfaces[i][5] = 0.1;
                        surfaces[i][6] = 0.99_f32.sqrt();
                        surfaces[i][2] = 0.0001;
                    }
                    _ => {}
                }
            }
        }
        queue.write_buffer(&surface, 0, bytemuck::cast_slice(&surfaces));
        queue.write_buffer(&hits, 0, bytemuck::cast_slice(&records));
        let job = pipeline.prepare_raw(
            &surface,
            &hits,
            &radiance,
            ReflectionSpatialOptions::default(),
        )?;
        let read = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("spatial filter pixel proof"),
            size: 768,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        job.encode(&mut encoder);
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: job.output(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &read,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(3),
                },
            },
            radiance.size(),
        );
        queue.submit([encoder.finish()]);
        let (sender, receiver) = std::sync::mpsc::channel();
        read.slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result);
            });
        device.poll(wgpu::PollType::wait_indefinitely())?;
        receiver.recv()??;
        let data = read.slice(..).get_mapped_range()?;
        for (x, y, expected) in [
            (
                1_usize,
                1_usize,
                if mode == 0 || mode == 8 { 1.5 } else { 0. },
            ),
            (4, 1, 0.),
            (7, 1, 1.5),
            (8, 1, 5.0 / 3.0),
            (8, 2, 16.0 / 9.0),
        ] {
            let start = y * 256 + x * 16;
            for channel in 0..3 {
                let offset = start + channel * 4;
                let actual = f32::from_le_bytes(data[offset..offset + 4].try_into()?);
                let reference = expected * (channel + 1) as f32;
                if (actual - reference).abs() > 1e-5 {
                    numerical_errors.push(format!(
                        "mode {mode}, pixel ({x},{y}), channel {channel}: {actual} != {reference}"
                    ));
                }
            }
        }
        drop(data);
        read.unmap();
    }
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.into());
    }
    if !numerical_errors.is_empty() {
        return Err(numerical_errors.join("; ").into());
    }
    println!(
        "REFLECTION SPATIAL PASS: 135 channel references; compatible curved normals; noisy HDR average, geometry/plane/normal/material/distance/miss/nonfinite rejection, mirror bypass, stochastic miss smoothing, partial workgroups"
    );
    Ok(())
}
