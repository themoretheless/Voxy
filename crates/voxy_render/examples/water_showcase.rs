//! voxy_water showcase on the archive scene renderer: a beach with breaking surf, a
//! river that runs off a plateau as a waterfall into the sea, and crates floating on the
//! waves (CPU buoyancy against the same surface the GPU draws, with ripple wakes).
//!
//! Headless: `cargo run -p voxy_render --example water_showcase -- --shot out.png
//! [--view beach|falls|crates|x,y,z,dx,dy,dz] [--size 960x540] [--frames 8]`.
use glam::{Mat4, Quat, Vec2, Vec3};
use voxy_render::archive_scene::*;
use voxy_render::voxy_water::{
    Bathymetry, Floater, River, RiverNode, ShoreField, WaterBody, WaterWorld, WaveSpectrum, buoyancy,
};

const SIZE: u32 = 220;
const CELL: f32 = 1.0;
const ORIGIN: Vec2 = Vec2::new(-110.0, -110.0);

/// River centre line: from the plateau (x > 40) over the cliff and across the beach.
fn river_nodes() -> Vec<RiverNode> {
    let node = |x: f32, y: f32, z: f32, width: f32| RiverNode { position: Vec3::new(x, y, z), width, depth: 0.8 };
    vec![
        node(95., 8.6, 30., 5.0),
        node(75., 8.4, 22., 5.5),
        node(58., 8.25, 12., 6.0),
        node(45.6, 8.15, 7.8, 6.0),
        node(41.2, 2.25, 6.6, 7.0),
        node(30., 2.0, 4., 8.0),
        node(10., 1.2, 0., 9.0),
        node(-10., 0.6, -2., 10.0),
        node(-30., 0.2, -3., 12.0),
    ]
}

/// Sea to the west, a beach rising east, a plateau with a cliff, and the river channel.
fn terrain(p: Vec2, river: &[Vec3]) -> f32 {
    let beach = ((p.x + 60.0) * 0.07).clamp(-6.0, 1.6) + (p.y * 0.05).sin() * 0.3;
    let cliff = smooth((p.x - 42.0) / 3.0) * 6.2;
    let dunes = ((p.x * 0.11).sin() * (p.y * 0.13).cos()).max(0.0) * 1.2 * smooth((p.x + 40.0) / 20.0);
    let mut h = beach + cliff + dunes;
    // carve the channel: a trough under the water surface along the river
    let mut best = f32::MAX;
    let mut level = 0.0;
    for pair in river.windows(2) {
        let (a, b) = (Vec2::new(pair[0].x, pair[0].z), Vec2::new(pair[1].x, pair[1].z));
        let ab = b - a;
        let t = ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
        let d = p.distance(a + ab * t);
        if d < best {
            best = d;
            level = pair[0].y + (pair[1].y - pair[0].y) * t;
        }
    }
    let channel = level - 0.9 * (1.0 - (best / 6.5).powi(2)).max(0.0);
    if best < 6.5 && p.x > -35.0 {
        h = h.min(channel.max(h - 3.0));
    }
    h
}

fn smooth(x: f32) -> f32 {
    let t = x.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn material(colour: [f32; 3], rough: f32, pattern: f32) -> ArchiveMaterialSource {
    ArchiveMaterialSource {
        parameters: ArchiveMaterial {
            diffuse: [colour[0], colour[1], colour[2], 1.],
            specular_gloss: [0.04, 0.04, 0.04, 1.],
            emissive_normal: [0., 0., 0., 1.],
            surface: [0., rough, 0., -1.],
            workflow: [1., 0., pattern, 0.],
        },
        images: [None; 4],
        double_sided: false,
    }
}

fn mesh(material: usize, vertices: Vec<ArchiveVertex>, indices: Vec<u32>, instances: Vec<ArchiveInstance>) -> ArchiveMeshSource {
    ArchiveMeshSource { vertices, indices, instances, material, center: Vec3::ZERO }
}

fn cube() -> (Vec<ArchiveVertex>, Vec<u32>) {
    let (mut vertices, mut indices) = (Vec::new(), Vec::new());
    for normal in [Vec3::X, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y, Vec3::Z, Vec3::NEG_Z] {
        let tangent = if normal.y.abs() > 0.5 { Vec3::X } else { Vec3::Y };
        let bitangent = normal.cross(tangent);
        let o = vertices.len() as u32;
        for (u, v) in [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)] {
            vertices.push(ArchiveVertex {
                position: (normal * 0.5 + tangent * u + bitangent * v).to_array(),
                normal: normal.to_array(),
                tangent: [tangent.x, tangent.y, tangent.z, 1.],
                uv: [u + 0.5, v + 0.5],
            });
        }
        indices.extend([o, o + 1, o + 2, o, o + 2, o + 3]);
    }
    (vertices, indices)
}

struct Args {
    shot: String,
    view: (Vec3, Vec3),
    size: [u32; 2],
    frames: u32,
}

fn parse() -> Result<Args, Box<dyn std::error::Error>> {
    let mut args = Args { shot: "water_showcase.png".into(), view: view("beach")?, size: [960, 540], frames: 8 };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let value = it.next().ok_or_else(|| format!("{flag} expects a value"))?;
        match flag.as_str() {
            "--shot" => args.shot = value,
            "--view" => args.view = view(&value)?,
            "--frames" => args.frames = value.parse()?,
            "--size" => {
                let (w, h) = value.split_once('x').ok_or("--size expects WxH")?;
                args.size = [w.parse()?, h.parse()?];
            }
            other => return Err(format!("unknown argument {other}").into()),
        }
    }
    Ok(args)
}

fn view(name: &str) -> Result<(Vec3, Vec3), Box<dyn std::error::Error>> {
    Ok(match name {
        "beach" => (Vec3::new(-48., 5.5, -30.), Vec3::new(-0.75, -0.22, 0.55)),
        "falls" => (Vec3::new(33., 4.2, -3.), Vec3::new(0.72, 0.1, 0.62)),
        "crates" => (Vec3::new(-62., 3.2, -12.), Vec3::new(-0.6, -0.3, 0.55)),
        other => {
            let v: Vec<f32> = other.split(',').map(str::parse).collect::<Result<_, _>>()?;
            (Vec3::new(v[0], v[1], v[2]), Vec3::new(v[3], v[4], v[5]))
        }
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse()?;
    let nodes = river_nodes();
    let centre: Vec<Vec3> = River::new(nodes.clone(), 1.2).samples(2.0).iter().map(|s| s.position).collect();
    let bathymetry = Bathymetry::from_fn(ORIGIN, CELL, [SIZE, SIZE], |p| terrain(p, &centre));
    let shore = ShoreField::new(&bathymetry, 0.0);
    let sea_outline = vec![Vec2::new(-110., -110.), Vec2::new(-20., -110.), Vec2::new(-20., 110.), Vec2::new(-110., 110.)];
    let world = WaterWorld::new(
        vec![
            WaterBody::sea(sea_outline, 0.0, -8.0, shore),
            // open sea beyond the surveyed coast, out to the horizon
            WaterBody { material: voxy_render::voxy_water::WaterMaterial::SEA, ..WaterBody::lake(vec![Vec2::new(-3000., -3000.), Vec2::new(-110., -3000.), Vec2::new(-110., 3000.), Vec2::new(-3000., 3000.)], 0.0, -40.0, None) },
            WaterBody::river(River::new(nodes, 1.2)),
        ],
        WaveSpectrum { swell_height: 0.6, wind_speed: 5.0, fetch_m: 20_000.0, ..WaveSpectrum::ocean() },
    );

    // ---- floating crates: integrate buoyancy against the CPU water surface ----
    let floater = Floater::cuboid(Vec3::new(1.2, 0.6, 0.8), 3);
    let mass = 0.45 * 1000.0 * floater.volume();
    let inertia = mass * 0.15;
    let time = 6.0;
    let mut crates: Vec<(Vec3, Quat, Vec3, Vec3)> = [(-70., -8.), (-74., -3.), (-66., -15.), (-78., -11.)]
        .iter()
        .map(|&(x, z)| (Vec3::new(x, 1.0, z), Quat::from_rotation_y(x * 0.3), Vec3::ZERO, Vec3::ZERO))
        .collect();
    let dt = 1.0 / 120.0;
    for step in 0..(time / dt) as usize {
        let t = step as f32 * dt;
        for (p, q, v, w) in &mut crates {
            let f = buoyancy(&world, &floater, *p, *q, *v, *w, t);
            *v += (f.force / mass - Vec3::Y * 9.81) * dt;
            *w += f.torque / inertia * dt;
            *w *= 0.995;
            *p += *v * dt;
            *q = (Quat::from_scaled_axis(*w * dt) * *q).normalize();
        }
    }

    // ---- scene geometry ----
    let mut vertices = Vec::new();
    for y in 0..SIZE {
        for x in 0..SIZE {
            let p = ORIGIN + Vec2::new(x as f32, y as f32) * CELL;
            let h = |dx: f32, dz: f32| bathymetry.height(p + Vec2::new(dx, dz));
            let normal = Vec3::new(h(-1., 0.) - h(1., 0.), 2.0 * CELL, h(0., -1.) - h(0., 1.)).normalize();
            vertices.push(ArchiveVertex {
                position: [p.x, h(0., 0.), p.y],
                normal: normal.to_array(),
                tangent: [1., 0., 0., 1.],
                uv: [x as f32 * 0.25, y as f32 * 0.25],
            });
        }
    }
    let (mut sand, mut grass, mut rock) = (Vec::new(), Vec::new(), Vec::new());
    for y in 0..SIZE - 1 {
        for x in 0..SIZE - 1 {
            let a = y * SIZE + x;
            let quad = [a, a + SIZE, a + SIZE + 1, a, a + SIZE + 1, a + 1];
            let mid = (vertices[a as usize].position[1] + vertices[(a + SIZE + 1) as usize].position[1]) * 0.5;
            let steep = vertices[a as usize].normal[1] < 0.75;
            let list = if steep { &mut rock } else if mid < 2.0 && vertices[a as usize].position[0] < 0.0 { &mut sand } else { &mut grass };
            list.extend_from_slice(&quad);
        }
    }
    let identity = vec![ArchiveInstance::new(Mat4::IDENTITY)];
    let materials = vec![
        material([0.76, 0.66, 0.48], 0.9, 1.),
        material([0.16, 0.25, 0.08], 0.95, 1.),
        material([0.32, 0.30, 0.28], 0.85, 1.),
        material([0.42, 0.26, 0.12], 0.7, 4.),
    ];
    let (cube_vertices, cube_indices) = cube();
    let crate_instances = crates
        .iter()
        .map(|(p, q, _, _)| ArchiveInstance::new(Mat4::from_scale_rotation_translation(Vec3::new(1.2, 0.6, 0.8), *q, *p)))
        .collect();
    let geometry = vec![
        mesh(0, vertices.clone(), sand, identity.clone()),
        mesh(1, vertices.clone(), grass, identity.clone()),
        mesh(2, vertices, rock, identity),
        mesh(3, cube_vertices, cube_indices, crate_instances),
    ];

    // ---- render ----
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
        ..Default::default()
    }))?;
    let [width, height] = args.size;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let sun = Vec3::new(-0.45, 0.42, 0.6).normalize();
    let mut renderer = ArchiveSceneRenderer::new_with_environment(
        &device,
        &queue,
        format,
        width,
        height,
        Vec::new(),
        materials,
        geometry,
        &[],
        sun,
        [1., 0.93, 0.83, 4800.],
        (Vec3::new(-110., -9., -110.), Vec3::new(110., 12., 110.)),
        None,
    )?;
    renderer.set_antialiasing(voxy_render::ArchiveAntialiasing::Off)?;
    renderer.set_water(&world);
    renderer.advance_water(time);
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("water showcase"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let (eye, direction) = (args.view.0, args.view.1.normalize());
    for _ in 0..args.frames {
        renderer.advance_water(1.0 / 30.0);
        // the crates bob in place and push small wakes into the ripple field
        for (p, ..) in &crates {
            let xz = Vec2::new(p.x, p.z);
            renderer.water_wake(xz - Vec2::X * 0.4, xz + Vec2::X * 0.4, 0.5, 0.004);
        }
        renderer.draw(&view, eye, direction);
        device.poll(wgpu::PollType::wait_indefinitely())?;
    }
    let row = (width * 4).div_ceil(256) * 256;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(row * height),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo { buffer: &readback, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: None } },
        target.size(),
    );
    queue.submit([encoder.finish()]);
    readback.map_async(wgpu::MapMode::Read, .., |r| r.expect("map showcase"));
    device.poll(wgpu::PollType::wait_indefinitely())?;
    let mapped = readback.get_mapped_range(..)?;
    let pixels: Vec<u8> = mapped.chunks_exact(row as usize).flat_map(|r| &r[..(width * 4) as usize]).copied().collect();
    image::save_buffer(&args.shot, &pixels, width, height, image::ColorType::Rgba8)?;
    let heights: Vec<String> = crates.iter().map(|(p, ..)| format!("{:.2}", p.y)).collect();
    println!("WATER SHOWCASE {}: crates afloat at y = [{}]", args.shot, heights.join(", "));
    Ok(())
}
