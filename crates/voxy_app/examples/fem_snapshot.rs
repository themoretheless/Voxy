//! GPU readback of current FEM geometry before/after accepted cohesive fracture.
#[path = "../src/fem_surface.rs"]
mod fem_surface;
use physics::{
    cohesive::Material as Bond,
    plasticity::{Material, mesh::QuadraticBody},
};
fn coupon() -> Result<QuadraticBody, Box<dyn std::error::Error>> {
    let m = Material::new(1e6, 0.3, 1e9, 0.)?;
    let mut body = QuadraticBody::from_linear(
        vec![
            [0.; 3],
            [1., 0., 0.],
            [0., 1., 0.],
            [0., 0., 1.],
            [0.; 3],
            [1., 0., 0.],
            [0., 1., 0.],
            [0., 0., -1.],
        ],
        vec![([0, 1, 2, 3], m), ([4, 5, 6, 7], m)],
    )?;
    let edges = body.edge_midpoints();
    let midpoint = |a: usize, b: usize| {
        edges
            .iter()
            .find(|(edge, _)| *edge == [a.min(b), a.max(b)])
            .unwrap()
            .1
    };
    body.add_cohesive_interface(
        [4, 5, 6, midpoint(4, 5), midpoint(5, 6), midpoint(4, 6)],
        [0, 1, 2, midpoint(0, 1), midpoint(1, 2), midpoint(0, 2)],
        Bond::new(1e6, 2e6, 1000., 10.)?,
    )?;
    Ok(body)
}
use glam::Vec3;
use voxy_render::{GraphicsOptions, SceneCamera, SceneDraw, SceneProjection, SceneRenderer};
#[allow(clippy::too_many_lines)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/private/tmp/voxy-fem.png".into());
    let mut body = coupon()?;
    let instance = GraphicsOptions::default().create_instance();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let mut geometry = renderer.reserve_geometry(&device, 60_024, 60_024)?;
    let texture = renderer.upload_texture(&device, &queue, 1, 1, &[255; 4])?;
    let transform = renderer.create_transform(
        &device,
        SceneCamera {
            eye: Vec3::new(3.0, 1.4, 2.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            projection: SceneProjection::Perspective {
                vertical_fov: 55_f32.to_radians(),
                aspect: 4.0 / 3.0,
                near: 0.1,
                far: 100.0,
            },
        }
        .view_projection()?,
    )?;
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
        wgpu::TextureFormat::Rgba8Unorm,
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
    for frame in 0..2 {
        if frame == 1 {
            let n = body.positions().len();
            let mut upper = vec![false; n];
            for u in &mut upper[..4] {
                *u = true;
            }
            for (edge, node) in body.edge_midpoints() {
                upper[node] = edge[0] < 4 && edge[1] < 4;
            }
            let prescribed: Vec<_> = upper
                .iter()
                .map(|u| [Some(0.), Some(0.), Some(if *u { 0.12 } else { -0.12 })])
                .collect();
            if !body
                .equilibrate(&vec![[0.; 3]; n], &prescribed, 4, 1e-7)?
                .converged
            {
                return Err("FEM snapshot equilibrium failed".into());
            }
        }
        let faces = body.exposed_faces_at(body.positions())?.len();
        let fragments = body.fragment_nodes().len();
        if (frame == 0 && (faces != 6 || fragments != 1))
            || (frame == 1 && (faces != 8 || fragments != 2))
        {
            return Err("FEM fracture topology not reflected in render snapshot".into());
        }
        let mesh = fem_surface::fem_surface_scene_mesh(
            &body,
            2,
            1000,
            [0.35, 0.35, 0.],
            1.,
            |component, normal| {
                let shade = (0.35 + 0.65 * (0.6 * normal[0] + 0.8 * normal[1]).max(0.)) as f32;
                let base = if component == 0 {
                    [0.9, 0.5, 0.08]
                } else {
                    [0.08, 0.5, 0.95]
                };
                [base[0] * shade, base[1] * shade, base[2] * shade, 1.]
            },
        )?;
        geometry.update(&queue, &mesh)?;
        println!(
            "frame={frame}: exposed_faces={faces}, fragments={fragments}, triangles={}",
            mesh.indices().len() / 3
        );
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
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
        if gold < 100 || (frame == 1 && blue < 100) {
            return Err("FEM fragments missing from rendered pixels".into());
        }
        println!("frame={frame}: second_fragment_pixels={blue}, first_fragment_pixels={gold}");
        frames.push(pixels.to_vec());
        drop(pixels);
        readback.unmap();
    }
    if frames[0] == frames[1] {
        return Err("rendered FEM surface did not change".into());
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
    image::save_buffer(&path, &output, 2048, 768, image::ColorType::Rgba8)?;
    println!("FEM SNAPSHOT PASS: {path}");
    Ok(())
}
