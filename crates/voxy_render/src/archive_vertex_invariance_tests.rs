//! Position-contract regressions for the actual archive shader producer.
use super::archive_shader_source;

#[test]
fn scene_vertex_positions_remain_invariant() {
    let module = naga::front::wgsl::parse_str(&archive_shader_source()).unwrap();
    let vertices: Vec<_> = module.entry_points.iter()
        .filter(|entry| entry.stage == naga::ShaderStage::Vertex).collect();
    assert!(vertices.len() >= 2, "material and shadow pipelines must be covered");
    for entry in vertices {
        let result = entry.function.result.as_ref().unwrap();
        let naga::TypeInner::Struct { ref members, .. } = module.types[result.ty].inner else {
            panic!("vertex output must expose its position binding");
        };
        let position = members.iter().find(|member| matches!(member.binding,
            Some(naga::Binding::BuiltIn(naga::BuiltIn::Position { .. })))).unwrap();
        assert!(matches!(position.binding,
            Some(naga::Binding::BuiltIn(naga::BuiltIn::Position { invariant: true }))),
            "{} lost cross-pass position invariance", entry.name);
    }
}
#[cfg(target_os = "macos")]
#[test]
fn scene_vertex_position_invariance_survives_metal_translation() {
    let module = naga::front::wgsl::parse_str(&archive_shader_source()).unwrap();
    let info = naga::valid::Validator::new(naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all()).validate(&module).unwrap();
    for entry in module.entry_points.iter().filter(|entry| entry.stage == naga::ShaderStage::Vertex) {
        let (active, active_info) = naga::back::pipeline_constants::process_overrides(
            &module, &info, Some((entry.stage, &entry.name)), &Default::default()).unwrap();
        let (metal, translated) = naga::back::msl::write_string(&active, &active_info,
            &naga::back::msl::Options { lang_version: (2, 4), ..Default::default() },
            &Default::default()).unwrap();
        assert!(translated.entry_point_names.iter().all(Result::is_ok));
        assert!(metal.contains("[[position, invariant]]"),
            "{} lost the actual Metal position attribute", entry.name);
    }
}

/// Requires exclusive ownership of the test GPU. This checks actual vertex
/// coverage/depth across pipeline variants, not scene realism or frame rate.
#[cfg(target_os = "macos")]
#[test]
#[ignore = "actual Metal GPU regression; run alone with VOXY_VERTEX_INVARIANCE_CAPTURE"]
fn metal_gpu_position_invariance() -> Result<(), Box<dyn std::error::Error>> {
    use wgpu::util::DeviceExt;
    let root = std::path::PathBuf::from(std::env::var_os("VOXY_VERTEX_INVARIANCE_CAPTURE")
        .ok_or("new artifact directory is required")?);
    std::fs::create_dir(&root)?;
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL, ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))?;
    assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))?;
    let source = format!("{}\n@fragment fn capture_position_a(f:Fragment)->@location(0) vec4<f32>{{return f.clip;}}\n@fragment fn capture_position_b(f:Fragment)->@location(0) vec4<f32>{{return vec4<f32>(f.clip.xy,f.clip.zw);}}",archive_shader_source());
    std::fs::write(root.join("actual-shader.wgsl"), &source)?;
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("actual archive invariant vertex source"), source: wgpu::ShaderSource::Wgsl(source.into())
    });
    let vertex_attributes = wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x3,2=>Float32x4,3=>Float32x2];
    let instance_attributes = wgpu::vertex_attr_array![4=>Float32x4,5=>Float32x4,6=>Float32x4,7=>Float32x4,8=>Float32x4,9=>Float32x4,10=>Float32x4];
    let layouts = [Some(wgpu::VertexBufferLayout {array_stride:48,step_mode:wgpu::VertexStepMode::Vertex,attributes:&vertex_attributes}),Some(wgpu::VertexBufferLayout {array_stride:112,step_mode:wgpu::VertexStepMode::Instance,attributes:&instance_attributes})];
    let mut pipelines = Vec::new();
    for (vertex, fragment) in [("vs_main","capture_position_a"),("vs_main","capture_position_b"),("vs_shadow","capture_position_a")] {
        pipelines.push(device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label:Some(vertex),layout:None,
            vertex:wgpu::VertexState {module:&module,entry_point:Some(vertex),compilation_options:Default::default(),buffers:&layouts},
            primitive:Default::default(),depth_stencil:None,multisample:Default::default(),
            fragment:Some(wgpu::FragmentState {module:&module,entry_point:Some(fragment),compilation_options:Default::default(),targets:&[Some(wgpu::ColorTargetState {format:wgpu::TextureFormat::Rgba32Float,blend:None,write_mask:wgpu::ColorWrites::ALL})]}),multiview_mask:None,cache:None,
        }));
    }
    let mut vertices = Vec::<f32>::new();
    for y in 0..16 {for x in 0..16 {
        let cx=x as f32/8.-0.94;let cy=y as f32/8.-0.94;
        let radius=0.025+((x+y)%3) as f32*0.004;
        for (dx,dy) in [(-radius,-radius),(radius,-radius),(0.,radius)] {
            vertices.extend_from_slice(&[cx+dx,cy+dy,0.,0.,0.,1.,1.,0.,0.,1.,0.,0.]);
        }
    }}
    let input=device.create_buffer_init(&wgpu::util::BufferInitDescriptor {label:Some("original vertex words"),contents:bytemuck::cast_slice(&vertices),usage:wgpu::BufferUsages::VERTEX});
    std::fs::write(root.join("original-vertices.bin"),bytemuck::cast_slice(&vertices))?;
    let mut compared_words=0usize;let mut covered_pixels=Vec::new();
    for (case,translation) in [0f32,8192.,262144.].into_iter().enumerate() {
        let mut instance_words=[0f32;28];
        let world=glam::Mat4::from_cols(glam::Vec4::new(0.875,0.125,0.,0.),glam::Vec4::new(-0.125,0.875,0.,0.),glam::Vec4::Z,glam::Vec4::new(translation,translation*0.5,-translation,1.));
        instance_words[..16].copy_from_slice(&world.to_cols_array());instance_words[16]=1.;instance_words[21]=1.;instance_words[26]=1.;
        let copies=device.create_buffer_init(&wgpu::util::BufferInitDescriptor {label:Some("original instance words"),contents:bytemuck::cast_slice(&instance_words),usage:wgpu::BufferUsages::VERTEX});
        let vp=glam::Mat4::from_cols(glam::Vec4::X,glam::Vec4::Y,glam::Vec4::new(0.,0.,0.25,0.),glam::Vec4::new(-translation,-translation*0.5,translation*0.25+0.5,1.));
        let mut camera_words=[0f32;112];camera_words[..16].copy_from_slice(&vp.to_cols_array());camera_words[16..32].copy_from_slice(&vp.to_cols_array());
        let camera=device.create_buffer_init(&wgpu::util::BufferInitDescriptor {label:Some("identical original main/shadow matrices"),contents:bytemuck::cast_slice(&camera_words),usage:wgpu::BufferUsages::UNIFORM});
        std::fs::write(root.join(format!("case-{case}-camera.bin")),bytemuck::cast_slice(&camera_words))?;
        std::fs::write(root.join(format!("case-{case}-instances.bin")),bytemuck::cast_slice(&instance_words))?;
        let mut reference:Option<Vec<u8>>=None;
        for (index,pipeline) in pipelines.iter().enumerate() {
            let target=device.create_texture(&wgpu::TextureDescriptor {label:Some("actual rasterized position words"),size:wgpu::Extent3d {width:256,height:256,depth_or_array_layers:1},mip_level_count:1,sample_count:1,dimension:wgpu::TextureDimension::D2,format:wgpu::TextureFormat::Rgba32Float,usage:wgpu::TextureUsages::RENDER_ATTACHMENT|wgpu::TextureUsages::COPY_SRC,view_formats:&[]});
            let view=target.create_view(&Default::default());
            let bind=device.create_bind_group(&wgpu::BindGroupDescriptor {label:None,layout:&pipeline.get_bind_group_layout(0),entries:&[wgpu::BindGroupEntry {binding:0,resource:camera.as_entire_binding()}]});
            let mut encoder=device.create_command_encoder(&Default::default());
            {let mut pass=encoder.begin_render_pass(&wgpu::RenderPassDescriptor {label:Some("actual production vertex coverage"),color_attachments:&[Some(wgpu::RenderPassColorAttachment {view:&view,depth_slice:None,resolve_target:None,ops:wgpu::Operations {load:wgpu::LoadOp::Clear(wgpu::Color {r:-1.,g:-1.,b:-1.,a:-1.}),store:wgpu::StoreOp::Store}})],..Default::default()});pass.set_pipeline(pipeline);pass.set_bind_group(0,&bind,&[]);pass.set_vertex_buffer(0,input.slice(..));pass.set_vertex_buffer(1,copies.slice(..));pass.draw(0..(vertices.len()/12) as u32,0..1);}
            queue.submit([encoder.finish()]);
            let folder=root.join(format!("case-{case}-pipeline-{index}"));
            crate::archive_hdr_diagnostics::capture(&device,&queue,vec![("position",view)],None,&folder,true)?;
            let actual=std::fs::read(folder.join("position.native.bin"))?;
            assert_eq!(actual.len(),256*256*16);
            let covered=actual.chunks_exact(16).filter(|pixel|f32::from_le_bytes(pixel[12..16].try_into().unwrap())>0.).count();assert!(covered>512,"test must actually rasterize geometry");
            assert!(actual.chunks_exact(4).all(|word|f32::from_le_bytes(word.try_into().unwrap()).is_finite()));
            if let Some(expected)=&reference {assert!(actual.as_slice()==expected.as_slice(),"coverage or fragment position differs in case {case} pipeline {index}");compared_words+=actual.len()/4;}else{covered_pixels.push(covered);reference=Some(actual);}
        }
    }
    let proof=serde_json::json!({"status":"actual_Metal_production_vertex_coverage_and_position_words_match_across_pipeline_variants","matrix_cases":3,"pipelines_per_case":3,"original_triangles_per_case":256,"actual_position_words_compared":compared_words,"covered_pixels_per_case":covered_pixels,"CPU_expected_position_results_uploaded":false,"original_shader_position_expressions_replaced":false,"whole_scene_HDR_bug_fixed":false,"minimum_120_fps_qualified":false});
    std::fs::write(root.join("proof.json"),serde_json::to_vec_pretty(&proof)?)?;
    println!("{proof}");Ok(())
}
