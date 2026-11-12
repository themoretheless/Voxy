//! Unified render graph composing `Renderer` (voxels, resident skinned meshes, indirect draws,
//! native vegetation), `SceneRenderer` (general 3D meshes, overlays, transparencies),
//! `SceneInstancer` (GPU-instanced mesh batches), `ArchiveSceneRenderer` (baked GI / PBR street scenes),
//! and `BodycamPostProcess` into a single declarative frame pipeline over shared color and
//! Reversed-Z depth attachments.

use glam::Vec3;

use crate::{
    BodycamPostProcess, OceanCamera, OceanRenderer, Renderer, SceneDraw, SceneError,
    SceneInstancedDraw, SceneInstancer, SceneRenderer, ScreenSpaceFluidRenderer,
    archive_scene::ArchiveSceneRenderer,
};

/// Declarative pass node in a [`RenderGraph`].
#[derive(Debug)]
pub enum RenderGraphNode<'a> {
    /// Clears color and Reversed-Z depth (`0.0`) on the active target.
    Clear { color: wgpu::Color },
    /// Encodes the voxel world (`Renderer`: opaque/cutout/translucent chunks, resident skinned mesh,
    /// indirect voxel buffer, and native vegetation) into the active target.
    VoxelWorld {
        renderer: &'a Renderer,
        /// When `true`, clears color and depth before drawing voxels; when `false`, composites
        /// over the existing color and Reversed-Z depth buffers.
        clear: bool,
    },
    /// Encodes `SceneRenderer` draws (solid opaque, alpha-tested opaque, transparent, X-ray, overlays).
    Scene {
        scene: &'a SceneRenderer,
        draws: &'a [SceneDraw<'a>],
        /// Optional clear color; when `None`, draws over existing color and depth attachments
        /// (`SceneRenderer::encode_over`).
        clear: Option<wgpu::Color>,
    },
    /// Encodes GPU-instanced `SceneGeometry` batches in a single `draw_indexed` call per batch.
    InstancedScene {
        instancer: &'a SceneInstancer,
        draws: &'a [SceneInstancedDraw<'a>],
        /// Optional clear color; when `None`, draws over existing color and depth attachments.
        clear: Option<wgpu::Color>,
    },
    /// Encodes UI/overlay `SceneDraw`s over existing color and depth without modifying depth.
    SceneOverlays {
        scene: &'a SceneRenderer,
        draws: &'a [SceneDraw<'a>],
    },
    /// Executes `ArchiveSceneRenderer::draw` into the target color view and synchronizes its
    /// Reversed-Z depth buffer so subsequent `Scene` or `InstancedScene` nodes can composite over it.
    ArchiveScene {
        archive: &'a mut ArchiveSceneRenderer,
        eye: Vec3,
        direction: Vec3,
    },
    /// Resolves linear `Rgba16Float` HDR color through `BodycamPostProcess` into `output_view`.
    BodycamPost {
        post: &'a BodycamPostProcess,
        output_view: &'a wgpu::TextureView,
    },
    /// Advances and encodes the Tessendorf FFT `OceanRenderer` into the active color target.
    Ocean {
        ocean: &'a mut OceanRenderer,
        queue: &'a wgpu::Queue,
        camera: OceanCamera,
        dt: f32,
    },
    /// Encodes `ScreenSpaceFluidRenderer` (opaque scene background, sphere/film depth, ray
    /// thickness, bilateral smoothing, and dielectric refraction/Beer-Lambert composite) into
    /// the active color target.
    ScreenSpaceFluid {
        fluid: &'a ScreenSpaceFluidRenderer,
        scene: &'a SceneRenderer,
        clear: wgpu::Color,
        draws: &'a [SceneDraw<'a>],
    },
}

/// Composable frame render graph uniting `Renderer`, `SceneRenderer`, `SceneInstancer`,
/// and `ArchiveSceneRenderer` over shared color and Reversed-Z depth targets.
#[derive(Debug, Default)]
pub struct RenderGraph<'a> {
    nodes: Vec<RenderGraphNode<'a>>,
}

impl<'a> RenderGraph<'a> {
    /// Creates an empty render graph.
    #[must_use]
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Returns the number of scheduled pass nodes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Appends a pass node to the graph.
    pub fn push(&mut self, node: RenderGraphNode<'a>) -> &mut Self {
        self.nodes.push(node);
        self
    }

    /// Schedules a clear of both color and Reversed-Z depth (`0.0`).
    pub fn add_clear(&mut self, color: wgpu::Color) -> &mut Self {
        self.push(RenderGraphNode::Clear { color })
    }

    /// Schedules resident voxel world rendering (`Renderer`).
    pub fn add_voxel_world(&mut self, renderer: &'a Renderer, clear: bool) -> &mut Self {
        self.push(RenderGraphNode::VoxelWorld { renderer, clear })
    }

    /// Schedules `SceneRenderer` mesh draws, either clearing with `clear` or compositing over
    /// existing color and depth (`clear = None`).
    pub fn add_scene(
        &mut self,
        scene: &'a SceneRenderer,
        draws: &'a [SceneDraw<'a>],
        clear: Option<wgpu::Color>,
    ) -> &mut Self {
        self.push(RenderGraphNode::Scene {
            scene,
            draws,
            clear,
        })
    }

    /// Schedules GPU-instanced `SceneInstancedDraw` batches.
    pub fn add_instanced_scene(
        &mut self,
        instancer: &'a SceneInstancer,
        draws: &'a [SceneInstancedDraw<'a>],
        clear: Option<wgpu::Color>,
    ) -> &mut Self {
        self.push(RenderGraphNode::InstancedScene {
            instancer,
            draws,
            clear,
        })
    }

    /// Schedules overlay-only `SceneDraw`s on top of the composed frame.
    pub fn add_scene_overlays(
        &mut self,
        scene: &'a SceneRenderer,
        draws: &'a [SceneDraw<'a>],
    ) -> &mut Self {
        self.push(RenderGraphNode::SceneOverlays { scene, draws })
    }

    /// Schedules an `ArchiveSceneRenderer` pass into the unified frame.
    pub fn add_archive_scene(
        &mut self,
        archive: &'a mut ArchiveSceneRenderer,
        eye: Vec3,
        direction: Vec3,
    ) -> &mut Self {
        self.push(RenderGraphNode::ArchiveScene {
            archive,
            eye,
            direction,
        })
    }

    /// Schedules a `BodycamPostProcess` resolve pass into `output_view`.
    pub fn add_bodycam_post(
        &mut self,
        post: &'a BodycamPostProcess,
        output_view: &'a wgpu::TextureView,
    ) -> &mut Self {
        self.push(RenderGraphNode::BodycamPost { post, output_view })
    }

    /// Schedules an `OceanRenderer` FFT simulation and surface/atmosphere/bloom composite pass.
    pub fn add_ocean(
        &mut self,
        ocean: &'a mut OceanRenderer,
        queue: &'a wgpu::Queue,
        camera: OceanCamera,
        dt: f32,
    ) -> &mut Self {
        self.push(RenderGraphNode::Ocean {
            ocean,
            queue,
            camera,
            dt,
        })
    }

    /// Schedules a `ScreenSpaceFluidRenderer` pass over the active color target.
    pub fn add_screen_space_fluid(
        &mut self,
        fluid: &'a ScreenSpaceFluidRenderer,
        scene: &'a SceneRenderer,
        clear: wgpu::Color,
        draws: &'a [SceneDraw<'a>],
    ) -> &mut Self {
        self.push(RenderGraphNode::ScreenSpaceFluid {
            fluid,
            scene,
            clear,
            draws,
        })
    }

    /// Encodes all scheduled passes in order into `encoder`, targeting `color` and `depth`
    /// (`Depth32Float`, Reversed-Z clear `0.0`).
    ///
    /// Note: if an `ArchiveScene` node is present, its internal command buffers are submitted to
    /// its queue first, and subsequent graph nodes continue on `encoder` over the archive's
    /// rendered color and depth targets.
    pub fn encode(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
    ) -> Result<(), SceneError> {
        for node in &mut self.nodes {
            match node {
                RenderGraphNode::Clear { color: clear_color } => {
                    let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("unified render graph clear"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: color,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(*clear_color),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: depth,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(0.0),
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        }),
                        ..Default::default()
                    });
                }
                RenderGraphNode::VoxelWorld { renderer, clear } => {
                    renderer.encode_resident_voxels_to_targets(encoder, color, depth, *clear);
                }
                RenderGraphNode::Scene {
                    scene,
                    draws,
                    clear,
                } => {
                    if let Some(clear_color) = clear {
                        scene.encode(encoder, color, depth, *clear_color, draws);
                    } else {
                        scene.encode_over(encoder, color, depth, draws);
                    }
                }
                RenderGraphNode::InstancedScene {
                    instancer,
                    draws,
                    clear,
                } => {
                    if let Some(clear_color) = clear {
                        instancer.encode(encoder, color, depth, *clear_color, draws);
                    } else {
                        instancer.encode_over(encoder, color, depth, draws);
                    }
                }
                RenderGraphNode::SceneOverlays { scene, draws } => {
                    scene.encode_overlays(encoder, color, depth, draws);
                }
                RenderGraphNode::ArchiveScene {
                    archive,
                    eye,
                    direction,
                } => {
                    archive.draw(color, *eye, *direction);
                }
                RenderGraphNode::BodycamPost { post, output_view } => {
                    post.encode(encoder, output_view);
                }
                RenderGraphNode::Ocean {
                    ocean,
                    queue,
                    camera,
                    dt,
                } => {
                    ocean.encode(queue, encoder, color, camera, *dt);
                }
                RenderGraphNode::ScreenSpaceFluid {
                    fluid,
                    scene,
                    clear,
                    draws,
                } => {
                    fluid.encode(scene, encoder, color, *clear, draws);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SceneInstanceData, SceneMesh, SceneVertex};
    use glam::Mat4;

    #[test]
    fn unified_render_graph_composes_clear_scene_instanced_and_overlays() {
        let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let scene = SceneRenderer::new_reverse_z(&device, wgpu::TextureFormat::Rgba16Float);
        let instancer = SceneInstancer::new(&scene);
        let mesh = SceneMesh::new(
            vec![
                SceneVertex {
                    position: [0.0, 0.0, 0.5],
                    uv: [0.0, 0.0],
                    color: [1.0; 4],
                },
                SceneVertex {
                    position: [0.5, 0.0, 0.5],
                    uv: [1.0, 0.0],
                    color: [1.0; 4],
                },
                SceneVertex {
                    position: [0.0, 0.5, 0.5],
                    uv: [0.0, 1.0],
                    color: [1.0; 4],
                },
            ],
            vec![0, 1, 2],
        )
        .unwrap();
        let geometry = scene.upload_mesh(&device, &mesh).unwrap();
        let texture = scene
            .upload_texture(&device, &queue, 1, 1, &[255, 255, 255, 255])
            .unwrap();
        let transform = scene.create_transform(&device, Mat4::IDENTITY).unwrap();
        let batch = instancer
            .create_batch(
                &device,
                &[
                    SceneInstanceData::new(Mat4::IDENTITY),
                    SceneInstanceData::new(Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0))),
                ],
            )
            .unwrap();
        let scene_draws = [
            SceneDraw {
                geometry: &geometry,
                texture: &texture,
                transform: &transform,
                overlay: false,
            },
            SceneDraw {
                geometry: &geometry,
                texture: &texture,
                transform: &transform,
                overlay: true,
            },
        ];
        let instanced_draws = [SceneInstancedDraw {
            geometry: &geometry,
            texture: &texture,
            instances: &batch,
            overlay: false,
        }];

        let color = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("graph color"),
                size: wgpu::Extent3d {
                    width: 64,
                    height: 64,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());
        let depth = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("graph depth"),
                size: wgpu::Extent3d {
                    width: 64,
                    height: 64,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut graph = RenderGraph::new();
        graph
            .add_clear(wgpu::Color::BLACK)
            .add_scene(&scene, &scene_draws, None)
            .add_instanced_scene(&instancer, &instanced_draws, None)
            .add_scene_overlays(&scene, &scene_draws);
        assert_eq!(graph.len(), 4);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        graph.encode(&mut encoder, &color, &depth).unwrap();
        queue.submit([encoder.finish()]);
    }
}
