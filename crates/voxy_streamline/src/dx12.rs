//! Standalone native recording for Streamline; queue/barrier integration is external.
#![allow(unsafe_code)]
use crate::{
    CameraConstants, Dx12TextureLease, FrameGenerationState, StreamlineError, StreamlineFeature,
    StreamlineFrame, TextureRole,
};
use std::{ffi::c_void, marker::PhantomData, ptr::NonNull, rc::Rc};
use windows::Win32::Graphics::Direct3D12::{
    D3D12_RESOURCE_BARRIER, D3D12_RESOURCE_BARRIER_0, D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
    D3D12_RESOURCE_BARRIER_FLAG_NONE, D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
    D3D12_RESOURCE_BARRIER_TYPE_UAV, D3D12_RESOURCE_STATES, D3D12_RESOURCE_TRANSITION_BARRIER,
    D3D12_RESOURCE_UAV_BARRIER,
};
/// Known resource states shared by wgpu's tracker and native DLSS commands.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextureAccess {
    ShaderRead,
    StorageReadWrite,
}
impl TextureAccess {
    #[must_use]
    pub fn native_state(self) -> D3D12_RESOURCE_STATES {
        use windows::Win32::Graphics::Direct3D12::{
            D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE,
            D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE, D3D12_RESOURCE_STATE_UNORDERED_ACCESS,
        };
        match self {
            Self::ShaderRead => {
                D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE
                    | D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE
            }
            Self::StorageReadWrite => D3D12_RESOURCE_STATE_UNORDERED_ACCESS,
        }
    }
}
/// Encode a full-texture handoff state and update wgpu's resource tracking.
/// Submit this encoder before native commands on the same serialized queue.
/// Native commands must restore this state before subsequent wgpu submissions.
/// This records a barrier; it does not submit, initialize contents or wait.
/// # Errors
/// Rejects textures without the usage required by the requested state.
pub fn prepare_texture(
    encoder: &mut wgpu::CommandEncoder,
    texture: &Dx12TextureLease,
    access: TextureAccess,
) -> Result<(), StreamlineError> {
    let (usage, state) = match access {
        TextureAccess::ShaderRead => (
            wgpu::TextureUsages::TEXTURE_BINDING,
            wgpu::TextureUses::RESOURCE,
        ),
        TextureAccess::StorageReadWrite => (
            wgpu::TextureUsages::STORAGE_BINDING,
            wgpu::TextureUses::STORAGE_READ_WRITE,
        ),
    };
    if !texture.texture.usage().contains(usage) {
        return Err(StreamlineError::InvalidOptions);
    }
    encoder.transition_resources(
        std::iter::empty(),
        std::iter::once(wgpu::TextureTransition {
            texture: &texture.texture,
            selector: None,
            state,
        }),
    );
    Ok(())
}
use windows::{
    Win32::Graphics::Direct3D12::{
        D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_FENCE_FLAG_NONE, ID3D12CommandAllocator,
        ID3D12CommandList, ID3D12CommandQueue, ID3D12Device, ID3D12Fence,
        ID3D12GraphicsCommandList, ID3D12PipelineState,
    },
    core::Interface,
};

/// Native handles derived together from a wgpu device and its direct queue.
#[derive(Debug)]
pub struct WgpuQueue {
    device: ID3D12Device,
    queue: ID3D12CommandQueue,
    _device_owner: wgpu::Device,
    thread_bound: PhantomData<Rc<()>>,
}
impl WgpuQueue {
    /// # Safety
    /// Uphold wgpu-hal lifetime requirements. Do not explicitly destroy the device
    /// during native work. Serialize queue access with wgpu submissions and keep
    /// resource states consistent with wgpu's trackers.
    /// # Errors
    /// Rejects a device that does not use DX12.
    pub unsafe fn from_wgpu(device: &wgpu::Device) -> Result<Self, StreamlineError> {
        // SAFETY: Caller guarantees HAL lifetime; guard protects handles while cloned.
        let hal = unsafe { device.as_hal::<wgpu::hal::api::Dx12>() }
            .ok_or(StreamlineError::WrongBackend)?;
        Ok(Self {
            device: hal.raw_device().clone(),
            queue: hal.raw_queue().clone(),
            _device_owner: device.clone(),
            thread_bound: PhantomData,
        })
    }
    /// Enqueue waiting for SDK consumption of previous frame inputs.
    /// # Safety
    /// Uphold `wait_frame_generation_inputs` lifetime and ordering requirements.
    /// Serialize with wgpu submissions; retain inputs until GPU work completes.
    /// # Errors
    /// Preserves validation, device-removal and native wait errors.
    pub unsafe fn wait_frame_generation_inputs(
        &self,
        state: &FrameGenerationState,
    ) -> Result<(), StreamlineError> {
        // SAFETY: Caller provides current SDK fence and producer ordering.
        unsafe { wait_frame_generation_inputs(&self.queue, state) }
    }
    /// Create commands on this queue's device.
    /// # Errors
    /// Preserves native allocator/list creation errors.
    pub fn recorder(&self) -> Result<Recorder, StreamlineError> {
        Recorder::new(&self.device)
    }
    /// Submit native commands to the exact direct queue used by this wgpu device.
    /// # Safety
    /// Commands/resources must belong to this device. Serialize native submission
    /// with wgpu work. Resource states must be reconciled before and after DLSS.
    /// Supply all leases and retain SDK/presentation dependencies separately.
    /// # Errors
    /// Preserves native fence creation/signal errors.
    pub unsafe fn submit(
        &self,
        commands: RecordedCommands,
        resources: Vec<Dx12TextureLease>,
    ) -> Result<Submission, StreamlineError> {
        // SAFETY: Native device/queue pair is derived from one HAL device;
        // caller guarantees resource states, ownership and queue serialization.
        unsafe { commands.submit(&self.device, &self.queue, resources) }
    }
}

/// Enqueue GPU-side waiting before subsequent work overwrites previous FG inputs.
/// This does not block the CPU or make immediate CPU destruction safe.
/// # Safety
/// Snapshot must come from the current live SDK session on the present thread.
/// Its non-null pointer must be a live `ID3D12Fence` for this queue's device.
/// Preserve SDK, fence and tagged resources until queued work completes; serialize
/// queue operations. Ensure the SDK producer can signal independently of this wait
/// (never insert a wait before the work that signals it on the same queue).
/// # Errors
/// Rejects invalid fence/value pairs, device removal, and native queue wait failures.
pub unsafe fn wait_frame_generation_inputs(
    queue: &ID3D12CommandQueue,
    state: &FrameGenerationState,
) -> Result<(), StreamlineError> {
    if state.inputs_completion_value == u64::MAX {
        return Err(StreamlineError::InvalidOptions);
    }
    if state.inputs_completion_fence.is_null() {
        return if state.inputs_completion_value == 0 {
            Ok(())
        } else {
            Err(StreamlineError::InvalidOptions)
        };
    }
    // SAFETY: Caller guarantees a live SDK-owned fence; borrow without adopting
    // or releasing its reference. SDK lifetime extends through queued waiting.
    let fence = unsafe { ID3D12Fence::from_raw_borrowed(&state.inputs_completion_fence) }
        .ok_or(StreamlineError::InvalidOptions)?;
    // SAFETY: Read-only query on the borrowed live native fence.
    let completed = unsafe { fence.GetCompletedValue() };
    if completed == u64::MAX {
        return Err(StreamlineError::Dxgi {
            hresult: 0x887A_0005_u32.cast_signed(),
        });
    }
    if completed >= state.inputs_completion_value {
        return Ok(());
    }
    // SAFETY: Caller guarantees same-device queue, producer ordering, and lifetime.
    unsafe { queue.Wait(fence, state.inputs_completion_value) }.map_err(native_error)
}

/// Owned depth, motion vectors and HUD-less color for presentation-driven FG.
#[derive(Debug)]
pub struct FrameGenerationResources {
    depth: Dx12TextureLease,
    motion: Dx12TextureLease,
    color: Dx12TextureLease,
}
impl FrameGenerationResources {
    /// Exact retained HUD-less input, available for read-only composition.
    /// Do not destroy it or overwrite it until all SDK/GPU consumers complete.
    #[must_use]
    pub fn hudless_color(&self) -> &wgpu::Texture {
        &self.color.texture
    }

    /// Import initialized renderer textures; color can use output resolution while
    /// depth and motion vectors share the render resolution.
    /// # Safety
    /// All textures must share the registered SDK device and stay valid through
    /// SDK/GPU use. Do not explicitly destroy them; match SDK format conventions.
    /// # Errors
    /// Rejects missing sampling usage, mismatched depth/motion sizes or aliasing.
    pub unsafe fn import(
        depth: &wgpu::Texture,
        motion: &wgpu::Texture,
        color: &wgpu::Texture,
    ) -> Result<Self, StreamlineError> {
        if depth.size() != motion.size()
            || [depth, motion, color].iter().any(|texture| {
                !texture
                    .usage()
                    .contains(wgpu::TextureUsages::TEXTURE_BINDING)
            })
        {
            return Err(StreamlineError::InvalidOptions);
        }
        // SAFETY: Caller guarantees initialized same-device textures and HAL lifetime.
        let resources = unsafe {
            Self {
                depth: Dx12TextureLease::from_wgpu(depth)?,
                motion: Dx12TextureLease::from_wgpu(motion)?,
                color: Dx12TextureLease::from_wgpu(color)?,
            }
        };
        let leases = [&resources.depth, &resources.motion, &resources.color];
        for (index, lease) in leases.iter().enumerate() {
            if leases[..index]
                .iter()
                .any(|other| other.resource.as_raw() == lease.resource.as_raw())
            {
                return Err(StreamlineError::InvalidOptions);
            }
        }
        Ok(resources)
    }
    /// Encode shader-read handoff. Submit before SDK tagging/presentation on the
    /// same queue, after renderer writes and any previous FG input completion wait.
    /// # Errors
    /// Rejects missing sampling usage.
    pub fn prepare(&self, encoder: &mut wgpu::CommandEncoder) -> Result<(), StreamlineError> {
        for lease in [&self.depth, &self.motion, &self.color] {
            prepare_texture(encoder, lease, TextureAccess::ShaderRead)?;
        }
        Ok(())
    }
    /// Tag FG inputs for this frame; FG evaluation occurs during presentation.
    /// # Safety
    /// Submit preparation before SDK consumption; camera/motion conventions and
    /// viewport sizes must match configuration. Preserve SDK and device through
    /// presentation and FG input processing. Call on the present thread.
    /// # Errors
    /// Preserves SDK constants/tag errors. On tagging failure resources are retained
    /// for process lifetime because partial SDK consumption cannot be excluded.
    pub unsafe fn tag(
        self,
        frame: &mut StreamlineFrame<'_>,
        viewport: u32,
        constants: &CameraConstants,
    ) -> Result<TaggedFrameGenerationInputs, StreamlineError> {
        let state = TextureAccess::ShaderRead.native_state().0.cast_unsigned();
        let tags = [
            self.depth.tag(TextureRole::Depth, state, 1)?,
            self.motion.tag(TextureRole::MotionVectors, state, 1)?,
            self.color.tag(TextureRole::HudLessColor, state, 1)?,
        ];
        frame.set_camera_constants(viewport, constants)?;
        // SAFETY: Caller guarantees states and SDK lifetime; until-present tags
        // remain valid through the returned ownership guard without a command list.
        if let Err(error) = unsafe { frame.tag_dx12(viewport, &tags, None) } {
            std::mem::forget(self);
            return Err(error);
        }
        Ok(TaggedFrameGenerationInputs {
            resources: Some(self),
        })
    }
}
/// Retains tagged inputs beyond present and until SDK input processing finishes.
/// Dropping without explicit reclamation conservatively retains them for process
/// lifetime; normal application flow must reclaim completed guards for reuse.
#[derive(Debug)]
pub struct TaggedFrameGenerationInputs {
    resources: Option<FrameGenerationResources>,
}
impl TaggedFrameGenerationInputs {
    /// Attach the SDK fence snapshot obtained after this input set was presented.
    /// # Safety
    /// Snapshot must identify this exact presentation's final SDK input consumer.
    /// Its pointer must be a live same-device `ID3D12Fence`. All other GPU users
    /// must already be complete; no future use of these tags may be scheduled.
    /// Keep runtime/device live until completion; serialize on the present thread.
    /// # Errors
    /// Rejects missing fences, zero/reserved values or missing resources. Failure
    /// conservatively retains the tagged resources through this guard's drop.
    pub unsafe fn track_completion(
        self,
        state: &FrameGenerationState,
    ) -> Result<TrackedFrameGenerationInputs, StreamlineError> {
        if self.resources.is_none()
            || state.inputs_completion_fence.is_null()
            || state.inputs_completion_value == 0
            || state.inputs_completion_value == u64::MAX
        {
            return Err(StreamlineError::InvalidOptions);
        }
        // SAFETY: Caller guarantees current SDK-owned fence. Clone adds an owned
        // COM reference; the original borrowed SDK reference is never released.
        let fence = unsafe { ID3D12Fence::from_raw_borrowed(&state.inputs_completion_fence) }
            .ok_or(StreamlineError::InvalidOptions)?
            .clone();
        Ok(TrackedFrameGenerationInputs {
            inputs: self,
            fence,
            value: state.inputs_completion_value,
            thread_bound: PhantomData,
        })
    }
    /// Recover resources after all SDK and GPU consumers finish.
    /// # Safety
    /// Presentation must have finished and the SDK completion fence must have
    /// completed on the CPU (or equivalent verified idle synchronization). An
    /// enqueued queue wait alone is insufficient. No future work may reference
    /// these tags. Runtime/device must remain live during consumption.
    /// # Errors
    /// Rejects a missing internal resource owner.
    pub unsafe fn reclaim(mut self) -> Result<FrameGenerationResources, StreamlineError> {
        self.resources.take().ok_or(StreamlineError::InvalidOptions)
    }
}
impl Drop for TaggedFrameGenerationInputs {
    fn drop(&mut self) {
        if let Some(resources) = self.resources.take() {
            std::mem::forget(resources);
        }
    }
}

/// CPU-verifiable completion guard for one presented FG input set.
/// Pending/error drops retain inputs; completed drops release them normally.
#[derive(Debug)]
pub struct TrackedFrameGenerationInputs {
    inputs: TaggedFrameGenerationInputs,
    fence: ID3D12Fence,
    value: u64,
    thread_bound: PhantomData<Rc<()>>,
}
impl TrackedFrameGenerationInputs {
    /// Query actual CPU-observed fence completion without blocking.
    /// # Errors
    /// Reports device removal instead of treating its sentinel as completion.
    pub fn is_complete(&self) -> Result<bool, StreamlineError> {
        // SAFETY: Owned COM reference; unsafe constructor guarantees SDK lifetime.
        let completed = unsafe { self.fence.GetCompletedValue() };
        if completed == u64::MAX {
            return Err(StreamlineError::Dxgi {
                hresult: 0x887A_0005_u32.cast_signed(),
            });
        }
        Ok(completed >= self.value)
    }
    /// Recover inputs only after the CPU observes completion. Pending inputs stay
    /// in this guard; a successful call transfers them once.
    /// # Errors
    /// Reports device removal. Returns `None` while pending or already reclaimed.
    pub fn try_reclaim(&mut self) -> Result<Option<FrameGenerationResources>, StreamlineError> {
        if self.is_complete()? {
            Ok(self.inputs.resources.take())
        } else {
            Ok(None)
        }
    }
}
impl Drop for TrackedFrameGenerationInputs {
    fn drop(&mut self) {
        if self.is_complete() == Ok(true) {
            // Completion contract covers all users, so regular resource release
            // is safe. Empty the conservative inner guard before its drop runs.
            drop(self.inputs.resources.take());
        }
    }
}

/// Owns all four imported SR resources through preparation, evaluation and submission.
#[derive(Debug)]
pub struct SuperResolutionResources {
    color: Dx12TextureLease,
    depth: Dx12TextureLease,
    motion: Dx12TextureLease,
    output: Dx12TextureLease,
}
impl SuperResolutionResources {
    /// Borrow the exact leases retained for SR recording and GPU submission.
    #[must_use]
    pub fn textures(&self) -> SuperResolutionTextures<'_> {
        SuperResolutionTextures {
            color: &self.color,
            depth: &self.depth,
            motion: &self.motion,
            output: &self.output,
        }
    }
    /// # Safety
    /// Textures must share the registered device. Do not explicitly destroy them
    /// during SDK/GPU use; obey native/wgpu synchronization and state requirements.
    /// # Errors
    /// Rejects invalid HAL textures, missing usages, aliasing and misaligned input sizes.
    pub unsafe fn import(
        color: &wgpu::Texture,
        depth: &wgpu::Texture,
        motion: &wgpu::Texture,
        output: &wgpu::Texture,
    ) -> Result<Self, StreamlineError> {
        if color.size() != depth.size()
            || color.size() != motion.size()
            || [color, depth, motion].iter().any(|texture| {
                !texture
                    .usage()
                    .contains(wgpu::TextureUsages::TEXTURE_BINDING)
            })
            || !output.usage().contains(
                wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            )
        {
            return Err(StreamlineError::InvalidOptions);
        }
        // SAFETY: Caller guarantees ownership and state/synchronization requirements for all textures.
        let resources = unsafe {
            Self {
                color: Dx12TextureLease::from_wgpu(color)?,
                depth: Dx12TextureLease::from_wgpu(depth)?,
                motion: Dx12TextureLease::from_wgpu(motion)?,
                output: Dx12TextureLease::from_wgpu(output)?,
            }
        };
        resources.textures().validate()?;
        Ok(resources)
    }
    /// Encode wgpu state handoff. Submit this encoder before native evaluation work.
    /// # Errors
    /// Rejects missing usage flags; all flags were checked during import.
    pub fn prepare(&self, encoder: &mut wgpu::CommandEncoder) -> Result<(), StreamlineError> {
        for texture in [&self.color, &self.depth, &self.motion] {
            prepare_texture(encoder, texture, TextureAccess::ShaderRead)?;
        }
        prepare_texture(encoder, &self.output, TextureAccess::StorageReadWrite)
    }
    /// # Safety
    /// Prepared handoff must precede this list on the same serialized queue.
    /// Match camera/viewport/configured sizes, and preserve SDK state contracts.
    /// # Errors
    /// Preserves camera/tagging/evaluation failures.
    pub unsafe fn evaluate(
        &self,
        recorder: &mut Recorder,
        frame: &mut StreamlineFrame<'_>,
        viewport: u32,
        constants: &CameraConstants,
    ) -> Result<(), StreamlineError> {
        // SAFETY: Caller upholds frame/device/resource state requirements.
        unsafe { recorder.evaluate_super_resolution(frame, viewport, constants, &self.textures()) }
    }
    /// Transfer every SR lease to native fence ownership during submission.
    /// # Safety
    /// Commands must be from this queue/device and use correct prepared states.
    /// Later SDK/presentation uses require independent lifetime protection.
    /// # Errors
    /// Preserves native queue/fence errors.
    pub unsafe fn submit(
        self,
        queue: &WgpuQueue,
        commands: RecordedCommands,
    ) -> Result<Submission, StreamlineError> {
        // SAFETY: Caller guarantees queue/device compatibility and serialized state handoff.
        unsafe {
            queue.submit(
                commands,
                vec![self.color, self.depth, self.motion, self.output],
            )
        }
    }
}

/// Full-resolution motion/depth and input color plus separately sized SR output.
#[derive(Debug)]
pub struct SuperResolutionTextures<'a> {
    pub color: &'a Dx12TextureLease,
    pub depth: &'a Dx12TextureLease,
    pub motion: &'a Dx12TextureLease,
    pub output: &'a Dx12TextureLease,
}
impl SuperResolutionTextures<'_> {
    /// Validate render dimensions and reject aliased native resources before SDK use.
    /// # Errors
    /// Rejects mismatched input sizes or any repeated native texture identity.
    pub fn validate(&self) -> Result<(), StreamlineError> {
        if self.color.texture.size() != self.depth.texture.size()
            || self.color.texture.size() != self.motion.texture.size()
        {
            return Err(StreamlineError::InvalidOptions);
        }
        let resources = [self.color, self.depth, self.motion, self.output];
        for (index, resource) in resources.iter().enumerate() {
            if resources[..index]
                .iter()
                .any(|previous| previous.resource.as_raw() == resource.resource.as_raw())
            {
                return Err(StreamlineError::InvalidOptions);
            }
        }
        Ok(())
    }
    /// Restore the states established by SR preparation, then order output writes.
    /// `exit_states` follows color, depth, motion, output order.
    /// # Safety
    /// Each exit state must describe every subresource's actual state after SDK
    /// evaluation. All resources must belong to the recorder's device; preserve
    /// resource/SDK lifetimes and serialized queue access through GPU completion.
    pub unsafe fn restore_prepared_states(
        &self,
        recorder: &mut Recorder,
        exit_states: [D3D12_RESOURCE_STATES; 4],
    ) {
        let resources = [self.color, self.depth, self.motion, self.output];
        for (index, (resource, before)) in resources.into_iter().zip(exit_states).enumerate() {
            let after = if index == 3 {
                TextureAccess::StorageReadWrite.native_state()
            } else {
                TextureAccess::ShaderRead.native_state()
            };
            // SAFETY: Caller supplies actual full-resource exit states and ownership.
            unsafe { recorder.transition_texture(resource, before, after) };
        }
        // SAFETY: Output has just been restored to its prepared UAV state.
        unsafe { recorder.uav_barrier(self.output) };
    }
}

/// Owns one recording command list. Allocators are never reset or reused.
#[derive(Debug)]
pub struct Recorder {
    list: ID3D12GraphicsCommandList,
    allocator: ID3D12CommandAllocator,
    thread_bound: PhantomData<Rc<()>>,
}
impl Recorder {
    /// Set camera constants, tag four inputs and record SR/DLAA for one frame.
    /// # Safety
    /// All resources must use this device and the frame's configured viewport.
    /// Inputs must be initialized, in shader-read state and match camera/motion
    /// conventions. Output must be in UAV state and match configured output size.
    /// Caller must serialize queues, restore wgpu tracker states after evaluation
    /// and retain every lease/SDK dependency through GPU completion even on error.
    /// # Errors
    /// Rejects misaligned input sizes and preserves tagging/constants/SDK errors.
    pub unsafe fn evaluate_super_resolution(
        &mut self,
        frame: &mut StreamlineFrame<'_>,
        viewport: u32,
        constants: &CameraConstants,
        textures: &SuperResolutionTextures<'_>,
    ) -> Result<(), StreamlineError> {
        textures.validate()?;
        let input_state = TextureAccess::ShaderRead.native_state().0.cast_unsigned();
        let output_state = TextureAccess::StorageReadWrite
            .native_state()
            .0
            .cast_unsigned();
        // Until-evaluate lifecycle: SDK may encode copies onto this recording list.
        // Native queue submission still retains source leases through GPU completion.
        let tags = [
            textures
                .color
                .tag(TextureRole::ScalingInputColor, input_state, 2)?,
            textures.depth.tag(TextureRole::Depth, input_state, 2)?,
            textures
                .motion
                .tag(TextureRole::MotionVectors, input_state, 2)?,
            textures
                .output
                .tag(TextureRole::ScalingOutputColor, output_state, 2)?,
        ];
        frame.set_camera_constants(viewport, constants)?;
        // SAFETY: This recorder owns a live list; caller supplies resource/state validity.
        let pointer = unsafe { self.recording_pointer()? };
        // SAFETY: Caller upholds tag resource lifetimes and queue/state synchronization.
        unsafe { frame.tag_dx12(viewport, &tags, Some(pointer)) }?;
        // SAFETY: Same frame token and viewport; resource requirements guaranteed by caller.
        unsafe { frame.evaluate_dx12(viewport, StreamlineFeature::SuperResolution, pointer) }
    }
    /// Order preceding and following UAV accesses without changing texture state.
    /// # Safety
    /// Texture/list must share a device, and the texture must support UAV access.
    /// Caller must establish the actual UAV state and retain resources through GPU
    /// completion. This does not update wgpu tracking or synchronize other queues.
    pub unsafe fn uav_barrier(&mut self, texture: &Dx12TextureLease) {
        let mut barrier = D3D12_RESOURCE_BARRIER {
            Type: D3D12_RESOURCE_BARRIER_TYPE_UAV,
            Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
            Anonymous: D3D12_RESOURCE_BARRIER_0 {
                UAV: std::mem::ManuallyDrop::new(D3D12_RESOURCE_UAV_BARRIER {
                    pResource: std::mem::ManuallyDrop::new(Some(texture.resource.clone())),
                }),
            },
        };
        // SAFETY: Caller guarantees valid UAV usage; descriptor remains live during recording.
        unsafe { self.list.ResourceBarrier(std::slice::from_ref(&barrier)) };
        // SAFETY: UAV is the active union arm; release exactly the temporary COM clone.
        unsafe { std::mem::ManuallyDrop::drop(&mut (*barrier.Anonymous.UAV).pResource) };
    }
    /// Record a full-texture state transition; identical states emit no command.
    /// # Safety
    /// Texture/list must share a device. `before` must match every subresource's
    /// actual state at execution. Caller must retain the lease through completion
    /// and restore/reconcile state before further wgpu use. This does not update
    /// wgpu's internal resource trackers.
    pub unsafe fn transition_texture(
        &mut self,
        texture: &Dx12TextureLease,
        before: D3D12_RESOURCE_STATES,
        after: D3D12_RESOURCE_STATES,
    ) {
        if before == after {
            return;
        }
        let mut barrier = D3D12_RESOURCE_BARRIER {
            Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
            Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
            Anonymous: D3D12_RESOURCE_BARRIER_0 {
                Transition: std::mem::ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
                    pResource: std::mem::ManuallyDrop::new(Some(texture.resource.clone())),
                    Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                    StateBefore: before,
                    StateAfter: after,
                }),
            },
        };
        // SAFETY: Caller supplies valid states/ownership; descriptor lives for this call.
        unsafe { self.list.ResourceBarrier(std::slice::from_ref(&barrier)) };
        // SAFETY: Transition is the active union arm; release exactly the COM clone
        // constructed above. The lease retains the actual GPU resource independently.
        unsafe { std::mem::ManuallyDrop::drop(&mut (*barrier.Anonymous.Transition).pResource) };
    }
    /// Create a direct command list on the same device registered with Streamline.
    /// # Errors
    /// Returns the HRESULT from native allocator/list creation.
    pub fn new(device: &ID3D12Device) -> Result<Self, StreamlineError> {
        // SAFETY: Live COM device, valid direct-list type, independently owned allocator.
        let allocator = unsafe { device.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT) }
            .map_err(native_error)?;
        // SAFETY: Allocator is new and idle; list starts recording with no initial PSO.
        let list = unsafe {
            device.CreateCommandList(
                0,
                D3D12_COMMAND_LIST_TYPE_DIRECT,
                &allocator,
                None::<&ID3D12PipelineState>,
            )
        }
        .map_err(native_error)?;
        Ok(Self {
            list,
            allocator,
            thread_bound: PhantomData,
        })
    }

    /// Access the recording list for resource barriers and frame tagging.
    /// # Safety
    /// Do not close, reset, submit, destroy or retain this pointer past its owner.
    /// Commands must respect resource states, device ownership and SDK lifetimes.
    /// # Errors
    /// Rejects a missing native pointer.
    pub unsafe fn recording_pointer(&mut self) -> Result<NonNull<c_void>, StreamlineError> {
        NonNull::new(self.list.as_raw()).ok_or(StreamlineError::InvalidOptions)
    }

    /// Record DLSS work into this direct command list.
    /// # Safety
    /// Frame and recorder must use the same device. Matching constants/tags and
    /// resource barriers must be prepared. Retain resources through GPU completion.
    /// # Errors
    /// Preserves SDK failures and rejects non-evaluated feature selectors.
    pub unsafe fn evaluate(
        &mut self,
        frame: &mut StreamlineFrame<'_>,
        viewport: u32,
        feature: StreamlineFeature,
    ) -> Result<(), StreamlineError> {
        // SAFETY: Caller provides resource/frame validity; this owner keeps the list live.
        let pointer = unsafe { self.recording_pointer()? };
        // SAFETY: Same caller obligations as the native frame evaluation entrypoint.
        unsafe { frame.evaluate_dx12(viewport, feature, pointer) }
    }

    /// Finish recording. No submission or GPU synchronization is performed.
    /// # Errors
    /// Returns native close/cast failures; a failed recorder is discarded.
    pub fn finish(self) -> Result<RecordedCommands, StreamlineError> {
        // SAFETY: This uniquely managed list is recording and has not been submitted.
        unsafe { self.list.Close() }.map_err(native_error)?;
        let list = self.list.cast().map_err(native_error)?;
        Ok(RecordedCommands {
            list,
            _allocator: self.allocator,
            thread_bound: PhantomData,
        })
    }
}

/// Closed commands plus their allocator. Retain through GPU completion after submission.
#[derive(Debug)]
pub struct RecordedCommands {
    list: ID3D12CommandList,
    _allocator: ID3D12CommandAllocator,
    thread_bound: PhantomData<Rc<()>>,
}
impl RecordedCommands {
    /// Submit once on a direct queue and signal a private completion fence.
    /// # Safety
    /// Queue, device, commands and resources must belong to the registered device.
    /// Caller must order wgpu/native work and reconcile resource states. Include
    /// every texture used by these commands. SDK use after these commands (such as
    /// FG presentation) requires separate retained owners. No explicit destruction
    /// is allowed while GPU work is pending; keep the SDK runtime live as needed.
    /// # Errors
    /// Rejects non-direct queues and preserves fence creation/signal HRESULTs.
    /// A signal failure after submission conservatively retains GPU dependencies.
    pub unsafe fn submit(
        self,
        device: &ID3D12Device,
        queue: &ID3D12CommandQueue,
        resources: Vec<Dx12TextureLease>,
    ) -> Result<Submission, StreamlineError> {
        // SAFETY: Live borrowed COM queue; querying its description does not submit work.
        if unsafe { queue.GetDesc() }.Type != D3D12_COMMAND_LIST_TYPE_DIRECT {
            return Err(StreamlineError::InvalidOptions);
        }
        // SAFETY: Live device and valid fence parameters; fence has independent ownership.
        let fence: ID3D12Fence =
            unsafe { device.CreateFence(0, D3D12_FENCE_FLAG_NONE) }.map_err(native_error)?;
        let submission = Submission {
            fence,
            commands: Some(self),
            resources: Some(resources),
            queue: queue.clone(),
        };
        let lists = [submission
            .commands
            .as_ref()
            .map(|commands| commands.list.clone())];
        // SAFETY: Caller guarantees device compatibility, resource states and queue order.
        unsafe { queue.ExecuteCommandLists(&lists) };
        // SAFETY: The private fence belongs to the queue's device; value 1 is used once.
        unsafe { queue.Signal(&submission.fence, 1) }.map_err(native_error)?;
        Ok(submission)
    }
    /// Borrow the closed list for submission to a compatible native direct queue.
    /// # Safety
    /// Keep this owner, resources and SDK dependencies until GPU completion.
    /// Do not reset the list/allocator or resubmit without appropriate synchronization.
    #[must_use]
    pub unsafe fn command_list(&self) -> &ID3D12CommandList {
        &self.list
    }
}

/// Owns submitted allocator/list and textures until the native fence completes.
/// Dropping while pending retains dependencies for process lifetime rather than
/// freeing resources still in use. Poll completion before dropping normally.
#[derive(Debug)]
pub struct Submission {
    fence: ID3D12Fence,
    commands: Option<RecordedCommands>,
    resources: Option<Vec<Dx12TextureLease>>,
    queue: ID3D12CommandQueue,
}
impl Submission {
    /// # Errors
    /// Returns a device-removed HRESULT if fence completion reports removal.
    pub fn is_complete(&self) -> Result<bool, StreamlineError> {
        // SAFETY: Live privately owned fence; querying completion is nonblocking.
        let value = unsafe { self.fence.GetCompletedValue() };
        if value == u64::MAX {
            return Err(StreamlineError::Dxgi {
                hresult: 0x887A_0005_u32.cast_signed(),
            });
        }
        Ok(value >= 1)
    }
}
impl Drop for Submission {
    fn drop(&mut self) {
        if self.is_complete() != Ok(true) {
            std::mem::forget(self.queue.clone());
            std::mem::forget(self.fence.clone());
            if let Some(commands) = self.commands.take() {
                std::mem::forget(commands);
            }
            if let Some(resources) = self.resources.take() {
                std::mem::forget(resources);
            }
        }
    }
}
#[allow(clippy::needless_pass_by_value)] // Used directly as Result::map_err callback.
fn native_error(error: windows::core::Error) -> StreamlineError {
    StreamlineError::Dxgi {
        hresult: error.code().0,
    }
}

/// Result of an actual DXGI presentation attempt, preserving both API and marker
/// outcomes. A successful HRESULT can still describe an occluded presentation.
#[derive(Debug, Clone, Copy)]
pub struct PresentReport {
    pub backbuffer_index: u32,
    pub hresult: i32,
    /// Present already happened when this is set. Retry only the end marker,
    /// never repeat presentation merely to repair the marker sequence.
    pub end_marker_error: Option<StreamlineError>,
}
impl StreamlineFrame<'_> {
    /// Retry a failed presentation end marker without repeating DXGI Present.
    /// No SDK call is made when the original end marker already succeeded.
    /// # Safety
    /// Report must come from this exact live frame's `present_dxgi` call. No
    /// other marker or presentation operation may have occurred since that call.
    /// Preserve SDK/resource lifetimes until the presentation consumer completes.
    /// # Errors
    /// Preserves SDK marker errors and updates the report's end-marker outcome.
    pub unsafe fn finish_present_marker(
        &mut self,
        report: &mut PresentReport,
    ) -> Result<(), StreamlineError> {
        if report.end_marker_error.is_none() {
            return Ok(());
        }
        let result = self.mark(crate::ReflexMarker::PresentEnd);
        report.end_marker_error = result.err();
        result
    }
    /// Present a proxy swapchain between the frame's Reflex presentation markers.
    /// Obtains the current backbuffer index as required by SDK DX12 FG integration.
    /// # Safety
    /// Swapchain belongs to this initialized SDK/device and has valid presentation
    /// interception. Submit markers must already surround real completed submission
    /// work for this token. Inputs/constants/backbuffer must be initialized with
    /// correct states. Serialize on the present thread; preserve SDK/resources
    /// through FG consumption and obey swapchain/queue synchronization contracts.
    /// # Errors
    /// Rejects invalid sync intervals and test-only presents. A start-marker error
    /// prevents presentation. Actual Present HRESULT and end-marker error are both
    /// returned in the report so neither failure hides the other.
    pub unsafe fn present_dxgi(
        &mut self,
        swapchain: &windows::Win32::Graphics::Dxgi::IDXGISwapChain3,
        sync_interval: u32,
        flags: windows::Win32::Graphics::Dxgi::DXGI_PRESENT,
    ) -> Result<PresentReport, StreamlineError> {
        use crate::ReflexMarker;
        use windows::Win32::Graphics::Dxgi::DXGI_PRESENT_TEST;
        if sync_interval > 4 || flags.0 & DXGI_PRESENT_TEST.0 != 0 {
            return Err(StreamlineError::InvalidOptions);
        }
        // SAFETY: Live swapchain and caller-provided presentation synchronization.
        let backbuffer_index = unsafe { swapchain.GetCurrentBackBufferIndex() };
        self.mark(ReflexMarker::PresentStart)?;
        // SAFETY: Caller establishes valid presentation state and SDK interception.
        let hresult = unsafe { swapchain.Present(sync_interval, flags) }.0;
        // Close the marker span even if DXGI fails or reports occlusion.
        let end_marker_error = self.mark(ReflexMarker::PresentEnd).err();
        Ok(PresentReport {
            backbuffer_index,
            hresult,
            end_marker_error,
        })
    }
}
