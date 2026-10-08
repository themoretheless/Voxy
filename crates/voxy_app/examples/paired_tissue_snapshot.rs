//! GPU snapshot of current layered paired-tissue FEM surfaces, before/after dynamics.
use physics::biomechanics::{InertialBody,Material,TesticularGeometry};
use voxy_app::fem_surface;
use glam::Vec3;
use voxy_render::{GraphicsOptions, SceneCamera, SceneDraw, SceneProjection, SceneRenderer};
#[allow(clippy::too_many_lines)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/private/tmp/voxy-paired-tissue.png".into());
    let verify_reference_transform = std::env::args().any(|arg|arg == "--verify-reference-transform");
    let reference_pattern = std::env::args().any(|arg|arg == "--reference-pattern");
    let material = Material {shear_pa:1000.,bulk_pa:100_000.,fibers:vec![]};
    let shape = TesticularGeometry {centers_m:[[-0.02,0.,0.],[0.02,0.,0.]],
        radii_m:[[0.015,0.02,0.025];2],sectors:16,rings:6};
    let mut bodies = Vec::new();
    for body in shape.build_layered([0.8;2],[material.clone(),material.clone()],
        [material.clone(),material])? {
        let density = vec![1000.;body.elements().len()];
        let mut velocities = vec![[0.;3];body.positions().len()];
        let center = *body.rest_positions().last().unwrap();
        let outer_count = (body.positions().len()-1)/2;
        for (velocity,p) in velocities[..outer_count].iter_mut().zip(body.rest_positions()) {
            *velocity = std::array::from_fn(|axis|0.5*(p[axis]-center[axis]));
        }
        let mut dynamic = InertialBody::new(body,&density,velocities)?;
        dynamic.set_uniform_acceleration([0.,-9.81,0.])?;
        bodies.push(dynamic);
    }
    let instance = GraphicsOptions::default().create_instance();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let mut geometry = renderer.reserve_geometry(&device, 60_024, 60_024)?;
    let negative_coordinate_control = std::env::args().any(|arg|arg == "--negative-coordinate-control");
    let source = include_str!("../src/tissue_surface_material.wgsl");
    let source = if negative_coordinate_control {
        source.replace("out.reference=reference;", "out.reference=position;")
    } else { source.to_owned() };
    pollster::block_on(renderer.set_geometry_opaque_shader(&device,&mut geometry,&source))?;
    let texture = renderer.upload_texture(&device, &queue, 1, 1, &[255; 4])?;
    let base_projection = SceneCamera {
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
        .view_projection()?;
    let transform = renderer.create_transform(&device,base_projection)?;
    transform.update_view_position(&queue,Vec3::new(3.0,1.4,2.0))?;
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
    for frame in 0..if verify_reference_transform {3} else {2} {
        if frame == 1 {
            for body in &mut bodies {
                for _ in 0..5000 { body.step(2.5e-6,1e-8)?; }
                let current = body.body().positions();
                let rest = body.body().rest_positions();
                let center = current.len()-1;
                let distance = |a: [f64;3],b: [f64;3]| -> f64 {
                    (0..3).map(|k|(a[k]-b[k]).powi(2)).sum::<f64>().sqrt()
                };
                let deformation = current.iter().zip(rest).map(|(p,r)|
                    (distance(*p,current[center])-distance(*r,rest[center])).abs()).fold(0.,f64::max);
                if deformation <= 1e-8 { return Err("GPU specimen has only rigid motion".into()); }
                println!("nonrigid_distance_change_m={deformation:e}");
            }
        }
        let origin = if frame == 2 {[0.17/30.,0.11337/30.,0.07/30.]} else {[0.;3]};
        if frame == 2 {
            let shift = Vec3::new(0.17,0.11337,0.07);
            transform.update(&queue,base_projection*glam::Mat4::from_translation(shift))?;
            transform.update_view_position(&queue,Vec3::new(3.0,1.4,2.0)-shift)?;
        }
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut material_coordinates = Vec::new();
        for body in &bodies {
            let mesh = fem_surface::tissue_surface_scene_mesh(body.body(),origin,30.,[0.15,0.5,0.8,1.])?;
            let offset = vertices.len() as u32;
            indices.extend(mesh.indices().iter().map(|i|i+offset));
            vertices.extend_from_slice(mesh.vertices());
            material_coordinates.extend_from_slice(mesh.explicit_material_coordinates().ok_or("missing tissue reference coordinates")?);
        }
        let mesh = voxy_render::SceneMesh::new(vertices,indices)?.with_material_coordinates(material_coordinates)?
            .with_material_parameters([0.6,if reference_pattern {1.} else {0.},1000.,0.4])?;
        geometry.update(&queue, &mesh)?;
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
        let tissue_pixels = pixels.chunks_exact(4).filter(|p|p[2] > 70 && p[1] > 40).count();
        if tissue_pixels < 100 { return Err("tissue surface missing from GPU snapshot".into()); }
        println!("frame={frame} tissue_pixels={tissue_pixels}");
        let shades: std::collections::HashSet<_> = pixels.chunks_exact(4)
            .filter(|p|p[2]>70 && p[1]>40).map(|p|[p[0],p[1],p[2]]).collect();
        if shades.len() < 32 { return Err("surface normal lighting missing from GPU pixels".into()); }
        println!("frame={frame} surface_shades={}",shades.len());
        frames.push(pixels.to_vec());
        drop(pixels);
        readback.unmap();
    }
    if frames[0] == frames[1] {
        return Err("rendered tissue surface did not follow dynamics".into());
    }
    if verify_reference_transform {
        let changed = frames[1].chunks_exact(4).zip(frames[2].chunks_exact(4))
            .filter(|(a,b)| (0..3).any(|i|a[i].abs_diff(b[i])>1)).count();
        let fraction = changed as f64/(1024.*768.);
        println!("REFERENCE TRANSFORM changed_pixels={changed} fraction={fraction:e}");
        if fraction > 0.001 { return Err("material changed under compensated rigid display translation".into()); }
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
    image::save_buffer(&path, &output, 1024*frames.len() as u32, 768, image::ColorType::Rgba8)?;
    println!("TISSUE SNAPSHOT PASS: {path}");
    Ok(())
}
