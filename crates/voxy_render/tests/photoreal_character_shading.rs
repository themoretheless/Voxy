//! The photoreal voxel/skinned shader with skin SSS and fibre scattering
//! must parse and validate as one module, exactly as renderer.rs builds it.
#[test]
fn photoreal_shader_with_skin_and_hair_is_valid() {
    let source = concat!(
        include_str!("../src/skin_hair.wgsl"),
        include_str!("../src/voxel_photoreal.wgsl")
    );
    let module = naga::front::wgsl::parse_str(source).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap();
}

/// Evaluates the real skin_hair.wgsl functions on the GPU.
fn evaluate(entry_body: &str, outputs: usize) -> Option<Vec<f32>> {
    use wgpu::util::DeviceExt;
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).ok()?;
    let source = format!(
        "{}\n@group(0) @binding(0) var<storage,read_write> out:array<f32>;\n@compute @workgroup_size(1) fn main() {{\n{entry_body}\n}}",
        include_str!("../src/skin_hair.wgsl")
    );
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let size = (outputs * 4) as u64;
    let storage = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: &vec![0; outputs * 4],
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry { binding: 0, resource: storage.as_entire_binding() }],
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&storage, 0, &readback, 0, size);
    queue.submit([encoder.finish()]);
    readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::PollType::wait_indefinitely()).ok()?;
    let data = readback.slice(..).get_mapped_range().ok()?;
    Some(bytemuck::cast_slice::<u8, f32>(&data).to_vec())
}

#[test]
fn skin_scatters_red_past_the_terminator_and_conserves_lit_energy() {
    let Some(v) = evaluate(
        "let past=skin_diffuse(-0.2,80.);out[0]=past.r;out[1]=past.g;out[2]=past.b;\
         let lit=skin_diffuse(1.,80.);out[3]=lit.r;out[4]=lit.g;out[5]=lit.b;\
         let flat=skin_diffuse(-0.2,0.);out[6]=flat.r;",
        7,
    ) else {
        eprintln!("no GPU adapter; skipped");
        return;
    };
    // Beyond the geometric terminator red still arrives, blue far less.
    assert!(v[0] > 0.02 && v[0] > 3. * v[2], "{v:?}");
    // Fully lit skin stays near Lambert (no energy gain above ~10%).
    assert!(v[3..6].iter().all(|&c| (0.9..=1.1).contains(&c)), "{v:?}");
    // Flatter geometry scatters less past the terminator than curved features.
    assert!(v[6] < v[0], "{v:?}");
}

#[test]
fn hair_highlight_follows_the_fibre_and_cornea_adds_a_sharp_glint() {
    let Some(v) = evaluate(
        "let t=vec3<f32>(0.,1.,0.);let c=vec3<f32>(0.3,0.2,0.1);\
         let l=normalize(vec3<f32>(0.,0.3,1.));\
         let mirror=normalize(vec3<f32>(0.,-0.3,1.));let off=normalize(vec3<f32>(0.,0.8,0.6));\
         out[0]=dot(hair_shading(t,l,mirror,c,0.35),vec3<f32>(1.));\
         out[1]=dot(hair_shading(t,l,off,c,0.35),vec3<f32>(1.));\
         let n=vec3<f32>(0.,0.,1.);let base=vec3<f32>(0.1);\
         out[2]=cornea_coat(n,n,n,base).r;\
         out[3]=cornea_coat(n,n,normalize(vec3<f32>(0.6,0.,0.8)),base).r;",
        4,
    ) else {
        eprintln!("no GPU adapter; skipped");
        return;
    };
    assert!(v.iter().all(|x| x.is_finite() && *x >= 0.), "{v:?}");
    // The R lobe peaks at the longitudinal mirror direction of the fibre.
    assert!(v[0] > 1.5 * v[1], "{v:?}");
    // Facing the light, the wet cornea adds a strong specular glint; off-axis it is gone.
    assert!(v[2] > 10. * v[3] && v[3] < 0.2, "{v:?}");
}
