//! Photorealistic *Bodycam* / *Unrecord* Tactical Showcase & Snapshot Generator.
//!
//! Demonstrates Voxy's full photorealistic rendering pipeline:
//! - CIE Standard Overcast Sky ($L_z \cdot (1 + 2\cos\theta)/3$) diffuse hemispherical GI
//! - Jimenez 2016 Multi-Bounce Albedo Ambient Occlusion (`gtao_multibounce`) & Lagarde Specular Occlusion
//! - Tokuyoshi & Kaplanyan 2021 Screen-Space Derivative Specular Anti-Aliasing (`dpdx`/`dpdy`)
//! - Dual-Lobe Wet Clearcoat ($F_0 = 0.02$) & Procedural FBM Concrete/Soil Micro-Relief
//! - Dual-Zone IES Tactical Flashlight + Henyey-Greenstein ($g = 0.72$) Volumetric Dust Beam
//! - 6-DOF Chest-Mounted Camera Rig (Elliptical Free-Aim Deadzone + Analytical Spring-Damper + Stepped AE)
//! - Physical Lens/CMOS/ISP Post-Processing (Kannala-Brandt Fisheye, Cauchy Chromatic Aberration,
//!   $\cos^4\theta$ + Bezel Vignette, Rolling Shutter, Foi Poisson-Gaussian Sensor Noise, AgX Tone Mapping,
//!   ISP $3\times 3$ Unsharp Mask Halos, 4:2:0 YCbCr Quantization)
//!
//! Run interactively:
//!   `cargo run -p voxy_app --example bodycam`
//! Capture deterministic PNG snapshot:
//!   `cargo run -p voxy_app --example bodycam -- --snapshot /tmp/voxy-bodycam-preview.png`

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    dead_code
)]

use std::{path::PathBuf, sync::Arc, time::Instant};

use glam::{Mat4, Vec2, Vec3};
use voxy_core::{ChunkPos, VoxelPos};
use voxy_demos::{female_complexion, female_demo::FemaleDemo, female_eyes, full_model_worker};
use voxy_render::{
    CameraView, RenderOutcome, Renderer, SceneCamera, SceneDraw, SceneGeometry, SceneProjection,
    SceneRenderer, SceneTexture, SceneTransform, TextureFilter, TextureSampling,
};
use voxy_runtime::{build_procedural_scene, rebuild_bootstrap_chunks};
use voxy_world::{BlockStateId, EditSource, EditTxn, ResourceKey, VoxelWrite};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

/// Photorealistic female character (`FemaleDemo`) placed inside the concrete tactical bunker
/// and rendered into the linear `Rgba16Float` HDR target before Bodycam/CleanCam/Camcorder
/// lens/CMOS/ISP post-processing.
pub(crate) struct BunkerFemaleScene {
    pub model: FemaleDemo,
    pub scene_renderer: SceneRenderer,
    pub body_geometry: SceneGeometry,
    pub hair_geometry: SceneGeometry,
    pub texture: SceneTexture,
    pub transform: SceneTransform,
    pub world_pos: Vec3,
    pub world_yaw: f32,
}

impl BunkerFemaleScene {
    pub fn new(renderer: &Renderer) -> Result<Self, Box<dyn std::error::Error>> {
        let device = renderer.device();
        let queue = renderer.queue();
        let mut model = FemaleDemo::new().map_err(|e| format!("FemaleDemo::new: {e}"))?;
        model.show_complexion = true;
        model.secondary_only = false;
        model.animation_only = true;
        model.surface_diffusion_enabled = true;
        model.preview_pose(0.35);

        let scene_renderer = renderer.create_hdr_scene_renderer();
        // Enhance the skin/eye/hair shaders for the HDR tactical bunker lighting:
        // combine the overcast skylight shaft with the operator's tactical LED flashlight
        // (aligned with `+Z` in character model space, producing realistic corneal catchlights,
        // epidermal subsurface wrap, and dual-lobe wet/sebaceous specular highlights).
        let bunker_body_shader = female_eyes::MATERIAL_SHADER.replace(
            "return vec4<f32>(texel.rgb*in.color.rgb*diffuse_ratio+vec3<f32>(1.0,0.96,0.92)*highlight*visibility,1.0);",
            "let flash_dir = normalize(in.to_camera + vec3<f32>(0.08, -0.05, 0.0)); \
             let flash_dist = max(length(in.to_camera), 0.55); \
             let flash_atten = 2.85 / (1.0 + 0.28 * flash_dist * flash_dist); \
             let flash_cone = smoothstep(0.72, 0.96, dot(flash_dir, normalize(vec3<f32>(0.0, 0.05, 1.0)))); \
             let flash_wrap = vec3<f32>(0.38, 0.19, 0.09); \
             let flash_diff = max((vec3<f32>(dot(diffuse_normal, flash_dir)) + flash_wrap) / (vec3<f32>(1.0) + flash_wrap), vec3<f32>(0.0)) / (vec3<f32>(1.0) + flash_wrap); \
             let flash_spec = skin_area_highlight(normal, view, flash_dir, max(0.16, roughness * 0.72), min(1.0, oil + 0.35)); \
             let overcast_amb = vec3<f32>(0.26, 0.29, 0.34) * (0.55 + 0.45 * clamp(normal.y * 0.5 + 0.5, 0.0, 1.0)); \
             let flash_rgb = vec3<f32>(1.08, 1.03, 0.96) * flash_atten * (0.50 + 0.50 * flash_cone); \
             let hdr_skin = texel.rgb * in.color.rgb * (0.55 * diffuse_ratio + overcast_amb + 0.85 * flash_diff * flash_rgb) \
                 + vec3<f32>(1.0, 0.97, 0.93) * (0.70 * highlight + 1.45 * flash_spec * flash_atten) * visibility; \
             return vec4<f32>(hdr_skin, 1.0);",
        ).replace(
            "return vec4<f32>(pigment*irradiance*transmission+vec3<f32>(reflection),1.0);",
            "let flash_dir = normalize(in.to_camera + vec3<f32>(0.05, -0.03, 0.0)); \
             let flash_catch = corneal_catchlight(eye_normal, view, flash_dir, 0.075, 34.0) * visibility; \
             return vec4<f32>(pigment * irradiance * transmission * 1.18 + vec3<f32>(reflection + flash_catch), 1.0);",
        ).replace(
            "let color=texel.rgb*in.color.rgb*pigmentation*areola_tint*(diffuse/max(original,0.2))\n            +vec3<f32>(1.0,0.96,0.92)*specular;\n        return vec4<f32>(color,alpha);",
            "let flash_dir = normalize(in.to_camera + vec3<f32>(0.08, -0.05, 0.0)); \
             let flash_dist = max(length(in.to_camera), 0.55); \
             let flash_atten = 2.65 / (1.0 + 0.30 * flash_dist * flash_dist); \
             let flash_cone = smoothstep(0.70, 0.95, dot(flash_dir, normalize(vec3<f32>(0.0, 0.02, 1.0)))); \
             let flash_kd = max((vec3<f32>(dot(normal, flash_dir)) + wrap) / (vec3<f32>(1.0) + wrap), vec3<f32>(0.0)) / (vec3<f32>(1.0) + wrap); \
             let flash_spec = 1.15 * ggx(skin_normal, view, flash_dir, max(0.24, roughness * 0.78)) \
                 + 0.35 * ggx_f0(skin_normal, view, flash_dir, 0.18, 0.038); \
             let rim = pow(clamp(1.0 - max(dot(skin_normal, view), 0.0), 0.0, 1.0), 3.2); \
             let sss_rim = vec3<f32>(0.22, 0.09, 0.06) * rim * (0.4 + 0.6 * max(normal.y, 0.0)); \
             let overcast_amb = vec3<f32>(0.22, 0.25, 0.30) * (0.50 + 0.50 * clamp(normal.y * 0.5 + 0.5, 0.0, 1.0)); \
             let flash_light = vec3<f32>(1.06, 1.01, 0.95) * flash_atten * (0.45 + 0.55 * flash_cone); \
             let color = texel.rgb * in.color.rgb * pigmentation * areola_tint * (0.48 * (diffuse / max(original, 0.2)) + overcast_amb + 0.72 * flash_kd * flash_light) \
                 + sss_rim + vec3<f32>(1.0, 0.97, 0.93) * (0.65 * specular + flash_spec * flash_atten * (0.5 + 0.5 * flash_cone)); \
             return vec4<f32>(color, alpha);",
        );

        let mesh = model.mesh().map_err(|e| format!("model.mesh(): {e}"))?;
        let (body_indices, hair_indices) =
            full_model_worker::partition_hair(&mesh).map_err(|e| format!("partition_hair: {e}"))?;
        let mut partitions = scene_renderer
            .upload_shared_mesh_partitions(device, &mesh)
            .map_err(|e| format!("upload_shared_mesh_partitions: {e}"))?;
        let mut hair_geometry = partitions.pop().ok_or("missing hair partition")?;
        let mut body_geometry = partitions.pop().ok_or("missing body partition")?;
        body_geometry
            .update_index_partition(queue, &body_indices)
            .map_err(|e| format!("body update_index_partition: {e}"))?;
        hair_geometry
            .update_index_partition(queue, &hair_indices)
            .map_err(|e| format!("hair update_index_partition: {e}"))?;
        pollster::block_on(scene_renderer.set_geometry_opaque_shader(
            device,
            &mut body_geometry,
            &bunker_body_shader,
        ))
        .map_err(|e| format!("set body shader: {e}"))?;
        pollster::block_on(scene_renderer.set_geometry_opaque_shader(
            device,
            &mut hair_geometry,
            voxy_demos::FEMALE_HAIR_MATERIAL_SHADER,
        ))
        .map_err(|e| format!("set hair shader: {e}"))?;

        let texture = scene_renderer.upload_texture_with_sampling(
            device,
            queue,
            female_complexion::WIDTH,
            female_complexion::SIZE,
            female_complexion::atlas(),
            TextureSampling {
                min_filter: TextureFilter::Linear,
                mag_filter: TextureFilter::Linear,
                ..Default::default()
            },
        )?;
        let transform = scene_renderer.create_transform(device, Mat4::IDENTITY)?;

        // Place her standing on the wet concrete bunker floor (`y = 15.0 + 0.82 = 15.82`)
        // directly in the skylight + tactical flashlight beam at ~1.48 m in front of the operator,
        // facing the operator (`yaw = PI`, slightly angled for 3D depth).
        Ok(Self {
            model,
            scene_renderer,
            body_geometry,
            hair_geometry,
            texture,
            transform,
            world_pos: Vec3::new(15.65, 15.82, 9.92),
            world_yaw: std::f32::consts::PI - 0.12,
        })
    }

    pub fn update_for_camera(
        &mut self,
        queue: &wgpu::Queue,
        camera: CameraView,
        width: u32,
        height: u32,
        elapsed_time: f32,
        animate_step: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if animate_step {
            self.model
                .preview_pose(0.35 + f64::from(elapsed_time) * 0.45);
            let mesh = self.model.mesh()?;
            let (body_indices, hair_indices) = full_model_worker::partition_hair(&mesh)?;
            self.body_geometry.update(queue, &mesh)?;
            self.body_geometry
                .update_index_partition(queue, &body_indices)?;
            self.hair_geometry
                .update_index_partition(queue, &hair_indices)?;
        }

        let aspect = width.max(1) as f32 / height.max(1) as f32;
        let scene_cam = SceneCamera {
            eye: camera.eye,
            target: camera.target,
            up: camera.up,
            projection: SceneProjection::Perspective {
                vertical_fov: camera.vertical_fov_radians.clamp(0.15, 2.65),
                aspect,
                near: camera.near_plane.clamp(0.02, 10.0),
                far: 512.0,
            },
        };
        let model_mat =
            Mat4::from_translation(self.world_pos) * Mat4::from_rotation_y(self.world_yaw);
        let model_inv = model_mat.inverse();
        let mvp = scene_cam.view_projection()? * model_mat;
        let local_eye = model_inv.transform_point3(camera.eye);

        self.transform.update(queue, mvp)?;
        self.transform.update_view_position(queue, local_eye)?;
        Ok(())
    }

    pub fn draws(&self) -> [SceneDraw<'_>; 2] {
        [
            SceneDraw {
                geometry: &self.body_geometry,
                texture: &self.texture,
                transform: &self.transform,
                overlay: false,
            },
            SceneDraw {
                geometry: &self.hair_geometry,
                texture: &self.texture,
                transform: &self.transform,
                overlay: false,
            },
        ]
    }
}

struct BodycamDemo {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    female_scene: Option<BunkerFemaleScene>,
    snapshot_path: Option<PathBuf>,
    smoke: bool,
    start: Instant,
    last_tick: Instant,
    frames: u64,
    flashlight_on: bool,
    wetness: f32,
    failure: Option<String>,
}

impl Default for BodycamDemo {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            window: None,
            renderer: None,
            female_scene: None,
            snapshot_path: None,
            smoke: false,
            start: now,
            last_tick: now,
            frames: 0,
            flashlight_on: true,
            wetness: 0.72,
            failure: None,
        }
    }
}

#[allow(clippy::too_many_lines)]
pub(crate) fn build_tactical_compound_chunks() -> Result<Vec<voxy_runtime::BootstrapChunk>, String>
{
    let scene = build_procedural_scene(0x56_4f_58_59, 1).map_err(|e| e.to_string())?;
    let mut world = scene.world;
    let stone = world
        .registry()
        .find(&ResourceKey::parse("voxy:stone").map_err(|e| e.to_string())?)
        .ok_or("missing voxy:stone")?;
    let dirt = world
        .registry()
        .find(&ResourceKey::parse("voxy:dirt").map_err(|e| e.to_string())?)
        .ok_or("missing voxy:dirt")?;
    let grass = world
        .registry()
        .find(&ResourceKey::parse("voxy:grass").map_err(|e| e.to_string())?)
        .ok_or("missing voxy:grass")?;
    let water = world
        .registry()
        .find(&ResourceKey::parse("voxy:water_8").map_err(|e| e.to_string())?)
        .ok_or("missing voxy:water_8")?;

    let mut writes = Vec::new();
    let base_y = 14;

    // 1. Excavate interior courtyard and tactical concrete bunker volume
    for z in 4..=29 {
        for x in 4..=28 {
            for y in base_y..=(base_y + 11) {
                writes.push(VoxelWrite {
                    pos: VoxelPos { x, y, z },
                    block: BlockStateId::AIR,
                });
            }
            // Reinforced concrete slab floor with weathered patches and rainwater puddles
            let floor_block =
                if (12..=19).contains(&x) && (13..=20).contains(&z) && (x + z) % 3 == 0 {
                    water
                } else if (x * 7 + z * 13) % 11 == 0 {
                    dirt
                } else {
                    stone
                };
            writes.push(VoxelWrite {
                pos: VoxelPos { x, y: base_y, z },
                block: floor_block,
            });
        }
    }

    // 2. Perimeter concrete walls with tactical window embrasures and breached doorway
    for y in (base_y + 1)..=(base_y + 8) {
        for x in 6..=26 {
            // North back wall (z = 27) with high clerestory skylight slots
            let is_north_skylight = (y >= base_y + 5 && y <= base_y + 7)
                && ((9..=12).contains(&x) || (20..=23).contains(&x));
            if !is_north_skylight {
                writes.push(VoxelWrite {
                    pos: VoxelPos { x, y, z: 27 },
                    block: stone,
                });
            }
            // Mid-hallway partition wall (z = 18) with wide breached central archway
            let is_archway = (12..=19).contains(&x) && y <= base_y + 6;
            let is_side_window = (8..=9).contains(&x) && (base_y + 3..=base_y + 5).contains(&y);
            if !is_archway && !is_side_window {
                writes.push(VoxelWrite {
                    pos: VoxelPos { x, y, z: 18 },
                    block: stone,
                });
            }
        }
        for z in 6..=27 {
            // West wall (x = 6) with vertical daylight slits casting overcast sky shafts
            let is_west_window = (base_y + 3..=base_y + 6).contains(&y)
                && ((10..=13).contains(&z) || (21..=24).contains(&z));
            if !is_west_window {
                writes.push(VoxelWrite {
                    pos: VoxelPos { x: 6, y, z },
                    block: stone,
                });
            }
            // East wall (x = 26) with tactical breach opening
            let is_east_breach = (base_y + 2..=base_y + 6).contains(&y) && (12..=15).contains(&z);
            if !is_east_breach {
                writes.push(VoxelWrite {
                    pos: VoxelPos { x: 26, y, z },
                    block: stone,
                });
            }
        }
    }

    // 3. Heavy concrete roof beams & partial collapsed ceiling slabs (allowing overcast sky GI in)
    for z in 7..=26 {
        for x in 7..=25 {
            let is_skylight_opening = ((10..=15).contains(&x) && (10..=15).contains(&z))
                || ((17..=22).contains(&x) && (20..=24).contains(&z));
            if !is_skylight_opening {
                writes.push(VoxelWrite {
                    pos: VoxelPos {
                        x,
                        y: base_y + 8,
                        z,
                    },
                    block: stone,
                });
            }
            // Exposed structural ceiling ribs every 4 meters
            if z % 4 == 0 {
                writes.push(VoxelWrite {
                    pos: VoxelPos {
                        x,
                        y: base_y + 7,
                        z,
                    },
                    block: stone,
                });
            }
        }
    }

    // 4. Interior structural support pillars & ballistic cover barriers
    for &(px, pz) in &[(11, 12), (21, 12), (11, 22), (21, 22)] {
        for y in (base_y + 1)..=(base_y + 7) {
            writes.push(VoxelWrite {
                pos: VoxelPos { x: px, y, z: pz },
                block: stone,
            });
            writes.push(VoxelWrite {
                pos: VoxelPos {
                    x: px + 1,
                    y,
                    z: pz,
                },
                block: stone,
            });
        }
    }

    // Low concrete jersey barrier & rubble blocks in the midground behind the subject
    for x in 14..=18 {
        for y in (base_y + 1)..=(base_y + 2) {
            writes.push(VoxelWrite {
                pos: VoxelPos { x, y, z: 14 },
                block: if x == 16 && y == base_y + 2 {
                    dirt
                } else {
                    stone
                },
            });
        }
    }
    // Overgrown moss/foliage and fallen masonry blocks near the skylight shaft
    for &(rx, ry, rz, blk) in &[
        (10, base_y + 1, 15, dirt),
        (10, base_y + 2, 15, grass),
        (13, base_y + 1, 16, stone),
        (19, base_y + 1, 20, stone),
        (20, base_y + 1, 21, dirt),
        (20, base_y + 2, 21, grass),
        (15, base_y + 1, 23, stone),
        (16, base_y + 1, 23, stone),
        (16, base_y + 2, 23, stone),
        (16, base_y + 3, 23, dirt),
    ] {
        writes.push(VoxelWrite {
            pos: VoxelPos {
                x: rx,
                y: ry,
                z: rz,
            },
            block: blk,
        });
    }

    let mut deduped = std::collections::BTreeMap::new();
    for write in writes {
        deduped.insert((write.pos.x, write.pos.y, write.pos.z), write);
    }
    let writes: Vec<VoxelWrite> = deduped.into_values().collect();

    world
        .commit(EditTxn {
            source: EditSource::Simulation,
            expected: Vec::new(),
            writes,
        })
        .map_err(|e| e.to_string())?;

    let positions: Vec<ChunkPos> = scene.chunks.iter().map(|c| c.pos).collect();
    rebuild_bootstrap_chunks(&world, &positions, 2).map_err(|e| e.to_string())
}

impl BodycamDemo {
    fn update_and_draw(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let now = Instant::now();
        let dt = now
            .duration_since(self.last_tick)
            .as_secs_f32()
            .clamp(1.0 / 60.0, 0.05);
        self.last_tick = now;
        let t = now.duration_since(self.start).as_secs_f32();

        let renderer = self.renderer.as_mut().expect("initialized renderer");
        renderer.set_bodycam_enabled(true);

        // Tactical operator standing at feet_pos inside the concrete bunker facing the female character
        let actor_feet_pos = Vec3::new(
            15.6 + (t * 0.55).sin() * 0.18,
            15.0,
            8.45 + (t * 0.35).cos() * 0.14,
        );
        let look_delta = Vec2::new((t * 1.1).cos() * 0.0012, (t * 0.85).sin() * 0.0009);
        let planar_speed = if self.snapshot_path.is_some() {
            0.35
        } else {
            1.15
        };

        {
            let rig = renderer.bodycam_rig_mut();
            rig.flashlight_enabled = self.flashlight_on;
            rig.apply_look(look_delta);
            rig.step(dt, planar_speed, true, false, 0.34);
        }
        let (camera, flash_pos, flash_dir) =
            renderer.bodycam_rig().camera_and_flashlight(actor_feet_pos);
        renderer.update_camera(camera)?;
        renderer.set_bodycam_flashlight_and_wetness(flash_pos, flash_dir, self.wetness);

        let size = self.window.as_ref().map_or((1280, 720), |w| {
            let s = w.inner_size();
            (s.width.max(1280), s.height.max(720))
        });
        if let Some(female_scene) = &mut self.female_scene {
            female_scene.update_for_camera(
                renderer.queue(),
                camera,
                size.0,
                size.1,
                dt,
                self.snapshot_path.is_none() && self.frames.is_multiple_of(2),
            )?;
        }

        if let Some(snapshot_path) = self.snapshot_path.take() {
            let (width, height, rgba) = if let Some(female_scene) = &self.female_scene {
                let draws = female_scene.draws();
                renderer.capture_bodycam_with_scene_rgba8(
                    self.frames + 1,
                    Some((&female_scene.scene_renderer, &draws)),
                )?
            } else {
                renderer.capture_bodycam_rgba8(self.frames + 1)?
            };
            image::save_buffer(
                &snapshot_path,
                &rgba,
                width,
                height,
                image::ColorType::Rgba8,
            )?;
            println!(
                "BODYCAM SNAPSHOT OK: saved {}x{} PNG to {}",
                width,
                height,
                snapshot_path.display()
            );
            event_loop.exit();
            return Ok(());
        }

        let outcome = if let Some(female_scene) = &self.female_scene {
            let draws = female_scene.draws();
            renderer.render_bodycam_with_scene(&female_scene.scene_renderer, &draws)?
        } else {
            renderer.render()?
        };
        if matches!(outcome, RenderOutcome::Presented) {
            self.frames += 1;
        }
        if self.smoke && self.frames >= 12 {
            println!("BODYCAM SMOKE OK: {} frames presented", self.frames);
            event_loop.exit();
        }
        Ok(())
    }
}

impl ApplicationHandler for BodycamDemo {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let headless = self.snapshot_path.is_some() || self.smoke;
            let window = Arc::new(
                event_loop.create_window(
                    Window::default_attributes()
                        .with_title("Voxy Bodycam Photorealistic Showcase — Female Character in Tactical Bunker")
                        .with_visible(!headless)
                        .with_inner_size(winit::dpi::LogicalSize::new(1280, 720)),
                )?,
            );
            let size = window.inner_size();
            let mut renderer = pollster::block_on(Renderer::new(
                Arc::clone(&window),
                size.width.max(1280),
                size.height.max(720),
            ))?;
            renderer.set_bodycam_enabled(true);
            {
                let rig = renderer.bodycam_rig_mut();
                // Frame the female character standing at ~1.9 m in front of the chest-mounted bodycam
                rig.aim_angles = Vec2::new(0.04, -0.04);
                rig.chest_goal_angles = Vec2::new(0.04, -0.04);
                for _ in 0..24 {
                    rig.step(1.0 / 60.0, 0.35, true, false, 0.34);
                }
            }
            let chunks = build_tactical_compound_chunks()?;
            let uploads: Vec<_> = chunks.iter().map(|c| (c.pos, &c.mesh, &c.light)).collect();
            renderer.upload_lit_chunks(&uploads, ChunkPos::default())?;
            let female_scene = BunkerFemaleScene::new(&renderer)?;
            println!(
                "Voxy Bodycam GPU ready: {} ({} chunks + CC0 female character uploaded)",
                renderer.adapter_info().name,
                uploads.len()
            );
            self.female_scene = Some(female_scene);
            self.renderer = Some(renderer);
            self.start = Instant::now();
            self.last_tick = self.start;
            self.window = Some(window);
            if headless {
                self.update_and_draw(event_loop)?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("Bodycam showcase startup failed: {error}");
            self.failure = Some(error.to_string());
            event_loop.exit();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let pressed = event.state == ElementState::Pressed;
                if let PhysicalKey::Code(key) = event.physical_key {
                    match key {
                        KeyCode::Escape if pressed => event_loop.exit(),
                        KeyCode::KeyF if pressed && !event.repeat => {
                            self.flashlight_on = !self.flashlight_on;
                        }
                        KeyCode::KeyB if pressed && !event.repeat => {
                            if let Some(renderer) = &mut self.renderer {
                                renderer
                                    .bodycam_rig_mut()
                                    .set_lens_profile(voxy_render::PhotorealLensProfile::Bodycam);
                                renderer.set_bodycam_enabled(true);
                            }
                        }
                        KeyCode::KeyV if pressed && !event.repeat => {
                            if let Some(renderer) = &mut self.renderer {
                                renderer
                                    .bodycam_rig_mut()
                                    .set_lens_profile(voxy_render::PhotorealLensProfile::CleanCam);
                                renderer.set_bodycam_enabled(true);
                            }
                        }
                        KeyCode::KeyN if pressed && !event.repeat => {
                            if let Some(renderer) = &mut self.renderer {
                                renderer
                                    .bodycam_rig_mut()
                                    .set_lens_profile(voxy_render::PhotorealLensProfile::Camcorder);
                                renderer.set_bodycam_enabled(true);
                            }
                        }
                        KeyCode::Space if pressed && !event.repeat => {
                            if let Some(renderer) = &mut self.renderer {
                                renderer
                                    .bodycam_rig_mut()
                                    .add_recoil_impulse(0.055, -0.018, 0.032);
                            }
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let lines = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 40.0,
                };
                if let Some(renderer) = &mut self.renderer {
                    renderer.bodycam_rig_mut().apply_servo_zoom(lines);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.update_and_draw(event_loop) {
                    eprintln!("Bodycam showcase draw failed: {error}");
                    self.failure = Some(error.to_string());
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mut snapshot_path = None;
    let mut smoke = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--snapshot" => {
                let path = args
                    .next()
                    .ok_or("--snapshot requires an output PNG path")?;
                snapshot_path = Some(PathBuf::from(path));
            }
            "--smoke" => smoke = true,
            other => return Err(format!("unknown argument: {other}").into()),
        }
    }
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = BodycamDemo {
        snapshot_path,
        smoke,
        ..BodycamDemo::default()
    };
    event_loop.run_app(&mut app)?;
    if let Some(error) = app.failure {
        return Err(error.into());
    }
    Ok(())
}
