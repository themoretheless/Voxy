//! Engine-owned DLSS 5 handoff contract, not an NVIDIA SDK ABI.
//! The SDK adapter must translate this only after its input requirements are verified.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArtisticControls {
    pub structure: f32,
    pub tone: f32,
}
impl Default for ArtisticControls {
    fn default() -> Self {
        Self {
            structure: 0.,
            tone: 0.,
        }
    }
}

/// All resources belong to the same rendered frame. The preservation mask uses
/// one to protect authored appearance (e.g. makeup), zero to permit enhancement.
#[derive(Debug)]
pub struct NeuralFrame<'a, Resource> {
    pub frame_id: u64,
    pub extent: [u32; 2],
    pub color: &'a Resource,
    pub motion: &'a Resource,
    pub depth: &'a Resource,
    pub preservation_mask: Option<&'a Resource>,
    pub reset_history: bool,
    pub controls: ArtisticControls,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandoffError {
    InvalidExtent,
    InvalidControls,
    StaleFrame,
    AdapterUnavailable,
}

/// Tracks temporal continuity independently of native resource/SDK ownership.
#[derive(Debug, Default)]
pub struct NeuralHistory {
    previous: Option<(u64, [u32; 2])>,
}
impl NeuralHistory {
    /// Validate before submission. History is committed only after successful
    /// inference, so a failed submission can retry the same frame.
    /// # Errors
    /// Rejects zero extents, invalid controls and already committed frame IDs.
    pub fn needs_reset<R>(&self, frame: &NeuralFrame<'_, R>) -> Result<bool, HandoffError> {
        if frame.extent.contains(&0) {
            return Err(HandoffError::InvalidExtent);
        }
        if [frame.controls.structure, frame.controls.tone]
            .iter()
            .any(|v| !v.is_finite() || !(0. ..=1.).contains(v))
        {
            return Err(HandoffError::InvalidControls);
        }
        match self.previous {
            None => Ok(true),
            Some((id, size)) => {
                if frame.frame_id <= id {
                    return Err(HandoffError::StaleFrame);
                }
                Ok(frame.reset_history
                    || size != frame.extent
                    || id.checked_add(1) != Some(frame.frame_id))
            }
        }
    }
    /// Commit continuity after successful inference.
    /// # Errors
    /// Preserves frame validation errors and leaves history unchanged on failure.
    pub fn commit<R>(&mut self, frame: &NeuralFrame<'_, R>) -> Result<(), HandoffError> {
        self.needs_reset(frame)?;
        self.previous = Some((frame.frame_id, frame.extent));
        Ok(())
    }
    pub fn invalidate(&mut self) {
        self.previous = None;
    }
}

/// Implemented by the authorized SDK adapter. Color-space conversion, resource
/// transitions, synchronization and actual NR options belong to that adapter.
pub trait NeuralAdapter<R> {
    /// Execute inference using the requested temporal reset.
    /// # Errors
    /// Returns adapter availability, resource or inference failures.
    fn evaluate(
        &mut self,
        frame: &NeuralFrame<'_, R>,
        reset: bool,
        output: &mut R,
    ) -> Result<(), HandoffError>;
}

/// Validate, evaluate and commit temporal history after successful inference.
/// # Errors
/// Preserves frame validation and adapter errors without committing history.
pub fn evaluate<R>(
    adapter: &mut impl NeuralAdapter<R>,
    history: &mut NeuralHistory,
    frame: &NeuralFrame<'_, R>,
    output: &mut R,
) -> Result<(), HandoffError> {
    let reset = history.needs_reset(frame)?;
    adapter.evaluate(frame, reset, output)?;
    history.commit(frame)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Adapter(bool);
    impl NeuralAdapter<u32> for Adapter {
        fn evaluate(
            &mut self,
            frame: &NeuralFrame<'_, u32>,
            _: bool,
            out: &mut u32,
        ) -> Result<(), HandoffError> {
            if self.0 {
                return Err(HandoffError::AdapterUnavailable);
            }
            *out = *frame.color;
            Ok(())
        }
    }
    #[test]
    fn history_resets_on_gap_resize_and_cut_but_failed_inference_can_retry() {
        let mut history = NeuralHistory::default();
        let value = 1;
        let mut out = 0;
        let mut frame = NeuralFrame {
            frame_id: 10,
            extent: [640, 480],
            color: &value,
            motion: &value,
            depth: &value,
            preservation_mask: None,
            reset_history: false,
            controls: ArtisticControls::default(),
        };
        assert_eq!(history.needs_reset(&frame), Ok(true));
        assert!(evaluate(&mut Adapter(true), &mut history, &frame, &mut out).is_err());
        evaluate(&mut Adapter(false), &mut history, &frame, &mut out).unwrap();
        assert_eq!(history.needs_reset(&frame), Err(HandoffError::StaleFrame));
        frame.frame_id += 1;
        assert_eq!(history.needs_reset(&frame), Ok(false));
        frame.extent[0] += 1;
        assert_eq!(history.needs_reset(&frame), Ok(true));
        frame.extent[0] -= 1;
        frame.frame_id += 1;
        assert_eq!(history.needs_reset(&frame), Ok(true));
        frame.frame_id -= 1;
        frame.reset_history = true;
        assert_eq!(history.needs_reset(&frame), Ok(true));
        frame.controls.tone = f32::NAN;
        assert_eq!(
            history.needs_reset(&frame),
            Err(HandoffError::InvalidControls)
        );
    }
}
