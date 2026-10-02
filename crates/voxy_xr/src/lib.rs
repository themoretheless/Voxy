//! `OpenXR` runtime discovery boundary, independent of desktop swapchains.
mod actions;
mod history;
mod panel;
mod session;
mod swapchain;
mod ui;
pub use actions::{ControllerProfile, Hand, HandInput, HapticPulse, XrActions};
pub use history::XrHistoryReset;
pub use panel::{QuadHit, hit_test_quad};
pub use session::{QuadSubmission, StereoSubmission, XrSession, XrSessionEvent};
pub use swapchain::{ImageOwnership, XrSwapchain};
pub use ui::XrPanelPointer;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum XrGraphics {
    Vulkan,
    DirectX12,
    OpenGl,
    OpenGlEs,
}
/// Runtime-owned requirements queried before any graphics session creation.
pub enum XrGraphicsRequirements {
    Vulkan(openxr::vulkan::Requirements),
    OpenGl(openxr::opengl::Requirements),
    OpenGlEs(openxr::opengles::Requirements),
    #[cfg(windows)]
    DirectX12(openxr::d3d::Requirements),
}
impl std::fmt::Debug for XrGraphicsRequirements {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Vulkan(value) => f.debug_tuple("Vulkan").field(value).finish(),
            Self::OpenGl(value) => f
                .debug_struct("OpenGl")
                .field("min", &value.min_api_version_supported)
                .field("max", &value.max_api_version_supported)
                .finish(),
            Self::OpenGlEs(value) => f
                .debug_struct("OpenGlEs")
                .field("min", &value.min_api_version_supported)
                .field("max", &value.max_api_version_supported)
                .finish(),
            #[cfg(windows)]
            Self::DirectX12(_) => f.debug_struct("DirectX12").finish_non_exhaustive(),
        }
    }
}
#[derive(Debug)]
pub enum XrRuntimeError {
    Loader(openxr::EntryError),
    Runtime(openxr::sys::Result),
    MissingGraphicsExtension,
    InvalidStereoViews,
    InvalidQuad,
    AndroidPlatformRequired,
    InvalidHaptic,
    InvalidSessionLifecycle,
    InvalidSubImage,
    UnsupportedBlendMode,
    UnsupportedReferenceSpace,
    UnsupportedSwapchainFormat,
}
impl std::fmt::Display for XrRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OpenXR error: {self:?}")
    }
}
impl std::error::Error for XrRuntimeError {}

/// Owns the runtime instance so discovered system handles stay valid.
pub struct XrRuntime {
    instance: openxr::Instance,
    system: openxr::SystemId,
    graphics: XrGraphics,
    requirements: XrGraphicsRequirements,
    stereo_views: [openxr::ViewConfigurationView; 2],
    blend_modes: Vec<openxr::EnvironmentBlendMode>,
    #[cfg(target_os = "android")]
    android_app: Option<android_activity::AndroidApp>,
}
impl std::fmt::Debug for XrRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XrRuntime")
            .field("system", &self.system)
            .field("graphics", &self.graphics)
            .finish_non_exhaustive()
    }
}
impl XrRuntime {
    /// Loads the system `OpenXR` loader and selects a stereo HMD with a strict API.
    /// Desktop discovery only; Android needs VM/activity loader initialization.
    /// # Errors
    /// Returns loader, extension, instance or unavailable-HMD errors.
    pub fn discover(graphics: XrGraphics) -> Result<Self, XrRuntimeError> {
        #[cfg(not(target_os = "android"))]
        {
            Self::discover_platform(graphics, &())
        }
        #[cfg(target_os = "android")]
        {
            let _ = graphics;
            Err(XrRuntimeError::AndroidPlatformRequired)
        }
    }

    /// Initializes the Android loader and retains its activity for the instance lifetime.
    /// # Errors
    /// Returns loader/runtime/extension or HMD discovery errors.
    #[cfg(target_os = "android")]
    #[allow(unsafe_code)]
    pub fn discover_android(
        graphics: XrGraphics,
        app: android_activity::AndroidApp,
    ) -> Result<Self, XrRuntimeError> {
        // SAFETY: AndroidApp owns valid VM/activity references; retain its clone
        // in the runtime until after the OpenXR instance is destroyed.
        let info =
            unsafe { openxr::AndroidPlatformInfo::new(app.vm_as_ptr(), app.activity_as_ptr()) };
        let mut runtime = Self::discover_platform(graphics, &info)?;
        runtime.android_app = Some(app);
        Ok(runtime)
    }

    #[allow(unsafe_code)]
    fn discover_platform(
        graphics: XrGraphics,
        info: &impl openxr::PlatformInfo,
    ) -> Result<Self, XrRuntimeError> {
        // SAFETY: Load only the platform-standard OpenXR loader name; no arbitrary
        // function pointers/path supplied by an asset or application file.
        let entry = unsafe { openxr::Entry::load(info) }.map_err(XrRuntimeError::Loader)?;
        let available = entry
            .enumerate_extensions()
            .map_err(XrRuntimeError::Runtime)?;
        let required = select_extensions(&available, graphics)?;
        let instance = entry
            .create_instance(
                &openxr::ApplicationInfo {
                    application_name: "Voxy",
                    application_version: 1,
                    engine_name: "Voxy",
                    engine_version: 1,
                    ..Default::default()
                },
                &required,
                &[],
                info,
            )
            .map_err(XrRuntimeError::Runtime)?;
        let system = instance
            .system(openxr::FormFactor::HEAD_MOUNTED_DISPLAY)
            .map_err(XrRuntimeError::Runtime)?;
        let requirements = match graphics {
            XrGraphics::Vulkan => XrGraphicsRequirements::Vulkan(
                instance
                    .graphics_requirements::<openxr::Vulkan>(system)
                    .map_err(XrRuntimeError::Runtime)?,
            ),
            XrGraphics::OpenGl => XrGraphicsRequirements::OpenGl(
                instance
                    .graphics_requirements::<openxr::OpenGL>(system)
                    .map_err(XrRuntimeError::Runtime)?,
            ),
            XrGraphics::OpenGlEs => XrGraphicsRequirements::OpenGlEs(
                instance
                    .graphics_requirements::<openxr::OpenGlEs>(system)
                    .map_err(XrRuntimeError::Runtime)?,
            ),
            #[cfg(windows)]
            XrGraphics::DirectX12 => XrGraphicsRequirements::DirectX12(
                instance
                    .graphics_requirements::<openxr::D3D12>(system)
                    .map_err(XrRuntimeError::Runtime)?,
            ),
            #[cfg(not(windows))]
            XrGraphics::DirectX12 => return Err(XrRuntimeError::MissingGraphicsExtension),
        };
        let views = instance
            .enumerate_view_configuration_views(
                system,
                openxr::ViewConfigurationType::PRIMARY_STEREO,
            )
            .map_err(XrRuntimeError::Runtime)?;
        if views.len() != 2
            || views.iter().any(|v| {
                v.recommended_image_rect_width == 0 || v.recommended_image_rect_height == 0
            })
        {
            return Err(XrRuntimeError::InvalidStereoViews);
        }
        let stereo_views = views
            .try_into()
            .map_err(|_| XrRuntimeError::InvalidStereoViews)?;
        let blend_modes = instance
            .enumerate_environment_blend_modes(
                system,
                openxr::ViewConfigurationType::PRIMARY_STEREO,
            )
            .map_err(XrRuntimeError::Runtime)?;
        if blend_modes.is_empty() {
            return Err(XrRuntimeError::UnsupportedBlendMode);
        }
        Ok(Self {
            instance,
            system,
            graphics,
            requirements,
            stereo_views,
            blend_modes,
            #[cfg(target_os = "android")]
            android_app: None,
        })
    }
    #[must_use]
    pub fn instance(&self) -> &openxr::Instance {
        &self.instance
    }
    /// Route at most `maximum_events` events from the shared instance queue.
    /// Call once at instance level and route each borrowed event to every
    /// interested session before returning from `handle`. A finite budget keeps
    /// event traffic from starving frame work; remaining events stay queued.
    /// Zero is a no-op. The returned count includes ignored/unrecognized events.
    /// # Errors
    /// Preserves runtime polling and handler failures. A handler failure occurs
    /// after that event has been consumed; it is not replayed on the next call.
    /// Clear temporal history if lifecycle processing cannot safely continue.
    pub fn poll_events(
        &self,
        storage: &mut openxr::EventDataBuffer,
        maximum_events: usize,
        mut handle: impl FnMut(&openxr::Event<'_>) -> Result<(), XrRuntimeError>,
    ) -> Result<usize, XrRuntimeError> {
        let mut processed = 0;
        while processed < maximum_events {
            let Some(event) = self
                .instance
                .poll_event(storage)
                .map_err(XrRuntimeError::Runtime)?
            else {
                break;
            };
            handle(&event)?;
            processed += 1;
        }
        Ok(processed)
    }
    #[must_use]
    pub fn system(&self) -> openxr::SystemId {
        self.system
    }
    #[must_use]
    pub fn graphics_requirements(&self) -> &XrGraphicsRequirements {
        &self.requirements
    }

    /// Runtime image-size/sample-count recommendations for the left and right eyes.
    #[must_use]
    pub fn stereo_view_configuration(&self) -> &[openxr::ViewConfigurationView; 2] {
        &self.stereo_views
    }

    /// Supported `PRIMARY_STEREO` composition modes in runtime preference order.
    #[must_use]
    pub fn environment_blend_modes(&self) -> &[openxr::EnvironmentBlendMode] {
        &self.blend_modes
    }

    /// Chooses the first supported mode in the application's preference list.
    /// An empty list explicitly accepts the runtime's preferred mode.
    /// A nonempty list never silently falls back to a mode the app cannot render.
    /// # Errors
    /// Returns an unsupported-mode error if no requested mode is available.
    pub fn select_environment_blend_mode(
        &self,
        preferences: &[openxr::EnvironmentBlendMode],
    ) -> Result<openxr::EnvironmentBlendMode, XrRuntimeError> {
        select_blend_mode(&self.blend_modes, preferences)
    }
}

fn select_blend_mode(
    supported: &[openxr::EnvironmentBlendMode],
    preferences: &[openxr::EnvironmentBlendMode],
) -> Result<openxr::EnvironmentBlendMode, XrRuntimeError> {
    let choice = if preferences.is_empty() {
        supported.first().copied()
    } else {
        preferences
            .iter()
            .copied()
            .find(|mode| supported.contains(mode))
    };
    choice.ok_or(XrRuntimeError::UnsupportedBlendMode)
}
fn select_extensions(
    available: &openxr::ExtensionSet,
    graphics: XrGraphics,
) -> Result<openxr::ExtensionSet, XrRuntimeError> {
    let mut required = openxr::ExtensionSet::default();
    #[cfg(target_os = "android")]
    {
        if !available.khr_android_create_instance {
            return Err(XrRuntimeError::MissingGraphicsExtension);
        }
        required.khr_android_create_instance = true;
    }
    match graphics {
        XrGraphics::Vulkan if available.khr_vulkan_enable2 => required.khr_vulkan_enable2 = true,
        #[cfg(windows)]
        XrGraphics::DirectX12 if available.khr_d3d12_enable => required.khr_d3d12_enable = true,
        XrGraphics::OpenGl if available.khr_opengl_enable => required.khr_opengl_enable = true,
        XrGraphics::OpenGlEs if available.khr_opengl_es_enable => {
            required.khr_opengl_es_enable = true;
        }
        _ => return Err(XrRuntimeError::MissingGraphicsExtension),
    }
    Ok(required)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn composition_selection_respects_application_and_runtime_preferences() {
        use openxr::EnvironmentBlendMode as Mode;
        let supported = [Mode::ALPHA_BLEND, Mode::OPAQUE];
        assert_eq!(
            select_blend_mode(&supported, &[]).unwrap(),
            Mode::ALPHA_BLEND
        );
        assert_eq!(
            select_blend_mode(&supported, &[Mode::OPAQUE, Mode::ALPHA_BLEND]).unwrap(),
            Mode::OPAQUE
        );
        assert_eq!(
            select_blend_mode(&supported, &[Mode::ADDITIVE, Mode::ALPHA_BLEND]).unwrap(),
            Mode::ALPHA_BLEND
        );
        assert!(matches!(
            select_blend_mode(&supported, &[Mode::ADDITIVE]),
            Err(XrRuntimeError::UnsupportedBlendMode)
        ));
        assert!(matches!(
            select_blend_mode(&[], &[]),
            Err(XrRuntimeError::UnsupportedBlendMode)
        ));
    }

    #[test]
    fn strict_graphics_negotiation() {
        let mut available = openxr::ExtensionSet::default();
        available.khr_vulkan_enable2 = true;
        #[cfg(target_os = "android")]
        {
            available.khr_android_create_instance = true;
        }
        let required = select_extensions(&available, XrGraphics::Vulkan).unwrap();
        assert!(required.khr_vulkan_enable2);
        assert!(!required.khr_opengl_enable);
        #[cfg(windows)]
        assert!(!required.khr_d3d12_enable);
        assert!(matches!(
            select_extensions(&available, XrGraphics::DirectX12),
            Err(XrRuntimeError::MissingGraphicsExtension)
        ));
    }
}
