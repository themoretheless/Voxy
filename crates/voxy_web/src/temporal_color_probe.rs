//! Read production resolve inputs/output; evaluate the blend independently on CPU.
use super::browser::error;
use wasm_bindgen::JsValue;
use wgpu::util::DeviceExt;
#[derive(Debug)]
pub(crate) struct ColorProbe {
    dispatch: Option<voxy_render::ComputeDispatch>,
    pending: Option<voxy_render::PendingComputeReadback>,
    reset: bool,
    count: usize,
}
pub(crate) fn pipeline(device: &wgpu::Device) -> wgpu::ComputePipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("temporal color input capture"),
        source: wgpu::ShaderSource::Wgsl(include_str!("temporal_color_probe.wgsl").into()),
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("temporal color input capture"),
        layout: None,
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    })
}
impl ColorProbe {
    pub(crate) fn encode(
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        pipeline: &wgpu::ComputePipeline,
        textures: [&wgpu::Texture; 6],
        pixels: &[[u32; 2]],
        reset: bool,
    ) -> Result<Self, JsValue> {
        let bytes: Vec<u8> = pixels
            .iter()
            .flat_map(|pixel| pixel.iter().flat_map(|v| v.to_le_bytes()))
            .collect();
        let positions = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("temporal color sample positions"),
            contents: &bytes,
            usage: wgpu::BufferUsages::STORAGE,
        });
        let size = (pixels.len() * 208) as u64;
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("temporal color samples"),
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let views: Vec<_> = textures
            .iter()
            .map(|texture| texture.create_view(&Default::default()))
            .collect();
        let mut entries: Vec<_> = views
            .iter()
            .enumerate()
            .map(|(index, view)| wgpu::BindGroupEntry {
                binding: index as u32,
                resource: wgpu::BindingResource::TextureView(view),
            })
            .collect();
        entries.push(wgpu::BindGroupEntry {
            binding: 6,
            resource: positions.as_entire_binding(),
        });
        entries.push(wgpu::BindGroupEntry {
            binding: 7,
            resource: output.as_entire_binding(),
        });
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("temporal color capture inputs"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bindings, &[]);
            pass.dispatch_workgroups((pixels.len() as u32).div_ceil(64), 1, 1);
        }
        let dispatch = voxy_render::ComputeDispatch::copy_buffer(device, encoder, &output, 0, size)
            .map_err(error)?;
        Ok(Self {
            dispatch: Some(dispatch),
            pending: None,
            reset,
            count: pixels.len(),
        })
    }
    pub(crate) fn begin_read(&mut self) {
        if let Some(dispatch) = self.dispatch.take() {
            self.pending = Some(dispatch.begin_read());
        }
    }
    pub(crate) fn poll(&mut self) -> Result<Option<(u32, u32, u32)>, JsValue> {
        let Some(pending) = &mut self.pending else {
            return Ok(None);
        };
        let Some(bytes) = pending.try_read().map_err(error)? else {
            return Ok(None);
        };
        let mut rejected = 0;
        let mut blended = 0;
        for pixel in 0..self.count {
            let value = |slot: usize, channel: usize| {
                let offset = pixel * 208 + slot * 16 + channel * 4;
                f32::from_le_bytes(
                    bytes[offset..offset + 4]
                        .try_into()
                        .expect("captured float"),
                )
            };
            let current = std::array::from_fn::<_, 3, _>(|c| {
                let v = value(0, c);
                if v.is_finite() { v.max(0.0) } else { 0.0 }
            });
            let old = std::array::from_fn::<_, 3, _>(|c| value(1, c));
            let expected_depth = value(3, 2);
            let history_depth = value(3, 3);
            let depth_match = expected_depth.is_finite()
                && history_depth.is_finite()
                && expected_depth > 0.0
                && expected_depth < 1.0
                && history_depth > 0.0
                && history_depth < 1.0
                && (expected_depth - history_depth).abs() <= 0.001;
            if !self.reset && expected_depth > 0.0 && history_depth > 0.0 && !depth_match {
                rejected += 1;
            }
            let mut reference = current;
            if !self.reset && depth_match && old.iter().all(|v| v.is_finite() && *v >= 0.0) {
                let mut low = current;
                let mut high = current;
                for neighbor in 0..9 {
                    let sample = std::array::from_fn::<_, 3, _>(|c| value(4 + neighbor, c));
                    if value(4 + neighbor, 3) > 0.5 && sample.iter().all(|v| v.is_finite()) {
                        for c in 0..3 {
                            low[c] = low[c].min(sample[c].max(0.0));
                            high[c] = high[c].max(sample[c].max(0.0));
                        }
                    }
                }
                for c in 0..3 {
                    let clipped = old[c].clamp(low[c], high[c]);
                    reference[c] = clipped + (current[c] - clipped) * (1.0 - 0.85_f32);
                }
            }
            if reference
                .iter()
                .zip(current)
                .any(|(expected, current)| (*expected - current).abs() > 0.0001)
            {
                blended += 1;
            }
            for c in 0..3 {
                let actual = value(2, c);
                if !actual.is_finite()
                    || (actual - reference[c]).abs() > 0.00003 * reference[c].abs().max(1.0)
                {
                    return Err(error(format!(
                        "production temporal color pixel {pixel} channel {c}: {actual} != {}",
                        reference[c]
                    )));
                }
            }
        }
        self.pending = None;
        Ok(Some(((self.count * 3) as u32, rejected, blended)))
    }
}
