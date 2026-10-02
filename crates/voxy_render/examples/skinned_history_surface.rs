//! Native window proof of resident skeletal history and presentation commits.
use glam::{Mat4, Vec3};
use std::sync::Arc;
use voxy_render::{RenderOutcome, Renderer, SkinnedMesh, SkinnedVertex};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};
#[derive(Default)]
struct Smoke {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    phase: u32,
    retries: u32,
    failure: Option<String>,
    options: voxy_render::GraphicsOptions,
}
impl Smoke {
    fn draw(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let host = self.renderer.as_mut().ok_or("renderer missing")?;
        check_anchor_transition(host, self.phase)?;
        if self.phase == 5 {
            let scene = host.create_scene_renderer();
            let outcome = host.render_scene(&scene, &[])?;
            assert!(host.skinned_motion_output().is_none());
            assert!(!host.prepare_skinned_motion()?.history_valid);
            if outcome != RenderOutcome::Presented {
                return Ok(false);
            }
        }
        if self.phase == 2 {
            host.resize(320, 240);
            assert!(host.skinned_motion_output().is_none());
        }
        if self.phase == 3 {
            host.resize(0, 0);
            assert_eq!(host.render()?, RenderOutcome::Suspended);
            assert!(host.skinned_motion_output().is_none());
            assert!(!host.prepare_skinned_motion()?.history_valid);
            host.resize(320, 240);
        }
        // Prepare an intermediate pose without presenting it.
        host.update_skin_matrices(&[Mat4::from_translation(Vec3::X * 99.0); 2])?;
        host.prepare_skinned_motion()?;
        let x = f32::from(u16::try_from(self.phase % 5)?) * 0.1;
        host.update_skin_matrices(&[
            Mat4::from_translation(Vec3::X * x),
            Mat4::from_translation(Vec3::Y * x * 0.2) * Mat4::from_rotation_x(x),
        ])?;
        if self.phase == 4 {
            // Camera and skeleton both move relative to the last presented frame.
            host.update_camera(voxy_render::CameraView {
                eye: Vec3::new(0.2, 0.0, 3.0),
                target: Vec3::new(0.2, 0.0, 0.0),
                ..Default::default()
            })?;
        }
        let before = host.prepare_skinned_motion()?;
        let valid = self.phase == 1 || self.phase == 4 || self.phase == 7;
        assert_eq!(before.history_valid, valid);
        assert_eq!(host.prepare_skinned_motion_camera()?.history_valid, valid);
        let previous_x = if valid { x - 0.1 } else { x } - 0.4;
        assert!((before.vertices[0].previous[0] - previous_x).abs() < 1e-6);
        let cameras = host.prepare_skinned_motion_camera()?;
        let projected = before
            .vertices
            .iter()
            .map(|vertex| {
                let current = cameras.current * Vec3::from_array(vertex.current).extend(1.0);
                let previous = cameras.previous * Vec3::from_array(vertex.previous).extend(1.0);
                [
                    current.truncate().truncate() / current.w,
                    previous.truncate().truncate() / previous.w,
                ]
            })
            .collect::<Vec<_>>();
        let outcome = host.render()?;
        if outcome != RenderOutcome::Presented {
            self.retries += 1;
            if self.retries > 100 {
                return Err(format!("surface did not present: {outcome:?}").into());
            }
            return Ok(false);
        }
        let after = host.prepare_skinned_motion()?;
        assert!(after.history_valid);
        let output = host
            .skinned_motion_output()
            .ok_or("motion metadata missing")?;
        assert_eq!(output.presentation_id, u64::from(self.phase) + 1);
        assert_eq!(output.reset_history, !valid);
        let motion = output.motion;
        assert_eq!(motion.format(), wgpu::TextureFormat::Rg16Float);
        assert_eq!([motion.width(), motion.height()], [320, 240]);
        assert!(host.prepare_skinned_motion_camera()?.history_valid);
        assert!((after.vertices[0].previous[0] - (x - 0.4)).abs() < 1e-6);
        verify_motion_pixels(host, &projected, [cameras.current, cameras.previous], valid)?;
        self.phase += 1;
        Ok(self.phase == 8)
    }
}
impl ApplicationHandler for Smoke {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let window = Arc::new(
                event_loop.create_window(
                    Window::default_attributes()
                        .with_title("Voxy skeletal presentation history")
                        .with_inner_size(winit::dpi::PhysicalSize::new(320, 240)),
                )?,
            );
            let instance = self
                .options
                .create_instance_with_display(event_loop.owned_display_handle());
            let mut host = pollster::block_on(Renderer::new_with_instance(
                window.clone(),
                320,
                240,
                self.options,
                instance,
            ))?;
            let vertices =
                [[0.0, 0.0, 0.0], [0.5, 0.0, 0.0], [0.0, 0.5, 0.0]].map(|position| SkinnedVertex {
                    position,
                    normal: [0.0, 0.0, 1.0],
                    uv: [0.0; 2],
                    joints: [0, 1, 0, 0],
                    weights: if position[0] > 0.0 {
                        [32768, 32767, 0, 0]
                    } else if position[1] > 0.0 {
                        [16384, 49151, 0, 0]
                    } else {
                        [65535, 0, 0, 0]
                    },
                });
            let mesh = SkinnedMesh::new(vertices.to_vec(), vec![0, 1, 2], 2)?;
            host.upload_skinned_mesh(
                &mesh,
                &[Mat4::IDENTITY; 2],
                Mat4::from_translation(Vec3::new(-0.4, 0.0, 0.0)),
                0,
            )?;
            host.upload_mesh(&voxel_mesh())?;
            host.update_camera(voxy_render::CameraView {
                eye: Vec3::new(0.0, 0.0, 3.0),
                target: Vec3::ZERO,
                ..Default::default()
            })?;
            println!("Motion window GPU: {:?}", host.adapter_info());
            host.set_skinned_motion_enabled(true);
            self.renderer = Some(host);
            window.focus_window();
            window.request_redraw();
            self.window = Some(window);
            Ok(())
        })();
        if let Err(error) = result {
            self.failure = Some(error.to_string());
            event_loop.exit();
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if event != WindowEvent::RedrawRequested || self.failure.is_some() {
            return;
        }
        match self.draw() {
            Ok(true) => {
                println!(
                    "SKINNED PRESENTATION PASS: commit, skipped pose, resize and suspension reset"
                );
                event_loop.exit();
            }
            Ok(false) => {
                if let Some(window) = &self.window {
                    window.focus_window();
                    window.request_redraw();
                }
            }
            Err(error) => {
                self.failure = Some(error.to_string());
                event_loop.exit();
            }
        }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = match std::env::var("VOXY_MOTION_BACKEND").as_deref() {
        Err(std::env::VarError::NotPresent) | Ok("auto") => voxy_render::GraphicsBackend::Auto,
        Ok("vulkan") => voxy_render::GraphicsBackend::Vulkan,
        Ok("gl") => voxy_render::GraphicsBackend::OpenGl,
        Ok("metal") => voxy_render::GraphicsBackend::Metal,
        Ok("dx12") => voxy_render::GraphicsBackend::DirectX12,
        _ => return Err("VOXY_MOTION_BACKEND expects auto|vulkan|gl|metal|dx12".into()),
    };
    let mut smoke = Smoke {
        options: voxy_render::GraphicsOptions {
            backend,
            ..Default::default()
        },
        ..Default::default()
    };
    EventLoop::new()?.run_app(&mut smoke)?;
    if let Some(error) = smoke.failure {
        return Err(error.into());
    }
    if smoke.phase != 8 {
        return Err("presentation proof incomplete".into());
    }
    Ok(())
}

fn verify_motion_pixels(
    host: &Renderer,
    projected: &[[glam::Vec2; 2]],
    cameras: [Mat4; 2],
    moving: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let buffer = read_motion_buffer(host)?;
    let bytes = buffer.slice(..).get_mapped_range()?;
    let static_min = (cameras[0] * Vec3::new(0.0, 0.0, 1.0).extend(1.0))
        .truncate()
        .truncate();
    let static_max = (cameras[0] * Vec3::new(1.0, 1.0, 1.0).extend(1.0))
        .truncate()
        .truncate();
    let static_previous = (cameras[1] * Vec3::new(0.0, 0.0, 1.0).extend(1.0))
        .truncate()
        .truncate();
    let static_motion = if moving {
        (static_previous - static_min) * glam::Vec2::new(0.5, -0.5)
    } else {
        glam::Vec2::ZERO
    };
    let mut moved = 0;
    let mut static_pixels = 0;
    let mut occluded_pixels = 0;
    let mut visible_skin_pixels = 0;
    for (index, pixel) in bytes.chunks_exact(4).enumerate() {
        let clip = glam::Vec2::new(
            (f32::from(u16::try_from(index % 320)?) + 0.5) / 160.0 - 1.0,
            1.0 - (f32::from(u16::try_from(index / 320)?) + 0.5) / 120.0,
        );
        let ab = projected[1][0] - projected[0][0];
        let ac = projected[2][0] - projected[0][0];
        let ap = clip - projected[0][0];
        let b = ap.perp_dot(ac) / ab.perp_dot(ac);
        let c = ab.perp_dot(ap) / ab.perp_dot(ac);
        let a = 1.0 - b - c;
        let previous = projected[0][1] * a + projected[1][1] * b + projected[2][1] * c;
        let on_static = clip.x > static_min.x
            && clip.x < static_max.x
            && clip.y > static_min.y
            && clip.y < static_max.y;
        if on_static {
            static_pixels += 1;
        }
        if a > 0.01 && b > 0.01 && c > 0.01 {
            if on_static {
                occluded_pixels += 1;
            } else {
                visible_skin_pixels += 1;
            }
        }
        let expected = if on_static {
            static_motion
        } else if moving {
            (previous - clip) * glam::Vec2::new(0.5, -0.5)
        } else {
            glam::Vec2::ZERO
        };

        let values = [
            half::f16::from_bits(u16::from_le_bytes(pixel[..2].try_into()?)).to_f32(),
            half::f16::from_bits(u16::from_le_bytes(pixel[2..].try_into()?)).to_f32(),
        ];
        assert!(values.iter().all(|value| value.is_finite()));
        if !on_static && (a < -0.01 || b < -0.01 || c < -0.01) {
            assert!(values.iter().all(|value| value.abs() < 1e-6));
        }
        if on_static || (moving && a > 0.01 && b > 0.01 && c > 0.01) {
            for (value, expected) in values.into_iter().zip(expected.to_array()) {
                assert!(
                    (value - expected).abs() < 0.0001,
                    "interior pixel {index}: {values:?} != {expected}"
                );
            }
        }
        if values.iter().any(|v| v.abs() > 1e-6) {
            moved += 1;
            for (value, expected) in values.into_iter().zip(expected.to_array()) {
                assert!((value - expected).abs() < 0.0001);
            }
        }
    }
    assert!(static_pixels > 100);
    assert!(
        occluded_pixels > 20,
        "occlusion not exercised: {occluded_pixels}"
    );
    assert!(
        visible_skin_pixels > 100,
        "visible skin not exercised: {visible_skin_pixels}"
    );
    if moving {
        assert!(moved > 100, "moving pixels: {moved}");
    } else {
        assert_eq!(moved, 0);
    }
    drop(bytes);
    buffer.unmap();
    Ok(())
}

fn read_motion_buffer(host: &Renderer) -> Result<wgpu::Buffer, Box<dyn std::error::Error>> {
    let texture = host.skinned_motion_texture().ok_or("motion absent")?;
    let buffer = host.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("resident motion readback"),
        size: 320 * 240 * 4,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = host
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(1280),
                rows_per_image: Some(240),
            },
        },
        texture.size(),
    );
    host.queue().submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    host.device().poll(wgpu::PollType::wait_indefinitely())?;
    rx.recv()??;
    Ok(buffer)
}

fn voxel_mesh() -> voxy_mesher::ChunkMesh {
    let quad = voxy_mesher::Quad {
        origin: [0, 0, 1],
        extent_u: std::num::NonZeroU8::new(1).unwrap(),
        extent_v: std::num::NonZeroU8::new(1).unwrap(),
        face: voxy_mesher::FaceDir::PosZ,
        material: voxy_mesher::MaterialId(0),
        layer: voxy_mesher::RenderLayer::Opaque,
        ao: [3; 4],
        diagonal: voxy_mesher::QuadDiagonal::Vu,
    };
    voxy_mesher::ChunkMesh {
        opaque: vec![quad].into_boxed_slice(),
        cutout: Box::default(),
        translucent: Box::default(),
        bounds: voxy_mesher::LocalAabb {
            min: [0, 0, 1],
            max: [1, 1, 2],
        },
    }
}

fn check_anchor_transition(
    host: &mut Renderer,
    phase: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    if phase == 7 {
        let id = host
            .skinned_motion_output()
            .ok_or("published motion missing")?
            .presentation_id;
        let mesh = voxel_mesh();
        assert!(
            host.upload_chunks(
                &[(
                    voxy_core::ChunkPos {
                        x: i64::MAX,
                        y: 0,
                        z: 0
                    },
                    &mesh
                )],
                voxy_core::ChunkPos { x: 2, y: 0, z: 0 }
            )
            .is_err()
        );
        assert_eq!(
            host.skinned_motion_output()
                .ok_or("failed upload lost motion")?
                .presentation_id,
            id
        );
        assert!(host.prepare_skinned_motion()?.history_valid);
    }
    if phase == 6 || phase == 7 {
        let anchor = voxy_core::ChunkPos { x: 1, y: 0, z: 0 };
        host.upload_chunks(&[(anchor, &voxel_mesh())], anchor)?;
        if phase == 6 {
            assert!(host.skinned_motion_output().is_none());
            assert!(!host.prepare_skinned_motion()?.history_valid);
        } else {
            assert!(host.skinned_motion_output().is_some());
            assert!(host.prepare_skinned_motion()?.history_valid);
        }
    }
    Ok(())
}
