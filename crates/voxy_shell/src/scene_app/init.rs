//! GPU host initialization, device-loss recovery and fallback primitive mesh builders.
use super::*;

impl SceneApp {
    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    pub(super) fn initialize(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.resources.is_some() {
            return Ok(());
        }
        self.host_request = self
            .demo
            .as_mut()
            .map(|demo| demo.configure(DemoEnv { smoke: self.smoke }))
            .unwrap_or_default();
        let window = if let Some(window) = &self.window {
            Arc::clone(window)
        } else {
            Arc::new(
                event_loop.create_window(
                    Window::default_attributes()
                        .with_title("Voxy 2D/3D | Space: pause | Esc: exit")
                        .with_visible(false)
                        .with_inner_size(match self.host_request.physical_size {
                            Some([width, height]) => winit::dpi::Size::Physical(
                                winit::dpi::PhysicalSize::new(width, height),
                            ),
                            None => {
                                winit::dpi::Size::Logical(winit::dpi::LogicalSize::new(900., 650.))
                            }
                        }),
                )?,
            )
        };
        let size = window.inner_size();
        let options = self.graphics;
        let instance = options.create_instance_with_display(event_loop.owned_display_handle());
        let mut host = pollster::block_on(Renderer::new_with_instance(
            Arc::clone(&window),
            size.width,
            size.height,
            options,
            instance,
        ))?;
        if self.host_request.unthrottled_presentation {
            match host.enable_unthrottled_presentation() {
                Ok(mode) => eprintln!("GPU SECONDARY presentation={mode:?}"),
                Err(error) => eprintln!(
                    "GPU SECONDARY non-vsync unavailable: {error}; retaining existing presentation"
                ),
            }
        }
        if self.motion_vectors == MotionMode::Enabled {
            pollster::block_on(host.enable_motion_vectors())?;
        }
        if self.host_request.msaa4 {
            host.enable_msaa4()?;
        }
        let mut scene = host.create_scene_renderer();
        let demo_texture = match &mut self.demo {
            Some(demo) => demo.create(DemoGpu {
                device: host.device(),
                queue: host.queue(),
                scene: &mut scene,
                size: [size.width, size.height],
                paused: self.paused,
            })?,
            None => None,
        };
        let cube = scene.upload_mesh(
            host.device(),
            &if self.rush.is_some() {
                rush_box_mesh([0.2, 0.65, 0.95, 1.0])?
            } else {
                cube_mesh()?
            },
        )?;
        let rush_floor = self
            .rush
            .as_ref()
            .map(|_| scene.upload_mesh(host.device(), &rush_box_mesh([0.15, 0.19, 0.25, 1.0])?))
            .transpose()?;
        let rush_crate = self
            .rush
            .as_ref()
            .map(|_| scene.upload_mesh(host.device(), &rush_box_mesh([0.95, 0.55, 0.15, 1.0])?))
            .transpose()?;
        let overlay = scene.upload_mesh(host.device(), &SceneMesh::quad([1.0, 1.0, 1.0, 0.8]))?;
        let texture = if let Some(texture) = demo_texture {
            texture
        } else if self.demo.is_some() || self.rush.is_some() {
            scene.upload_texture(host.device(), host.queue(), 1, 1, &[255; 4])?
        } else {
            scene.upload_texture(
                host.device(),
                host.queue(),
                2,
                2,
                &[
                    255, 180, 50, 255, 50, 180, 255, 255, 50, 180, 255, 255, 255, 180, 50, 255,
                ],
            )?
        };
        let world_transform = scene.create_transform(host.device(), Mat4::IDENTITY)?;
        let overlay_transform = scene.create_transform(host.device(), Mat4::IDENTITY)?;
        println!("SCENE GPU: {:?}", host.adapter_info());
        // Fresh resources (first start or GPU recovery): forget the cached title so it is re-sent.
        self.last_title.clear();
        if let Some(title) = self.demo.as_ref().and_then(|demo| demo.title(self.paused)) {
            set_title_if_changed(&window, &mut self.last_title, title);
        }
        self.motion.reset();
        self.resources = Some(Resources {
            host,
            scene,
            cube,
            overlay,
            texture,
            world_transform,
            last_world_motion: None,
            last_overlay_mvp: None,
            overlay_transform,
            motion_frames: 0,
            rush_floor,
            rush_crate,
            rush_panel: None,
            script_transforms: std::collections::HashMap::new(),
        });
        window.set_visible(true);
        if self.rush.is_some() || self.smoke || self.host_request.focus {
            window.focus_window();
        }
        self.window = Some(window);
        self.last = Instant::now();
        Ok(())
    }
    pub(super) fn recreate_gpu(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.recovering_gpu = true;
        self.release_gpu();
        self.motion.reset();
        self.frame_loop.reset();
        self.last = Instant::now();
        self.initialize(event_loop)?;
        eprintln!("SCENE GPU RECOVERED: resources recreated, history reset");
        Ok(())
    }
}

pub(super) fn rush_box_mesh(color: [f32; 4]) -> Result<SceneMesh, voxy_render::SceneError> {
    let cube = cube_mesh()?;
    let vertices = cube
        .vertices()
        .iter()
        .map(|vertex| {
            let mut v = *vertex;
            let shade = 0.65
                + if v.position[1] > 0.0 { 0.25 } else { 0.0 }
                + if v.position[0] < 0.0 { 0.1 } else { 0.0 };
            v.position = v.position.map(|coordinate| coordinate * 0.5);
            v.color = [
                color[0] * shade,
                color[1] * shade,
                color[2] * shade,
                color[3],
            ];
            v
        })
        .collect();
    SceneMesh::new(vertices, cube.indices().to_vec())
}

pub(super) fn cube_mesh() -> Result<SceneMesh, voxy_render::SceneError> {
    let positions = [
        [-1.0, -1.0, -1.0],
        [1.0, -1.0, -1.0],
        [1.0, 1.0, -1.0],
        [-1.0, 1.0, -1.0],
        [-1.0, -1.0, 1.0],
        [1.0, -1.0, 1.0],
        [1.0, 1.0, 1.0],
        [-1.0, 1.0, 1.0],
    ];
    let vertices = positions
        .into_iter()
        .enumerate()
        .map(|(i, position)| SceneVertex {
            position,
            uv: [
                if i % 2 == 0 { 0.0 } else { 1.0 },
                if i % 4 < 2 { 1.0 } else { 0.0 },
            ],
            color: [1.0; 4],
        })
        .collect();
    SceneMesh::new(
        vertices,
        vec![
            0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 3, 7, 6, 3, 6, 2, 0, 4, 7, 0, 7,
            3, 1, 2, 6, 1, 6, 5,
        ],
    )
}
