//! Ordered perfect-planar reflection filtering with presentation-owned history.
use crate::{
    ComputeError, PlanarReflectionCameras, PlanarReflectionJob, PlanarReflectionPipeline,
    PreviousReflectionTriangle, RaySceneError, ReflectionCorrespondenceJob,
    ReflectionCorrespondencePipeline, RenderOutcome, SurfaceReflectionJob, TemporalHistory,
    TemporalResolve, TemporalResolveFrame, TemporalResolveOptions,
};

#[derive(Debug)]
pub enum PlanarTemporalError {
    Ray(RaySceneError),
    Compute(ComputeError),
    InvalidLifecycle,
}
impl From<RaySceneError> for PlanarTemporalError {
    fn from(e: RaySceneError) -> Self {
        Self::Ray(e)
    }
}
impl From<ComputeError> for PlanarTemporalError {
    fn from(e: ComputeError) -> Self {
        Self::Compute(e)
    }
}
impl std::fmt::Display for PlanarTemporalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "planar temporal error: {self:?}")
    }
}
impl std::error::Error for PlanarTemporalError {}
#[derive(Debug)]
pub struct PlanarTemporalPipeline {
    device: wgpu::Device,
    correspondence: ReflectionCorrespondencePipeline,
    reprojection: PlanarReflectionPipeline,
    resolve: TemporalResolve,
}
/// Borrows history exclusively until completion, preventing premature commit or
/// another candidate from writing its pending slot. Drop discards the candidate.
#[derive(Debug)]
pub struct PlanarTemporalFrame<'a> {
    history: &'a mut TemporalHistory,
    correspondence: ReflectionCorrespondenceJob,
    guides: PlanarReflectionJob,
    resolved: TemporalResolveFrame,
    encoded: bool,
}
impl PlanarTemporalPipeline {
    /// # Errors
    /// Rejects unsupported compute/storage capability.
    pub fn new(device: &wgpu::Device) -> Result<Self, PlanarTemporalError> {
        Ok(Self {
            device: device.clone(),
            correspondence: ReflectionCorrespondencePipeline::new(device)?,
            reprojection: PlanarReflectionPipeline::new(device)?,
            resolve: TemporalResolve::new(device)?,
        })
    }
    /// Prepare filtering of this exact perfect-planar reflection producer.
    /// Resize/reset the separate reflection history before preparation. Previous
    /// triangles and mirror/camera poses must correspond to its last presented
    /// frame. Do not use this path for curved/rough surfaces or direct lighting.
    /// `clip_history` uses the existing current-neighborhood clipping policy.
    /// Encode after ray production, display output, then finish with the actual
    /// host RenderOutcome. No preparation/submission alone commits history.
    /// # Errors
    /// Rejects foreign devices, invalid correspondence/guides or resolve inputs.
    pub fn prepare<'a>(
        &self,
        reflected: &SurfaceReflectionJob,
        previous_triangles: &[PreviousReflectionTriangle],
        cameras: PlanarReflectionCameras,
        history: &'a mut TemporalHistory,
        mut options: TemporalResolveOptions,
        clip_history: bool,
    ) -> Result<PlanarTemporalFrame<'a>, PlanarTemporalError> {
        history.validate_device(&self.device)?;
        options.reset_history |= !history.valid();
        let correspondence =
            self.correspondence
                .prepare(reflected, previous_triangles, options.reset_history)?;
        let guides = self
            .reprojection
            .prepare_reflection(reflected, &correspondence, cameras)?;
        let resolved = history.prepare_resolve(
            &self.resolve,
            reflected.radiance(),
            guides.motion(),
            guides.expected_previous_depth(),
            options,
            clip_history,
        )?;
        Ok(PlanarTemporalFrame {
            history,
            correspondence,
            guides,
            resolved,
            encoded: false,
        })
    }
}
impl PlanarTemporalFrame<'_> {
    /// Encode once, after the matching ray producer, on the same device/queue.
    /// # Errors
    /// Rejects a second encoding and incompatible pending-depth resources.
    pub fn encode(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), PlanarTemporalError> {
        if self.encoded {
            return Err(PlanarTemporalError::InvalidLifecycle);
        }
        self.correspondence.encode(encoder);
        self.guides.encode(encoder);
        self.resolved.encode(encoder);
        self.history
            .encode_depth(encoder, self.guides.current_depth())?;
        self.encoded = true;
        Ok(())
    }
    #[must_use]
    pub fn output(&self) -> &wgpu::Texture {
        self.resolved.output()
    }
    #[must_use]
    pub fn guides(&self) -> &PlanarReflectionJob {
        &self.guides
    }
    /// Use only the outcome of the submission that encoded/displayed this frame.
    /// Skipped/failed outcomes preserve the old readable history. A candidate
    /// cannot publish an unencoded slot even if passed Presented.
    /// # Errors
    /// Rejects a Presented outcome before encoding; history stays unchanged.
    pub fn finish(self, outcome: RenderOutcome) -> Result<bool, PlanarTemporalError> {
        if outcome != RenderOutcome::Presented {
            return Ok(false);
        }
        if !self.encoded {
            return Err(PlanarTemporalError::InvalidLifecycle);
        }
        self.history.presented();
        Ok(true)
    }
}
