//! Portable compute with explicit command submission and nonblocking readback.
use crate::compute_readback::{ComputeReadbackPool, ReadbackLease};
use crate::{ComputeMemoryBudget, ComputeStorage};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComputeError {
    Unsupported,
    InvalidBuffer,
    ReadbackBudget,
    MemoryBudget,
    WorkBudget,
    InvalidDispatch,
    DeviceMismatch,
    Validation(String),
    Mapping(String),
    Consumed,
}
impl std::fmt::Display for ComputeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "compute error: {self:?}")
    }
}
impl std::error::Error for ComputeError {}

/// WGSL compute entry point (default `cs_main`), group 0 binding 0: read/write storage buffer.
/// The shader must guard its own out-of-range invocations. Use one device owner
/// for creation, validation scopes, command submission and readback polling.
#[derive(Debug)]
pub struct ComputeProgram {
    device: wgpu::Device,
    pipeline: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
    max_workgroups: u32,
    readback_pool: ComputeReadbackPool,
    memory_budget: ComputeMemoryBudget,
    source: String,
    entry_point: std::sync::Arc<str>,
    revision: u64,
    scene_inputs: bool,
    additional_layout: Option<wgpu::BindGroupLayout>,
}

impl ComputeProgram {
    /// # Errors
    /// Rejects devices without enabled compute limits and invalid WGSL/pipeline ABI.
    pub async fn new(device: &wgpu::Device, source: &str) -> Result<Self, ComputeError> {
        Self::with_entry_point(device, source, "cs_main").await
    }

    /// Compile an explicitly named WGSL compute entry point with the storage ABI.
    /// Reload preserves this selection; jobs retain their original pipeline.
    /// # Errors
    /// Preserves compute capacity, shader and entry-point validation errors.
    pub async fn with_entry_point(
        device: &wgpu::Device,
        source: &str,
        entry_point: &str,
    ) -> Result<Self, ComputeError> {
        Self::with_layout(device, source, entry_point, false, None).await
    }
    /// Compute storage plus read-only 2D scene color/depth texture inputs.
    pub async fn with_scene_textures(
        device: &wgpu::Device,
        source: &str,
    ) -> Result<Self, ComputeError> {
        Self::with_layout(device, source, "cs_main", true, None).await
    }
    pub(crate) async fn with_scene_binding_layout(
        device: &wgpu::Device,
        source: &str,
        additional: &wgpu::BindGroupLayout,
    ) -> Result<Self, ComputeError> {
        Self::with_layout(device, source, "cs_main", true, Some(additional)).await
    }
    async fn with_layout(
        device: &wgpu::Device,
        source: &str,
        entry_point: &str,
        scene_inputs: bool,
        additional_layout: Option<&wgpu::BindGroupLayout>,
    ) -> Result<Self, ComputeError> {
        let limits = device.limits();
        if limits.max_compute_workgroups_per_dimension == 0
            || limits.max_storage_buffers_per_shader_stage == 0
        {
            return Err(ComputeError::Unsupported);
        }
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("user compute shader"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        if let Some(error) = scope.pop().await {
            return Err(ComputeError::Validation(error.to_string()));
        }
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut entries = vec![wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }];
        if scene_inputs {
            for (binding, sample_type) in [
                (1, wgpu::TextureSampleType::Float { filterable: false }),
                (2, wgpu::TextureSampleType::Depth),
            ] {
                entries.push(wgpu::BindGroupLayoutEntry {
                    binding,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                });
            }
        }
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compute storage ABI"),
            entries: &entries,
        });
        let mut layouts = vec![Some(&layout)];
        if let Some(additional) = additional_layout {
            layouts.push(Some(additional));
        }
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("compute pipeline layout"),
            bind_group_layouts: &layouts,
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("user compute pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some(entry_point),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        if let Some(error) = scope.pop().await {
            return Err(ComputeError::Validation(error.to_string()));
        }
        Ok(Self {
            device: device.clone(),
            pipeline,
            layout,
            max_workgroups: limits.max_compute_workgroups_per_dimension,
            readback_pool: ComputeReadbackPool::for_device(device),
            memory_budget: ComputeMemoryBudget::for_device(device),
            source: source.to_owned(),
            entry_point: entry_point.into(),
            revision: 0,
            scene_inputs,
            additional_layout: additional_layout.cloned(),
        })
    }

    /// Replace the compute shader only after complete shader/ABI validation.
    /// Existing jobs retain the old pipeline; newly created jobs use the replacement.
    /// Identical source is a cache hit. Call from the device owner task.
    /// # Errors
    /// Preserves compilation errors without changing source, pipeline or revision.
    pub async fn reload_shader(&mut self, source: &str) -> Result<bool, ComputeError> {
        let entry_point = self.entry_point.clone();
        self.reload_with_entry_point(source, &entry_point).await
    }

    /// Atomically replace both WGSL source and selected compute entry point.
    /// Existing jobs retain their original kernel and revision. A cache hit requires
    /// both source and entry point to match. Call from the device owner task.
    /// # Errors
    /// Invalid replacements preserve the previous kernel, source and revision.
    pub async fn reload_with_entry_point(
        &mut self,
        source: &str,
        entry_point: &str,
    ) -> Result<bool, ComputeError> {
        if self.source == source && self.entry_point.as_ref() == entry_point {
            return Ok(false);
        }
        let mut candidate = Self::with_layout(
            &self.device,
            source,
            entry_point,
            self.scene_inputs,
            self.additional_layout.as_ref(),
        )
        .await?;
        candidate.revision = self.revision.saturating_add(1);
        *self = candidate;
        Ok(true)
    }
    #[must_use]
    pub const fn shader_revision(&self) -> u64 {
        self.revision
    }

    pub(crate) fn matches_shader(&self, source: &str, entry_point: &str) -> bool {
        self.source == source
            && self.entry_point.as_ref() == entry_point
            && !self.scene_inputs
            && self.additional_layout.is_none()
    }

    /// Creates a job with independent storage and readback. Supply shader-compatible
    /// data; byte count must be a nonzero multiple of four and fit device limits.
    /// # Errors
    /// Rejects unequal device handles, invalid storage or exhausted shared memory.
    /// Allocations always use the retained program owner: wgpu device equality
    /// alone does not distinguish equal resource IDs in separate instances.
    /// Submission and polling must also use the program owner.
    pub fn create_job(
        &self,
        device: &wgpu::Device,
        data: &[u8],
    ) -> Result<ComputeJob, ComputeError> {
        if self.scene_inputs {
            return Err(ComputeError::InvalidBuffer);
        }
        self.create_job_bound(device, data, None)
    }
    /// Scene views are retained by the job bind group; validation catches incompatible resources.
    pub async fn create_scene_job(
        &self,
        device: &wgpu::Device,
        data: &[u8],
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
    ) -> Result<ComputeJob, ComputeError> {
        if !self.scene_inputs {
            return Err(ComputeError::InvalidBuffer);
        }
        let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let result = self.create_job_bound(device, data, Some((color, depth)));
        if let Some(error) = scope.pop().await {
            return Err(ComputeError::Validation(error.to_string()));
        }
        result
    }
    fn create_job_bound(
        &self,
        device: &wgpu::Device,
        data: &[u8],
        scene: Option<(&wgpu::TextureView, &wgpu::TextureView)>,
    ) -> Result<ComputeJob, ComputeError> {
        if device != &self.device {
            return Err(ComputeError::DeviceMismatch);
        }
        let device = &self.device;
        let size = u64::try_from(data.len()).map_err(|_| ComputeError::InvalidBuffer)?;
        let limits = device.limits();
        if size == 0
            || !size.is_multiple_of(4)
            || size > limits.max_buffer_size
            || size > limits.max_storage_buffer_binding_size
        {
            return Err(ComputeError::InvalidBuffer);
        }
        let storage = self
            .memory_budget
            .allocate_storage("compute job storage", data)?;
        let mut entries = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: storage.as_entire_binding(),
        }];
        if let Some((color, depth)) = scene {
            entries.push(wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(color),
            });
            entries.push(wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(depth),
            });
        }
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compute job"),
            layout: &self.layout,
            entries: &entries,
        });
        Ok(ComputeJob {
            device: self.device.clone(),
            readback_pool: self.readback_pool.clone(),
            pipeline: self.pipeline.clone(),
            bind_group,
            storage,
            size,
            max_workgroups: self.max_workgroups,
            entry_point: self.entry_point.clone(),
            revision: self.revision,
            requires_additional_binding: self.additional_layout.is_some(),
        })
    }
}

/// One-shot job. Ownership prevents re-dispatch while its readback is mapped.
#[derive(Debug)]
pub struct ComputeJob {
    device: wgpu::Device,
    readback_pool: ComputeReadbackPool,
    pipeline: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
    storage: ComputeStorage,
    size: u64,
    max_workgroups: u32,
    entry_point: std::sync::Arc<str>,
    revision: u64,
    requires_additional_binding: bool,
}
impl ComputeJob {
    /// Switch a resident storage job to another plain-storage pipeline without
    /// copying or reallocating its buffer. Rejection leaves the job unchanged.
    /// The caller owns shader storage-ABI compatibility and command ordering.
    pub fn use_program(&mut self,program:&ComputeProgram)->Result<(),ComputeError> {
        if self.device!=program.device {return Err(ComputeError::DeviceMismatch);}
        if program.scene_inputs || program.additional_layout.is_some() {return Err(ComputeError::InvalidBuffer);}
        let bind_group=self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:Some("resident compute pipeline switch"),layout:&program.layout,
            entries:&[wgpu::BindGroupEntry {binding:0,resource:self.storage.as_entire_binding()}],
        });
        self.pipeline=program.pipeline.clone();self.bind_group=bind_group;
        self.max_workgroups=program.max_workgroups;self.entry_point=program.entry_point.clone();
        self.revision=program.revision;self.requires_additional_binding=false;
        Ok(())
    }
    /// Selected entry point retained when this job was created.
    #[must_use]
    pub fn entry_point(&self) -> &str {
        &self.entry_point
    }
    /// Program-local revision retained when this job was created.
    /// Different program owners can have equal revision numbers; this is not a
    /// global shader identity. Retain this value alongside application frame IDs.
    #[must_use]
    pub const fn shader_revision(&self) -> u64 {
        self.revision
    }

    /// Storage can also be bound by later graphics/compute passes on this device.
    #[must_use]
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.storage
    }

    /// Enqueues another in-place step on the resident storage without host
    /// transfer. Command order supplies dependencies between successive steps.
    /// The job remains usable until consumed by `encode` for final readback.
    /// # Errors
    /// Rejects zero or excessive workgroup counts before recording commands.
    pub fn encode_step(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        workgroups: [u32; 3],
    ) -> Result<(), ComputeError> {
        self.encode_step_with_binding(encoder, workgroups, None)
    }
    /// Profile one resident dispatch using pass-boundary GPU timestamps.
    pub fn encode_step_with_timestamps(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        workgroups: [u32; 3],
        timestamps: wgpu::ComputePassTimestampWrites<'_>,
    ) -> Result<(), ComputeError> {
        self.encode_step_options(encoder, workgroups, None, Some(timestamps))
    }
    pub(crate) fn encode_step_with_binding(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        workgroups: [u32; 3],
        additional: Option<&wgpu::BindGroup>,
    ) -> Result<(), ComputeError> {
        self.encode_step_options(encoder, workgroups, additional, None)
    }
    fn encode_step_options(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        workgroups: [u32; 3],
        additional: Option<&wgpu::BindGroup>,
        timestamps: Option<wgpu::ComputePassTimestampWrites<'_>>,
    ) -> Result<(), ComputeError> {
        if additional.is_some() != self.requires_additional_binding {
            return Err(ComputeError::InvalidBuffer);
        }
        if workgroups
            .into_iter()
            .any(|n| n == 0 || n > self.max_workgroups)
        {
            return Err(ComputeError::InvalidDispatch);
        }
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                timestamp_writes: timestamps,
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            if let Some(group) = additional {
                pass.set_bind_group(1, group, &[]);
            }
            pass.dispatch_workgroups(workgroups[0], workgroups[1], workgroups[2]);
        }
        Ok(())
    }

    /// Encodes compute and a storage-to-readback copy in command order.
    /// Submit the encoder before calling `ComputeDispatch::begin_read`.
    /// # Errors
    /// Rejects invalid workgroup counts or exhausted readback capacity before
    /// recording commands.
    pub fn encode(
        self,
        encoder: &mut wgpu::CommandEncoder,
        workgroups: [u32; 3],
    ) -> Result<ComputeDispatch, ComputeError> {
        if workgroups
            .into_iter()
            .any(|n| n == 0 || n > self.max_workgroups)
        {
            return Err(ComputeError::InvalidDispatch);
        }
        let readback = self.readback_pool.acquire(self.size)?;
        self.encode_step(encoder, workgroups)?;
        encoder.copy_buffer_to_buffer(&self.storage, 0, readback.buffer(), 0, self.size);
        Ok(ComputeDispatch { readback })
    }
    /// Copy the current resident result without dispatching another compute step.
    /// Leases staging only when a CPU result is actually requested.
    /// # Errors
    /// Rejects exhausted device-wide readback capacity before encoding a copy.
    /// Submit this encoder before calling `ComputeDispatch::begin_read`.
    pub fn encode_readback(
        self,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<ComputeDispatch, ComputeError> {
        self.encode_snapshot(encoder)
    }

    /// Copy resident storage at this command position without consuming the job.
    /// Later steps may continue while CPU mapping reads this independent snapshot.
    /// Snapshots lease independent staging from the bounded device pool.
    /// Submit before `begin_read`.
    /// # Errors
    /// Rejects exhausted readback capacity before recording commands.
    pub fn encode_snapshot(
        &self,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<ComputeDispatch, ComputeError> {
        let readback = self.readback_pool.acquire(self.size)?;
        encoder.copy_buffer_to_buffer(&self.storage, 0, readback.buffer(), 0, self.size);
        Ok(ComputeDispatch { readback })
    }
}

#[derive(Debug)]
pub struct ComputeDispatch {
    readback: Arc<ReadbackLease>,
}
impl ComputeDispatch {
    /// Gather ordered source ranges into one complete staging snapshot.
    /// Destination ranges must be contiguous, aligned and cover `size` exactly.
    /// Validates every range before allocating or recording commands.
    pub fn gather_buffer(device:&wgpu::Device,encoder:&mut wgpu::CommandEncoder,source:&wgpu::Buffer,ranges:&[(u64,u64,u64)],size:u64)->Result<Self,ComputeError> {
        if !source.usage().contains(wgpu::BufferUsages::COPY_SRC) || size>device.limits().max_buffer_size {return Err(ComputeError::InvalidBuffer);}
        validate_gather_ranges(source.size(),ranges,size)?;
        let readback=ComputeReadbackPool::for_device(device).acquire(size)?;
        for &(offset,target,length) in ranges {encoder.copy_buffer_to_buffer(source,offset,readback.buffer(),target,length);}
        Ok(Self {readback})
    }
    /// Records a bounded copy from caller-owned storage for one-shot readback.
    /// The source must belong to this device and permit `COPY_SRC`. Submit the
    /// encoder before `begin_read`; source ownership is managed by the caller.
    /// # Errors
    /// Rejects unaligned, empty, out-of-bounds or non-copyable ranges and
    /// exhausted device-wide staging capacity.
    pub fn copy_buffer(
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        source: &wgpu::Buffer,
        offset: u64,
        size: u64,
    ) -> Result<Self, ComputeError> {
        if size == 0
            || !size.is_multiple_of(4)
            || !offset.is_multiple_of(4)
            || offset
                .checked_add(size)
                .is_none_or(|end| end > source.size())
            || !source.usage().contains(wgpu::BufferUsages::COPY_SRC)
            || size > device.limits().max_buffer_size
        {
            return Err(ComputeError::InvalidBuffer);
        }
        let readback = ComputeReadbackPool::for_device(device).acquire(size)?;
        encoder.copy_buffer_to_buffer(source, offset, readback.buffer(), 0, size);
        Ok(Self { readback })
    }

    /// Start mapping after command submission. Native shells must drive device
    /// polling; browser shells must yield to their event loop. No blocking waits.
    #[must_use]
    pub fn begin_read(self) -> PendingComputeReadback {
        let (sender, receiver) = std::sync::mpsc::channel();
        self.readback.mapping_started();
        let callback_lease = self.readback.clone();
        self.readback
            .buffer()
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                callback_lease.mapping_finished(result.is_ok());
                let _ = sender.send(result.map_err(|error| error.to_string()));
            });
        PendingComputeReadback {
            buffer: Some(self.readback),
            receiver,
            consumed: false,
        }
    }
}

#[derive(Debug)]
pub struct PendingComputeReadback {
    buffer: Option<Arc<ReadbackLease>>,
    receiver: Receiver<Result<(), String>>,
    consumed: bool,
}
impl PendingComputeReadback {
    /// # Errors
    /// Returns mapping errors or `Consumed` when the result was already taken.
    pub fn try_read(&mut self) -> Result<Option<Vec<u8>>, ComputeError> {
        if self.consumed {
            return Err(ComputeError::Consumed);
        }
        let result = match self.receiver.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return Ok(None),
            Err(TryRecvError::Disconnected) => Err("mapping callback disconnected".into()),
        };
        self.consumed = true;
        if let Err(error) = result {
            self.buffer.take();
            return Err(ComputeError::Mapping(error));
        }
        let lease = self.buffer.as_ref().ok_or(ComputeError::Consumed)?;
        let view = lease
            .buffer()
            .slice(..)
            .get_mapped_range()
            .map_err(|error| {
                lease.failed();
                ComputeError::Mapping(error.to_string())
            })?;
        let data = view.to_vec();
        drop(view);
        lease.buffer().unmap();
        lease.unmapped();
        self.buffer.take();
        Ok(Some(data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resident_pipeline_switch_preserves_allocation_and_rejects_foreign_owner() {
        let (owner,_owner_queue)=wgpu::Device::noop(&Default::default());
        let (other,_other_queue)=wgpu::Device::noop(&Default::default());
        let source="@compute @workgroup_size(1) fn cs_main() {}";
        let first=pollster::block_on(ComputeProgram::new(&owner,source)).unwrap();
        let second=pollster::block_on(ComputeProgram::with_entry_point(&owner,"@compute @workgroup_size(1) fn next() {}","next")).unwrap();
        let foreign=pollster::block_on(ComputeProgram::new(&other,source)).unwrap();
        let mut job=first.create_job(&owner,&[0;4]).unwrap();
        let stats=ComputeMemoryBudget::for_device(&owner).stats();
        let buffer=job.buffer().clone();
        assert_eq!(job.use_program(&foreign),Err(ComputeError::DeviceMismatch));
        assert_eq!(job.entry_point(),"cs_main");
        job.use_program(&second).unwrap();
        assert_eq!(job.entry_point(),"next");assert_eq!(job.buffer(),&buffer);
        assert_eq!(ComputeMemoryBudget::for_device(&owner).stats(),stats);
        let mut encoder=owner.create_command_encoder(&Default::default());
        job.encode_step(&mut encoder,[1,1,1]).unwrap();let _=encoder.finish();
    }

    #[test]
    fn jobs_reject_different_devices_in_one_instance() {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::NOOP,
            backend_options: wgpu::BackendOptions {
                noop: wgpu::NoopBackendOptions::enabled(),
                ..Default::default()
            },
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .unwrap();
        let (owner, _owner_queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
        let (other, _other_queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
        let program = pollster::block_on(ComputeProgram::new(
            &owner,
            "@compute @workgroup_size(1) fn cs_main() {}",
        ))
        .unwrap();
        assert_ne!(owner, other);
        assert!(matches!(
            program.create_job(&other, &[0; 4]),
            Err(ComputeError::DeviceMismatch)
        ));
    }

    #[test]
    fn separate_noop_contexts_are_foreign_and_owner_clones_encode_jobs() {
        let (owner, _owner_queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let (other, _other_queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let program = pollster::block_on(ComputeProgram::new(
            &owner,
            "@group(0) @binding(0) var<storage, read_write> data: array<u32>;\n\
             @compute @workgroup_size(1) fn cs_main() { data[0] = 7u; }",
        ))
        .unwrap();
        // Numeric IDs are context-local; public handles include the context.
        // A foreign instance must not be accepted merely because its ID repeats.
        assert_ne!(owner, other);
        assert!(matches!(
            program.create_job(&other, &[0; 4]),
            Err(ComputeError::DeviceMismatch)
        ));
        let job = program.create_job(&owner.clone(), &[0; 4]).unwrap();
        let mut encoder = owner.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        job.encode_step(&mut encoder, [1, 1, 1]).unwrap();
        let _ = encoder.finish();
        assert!(program.create_job(&owner.clone(), &[0; 4]).is_ok());
        assert!(matches!(
            program.create_job(&owner, &[]),
            Err(ComputeError::InvalidBuffer)
        ));
    }
}

fn validate_gather_ranges(source_size:u64,ranges:&[(u64,u64,u64)],size:u64)->Result<(),ComputeError> {
    if size==0 || !size.is_multiple_of(4) || ranges.is_empty() {return Err(ComputeError::InvalidBuffer);}
    let mut end=0;
    for &(source,target,length) in ranges {
        if target!=end || length==0 || !source.is_multiple_of(4) || !length.is_multiple_of(4) || source.checked_add(length).is_none_or(|value|value>source_size) {return Err(ComputeError::InvalidBuffer);}
        end=target.checked_add(length).filter(|value|*value<=size).ok_or(ComputeError::InvalidBuffer)?;
    }
    if end!=size {return Err(ComputeError::InvalidBuffer);}
    Ok(())
}
#[cfg(test)]
mod gather_range_tests {
    use super::*;
    #[test]
    fn gathering_rejects_gaps_overlap_alignment_overflow_and_bounds() {
        assert!(validate_gather_ranges(100,&[(0,0,16),(80,16,20)],36).is_ok());
        for (ranges,size) in [
            (vec![],4),(vec![(0,0,0)],4),(vec![(0,0,4)],0),
            (vec![(0,0,4)],8),(vec![(0,0,8)],4),
            (vec![(0,0,4),(8,8,4)],12),(vec![(0,0,8),(8,4,4)],12),
            (vec![(1,0,4)],4),(vec![(0,0,5)],8),
            (vec![(100,0,4)],4),(vec![(u64::MAX-3,0,4)],4),
        ] {assert!(validate_gather_ranges(100,&ranges,size).is_err(),"admitted {ranges:?}");}
    }
}
