//! Graphics-session ownership boundary for runtime-compatible native devices.
use crate::{XrGraphics, XrHistoryReset, XrRuntime, XrRuntimeError};

/// Owned event effects, independent of the reusable runtime event buffer.
#[derive(Clone, Copy, Debug)]
pub enum XrSessionEvent {
    Ignored,
    StateChanged {
        state: openxr::SessionState,
        teardown: bool,
    },
    InstanceLoss {
        loss_time: openxr::Time,
    },
    ReferenceSpaceChange {
        kind: openxr::ReferenceSpaceType,
        change_time: openxr::Time,
        /// Absent when the runtime cannot relate the old and new origins.
        pose_in_previous_space: Option<openxr::Posef>,
    },
    InteractionProfileChanged,
    EventsLost(u32),
}

/// Inputs for one stereo projection submission, retaining all image/space borrows.
/// Eye poses must come from the pending frame's locate call in this space.
/// Images must be released after their GPU work completes before submission.
pub struct StereoSubmission<'a, G: openxr::Graphics> {
    pub blend: openxr::EnvironmentBlendMode,
    pub space: &'a openxr::Space,
    pub eyes: &'a [openxr::View; 2],
    /// Left, right runtime images.
    pub images: [openxr::SwapchainSubImage<'a, G>; 2],
    pub flags: openxr::CompositionLayerFlags,
}
impl<G: openxr::Graphics> std::fmt::Debug for StereoSubmission<'_, G> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StereoSubmission")
            .field("blend", &self.blend)
            .field("flags", &self.flags)
            .finish_non_exhaustive()
    }
}

/// A compositor-owned 2D panel positioned in its reference space, sized in metres.
/// Finish GPU work and release the image before submission. Its borrow remains
/// alive until submission returns. Eye visibility supports stereo or one eye.
pub struct QuadSubmission<'a, G: openxr::Graphics> {
    pub space: &'a openxr::Space,
    pub image: openxr::SwapchainSubImage<'a, G>,
    pub pose: openxr::Posef,
    pub size: openxr::Extent2Df,
    pub visibility: openxr::EyeVisibility,
    pub flags: openxr::CompositionLayerFlags,
}
impl<G: openxr::Graphics> std::fmt::Debug for QuadSubmission<'_, G> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QuadSubmission")
            .field("pose", &self.pose)
            .field("size", &self.size)
            .field("visibility", &self.visibility)
            .finish_non_exhaustive()
    }
}

pub struct XrSession<'a, G: openxr::Graphics> {
    pub session: openxr::Session<G>,
    waiter: openxr::FrameWaiter,
    frames: openxr::FrameStream<G>,
    running: bool,
    frame: Option<openxr::FrameState>,
    runtime: &'a XrRuntime,
}
impl<G: openxr::Graphics> XrSession<'_, G> {
    /// Negotiates a native swapchain format that the renderer can actually use.
    /// Preferences are graphics-API-specific formats in application order.
    /// No implicit format/conversion fallback is chosen, including for an empty list.
    /// # Errors
    /// Returns runtime enumeration errors or no supported format intersection.
    pub fn select_swapchain_format(
        &self,
        preferences: &[G::Format],
    ) -> Result<G::Format, XrRuntimeError>
    where
        G::Format: PartialEq,
    {
        let supported = self
            .session
            .enumerate_swapchain_formats()
            .map_err(XrRuntimeError::Runtime)?;
        select_format(&supported, preferences)
    }

    /// Routes one event polled from this session's runtime instance.
    /// Session-specific events for another session are ignored. Poll at the
    /// instance level and route each event to every interested session; this
    /// method does not consume events from a shared queue itself.
    /// Reference-space changes are returned with their effective time so the
    /// application can compensate its world at the correct predicted frame time.
    /// Instance loss stops new frames and requires instance/session recreation.
    /// # Errors
    /// Reports lifecycle errors applying this session's state changes.
    pub fn handle_event(
        &mut self,
        event: &openxr::Event<'_>,
    ) -> Result<XrSessionEvent, XrRuntimeError> {
        let own = self.session.as_raw();
        Ok(match event {
            openxr::Event::SessionStateChanged(value) if value.session() == own => {
                let state = value.state();
                let teardown = self.handle_state(state)?;
                XrSessionEvent::StateChanged { state, teardown }
            }
            openxr::Event::InstanceLossPending(value) => {
                self.running = false;
                XrSessionEvent::InstanceLoss {
                    loss_time: value.loss_time(),
                }
            }
            openxr::Event::ReferenceSpaceChangePending(value) if value.session() == own => {
                XrSessionEvent::ReferenceSpaceChange {
                    kind: value.reference_space_type(),
                    change_time: value.change_time(),
                    pose_in_previous_space: value
                        .pose_valid()
                        .then(|| value.pose_in_previous_space()),
                }
            }
            openxr::Event::InteractionProfileChanged(value) if value.session() == own => {
                XrSessionEvent::InteractionProfileChanged
            }
            openxr::Event::EventsLost(value) => {
                XrSessionEvent::EventsLost(value.lost_event_count())
            }
            _ => XrSessionEvent::Ignored,
        })
    }

    /// Routes an event and applies its temporal invalidation to combined history.
    /// Use this instead of `handle_event` when retaining motion/exposure history.
    /// # Errors
    /// Propagates lifecycle failures and clears history on those failures.
    pub fn handle_event_with_history<T>(
        &mut self,
        event: &openxr::Event<'_>,
        resets: &mut XrHistoryReset,
        history: &mut Option<T>,
    ) -> Result<XrSessionEvent, XrRuntimeError> {
        match self.handle_event(event) {
            Ok(effect) => {
                resets.handle_event(effect, history);
                Ok(effect)
            }
            Err(error) => {
                resets.reset(history);
                Err(error)
            }
        }
    }

    /// Creates an identity-origin tracking space using explicit application preferences.
    /// For example `[STAGE, LOCAL]` permits a seated fallback; `[STAGE]` requires
    /// floor-relative room tracking. An empty list never selects an implicit origin.
    /// The returned type identifies which coordinate system was actually selected.
    /// Handle runtime reference-space-change events before using new poses in a
    /// persistent world; this function does not compensate for recentering.
    /// # Errors
    /// Returns unsupported-space or runtime enumeration/creation errors.
    pub fn create_tracking_space(
        &self,
        preferences: &[openxr::ReferenceSpaceType],
    ) -> Result<(openxr::ReferenceSpaceType, openxr::Space), XrRuntimeError> {
        let available = self
            .session
            .enumerate_reference_spaces()
            .map_err(XrRuntimeError::Runtime)?;
        let selected = select_reference_space(&available, preferences)?;
        let space = self
            .session
            .create_reference_space(selected, openxr::Posef::IDENTITY)
            .map_err(XrRuntimeError::Runtime)?;
        Ok((selected, space))
    }

    /// Applies a `SessionStateChanged` event belonging to this session.
    /// Returns true when the application should tear down/recreate the session.
    /// Do not call raw begin/end methods alongside this lifecycle owner.
    /// # Errors
    /// Rejects duplicate READY or STOPPING during an unfinished frame; reports runtime errors.
    pub fn handle_state(&mut self, state: openxr::SessionState) -> Result<bool, XrRuntimeError> {
        match state {
            openxr::SessionState::READY => {
                if self.running {
                    return Err(XrRuntimeError::InvalidSessionLifecycle);
                }
                self.session
                    .begin(openxr::ViewConfigurationType::PRIMARY_STEREO)
                    .map_err(XrRuntimeError::Runtime)?;
                self.running = true;
            }
            openxr::SessionState::STOPPING => {
                if self.frame.is_some() {
                    return Err(XrRuntimeError::InvalidSessionLifecycle);
                }
                if self.running {
                    self.session.end().map_err(XrRuntimeError::Runtime)?;
                    self.running = false;
                }
            }
            openxr::SessionState::EXITING | openxr::SessionState::LOSS_PENDING => {
                self.running = false;
                return Ok(true);
            }
            _ => {}
        }
        Ok(false)
    }

    /// Waits for predicted display timing and begins one frame.
    /// # Errors
    /// Rejects stopped sessions/unfinished frames before runtime calls.
    pub fn begin_frame(&mut self) -> Result<openxr::FrameState, XrRuntimeError> {
        if !self.running || self.frame.is_some() {
            return Err(XrRuntimeError::InvalidSessionLifecycle);
        }
        let state = self.waiter.wait().map_err(XrRuntimeError::Runtime)?;
        self.frames.begin().map_err(XrRuntimeError::Runtime)?;
        self.frame = Some(state);
        Ok(state)
    }

    /// Begin a frame while retaining application motion/exposure history.
    /// Successful begin preserves history until stereo location validates poses
    /// and predicted time. Use `locate_stereo_views_with_history` next.
    /// # Errors
    /// Clears combined history on lifecycle, wait or begin failure. A previously
    /// pending frame remains pending when lifecycle validation rejects a retry.
    pub fn begin_frame_with_history<T>(
        &mut self,
        resets: &mut XrHistoryReset,
        history: &mut Option<T>,
    ) -> Result<openxr::FrameState, XrRuntimeError> {
        match self.begin_frame() {
            Ok(state) => Ok(state),
            Err(error) => {
                resets.reset(history);
                Err(error)
            }
        }
    }

    /// Locates both eyes at the current frame's predicted display time.
    /// Call immediately before rendering to minimize pose age. `None` means
    /// rendering was skipped or the runtime has no valid position/orientation;
    /// do not submit a projection layer using stale poses in that case.
    /// The base space must belong to this session.
    /// # Errors
    /// Rejects calls outside a running frame, foreign instances, non-stereo
    /// runtime output and runtime location errors.
    pub fn locate_stereo_views(
        &self,
        space: &openxr::Space,
    ) -> Result<Option<[openxr::View; 2]>, XrRuntimeError> {
        let state = self
            .frame
            .as_ref()
            .filter(|_| self.running)
            .ok_or(XrRuntimeError::InvalidSessionLifecycle)?;
        if space.instance().as_raw() != self.runtime.instance.as_raw() {
            return Err(XrRuntimeError::InvalidSessionLifecycle);
        }
        if !state.should_render {
            return Ok(None);
        }
        let (flags, views) = self
            .session
            .locate_views(
                openxr::ViewConfigurationType::PRIMARY_STEREO,
                state.predicted_display_time,
                space,
            )
            .map_err(XrRuntimeError::Runtime)?;
        validated_stereo_views(flags, views)
    }

    /// Locates the current stereo poses and invalidates history before rendering.
    /// A runtime-skipped frame preserves history unless an origin change is due;
    /// absent tracking on a renderable frame or location errors clear history.
    /// # Errors
    /// Propagates location/lifecycle errors after clearing combined history.
    pub fn locate_stereo_views_with_history<T>(
        &self,
        space: &openxr::Space,
        resets: &mut XrHistoryReset,
        history: &mut Option<T>,
    ) -> Result<Option<[openxr::View; 2]>, XrRuntimeError> {
        let Some(state) = self.frame.as_ref().filter(|_| self.running) else {
            resets.reset(history);
            return Err(XrRuntimeError::InvalidSessionLifecycle);
        };
        let located = self.locate_stereo_views(space);
        let tracking_valid = located_tracking_valid(&located, state.should_render);
        resets.begin_frame(state.predicted_display_time, tracking_valid, history);
        located
    }

    /// Ends the current frame at its predicted time. Submit no layers if `should_render` is false.
    /// Release acquired swapchain images after GPU completion before calling this method.
    /// # Errors
    /// Rejects absent frames, unsupported blend modes, oversized layer arrays
    /// and layers on skipped frames. Rejections leave the frame pending for retry.
    pub fn end_frame(
        &mut self,
        blend: openxr::EnvironmentBlendMode,
        layers: &[&openxr::CompositionLayerBase<'_, G>],
    ) -> Result<(), XrRuntimeError> {
        let state = self
            .frame
            .as_ref()
            .ok_or(XrRuntimeError::InvalidSessionLifecycle)?;
        if (!state.should_render && !layers.is_empty()) || u32::try_from(layers.len()).is_err() {
            return Err(XrRuntimeError::InvalidSessionLifecycle);
        }
        if !self.runtime.environment_blend_modes().contains(&blend) {
            return Err(XrRuntimeError::UnsupportedBlendMode);
        }
        self.frames
            .end(state.predicted_display_time, blend, layers)
            .map_err(XrRuntimeError::Runtime)?;
        self.frame = None;
        Ok(())
    }

    /// Cancel a pending stereo frame after rendering is skipped or abandoned.
    /// Finish all GPU work using either image before calling: no GPU fence wait
    /// is performed here. Released/idle eyes are skipped on retry, so a timeout
    /// on the second eye does not cause a second release of the first eye.
    /// Returns false on image wait timeout, retaining the pending frame. After
    /// both eyes are idle, submits an empty layer list without advancing any
    /// application-owned temporal history.
    /// ```no_run
    /// fn cancel<G: openxr::Graphics>(
    ///     session: &mut voxy_xr::XrSession<'_, G>,
    ///     left: &mut voxy_xr::XrSwapchain<G>,
    ///     right: &mut voxy_xr::XrSwapchain<G>,
    /// ) -> Result<bool, voxy_xr::XrRuntimeError> {
    ///     // GPU completion must be established by the renderer before this call.
    ///     session.discard_stereo_frame(openxr::EnvironmentBlendMode::OPAQUE,
    ///         [left, right], openxr::Duration::from_nanos(1_000_000))
    /// }
    /// ```
    /// # Errors
    /// Rejects absent frames, negative timeouts and unsupported blend modes
    /// before touching images. Runtime wait/release/end errors leave the frame
    /// pending and preserve each swapchain's recoverable ownership state.
    pub fn discard_stereo_frame(
        &mut self,
        blend: openxr::EnvironmentBlendMode,
        images: [&mut crate::XrSwapchain<G>; 2],
        timeout: openxr::Duration,
    ) -> Result<bool, XrRuntimeError> {
        let mut images = images;
        self.discard_frame_images(blend, &mut images, timeout)
    }

    /// Cancel a frame using any set of outstanding swapchains, including one
    /// shared array-layer swapchain for multiview stereo. Each chain appears
    /// once; Rust mutable borrows prohibit aliasing the same chain in the list.
    /// Finish all GPU use first. Timeout/errors retain the pending frame and
    /// retries skip already released images. Empty lists end a frame without
    /// touching resources; the caller must include every outstanding chain.
    /// ```no_run
    /// fn cancel_multiview<G: openxr::Graphics>(
    ///     session: &mut voxy_xr::XrSession<'_, G>,
    ///     stereo_array: &mut voxy_xr::XrSwapchain<G>,
    /// ) -> Result<bool, voxy_xr::XrRuntimeError> {
    ///     session.discard_frame_images(openxr::EnvironmentBlendMode::OPAQUE,
    ///         &mut [stereo_array], openxr::Duration::from_nanos(1_000_000))
    /// }
    /// ```
    /// # Errors
    /// Rejects missing frames, negative timeout, unsupported blend and foreign
    /// session ownership before releasing images; propagates runtime errors.
    pub fn discard_frame_images(
        &mut self,
        blend: openxr::EnvironmentBlendMode,
        images: &mut [&mut crate::XrSwapchain<G>],
        timeout: openxr::Duration,
    ) -> Result<bool, XrRuntimeError> {
        if self.frame.is_none() || timeout.as_nanos() < 0 {
            return Err(XrRuntimeError::InvalidSessionLifecycle);
        }
        if !self.runtime.environment_blend_modes().contains(&blend) {
            return Err(XrRuntimeError::UnsupportedBlendMode);
        }
        if images.iter().any(|image| !image.belongs_to(&self.session)) {
            return Err(XrRuntimeError::InvalidSessionLifecycle);
        }
        for image in images {
            if image.ownership() != crate::ImageOwnership::Idle && !image.discard_image(timeout)? {
                return Ok(false);
            }
        }
        self.end_frame(blend, &[])?;
        Ok(true)
    }

    /// Submits rendered layers and replaces their temporal history only on success.
    /// Bundle both eyes' motion and shared GPU exposure in `T` so they advance
    /// together. The candidate must describe these exact layers and already have
    /// completed GPU work; this method does not wait for GPU completion.
    /// Empty-layer/skipped frames must use `end_frame` and leave history unchanged.
    /// Reset the application's history on tracking/session/reference-space changes.
    /// # Errors
    /// Rejects empty layers, then propagates `end_frame` errors. On every error,
    /// the last submitted history is retained and the candidate is discarded.
    pub fn end_frame_with_history<T>(
        &mut self,
        blend: openxr::EnvironmentBlendMode,
        layers: &[&openxr::CompositionLayerBase<'_, G>],
        history: &mut Option<T>,
        candidate: T,
    ) -> Result<(), XrRuntimeError> {
        if layers.is_empty() {
            return Err(XrRuntimeError::InvalidSessionLifecycle);
        }
        commit_history_after(history, candidate, || self.end_frame(blend, layers))
    }

    /// Builds a stereo layer and commits combined temporal history on success.
    /// Use one `T` for both eyes' motion and their shared GPU exposure. All input
    /// validation and runtime submission finish before replacing that history.
    /// Candidate GPU work must be complete; skipped frames use `end_frame`.
    /// ```no_run
    /// fn submit<G: openxr::Graphics, T>(
    ///     session: &mut voxy_xr::XrSession<'_, G>,
    ///     space: &openxr::Space,
    ///     eyes: &[openxr::View; 2],
    ///     images: [openxr::SwapchainSubImage<'_, G>; 2],
    ///     history: &mut Option<T>,
    ///     candidate: T,
    /// ) -> Result<(), voxy_xr::XrRuntimeError> {
    ///     session.end_stereo_frame_with_history(voxy_xr::StereoSubmission {
    ///         blend: openxr::EnvironmentBlendMode::OPAQUE,
    ///         space, eyes, images,
    ///         flags: openxr::CompositionLayerFlags::EMPTY,
    ///     }, history, candidate)
    /// }
    /// ```
    /// # Errors
    /// Propagates `end_stereo_frame` errors, preserving prior history on failure.
    pub fn end_stereo_frame_with_history<T>(
        &mut self,
        submission: StereoSubmission<'_, G>,
        history: &mut Option<T>,
        candidate: T,
    ) -> Result<(), XrRuntimeError> {
        commit_history_after(history, candidate, || {
            self.end_stereo_frame(
                submission.blend,
                submission.space,
                submission.eyes,
                submission.images,
                submission.flags,
            )
        })
    }

    /// Builds and submits one stereo projection layer for the pending frame.
    /// Eyes and images are ordered left, right. Use views located at this frame's
    /// predicted time in `space`, and images obtained from released swapchains.
    /// Complete GPU rendering before release. The image borrows remain alive until
    /// submission returns, preventing reacquisition through the owning wrapper.
    /// Use `end_frame` with no layers when rendering/tracking is unavailable.
    /// # Errors
    /// Rejects non-finite eye data, non-unit rotations, a foreign space instance,
    /// invalid frame/blend mode and runtime submission failures.
    pub fn end_stereo_frame(
        &mut self,
        blend: openxr::EnvironmentBlendMode,
        space: &openxr::Space,
        eyes: &[openxr::View; 2],
        images: [openxr::SwapchainSubImage<'_, G>; 2],
        flags: openxr::CompositionLayerFlags,
    ) -> Result<(), XrRuntimeError> {
        self.end_stereo_with_optional_quad(
            StereoSubmission {
                blend,
                space,
                eyes,
                images,
                flags,
            },
            None,
        )
    }

    /// Submits stereo geometry followed by a compositor 2D menu/HUD panel.
    /// The panel can use a different reference space from the projection layer.
    /// Both spaces must belong to this runtime instance. Image ownership and GPU
    /// completion requirements are identical to `end_stereo_frame`.
    /// # Errors
    /// Rejects invalid panel geometry/visibility, foreign spaces, invalid stereo
    /// inputs and runtime submission failures.
    pub fn end_stereo_frame_with_quad(
        &mut self,
        submission: StereoSubmission<'_, G>,
        quad: QuadSubmission<'_, G>,
    ) -> Result<(), XrRuntimeError> {
        self.end_stereo_with_optional_quad(submission, Some(quad))
    }

    /// Commits combined stereo history only after both layers submit successfully.
    /// # Errors
    /// Propagates submission errors while retaining the previous history.
    pub fn end_stereo_frame_with_quad_and_history<T>(
        &mut self,
        submission: StereoSubmission<'_, G>,
        quad: QuadSubmission<'_, G>,
        history: &mut Option<T>,
        candidate: T,
    ) -> Result<(), XrRuntimeError> {
        commit_history_after(history, candidate, || {
            self.end_stereo_frame_with_quad(submission, quad)
        })
    }

    fn end_stereo_with_optional_quad(
        &mut self,
        submission: StereoSubmission<'_, G>,
        quad: Option<QuadSubmission<'_, G>>,
    ) -> Result<(), XrRuntimeError> {
        let StereoSubmission {
            blend,
            space,
            eyes,
            images,
            flags,
        } = submission;
        if space.instance().as_raw() != self.runtime.instance.as_raw() {
            return Err(XrRuntimeError::InvalidSessionLifecycle);
        }
        for eye in eyes {
            validate_projection_eye(eye)?;
        }
        let [left, right] = images;
        let views = [
            openxr::CompositionLayerProjectionView::new()
                .pose(eyes[0].pose)
                .fov(eyes[0].fov)
                .sub_image(left),
            openxr::CompositionLayerProjectionView::new()
                .pose(eyes[1].pose)
                .fov(eyes[1].fov)
                .sub_image(right),
        ];
        let layer = openxr::CompositionLayerProjection::new()
            .space(space)
            .layer_flags(flags)
            .views(&views);
        if let Some(quad) = quad {
            if quad.space.instance().as_raw() != self.runtime.instance.as_raw() {
                return Err(XrRuntimeError::InvalidSessionLifecycle);
            }
            validate_quad(quad.pose, quad.size, quad.visibility)?;
            let panel = openxr::CompositionLayerQuad::new()
                .space(quad.space)
                .layer_flags(quad.flags)
                .eye_visibility(quad.visibility)
                .sub_image(quad.image)
                .pose(quad.pose)
                .size(quad.size);
            self.end_frame(blend, &[&layer, &panel])
        } else {
            self.end_frame(blend, &[&layer])
        }
    }
}

pub(crate) fn validate_quad(
    pose: openxr::Posef,
    size: openxr::Extent2Df,
    visibility: openxr::EyeVisibility,
) -> Result<(), XrRuntimeError> {
    let p = pose.position;
    let q = pose.orientation;
    let norm = q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w;
    let values = [p.x, p.y, p.z, q.x, q.y, q.z, q.w, size.width, size.height];
    if values.iter().any(|v| !v.is_finite())
        || (norm - 1.0).abs() > 1.0e-4
        || size.width <= 0.0
        || size.height <= 0.0
        || ![
            openxr::EyeVisibility::BOTH,
            openxr::EyeVisibility::LEFT,
            openxr::EyeVisibility::RIGHT,
        ]
        .contains(&visibility)
    {
        return Err(XrRuntimeError::InvalidQuad);
    }
    Ok(())
}

fn located_tracking_valid(
    located: &Result<Option<[openxr::View; 2]>, XrRuntimeError>,
    should_render: bool,
) -> bool {
    located
        .as_ref()
        .is_ok_and(|views| !should_render || views.is_some())
}

fn commit_history_after<T>(
    history: &mut Option<T>,
    candidate: T,
    submit: impl FnOnce() -> Result<(), XrRuntimeError>,
) -> Result<(), XrRuntimeError> {
    submit()?;
    *history = Some(candidate);
    Ok(())
}

fn select_format<F: Copy + PartialEq>(
    supported: &[F],
    preferences: &[F],
) -> Result<F, XrRuntimeError> {
    preferences
        .iter()
        .copied()
        .find(|format| supported.contains(format))
        .ok_or(XrRuntimeError::UnsupportedSwapchainFormat)
}

fn validate_projection_eye(eye: &openxr::View) -> Result<(), XrRuntimeError> {
    let p = eye.pose.position;
    let q = eye.pose.orientation;
    let f = eye.fov;
    let values = [
        p.x,
        p.y,
        p.z,
        q.x,
        q.y,
        q.z,
        q.w,
        f.angle_left,
        f.angle_right,
        f.angle_up,
        f.angle_down,
    ];
    let norm = q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w;
    let half_pi = std::f32::consts::FRAC_PI_2;
    let valid_angles = [f.angle_left, f.angle_right, f.angle_up, f.angle_down]
        .into_iter()
        .all(|angle| angle > -half_pi && angle < half_pi);
    // Preserve legal flipped frusta while rejecting singular projection planes.
    let nondegenerate = (f.angle_right - f.angle_left).abs() > f32::EPSILON
        && (f.angle_up - f.angle_down).abs() > f32::EPSILON;
    if values.iter().any(|value| !value.is_finite())
        || (norm - 1.0).abs() > 1.0e-4
        || !valid_angles
        || !nondegenerate
    {
        return Err(XrRuntimeError::InvalidStereoViews);
    }
    Ok(())
}

fn select_reference_space(
    available: &[openxr::ReferenceSpaceType],
    preferences: &[openxr::ReferenceSpaceType],
) -> Result<openxr::ReferenceSpaceType, XrRuntimeError> {
    preferences
        .iter()
        .copied()
        .find(|kind| available.contains(kind))
        .ok_or(XrRuntimeError::UnsupportedReferenceSpace)
}

fn validated_stereo_views(
    flags: openxr::ViewStateFlags,
    views: Vec<openxr::View>,
) -> Result<Option<[openxr::View; 2]>, XrRuntimeError> {
    let views: [openxr::View; 2] = views
        .try_into()
        .map_err(|_| XrRuntimeError::InvalidStereoViews)?;
    let required =
        openxr::ViewStateFlags::POSITION_VALID | openxr::ViewStateFlags::ORIENTATION_VALID;
    if !flags.contains(required) {
        return Ok(None);
    }
    for eye in &views {
        validate_projection_eye(eye)?;
    }
    Ok(Some(views))
}
impl<G: openxr::Graphics> std::fmt::Debug for XrSession<'_, G> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XrSession")
            .field("runtime", &self.runtime)
            .finish_non_exhaustive()
    }
}
impl XrRuntime {
    /// Creates a desktop OpenGL session using the runtime-compatible native binding.
    /// Prefer GLX on Linux and WGL on Windows; the `OpenXR` Wayland binding is deprecated.
    /// The guard is retained by the session and every clone until destruction.
    /// # Safety
    /// All binding handles must be valid and refer to the same OpenGL context,
    /// whose version satisfies the previously queried graphics requirements.
    /// The guard must retain the context, display, drawable and configuration.
    /// Keep the context current on the calling thread as required by
    /// `XR_KHR_opengl_enable`, including swapchain operations, and respect its
    /// threading and external synchronization rules throughout session use.
    /// # Errors
    /// Rejects non-OpenGL selection, then reports runtime session creation errors.
    #[cfg(any(windows, target_os = "linux"))]
    #[allow(unsafe_code)]
    pub unsafe fn create_gl_session(
        &self,
        info: &openxr::opengl::SessionCreateInfo,
        guard: Box<dyn std::any::Any + Send + Sync>,
    ) -> Result<XrSession<'_, openxr::OpenGL>, XrRuntimeError> {
        // SAFETY: Public caller contract supplies native GL ownership and threading.
        unsafe { self.create_bound_session::<openxr::OpenGL>(XrGraphics::OpenGl, info, guard) }
    }

    /// Creates a Windows DX12 graphics session on the runtime-required adapter.
    /// # Safety
    /// Device must match the queried adapter LUID/minimum feature level; queue
    /// must belong to it. The guard must retain both COM objects for all session
    /// clones. Respect `XR_KHR_D3D12_enable` external synchronization requirements.
    /// # Errors
    /// Rejects non-DX12 selection, then reports runtime session creation errors.
    #[cfg(windows)]
    #[allow(unsafe_code)]
    pub unsafe fn create_dx12_session(
        &self,
        info: &openxr::d3d::SessionCreateInfoD3D12,
        guard: Box<dyn std::any::Any + Send + Sync>,
    ) -> Result<XrSession<'_, openxr::D3D12>, XrRuntimeError> {
        // SAFETY: Public caller contract above supplies device/queue ownership.
        unsafe { self.create_bound_session::<openxr::D3D12>(XrGraphics::DirectX12, info, guard) }
    }

    /// Creates an Android GLES session with a runtime-compatible EGL binding.
    /// # Safety
    /// Display/config/context must satisfy `XR_KHR_opengl_es_enable` requirements.
    /// Retain them in the guard, keep the context current as required, and respect
    /// the extension's threading/external synchronization rules for session use.
    /// # Errors
    /// Rejects non-GLES selection, then reports runtime session creation errors.
    #[cfg(target_os = "android")]
    #[allow(unsafe_code)]
    pub unsafe fn create_gles_session(
        &self,
        info: &openxr::opengles::SessionCreateInfo,
        guard: Box<dyn std::any::Any + Send + Sync>,
    ) -> Result<XrSession<'_, openxr::OpenGlEs>, XrRuntimeError> {
        // SAFETY: Public caller contract above supplies EGL ownership/synchronization.
        unsafe { self.create_bound_session::<openxr::OpenGlEs>(XrGraphics::OpenGlEs, info, guard) }
    }
    /// Creates a Vulkan session after discovery has queried graphics requirements.
    /// The guard is retained by the session and its clones until destruction.
    /// # Safety
    /// Handles must identify a runtime-compatible Vulkan instance, physical device,
    /// logical device and queue created according to `XR_KHR_vulkan_enable2`.
    /// The guard must own/retain all resources referenced by these handles.
    /// Respect the extension's external synchronization rules for all session use.
    /// # Errors
    /// Rejects a non-Vulkan runtime selection, then reports session creation errors.
    #[allow(unsafe_code)]
    pub unsafe fn create_vulkan_session(
        &self,
        info: &openxr::vulkan::SessionCreateInfo,
        guard: Box<dyn std::any::Any + Send + Sync>,
    ) -> Result<XrSession<'_, openxr::Vulkan>, XrRuntimeError> {
        // SAFETY: Public caller contract above supplies native Vulkan ownership.
        unsafe { self.create_bound_session::<openxr::Vulkan>(XrGraphics::Vulkan, info, guard) }
    }

    #[allow(unsafe_code)]
    unsafe fn create_bound_session<G: openxr::Graphics>(
        &self,
        selected: XrGraphics,
        info: &G::SessionCreateInfo,
        guard: Box<dyn std::any::Any + Send + Sync>,
    ) -> Result<XrSession<'_, G>, XrRuntimeError> {
        if self.graphics != selected {
            return Err(XrRuntimeError::MissingGraphicsExtension);
        }
        #[cfg(target_os = "android")]
        let guard: Box<dyn std::any::Any + Send + Sync> =
            Box::new((guard, self.android_app.clone()));
        // SAFETY: Native graphics validity, ownership and synchronization are the
        // caller's explicit contract. Android activity ownership is also retained.
        let (session, waiter, frames) = unsafe {
            self.instance
                .create_session_with_guard::<G>(self.system, info, guard)
        }
        .map_err(XrRuntimeError::Runtime)?;
        Ok(XrSession {
            session,
            waiter,
            frames,
            runtime: self,
            running: false,
            frame: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::validated_stereo_views;

    #[test]
    fn skipped_frame_preserves_history_but_tracking_loss_does_not() {
        let mut resets = crate::XrHistoryReset::new(openxr::ReferenceSpaceType::LOCAL);
        let mut history = Some(7);
        let skipped = Ok(None);
        resets.begin_frame(
            openxr::Time::from_nanos(1),
            super::located_tracking_valid(&skipped, false),
            &mut history,
        );
        assert_eq!(history, Some(7));
        resets.begin_frame(
            openxr::Time::from_nanos(2),
            super::located_tracking_valid(&skipped, true),
            &mut history,
        );
        assert_eq!(history, None);
        history = Some(8);
        let failed = Err(crate::XrRuntimeError::InvalidSessionLifecycle);
        resets.begin_frame(
            openxr::Time::from_nanos(3),
            super::located_tracking_valid(&failed, false),
            &mut history,
        );
        assert_eq!(history, None);
    }

    #[test]
    fn temporal_history_advances_only_after_successful_submission() {
        // Tuple stands for both eye histories plus their shared exposure.
        let mut history = Some(([1, 2], 3));
        let candidate = ([4, 5], 6);
        assert!(
            super::commit_history_after(&mut history, candidate, || {
                Err(crate::XrRuntimeError::Runtime(
                    openxr::sys::Result::ERROR_RUNTIME_FAILURE,
                ))
            })
            .is_err()
        );
        assert_eq!(history, Some(([1, 2], 3)));
        super::commit_history_after(&mut history, candidate, || Ok(())).unwrap();
        assert_eq!(history, Some(candidate));
        history = None;
        assert!(
            super::commit_history_after(&mut history, candidate, || {
                Err(crate::XrRuntimeError::InvalidSessionLifecycle)
            })
            .is_err()
        );
        assert_eq!(history, None);
    }

    #[test]
    fn swapchain_format_requires_renderer_runtime_intersection() {
        assert_eq!(super::select_format(&[1, 2], &[3, 2, 1]).unwrap(), 2);
        assert!(matches!(
            super::select_format(&[1, 2], &[3]),
            Err(crate::XrRuntimeError::UnsupportedSwapchainFormat)
        ));
        assert!(super::select_format::<u32>(&[], &[1]).is_err());
        assert!(super::select_format(&[1], &[]).is_err());
    }

    #[test]
    fn projection_rejects_invalid_runtime_or_application_pose_data() {
        let valid = openxr::View {
            pose: openxr::Posef::IDENTITY,
            fov: openxr::Fovf {
                angle_left: -0.8,
                angle_right: 0.9,
                angle_up: 0.7,
                angle_down: -0.6,
            },
        };
        assert!(super::validate_projection_eye(&valid).is_ok());
        let mut invalid = valid;
        invalid.pose.position.x = f32::NAN;
        assert!(super::validate_projection_eye(&invalid).is_err());
        invalid = valid;
        invalid.pose.orientation.w = 0.0;
        assert!(super::validate_projection_eye(&invalid).is_err());
        invalid = valid;
        invalid.pose.orientation.x = f32::MAX;
        assert!(super::validate_projection_eye(&invalid).is_err());
        invalid = valid;
        invalid.fov.angle_up = f32::INFINITY;
        assert!(super::validate_projection_eye(&invalid).is_err());
        for angle in [
            std::f32::consts::FRAC_PI_2,
            -std::f32::consts::FRAC_PI_2,
            2.0,
        ] {
            invalid = valid;
            invalid.fov.angle_left = angle;
            assert!(super::validate_projection_eye(&invalid).is_err());
        }
        invalid = valid;
        invalid.fov.angle_right = invalid.fov.angle_left;
        assert!(super::validate_projection_eye(&invalid).is_err());
        invalid = valid;
        invalid.fov.angle_up = invalid.fov.angle_down;
        assert!(super::validate_projection_eye(&invalid).is_err());
        let mut flipped = valid;
        std::mem::swap(&mut flipped.fov.angle_left, &mut flipped.fov.angle_right);
        std::mem::swap(&mut flipped.fov.angle_up, &mut flipped.fov.angle_down);
        assert!(super::validate_projection_eye(&flipped).is_ok());
    }

    #[test]
    fn tracking_origin_fallback_is_explicit() {
        use openxr::ReferenceSpaceType as Space;
        let available = [Space::VIEW, Space::LOCAL];
        assert_eq!(
            super::select_reference_space(&available, &[Space::STAGE, Space::LOCAL]).unwrap(),
            Space::LOCAL
        );
        assert!(matches!(
            super::select_reference_space(&available, &[Space::STAGE]),
            Err(crate::XrRuntimeError::UnsupportedReferenceSpace)
        ));
        assert!(super::select_reference_space(&available, &[]).is_err());
        assert!(super::select_reference_space(&[], &[Space::LOCAL]).is_err());
        assert_eq!(
            super::select_reference_space(
                &[Space::LOCAL, Space::STAGE],
                &[Space::STAGE, Space::LOCAL]
            )
            .unwrap(),
            Space::STAGE
        );
    }

    fn views(count: usize) -> Vec<openxr::View> {
        (0..count)
            .map(|_| openxr::View {
                pose: openxr::Posef::IDENTITY,
                fov: openxr::Fovf {
                    angle_left: -0.8,
                    angle_right: 0.9,
                    angle_up: 0.7,
                    angle_down: -0.6,
                },
            })
            .collect()
    }

    #[test]
    fn lost_tracking_validity_does_not_publish_eye_poses() {
        for flags in [
            openxr::ViewStateFlags::EMPTY,
            openxr::ViewStateFlags::POSITION_VALID,
            openxr::ViewStateFlags::ORIENTATION_VALID,
            openxr::ViewStateFlags::POSITION_TRACKED | openxr::ViewStateFlags::ORIENTATION_TRACKED,
        ] {
            assert!(validated_stereo_views(flags, views(2)).unwrap().is_none());
        }
        // Valid inferred poses are usable even without TRACKED bits.
        let valid =
            openxr::ViewStateFlags::POSITION_VALID | openxr::ViewStateFlags::ORIENTATION_VALID;
        assert!(validated_stereo_views(valid, views(2)).unwrap().is_some());
    }

    #[test]
    fn valid_flags_do_not_publish_malformed_eye_data() {
        let flags =
            openxr::ViewStateFlags::POSITION_VALID | openxr::ViewStateFlags::ORIENTATION_VALID;
        for eye in 0..2 {
            let mut data = views(2);
            data[eye].pose.position.x = f32::NAN;
            assert!(validated_stereo_views(flags, data).is_err());
            let mut data = views(2);
            data[eye].pose.orientation.w = 0.0;
            assert!(validated_stereo_views(flags, data).is_err());
            let mut data = views(2);
            data[eye].fov.angle_right = data[eye].fov.angle_left;
            assert!(validated_stereo_views(flags, data).is_err());
        }
        // Invalid-tracking values are unspecified and must not be published.
        let mut lost = views(2);
        lost[0].pose.position.x = f32::NAN;
        assert!(
            validated_stereo_views(openxr::ViewStateFlags::EMPTY, lost)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn malformed_stereo_count_is_rejected_even_when_tracking_is_lost() {
        for count in [0, 1, 3] {
            assert!(matches!(
                validated_stereo_views(openxr::ViewStateFlags::EMPTY, views(count)),
                Err(crate::XrRuntimeError::InvalidStereoViews)
            ));
        }
    }
}

#[cfg(test)]
mod quad_tests {
    use super::*;
    #[test]
    fn panel_geometry_and_eye_selection() {
        let pose = openxr::Posef::IDENTITY;
        let size = openxr::Extent2Df {
            width: 1.2,
            height: 0.6,
        };
        for eye in [
            openxr::EyeVisibility::BOTH,
            openxr::EyeVisibility::LEFT,
            openxr::EyeVisibility::RIGHT,
        ] {
            assert!(validate_quad(pose, size, eye).is_ok());
        }
        for invalid in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            for bad in [
                openxr::Extent2Df {
                    width: invalid,
                    ..size
                },
                openxr::Extent2Df {
                    height: invalid,
                    ..size
                },
            ] {
                assert!(matches!(
                    validate_quad(pose, bad, openxr::EyeVisibility::BOTH),
                    Err(XrRuntimeError::InvalidQuad)
                ));
            }
        }
        let mut bad_pose = pose;
        bad_pose.orientation.w = 2.0;
        assert!(validate_quad(bad_pose, size, openxr::EyeVisibility::BOTH).is_err());
        bad_pose = pose;
        bad_pose.position.x = f32::NAN;
        assert!(validate_quad(bad_pose, size, openxr::EyeVisibility::BOTH).is_err());
        assert!(validate_quad(pose, size, openxr::EyeVisibility::from_raw(99)).is_err());
    }
}
