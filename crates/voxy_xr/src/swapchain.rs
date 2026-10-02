//! Single outstanding image with explicit timeout-aware runtime ownership.
use crate::XrRuntimeError;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageOwnership {
    Idle,
    Acquired(u32),
    Ready(u32),
}
pub struct XrSwapchain<G: openxr::Graphics> {
    chain: openxr::Swapchain<G>,
    images: Vec<G::SwapchainImage>,
    state: ImageOwnership,
    dimensions: (u32, u32, u32),
    released: bool,
    owner: (openxr::sys::Instance, openxr::sys::Session),
}
impl<G: openxr::Graphics> std::fmt::Debug for XrSwapchain<G> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XrSwapchain")
            .field("images", &self.images.len())
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}
impl<G: openxr::Graphics> XrSwapchain<G> {
    /// # Errors
    /// Returns runtime swapchain creation/enumeration errors.
    pub fn new(
        session: &openxr::Session<G>,
        info: &openxr::SwapchainCreateInfo<G>,
    ) -> Result<Self, XrRuntimeError> {
        if info.width == 0 || info.height == 0 || info.array_size == 0 {
            return Err(XrRuntimeError::InvalidSubImage);
        }
        let chain = session
            .create_swapchain(info)
            .map_err(XrRuntimeError::Runtime)?;
        let images = chain.enumerate_images().map_err(XrRuntimeError::Runtime)?;
        if images.is_empty() {
            return Err(XrRuntimeError::InvalidSessionLifecycle);
        }
        Ok(Self {
            chain,
            images,
            state: ImageOwnership::Idle,
            dimensions: (info.width, info.height, info.array_size),
            released: false,
            owner: (session.instance().as_raw(), session.as_raw()),
        })
    }
    pub(crate) fn belongs_to(&self, session: &openxr::Session<G>) -> bool {
        self.owner == (session.instance().as_raw(), session.as_raw())
    }
    #[must_use]
    pub fn ownership(&self) -> ImageOwnership {
        self.state
    }
    /// # Errors
    /// Rejects a second outstanding acquire, then reports runtime errors.
    pub fn acquire(&mut self) -> Result<u32, XrRuntimeError> {
        if self.state != ImageOwnership::Idle {
            return Err(XrRuntimeError::InvalidSessionLifecycle);
        }
        let index = self
            .chain
            .acquire_image()
            .map_err(XrRuntimeError::Runtime)?;
        self.state = ImageOwnership::Acquired(index);
        self.released = false;
        Ok(index)
    }
    /// Returns false on timeout, retaining the acquisition for another wait.
    /// # Errors
    /// Rejects invalid ordering/negative timeout and reports runtime failures.
    #[allow(unsafe_code)]
    pub fn wait(&mut self, timeout: openxr::Duration) -> Result<bool, XrRuntimeError> {
        let ImageOwnership::Acquired(_) = self.state else {
            return Err(XrRuntimeError::InvalidSessionLifecycle);
        };
        if timeout.as_nanos() < 0 {
            return Err(XrRuntimeError::InvalidSessionLifecycle);
        }
        let info = openxr::sys::SwapchainImageWaitInfo {
            ty: openxr::sys::StructureType::SWAPCHAIN_IMAGE_WAIT_INFO,
            next: std::ptr::null(),
            timeout,
        };
        // SAFETY: owned valid swapchain; &mut serializes calls; valid wait-info.
        // Use raw status because the high-level wrapper discards positive timeout.
        let result = unsafe {
            (self.chain.instance().fp().wait_swapchain_image)(self.chain.as_raw(), &raw const info)
        };
        complete_wait(&mut self.state, result)
    }
    /// Only exposes the image after runtime wait succeeds. Keep GPU use within acquisition.
    #[must_use]
    pub fn ready_image(&self) -> Option<&G::SwapchainImage> {
        let ImageOwnership::Ready(index) = self.state else {
            return None;
        };
        self.images.get(index as usize)
    }
    /// Caller must complete all GPU use before releasing to the compositor.
    /// # Errors
    /// Rejects release before successful wait, then reports runtime failures.
    #[allow(unsafe_code)]
    pub fn release(&mut self) -> Result<(), XrRuntimeError> {
        if !matches!(self.state, ImageOwnership::Ready(_)) {
            return Err(XrRuntimeError::InvalidSessionLifecycle);
        }
        // SAFETY: valid owned chain, one successfully waited acquisition, serialized
        // calls. Raw release pairs with raw wait; no high-level waited flag is used.
        let result = unsafe {
            (self.chain.instance().fp().release_swapchain_image)(
                self.chain.as_raw(),
                std::ptr::null(),
            )
        };
        complete_release(&mut self.state, &mut self.released, result)
    }

    /// Finish an outstanding acquisition without submitting a projection layer.
    /// Returns false on timeout; the caller may retry with the same acquisition.
    /// Before calling, complete any GPU work already using a ready image. This
    /// method performs no GPU fence wait and must not release in-flight GPU work.
    /// # Errors
    /// Rejects idle state/negative timeout and reports runtime wait/release errors,
    /// retaining the outstanding image so the caller can retry or tear down.
    pub fn discard_image(&mut self, timeout: openxr::Duration) -> Result<bool, XrRuntimeError> {
        if timeout.as_nanos() < 0 || self.state == ImageOwnership::Idle {
            return Err(XrRuntimeError::InvalidSessionLifecycle);
        }
        if matches!(self.state, ImageOwnership::Acquired(_)) && !self.wait(timeout)? {
            return Ok(false);
        }
        self.release()?;
        Ok(true)
    }

    /// Creates a projection-layer image reference after successful image release.
    /// Its borrow prevents acquiring this swapchain again before layer submission.
    /// # Errors
    /// Rejects unreleased images, empty/out-of-bounds viewports and array layers.
    pub fn sub_image(
        &self,
        rect: openxr::Rect2Di,
        array_layer: u32,
    ) -> Result<openxr::SwapchainSubImage<'_, G>, XrRuntimeError> {
        if !self.released || self.state != ImageOwnership::Idle {
            return Err(XrRuntimeError::InvalidSessionLifecycle);
        }
        validate_rect(self.dimensions, rect, array_layer)?;
        Ok(openxr::SwapchainSubImage::new()
            .swapchain(&self.chain)
            .image_rect(rect)
            .image_array_index(array_layer))
    }
}

fn complete_wait(
    state: &mut ImageOwnership,
    result: openxr::sys::Result,
) -> Result<bool, XrRuntimeError> {
    let ImageOwnership::Acquired(index) = *state else {
        return Err(XrRuntimeError::InvalidSessionLifecycle);
    };
    if result == openxr::sys::Result::TIMEOUT_EXPIRED {
        return Ok(false);
    }
    if result.into_raw() < 0 {
        return Err(XrRuntimeError::Runtime(result));
    }
    *state = ImageOwnership::Ready(index);
    Ok(true)
}

fn complete_release(
    state: &mut ImageOwnership,
    released: &mut bool,
    result: openxr::sys::Result,
) -> Result<(), XrRuntimeError> {
    if !matches!(state, ImageOwnership::Ready(_)) {
        return Err(XrRuntimeError::InvalidSessionLifecycle);
    }
    if result.into_raw() < 0 {
        return Err(XrRuntimeError::Runtime(result));
    }
    *state = ImageOwnership::Idle;
    *released = true;
    Ok(())
}

fn validate_rect(
    dimensions: (u32, u32, u32),
    rect: openxr::Rect2Di,
    layer: u32,
) -> Result<(), XrRuntimeError> {
    let (width, height, layers) = dimensions;
    if rect.offset.x < 0
        || rect.offset.y < 0
        || rect.extent.width <= 0
        || rect.extent.height <= 0
        || layer >= layers
    {
        return Err(XrRuntimeError::InvalidSubImage);
    }
    let right = i64::from(rect.offset.x) + i64::from(rect.extent.width);
    let bottom = i64::from(rect.offset.y) + i64::from(rect.extent.height);
    if right > i64::from(width) || bottom > i64::from(height) {
        return Err(XrRuntimeError::InvalidSubImage);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wait_and_release_failures_preserve_runtime_ownership_for_retry() {
        let mut state = ImageOwnership::Acquired(3);
        let mut released = false;
        assert!(!complete_wait(&mut state, openxr::sys::Result::TIMEOUT_EXPIRED).unwrap());
        assert_eq!(state, ImageOwnership::Acquired(3));
        assert!(complete_wait(&mut state, openxr::sys::Result::ERROR_RUNTIME_FAILURE).is_err());
        assert_eq!(state, ImageOwnership::Acquired(3));
        assert!(complete_release(&mut state, &mut released, openxr::sys::Result::SUCCESS).is_err());
        assert!(!released);
        assert!(complete_wait(&mut state, openxr::sys::Result::SUCCESS).unwrap());
        assert_eq!(state, ImageOwnership::Ready(3));
        assert!(
            complete_release(
                &mut state,
                &mut released,
                openxr::sys::Result::ERROR_RUNTIME_FAILURE
            )
            .is_err()
        );
        assert_eq!(state, ImageOwnership::Ready(3));
        assert!(!released);
        complete_release(&mut state, &mut released, openxr::sys::Result::SUCCESS).unwrap();
        assert_eq!(state, ImageOwnership::Idle);
        assert!(released);
        assert!(complete_wait(&mut state, openxr::sys::Result::SUCCESS).is_err());
    }
    #[test]
    fn viewport_bounds_and_array_layers() {
        let rect = openxr::Rect2Di {
            offset: openxr::Offset2Di { x: 0, y: 0 },
            extent: openxr::Extent2Di {
                width: 1024,
                height: 512,
            },
        };
        assert!(validate_rect((1024, 512, 2), rect, 1).is_ok());
        assert!(validate_rect((1024, 512, 2), rect, 2).is_err());
        assert!(validate_rect((1023, 512, 2), rect, 0).is_err());
        for invalid in [
            openxr::Rect2Di {
                offset: openxr::Offset2Di { x: -1, y: 0 },
                ..rect
            },
            openxr::Rect2Di {
                offset: openxr::Offset2Di { x: i32::MAX, y: 0 },
                extent: openxr::Extent2Di {
                    width: i32::MAX,
                    height: 1,
                },
            },
            openxr::Rect2Di {
                extent: openxr::Extent2Di {
                    width: 0,
                    height: 1,
                },
                ..rect
            },
        ] {
            assert!(validate_rect((1024, 512, 2), invalid, 0).is_err());
        }
    }
}
