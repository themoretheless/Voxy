//! Actual body-surface material pixels: CPU reference versus resident GPU light.
use glam::Vec3;
use voxy_render::{
    SceneDraw, SceneMesh, SceneRenderer, SceneVertex, SceneView, SurfaceLightAdmission,
    SurfaceLightGeometry, SurfaceLightPcg, SurfaceLightTransfer, SurfaceLightVertexBinding,
};
use wgpu::util::DeviceExt;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    pollster::block_on(run())
}
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        return Err("usage: surface_light_material OUTPUT_PREFIX FIXTURE...".into());
    }
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter = instance.request_adapter(&Default::default()).await?;
    let (device, queue) = adapter.request_device(&Default::default()).await?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    renderer.enable_msaa4(&device).await?;
    let texture = renderer.upload_texture(&device, &queue, 1, 1, &[255; 4])?;
    let width = 512;
    let height = 768;
    let make_texture = |format, usage, sample_count| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("body material pixel audit"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let color = make_texture(
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        1,
    );
    let depth = make_texture(
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
        1,
    );
    let msaa_color = make_texture(
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
        4,
    );
    let msaa_depth = make_texture(
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
        4,
    );
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("body pixels and publication"),
        size: u64::from(width * height * 8) + 32,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut reports = Vec::new();
    for (sample, path) in args[1..].iter().enumerate() {
        let fixture: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
        let points: Vec<[f32; 3]> = serde_json::from_value(fixture["points"].clone())?;
        let triangles: Vec<[u32; 3]> = serde_json::from_value(fixture["triangles"].clone())?;
        let source: Vec<[f64; 3]> = serde_json::from_value(fixture["source"].clone())?;
        let reference: Vec<[f64; 3]> = serde_json::from_value(fixture["reference"].clone())?;
        let radii: [f32; 3] = serde_json::from_value(fixture["radii"].clone())?;
        if points.len() != source.len() || points.len() != reference.len() {
            return Err("fixture length mismatch".into());
        }
        let n = points.len();
        let albedo = [0.72, 0.46, 0.34];
        let fraction = 0.35;
        let mut normals = vec![Vec3::ZERO; n];
        for t in &triangles {
            let p = t.map(|i| Vec3::from(points[i as usize]));
            let normal = (p[1] - p[0]).cross(p[2] - p[0]);
            for i in t {
                normals[*i as usize] += normal;
            }
        }
        let normals: Vec<_> = normals
            .into_iter()
            .map(|v| v.normalize_or_zero().to_array())
            .collect();
        let gpu_vertices: Vec<_> = points
            .iter()
            .map(|p| SceneVertex {
                position: *p,
                uv: [0.; 2],
                color: [0., 0., 0., 1.],
            })
            .collect();
        let cpu_vertices: Vec<_> = points
            .iter()
            .enumerate()
            .map(|(i, p)| SceneVertex {
                position: *p,
                uv: [0.; 2],
                color: [
                    albedo[0]
                        * (fraction * source[i][0] + (1. - fraction) * reference[i][0]) as f32,
                    albedo[1]
                        * (fraction * source[i][1] + (1. - fraction) * reference[i][1]) as f32,
                    albedo[2]
                        * (fraction * source[i][2] + (1. - fraction) * reference[i][2]) as f32,
                    1.,
                ],
            })
            .collect();
        let indices: Vec<_> = triangles.iter().flatten().copied().collect();
        let gpu_mesh =
            SceneMesh::new(gpu_vertices, indices.clone())?.with_normals(normals.clone())?;
        let cpu_mesh = SceneMesh::new(cpu_vertices, indices)?.with_normals(normals)?;
        let mut gpu_draw = renderer.upload_compute_mesh(&device, &gpu_mesh)?;
        let mut cpu_draw = renderer.upload_mesh(&device, &cpu_mesh)?;
        let material = include_str!("../../voxy_demos/src/female_material.wgsl");
        renderer
            .set_geometry_opaque_shader(&device, &mut gpu_draw, material)
            .await?;
        renderer
            .set_geometry_opaque_shader(&device, &mut cpu_draw, material)
            .await?;
        // A separate posed input audits the material bridge; native skinning integration is separate.
        let positions = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("posed body operator input"),
            contents: bytemuck::cast_slice(&points),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let operator = SurfaceLightGeometry::new(&device, &positions, n, 3, &triangles)?;
        let mut solver = SurfaceLightPcg::from_geometry(&device, &operator, radii)?;
        let incident: Vec<_> = source
            .iter()
            .map(|s| [s[0] as f32, s[1] as f32, s[2] as f32, 1.])
            .collect();
        let incident = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("body incident light"),
            contents: bytemuck::cast_slice(&incident),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });
        let admission = SurfaceLightAdmission::new(
            &device,
            &operator,
            solver.output(),
            &incident,
            radii,
            1e-3,
            1e-6,
        )?;
        let mapping: Vec<_> = (0..n)
            .map(|i| SurfaceLightVertexBinding {
                albedo: [albedo[0], albedo[1], albedo[2], 1.],
                vertex: i as u32,
                light: i as u32,
                surface_fraction: fraction as f32,
                padding: 0,
            })
            .collect();
        let transfer = SurfaceLightTransfer::new(&device, &gpu_draw, &admission, &mapping).await?;
        let min = points
            .iter()
            .fold(Vec3::splat(f32::INFINITY), |a, p| a.min(Vec3::from(*p)));
        let max = points
            .iter()
            .fold(Vec3::splat(f32::NEG_INFINITY), |a, p| a.max(Vec3::from(*p)));
        let center = (min + max) * 0.5;
        let size = max - min;
        let half_height = (size.y * 0.55).max(size.x * 0.55 * 1.5);
        let eye = center + Vec3::Z * (size.length() * 2.);
        let view = glam::camera::rh::view::look_at_mat4(eye, center, Vec3::Y);
        let projection = glam::camera::rh::proj::directx::orthographic(
            -half_height / 1.5,
            half_height / 1.5,
            -half_height,
            half_height,
            0.01,
            size.length() * 4.,
        );
        let transform = renderer.create_transform(&device, projection * view)?;
        transform.update_view_position(&queue, eye)?;
        let left = [SceneDraw {
            geometry: &cpu_draw,
            texture: &texture,
            transform: &transform,
            overlay: false,
        }];
        let right = [SceneDraw {
            geometry: &gpu_draw,
            texture: &texture,
            transform: &transform,
            overlay: false,
        }];
        let mut encoder = device.create_command_encoder(&Default::default());
        operator.encode(&mut encoder);
        solver.reset_from_resident_source(&mut encoder, &incident)?;
        solver.encode_timed(&mut encoder, 64, None)?;
        admission.encode(&mut encoder);
        transfer.encode(&mut encoder);
        // Identical viewport coordinates eliminate procedural-footprint differences
        // caused by translating rasterization to a neighbouring viewport.
        for (frame, draws) in [&left[..], &right[..]].into_iter().enumerate() {
            let views = [SceneView {
                viewport: [0, 0, width, height],
                draws,
            }];
            renderer.encode_view_frame_msaa4(
                &mut encoder,
                (
                    &msaa_color.create_view(&Default::default()),
                    &msaa_depth.create_view(&Default::default()),
                ),
                &color.create_view(&Default::default()),
                &depth.create_view(&Default::default()),
                wgpu::Color::BLACK,
                &views,
                &[],
            )?;
            encoder.copy_texture_to_buffer(
                color.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: frame as u64 * u64::from(width * height * 4),
                        bytes_per_row: Some(width * 4),
                        rows_per_image: Some(height),
                    },
                },
                color.size(),
            );
        }
        encoder.copy_buffer_to_buffer(
            admission.header_buffer(),
            0,
            &readback,
            u64::from(width * height * 8),
            32,
        );
        queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::PollType::wait_indefinitely())?;
        rx.recv()??;
        let data = readback.slice(..).get_mapped_range()?;
        let pixels = &data[..(width * height * 8) as usize];
        let header: &[u32] = bytemuck::cast_slice(&data[(width * height * 8) as usize..]);
        assert_eq!(
            &header[0..4],
            &[0, 1, 1, 1],
            "the drawn illumination must be admitted"
        );
        let mut maximum = 0u8;
        let mut occupied = 0;
        let mut differing = 0;
        for y in 0..height {
            for x in 0..width {
                let a = ((y * width + x) * 4) as usize;
                let b = a + (width * height * 4) as usize;
                if pixels[a..a + 3].iter().any(|c| *c > 0) {
                    occupied += 1;
                }
                for c in 0..3 {
                    let d = pixels[a + c].abs_diff(pixels[b + c]);
                    maximum = maximum.max(d);
                    differing += usize::from(d > 0);
                }
            }
        }
        assert!(occupied > 1000, "empty rendering is not a parity result");
        assert!(maximum <= 1, "CPU/GPU skin material pixel error {maximum}");
        let mut comparison = Vec::with_capacity(pixels.len());
        let row = (width * 4) as usize;
        let image_bytes = (width * height * 4) as usize;
        for y in 0..height as usize {
            comparison.extend_from_slice(&pixels[y * row..(y + 1) * row]);
            comparison
                .extend_from_slice(&pixels[image_bytes + y * row..image_bytes + (y + 1) * row]);
        }
        image::save_buffer(
            format!("{}.{}.png", args[0], sample),
            &comparison,
            width * 2,
            height,
            image::ColorType::Rgba8,
        )?;
        reports.push(serde_json::json!({"fixture":path,"vertices":n,"triangles":triangles.len(),"occupied_pixels":occupied,
            "maximum_channel_error_u8":maximum,"differing_channels":differing,"admitted":true,"msaa":4}));
        eprintln!(
            "BODY MATERIAL PIXELS sample={sample} occupied={occupied} max_error_u8={maximum}"
        );
        drop(data);
        readback.unmap();
    }
    if let Some(error) = scope.pop().await {
        return Err(error.to_string().into());
    }
    std::fs::write(
        format!("{}.json", args[0]),
        serde_json::to_vec_pretty(&serde_json::json!({
        "adapter":format!("{:?}",adapter.get_info()),"samples":reports,"native_frame_loop_integrated":false,
        "scope":"Actual body surface, current female material, 4x MSAA CPU versus GPU pixels. No full head/hair or FPS qualification."}))?,
    )?;
    Ok(())
}
