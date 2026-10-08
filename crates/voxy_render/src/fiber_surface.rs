//! Full-density fibre geometry from solved local frames; physics remains caller-owned.
use crate::SceneError;
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FiberSurfaceFrame {
    pub position_arc: [f32; 4],
    pub u_red: [f32; 4],
    pub v_green: [f32; 4],
    pub w_blue: [f32; 4],
}
#[derive(Debug)]
pub struct FiberSurfaceInput {
    words: Vec<u32>,
    output_word: usize,
    vertex_count: u32,
}
impl FiberSurfaceInput {
    pub fn new(frames: &[FiberSurfaceFrame], followers: u32, offsets: &[[f32; 4]], radius: f32, taper: f32) -> Result<Self, SceneError> {
        let count = frames.len().checked_mul(followers as usize).and_then(|n| n.checked_mul(4))
            .filter(|n| *n > 0 && *n <= u32::MAX as usize).ok_or(SceneError::GeometryCapacityExceeded)?;
        if followers == 0 || offsets.len() != count / 4 || !radius.is_finite() || radius <= 0.
            || !taper.is_finite() || !(0.0..1.0).contains(&taper)
            || bytemuck::cast_slice::<_, f32>(frames).iter().any(|x| !x.is_finite())
            || offsets.iter().flatten().any(|x| !x.is_finite()) {
            return Err(SceneError::InvalidGeometry);
        }
        for f in frames {
            if !(0.0..=1.0).contains(&f.position_arc[3]) { return Err(SceneError::InvalidGeometry); }
            let u = glam::Vec3::from_slice(&f.u_red);
            let v = glam::Vec3::from_slice(&f.v_green);
            let w = glam::Vec3::from_slice(&f.w_blue);
            if [u,v,w].iter().any(|n| (n.length_squared()-1.).abs() > 1e-4)
                || u.dot(v).abs() > 1e-4 || u.cross(v).distance(w) > 1e-4 {
                return Err(SceneError::InvalidGeometry);
            }
        }
        let offsets_word = 8usize.checked_add(frames.len().checked_mul(16).ok_or(SceneError::GeometryCapacityExceeded)?)
            .ok_or(SceneError::GeometryCapacityExceeded)?;
        let output_word = offsets_word.checked_add(offsets.len().checked_mul(4).ok_or(SceneError::GeometryCapacityExceeded)?)
            .ok_or(SceneError::GeometryCapacityExceeded)?;
        let total = output_word.checked_add(count.checked_mul(12).ok_or(SceneError::GeometryCapacityExceeded)?)
            .filter(|n| *n <= u32::MAX as usize).ok_or(SceneError::GeometryCapacityExceeded)?;
        let mut words = Vec::new();
        words.try_reserve_exact(total).map_err(|_| SceneError::MemoryBudget)?;
        words.extend_from_slice(&[count as u32, followers, 8, offsets_word as u32, output_word as u32, radius.to_bits(), taper.to_bits(), 0]);
        words.extend_from_slice(bytemuck::cast_slice(frames));
        words.extend(offsets.iter().flatten().map(|x| x.to_bits()));
        words.resize(total, 0);
        Ok(Self { words, output_word, vertex_count: count as u32 })
    }
    pub fn bytes(&self) -> &[u8] { bytemuck::cast_slice(&self.words) }
    pub fn vertex_count(&self) -> u32 { self.vertex_count }
    /// Atomically replaces only dynamic frames; immutable offsets and resident output remain intact.
    pub fn replace_frames(&mut self, frames: &[FiberSurfaceFrame]) -> Result<&[u8], SceneError> {
        let end=self.words[3] as usize;
        if frames.len() != (end-8)/16 { return Err(SceneError::InvalidGeometry); }
        // Reuse the same admission rules before changing any resident input words.
        for f in frames {
            let values: &[f32]=bytemuck::cast_slice(std::slice::from_ref(f));
            let u=glam::Vec3::from_slice(&f.u_red);let v=glam::Vec3::from_slice(&f.v_green);let w=glam::Vec3::from_slice(&f.w_blue);
            if values.iter().any(|x| !x.is_finite()) || !(0.0..=1.0).contains(&f.position_arc[3])
                || [u,v,w].iter().any(|n| (n.length_squared()-1.).abs()>1e-4)
                || u.dot(v).abs()>1e-4 || u.cross(v).distance(w)>1e-4 {
                return Err(SceneError::InvalidGeometry);
            }
        }
        self.words[8..end].copy_from_slice(bytemuck::cast_slice(frames));
        Ok(bytemuck::cast_slice(&self.words[8..end]))
    }

    pub fn output<'a>(&self, bytes: &'a [u8]) -> Result<&'a [u8], SceneError> {
        if bytes.len() != self.words.len()*4 { return Err(SceneError::InvalidGeometry); }
        Ok(&bytes[self.output_word*4..])
    }
}
pub const FIBER_SURFACE_SHADER: &str = include_str!("fiber_surface.wgsl");
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frame_admission_rejects_invalid_basis_and_density() {
        let f = FiberSurfaceFrame { position_arc: [0.;4], u_red:[1.,0.,0.,0.1], v_green:[0.,1.,0.,0.2], w_blue:[0.,0.,1.,0.3] };
        let input = FiberSurfaceInput::new(&[f], 1, &[[0.,0.,0.,1.]], 0.0003, 0.6).unwrap();
        assert_eq!(input.vertex_count(), 4);
        assert_eq!(input.output(input.bytes()).unwrap().len(), 4*12*4);
        assert!(FiberSurfaceInput::new(&[f],0,&[],0.0003,0.6).is_err());
        assert!(FiberSurfaceInput::new(&[f],1,&[[f32::NAN;4]],0.0003,0.6).is_err());
        let mut resident=input;
        let immutable=resident.bytes().to_vec();
        let mut invalid=f; invalid.w_blue[2]=-1.;
        assert!(resident.replace_frames(&[invalid]).is_err());
        assert_eq!(resident.bytes(),immutable);
        let mut moved=f;moved.position_arc[1]=0.1;
        assert_eq!(resident.replace_frames(&[moved]).unwrap().len(),64);
        assert_eq!(&resident.bytes()[96..],&immutable[96..]);
        assert!(FiberSurfaceInput::new(&[invalid],1,&[[0.,0.,0.,1.]],0.0003,0.6).is_err());
    }
}

/// Ordered scatter into the exact separate vertex/normal streams used by drawing.
#[derive(Debug)]
pub struct FiberSurfaceTransfer {
    pipeline: wgpu::ComputePipeline,
    group: wgpu::BindGroup,
    _parameters: crate::ComputeStorage,
    groups: u32,
}
impl FiberSurfaceTransfer {
    pub async fn new(device: &wgpu::Device, geometry: &crate::SceneGeometry, source: &wgpu::Buffer,
        input: &FiberSurfaceInput, vertex_base: u32) -> Result<Self, crate::ComputeError> {
        use crate::{ComputeError, compute_memory::ManagedBufferDescriptor};
        let end = vertex_base.checked_add(input.vertex_count).ok_or(ComputeError::InvalidBuffer)?;
        let groups = input.vertex_count.div_ceil(64);
        if !geometry.belongs_to(device) { return Err(ComputeError::DeviceMismatch); }
        if end as usize > geometry.deformation_vertex_capacity()
            || source.size() < input.bytes().len() as u64
            || !source.usage().contains(wgpu::BufferUsages::STORAGE)
            || !geometry.deformation_vertices().usage().contains(wgpu::BufferUsages::STORAGE)
            || !geometry.deformation_normals().usage().contains(wgpu::BufferUsages::STORAGE)
            || groups > device.limits().max_compute_workgroups_per_dimension {
            return Err(ComputeError::InvalidBuffer);
        }
        let parameters=[input.vertex_count, input.output_word as u32, vertex_base, 0];
        let mut buffers=crate::ComputeMemoryBudget::for_device(device).allocate_buffer_batch(&[
            ManagedBufferDescriptor { label:"fibre scatter parameters",size:16,
                contents:Some(bytemuck::cast_slice(&parameters)),usage:wgpu::BufferUsages::UNIFORM }
        ]).map_err(|_| ComputeError::MemoryBudget)?;
        let parameters=buffers.pop().ok_or(ComputeError::MemoryBudget)?;
        let scope=device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader=device.create_shader_module(wgpu::ShaderModuleDescriptor { label:Some("fibre draw transfer"),
            source:wgpu::ShaderSource::Wgsl(include_str!("fiber_surface_transfer.wgsl").into()) });
        let pipeline=device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label:Some("fibre draw transfer"),layout:None,module:&shader,entry_point:Some("cs_main"),
            compilation_options:Default::default(),cache:None,
        });
        let group=device.create_bind_group(&wgpu::BindGroupDescriptor { label:Some("fibre draw streams"),
            layout:&pipeline.get_bind_group_layout(0),entries:&[
                wgpu::BindGroupEntry { binding:0,resource:source.as_entire_binding() },
                wgpu::BindGroupEntry { binding:1,resource:geometry.deformation_vertices().as_entire_binding() },
                wgpu::BindGroupEntry { binding:2,resource:geometry.deformation_normals().as_entire_binding() },
                wgpu::BindGroupEntry { binding:3,resource:parameters.as_entire_binding() },
            ] });
        if let Some(error)=scope.pop().await { return Err(ComputeError::Validation(error.to_string())); }
        Ok(Self { pipeline,group,_parameters:parameters,groups })
    }
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut pass=encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&self.pipeline);pass.set_bind_group(0,&self.group,&[]);
        pass.dispatch_workgroups(self.groups,1,1);
    }
}
