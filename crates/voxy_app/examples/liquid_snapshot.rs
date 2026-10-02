//! Renders initial, active-jet and settled native geometry using GPU readback.
#[path = "../src/liquid_demo.rs"]
mod liquid_demo;
use glam::Vec3;
use voxy_render::{GraphicsOptions, SceneCamera, SceneDraw, SceneProjection, SceneRenderer};
#[allow(clippy::too_many_lines)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/private/tmp/voxy-liquids.png".into());
    let reference = std::env::args().any(|a| a == "--reference");
    let close_water = std::env::args().any(|a| a == "--water-close");
    let optical = reference || std::env::args().any(|arg| arg == "--optical");
    let diagnostic = if std::env::args().any(|a| a == "--depth") {
        Some(voxy_render::FluidDiagnostic::Depth)
    } else if std::env::args().any(|a| a == "--thickness") {
        Some(voxy_render::FluidDiagnostic::Thickness)
    } else {
        None
    };
    let impacts = std::env::args().any(|arg| arg == "--impacts");
    let mut demo = if impacts {
        liquid_demo::LiquidDemo::new_impacts()?
    } else {
        liquid_demo::LiquidDemo::new()?
    };
    let instance = GraphicsOptions::default().create_instance();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let format = if optical {
        wgpu::TextureFormat::Rgba8UnormSrgb
    } else {
        wgpu::TextureFormat::Rgba8Unorm
    };
    let renderer = SceneRenderer::new(&device, format);
    let mut geometry = renderer.reserve_geometry(&device, 60_024, 60_024)?;
    let texture = renderer.upload_texture(&device, &queue, 1, 1, &[255; 4])?;
    let camera = SceneCamera {
        eye: if close_water {
            Vec3::new(-1.25, -0.1, 1.45)
        } else if optical {
            Vec3::new(0.0, 0.1, 3.5)
        } else {
            Vec3::new(0.0, 1.4, 6.0)
        },
        target: if close_water {
            Vec3::new(-1.25, -0.45, 0.0)
        } else if optical {
            Vec3::new(0.0, -0.35, 0.0)
        } else {
            Vec3::ZERO
        },
        up: Vec3::Y,
        projection: SceneProjection::Perspective {
            vertical_fov: 55_f32.to_radians(),
            aspect: 4.0 / 3.0,
            near: 0.1,
            far: 100.0,
        },
    };
    let transform = renderer.create_transform(&device, camera.view_projection()?)?;
    let mut fluid = voxy_render::ScreenSpaceFluidRenderer::new(&device, format, 1024, 768, 8192)?;
    let target = |format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("liquid snapshot"),
            size: wgpu::Extent3d {
                width: 1024,
                height: 768,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let color = target(
        format,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = target(
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("liquid pixels"),
        size: 4096 * 768,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut frames = Vec::new();
    let stages: &[usize] = if impacts {
        &[0, 30, 48, 42]
    } else {
        &[0, 30, 90]
    };
    for (frame, steps) in stages.iter().enumerate() {
        if frame > 0 {
            for _ in 0..*steps {
                demo.advance(1.0 / 120.0)?;
            }
        }
        if frame + 1 == stages.len() {
            demo.verify()?;
        }
        let mesh = if optical {
            let (mesh, particles) = demo.optical_scene()?;
            fluid.update(
                &queue,
                camera,
                &particles,
                1.1,
                voxy_render::FluidDepthFilter::Bilateral,
            )?;
            if reference {
                let mut vertices = mesh.vertices().to_vec();
                let mut indices = mesh.indices().to_vec();
                for p in &particles {
                    let base = vertices.len() as u32;
                    for i in 0..6 {
                        let mut position = [
                            p.position_radius[0],
                            p.position_radius[1],
                            p.position_radius[2],
                        ];
                        position[i / 2] += if i % 2 == 0 {
                            p.position_radius[3]
                        } else {
                            -p.position_radius[3]
                        };
                        vertices.push(voxy_render::SceneVertex {
                            position,
                            uv: [0.0; 2],
                            color: if p.absorption_ior[3] < 1.4 {
                                [0.08, 0.55, 0.95, 1.0]
                            } else {
                                [0.95, 0.62, 0.12, 1.0]
                            },
                        });
                    }
                    for face in [
                        [0, 2, 4],
                        [2, 1, 4],
                        [1, 3, 4],
                        [3, 0, 4],
                        [2, 0, 5],
                        [1, 2, 5],
                        [3, 1, 5],
                        [0, 3, 5],
                    ] {
                        indices.extend(face.map(|i| base + i));
                    }
                }
                voxy_render::SceneMesh::new(vertices, indices)?
            } else {
                mesh
            }
        } else {
            demo.mesh()?
        };
        geometry.update(&queue, &mesh)?;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let draws = [SceneDraw {
            geometry: &geometry,
            texture: &texture,
            transform: &transform,
            overlay: false,
        }];
        let clear = wgpu::Color {
            r: 0.025,
            g: 0.04,
            b: 0.065,
            a: 1.0,
        };
        if optical && !reference {
            fluid.encode(
                &renderer,
                &mut encoder,
                &color.create_view(&Default::default()),
                clear,
                &draws,
            );
            if let Some(diagnostic) = diagnostic {
                fluid.encode_diagnostic(
                    &mut encoder,
                    &color.create_view(&Default::default()),
                    diagnostic,
                );
            }
        } else {
            renderer.encode(
                &mut encoder,
                &color.create_view(&Default::default()),
                &depth.create_view(&Default::default()),
                wgpu::Color {
                    r: 0.025,
                    g: 0.04,
                    b: 0.065,
                    a: 1.0,
                },
                &[SceneDraw {
                    geometry: &geometry,
                    texture: &texture,
                    transform: &transform,
                    overlay: false,
                }],
            );
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &color,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4096),
                    rows_per_image: Some(768),
                },
            },
            color.size(),
        );
        queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::PollType::wait_indefinitely())?;
        rx.recv()??;
        let pixels = readback.slice(..).get_mapped_range()?;
        let blue = pixels
            .chunks_exact(4)
            .filter(|p| p[2] > 70 && p[0] < 40)
            .count();
        let gold = pixels
            .chunks_exact(4)
            .filter(|p| p[0] > 70 && p[2] < 40)
            .count();
        if !optical && !(impacts && frame == 0) && (blue < 100 || gold < 100) {
            return Err("liquid particles missing from rendered pixels".into());
        }
        println!("frame={frame}: water_pixels={blue}, oil_pixels={gold}");
        if frame == 2 {
            image::save_buffer(
                format!("{path}-active.png"),
                &pixels,
                1024,
                768,
                image::ColorType::Rgba8,
            )?;
        }
        frames.push(pixels.to_vec());
        drop(pixels);
        readback.unmap();
    }
    if frames.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("rendered liquid did not move".into());
    }
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.into());
    }
    let mut output = Vec::new();
    for row in 0..768 {
        for frame in &frames {
            output.extend_from_slice(&frame[row * 4096..(row + 1) * 4096]);
        }
    }
    image::save_buffer(
        &path,
        &output,
        1024 * frames.len() as u32,
        768,
        image::ColorType::Rgba8,
    )?;
    println!("LIQUID SNAPSHOT PASS: {path}");
    Ok(())
}
