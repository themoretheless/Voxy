//! Retained exact input receipt for the accepted single-clip root trajectory.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct RootRigidMotionReceipt {
    clip: Arc<AnimationClip>,
    joint: u16,
    start: f64,
    end: f64,
    axes: [bool; 3],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootRigidMotionCheckpoint {
    version: u32,
    clip: String,
    joint: u16,
    start: u64,
    end: u64,
    axes: [bool; 3],
    path: RootRigidPathCheckpoint,
}
impl Animator {
    /// Generate a path and its input receipt transactionally from the exact same
    /// source revision and f64 clock operands. Legacy callers need retain no receipt.
    /// # Errors
    /// Same validation and rollback as advance_with_root_rigid_motion_wall.
    pub fn advance_with_root_rigid_motion_receipt_wall(
        &mut self,
        skeleton: &Skeleton,
        dt: f64,
        axes: [bool; 3],
        max_spans: usize,
    ) -> Result<(AnimatorFrame, RootRigidPath, RootRigidMotionReceipt), AnimationError> {
        let receipt = RootRigidMotionReceipt {
            clip: self.current.clone(),
            joint: self.motion_joint,
            start: self.current.phase(self.time),
            end: self.current.phase(self.time) + dt * f64::from(self.speed),
            axes,
        };
        let (frame, path) =
            self.advance_with_root_rigid_motion_wall(skeleton, dt, axes, max_spans)?;
        Ok((frame, path, receipt))
    }
}
impl RootRigidMotionReceipt {
    #[must_use]
    pub fn root_motion_joint(&self) -> u16 {
        self.joint
    }

    fn program(&self) -> RootRigidPathProgram {
        RootRigidPathProgram::new(vec![RootRigidPathStep::Curve {
            revision: "receipt-source".into(),
            start_bits: self.start.to_bits(),
            end_bits: self.end.to_bits(),
            axes: self.axes,
        }])
    }
    /// Recreate the accepted source path without deriving clocks from normalized phase.
    /// # Errors
    /// Propagates the original compiler's source, timing and span-budget failures.
    pub fn path(&self, max_spans: usize) -> Result<RootRigidPath, AnimationError> {
        self.clip
            .root_rigid_curve(self.joint)?
            .path(self.start, self.end, self.axes, max_spans)
    }
    /// The owner must supply a stable identity for this exact immutable clip revision.
    /// # Errors
    /// Rejects unidentified sources, native replay mismatches and byte/work budgets.
    pub fn checkpoint(
        &self,
        identify: impl Fn(&Arc<AnimationClip>) -> Option<String>,
        limits: RootRigidPathCheckpointLimits,
    ) -> Result<RootRigidMotionCheckpoint, AnimatorCheckpointError> {
        let path = self.path(limits.max_spans)?;
        self.checkpoint_path(&path, identify, limits)
    }
    /// Compare an owner's actual queued path against the retained input receipt.
    /// # Errors
    /// Rejects any coefficient/certificate mismatch, missing identities and budgets.
    pub fn checkpoint_path(
        &self,
        path: &RootRigidPath,
        identify: impl Fn(&Arc<AnimationClip>) -> Option<String>,
        limits: RootRigidPathCheckpointLimits,
    ) -> Result<RootRigidMotionCheckpoint, AnimatorCheckpointError> {
        let clip = identify(&self.clip)
            .filter(|k| !k.is_empty())
            .ok_or(AnimatorCheckpointError::State)?;
        let curve = self.clip.root_rigid_curve(self.joint)?;
        let path = path.checkpoint(
            self.program(),
            |key| (key == "receipt-source").then(|| curve.clone()),
            limits,
        )?;
        let saved = RootRigidMotionCheckpoint {
            version: 1,
            clip,
            joint: self.joint,
            start: self.start.to_bits(),
            end: self.end.to_bits(),
            axes: self.axes,
            path,
        };
        saved.to_json(limits.max_bytes)?;
        Ok(saved)
    }
}
impl RootRigidMotionCheckpoint {
    /// # Errors
    /// Rejects unsupported versions and byte budgets.
    pub fn to_json(&self, max_bytes: usize) -> Result<String, AnimatorCheckpointError> {
        if self.version != 1 {
            return Err(AnimatorCheckpointError::State);
        }
        let text = serde_json::to_string(self)?;
        if text.len() > max_bytes {
            return Err(AnimatorCheckpointError::Capacity);
        }
        Ok(text)
    }
    /// Bound the underlying file read before this operation.
    /// # Errors
    /// Rejects oversized text before parsing, unknown fields and versions.
    pub fn from_json(text: &str, max_bytes: usize) -> Result<Self, AnimatorCheckpointError> {
        if text.len() > max_bytes {
            return Err(AnimatorCheckpointError::Capacity);
        }
        let saved: Self = serde_json::from_str(text)?;
        if saved.version != 1 {
            return Err(AnimatorCheckpointError::State);
        }
        Ok(saved)
    }
    /// Return a privately admitted receipt and matching path against a new rig/resource.
    /// The enclosing game still owns verification of the resource revision identity.
    /// # Errors
    /// Rejects invalid sources/clocks/rigs, changed programs or path bits and budgets.
    pub fn prepare(
        &self,
        skeleton: &Skeleton,
        resolve: impl Fn(&str) -> Option<Arc<AnimationClip>>,
        limits: RootRigidPathCheckpointLimits,
    ) -> Result<(RootRigidMotionReceipt, RootRigidPath), AnimatorCheckpointError> {
        self.to_json(limits.max_bytes)?;
        let clip = resolve(&self.clip)
            .filter(|c| !self.clip.is_empty() && c.is_compatible_with(skeleton))
            .ok_or(AnimatorCheckpointError::State)?;
        let start = f64::from_bits(self.start);
        let end = f64::from_bits(self.end);
        if !start.is_finite()
            || !end.is_finite()
            || start < 0.
            || start > f64::from(clip.duration)
            || end < start
        {
            return Err(AnimatorCheckpointError::State);
        }
        let receipt = RootRigidMotionReceipt {
            clip,
            joint: self.joint,
            start,
            end,
            axes: self.axes,
        };
        let curve = receipt.clip.root_rigid_curve(receipt.joint)?;
        let path = self.path.prepare_with_program(
            &receipt.program(),
            |key| (key == "receipt-source").then(|| curve.clone()),
            limits,
        )?;
        Ok((receipt, path))
    }
}
