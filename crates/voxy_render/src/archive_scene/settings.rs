use glam::{Mat4, Vec3};

use super::ArchiveSceneRenderer;
use super::types::{
    ArchiveToneMapper, display_mode, ev100_from_camera, exposure_from_ev100, step_eye_adaptation,
    step_wetness_accumulation,
};

impl ArchiveSceneRenderer {
    /// Replaces the scene's water; a world without bodies removes the water pass.
    pub fn set_water(&mut self, world: &voxy_water::WaterWorld) {
        self.water = (!world.bodies.is_empty()).then(|| {
            crate::archive_water::ArchiveWater::new(
                &self.device,
                &self.queue,
                &crate::archive_water::WaterTargets {
                    camera_layout: &self.camera_layout,
                    environment_layout: &self.environment_bindings.layout,
                    background: &self.background,
                    shadow: &self.shadow,
                    size: [self.width, self.height],
                },
                world,
            )
        });
    }

    /// Advances waves, breakers and ripples; without calls the water stays frozen.
    pub fn advance_water(&mut self, dt: f32) {
        if let Some(water) = &mut self.water {
            water.advance(dt);
        }
        if dt.is_finite() && dt > 0. {
            self.adapted_wetness =
                step_wetness_accumulation(self.adapted_wetness, self.weather.rain, dt);
            let target_eye_scale =
                (1. + 0.14 * self.weather.coverage + 0.18 * self.weather.rain).clamp(0.75, 1.45);
            self.adapted_exposure_scale =
                step_eye_adaptation(self.adapted_exposure_scale, target_eye_scale, dt);
        }
    }

    /// Seconds of water simulation; pass it to `WaterWorld::sample` so gameplay and
    /// buoyancy see the surface that is drawn.
    pub fn water_time(&self) -> f32 {
        self.water
            .as_ref()
            .map_or(0., super::super::archive_water::ArchiveWater::time)
    }

    /// Diagnostic submission counters; these do not certify visible fragments or FPS.
    #[doc(hidden)]
    pub fn water_render_stats(&self) -> Option<serde_json::Value> {
        self.water
            .as_ref()
            .map(super::super::archive_water::ArchiveWater::render_stats)
    }

    /// The water the renderer draws, for CPU queries.
    pub fn water(&self) -> Option<&voxy_water::WaterWorld> {
        self.water
            .as_ref()
            .map(super::super::archive_water::ArchiveWater::world)
    }

    /// Raises (positive) or depresses the surface around `at` (xz); ripples spread from it.
    pub fn water_splash(&mut self, at: glam::Vec2, radius: f32, height: f32) {
        if let Some(water) = &mut self.water {
            water.ripples().splash(at, radius, height);
        }
    }

    /// A body moving through the water from `from` to `to` (xz) this frame leaves a wake.
    pub fn water_wake(&mut self, from: glam::Vec2, to: glam::Vec2, radius: f32, height: f32) {
        if let Some(water) = &mut self.water {
            water.ripples().wake(from, to, radius, height);
        }
    }

    /// Update atmosphere without rebuilding geometry or allocating particles.
    pub fn set_weather(
        &mut self,
        weather: crate::weather::Weather,
    ) -> Result<(), crate::ComputeError> {
        if !weather.is_valid() {
            return Err(crate::ComputeError::Validation("invalid weather".into()));
        }
        if self.weather != weather {
            self.reset_temporal_history();
        }
        self.weather = weather;
        if self.water_time() == 0. {
            self.adapted_wetness = weather.rain;
        }
        Ok(())
    }

    /// Override ground wetness in [0, 1]; `None` follows `Weather::rain`.
    pub fn set_surface_wetness(&mut self, wetness: Option<f32>) -> Result<(), crate::ComputeError> {
        if wetness.is_some_and(|w| !w.is_finite() || !(0. ..=1.).contains(&w)) {
            return Err(crate::ComputeError::Validation(
                "invalid surface wetness".into(),
            ));
        }
        self.surface_wetness = wetness;
        Ok(())
    }

    /// Fraction of fully wet upward-facing ground that pools into flat puddles.
    pub fn set_puddle_coverage(&mut self, coverage: f32) -> Result<(), crate::ComputeError> {
        if !coverage.is_finite() || !(0. ..=1.).contains(&coverage) {
            return Err(crate::ComputeError::Validation(
                "invalid puddle coverage".into(),
            ));
        }
        self.puddle_coverage = coverage;
        Ok(())
    }

    pub fn puddle_coverage(&self) -> f32 {
        self.puddle_coverage
    }

    pub fn surface_wetness(&self) -> f32 {
        self.surface_wetness.unwrap_or(self.adapted_wetness)
    }

    pub fn set_weather_time(&mut self, seconds: f32) {
        if seconds.is_finite() {
            self.weather_time = seconds.rem_euclid(4096.);
        }
    }

    /// Direction points toward the sun. Rebuild the matching directional shadow map.
    pub fn set_sun(
        &mut self,
        direction: Vec3,
        colour: [f32; 4],
    ) -> Result<(), crate::ComputeError> {
        if !direction.is_finite()
            || direction.length_squared() < 1e-8
            || colour.iter().any(|v| !v.is_finite() || *v < 0.)
        {
            return Err(crate::ComputeError::Validation("invalid sun".into()));
        }
        self.sun = direction.normalize();
        self.sun_colour = colour;
        let centre = (self.shadow_bounds.0 + self.shadow_bounds.1) * 0.5;
        let extent = ((self.shadow_bounds.1 - self.shadow_bounds.0).length() * 0.44).max(1.);
        let up = if self.sun.dot(Vec3::Y).abs() > 0.99 {
            Vec3::Z
        } else {
            Vec3::Y
        };
        #[allow(deprecated)]
        {
            self.shadow_matrix =
                Mat4::orthographic_rh(-extent, extent, -extent, extent, 0.1, extent * 4.)
                    * Mat4::look_at_rh(centre + self.sun * extent * 2., centre, up);
        }
        self.shadow_ready = false;
        self.reset_temporal_history();
        Ok(())
    }

    /// Linear extended-sRGB output uses 1.0 as SDR reference white. No invented nits.
    /// Write scene-linear HDR into an `Rgba16Float` target instead of the
    /// display transform, for an external camera post-process such as
    /// [`crate::BodycamPostProcess`].
    pub fn set_linear_output(&mut self, linear: bool) {
        if self.linear_output != linear {
            self.linear_output = linear;
            self.write_display_parameters();
        }
    }

    /// Vertical field of view and camera up vector (roll) used by [`Self::draw`].
    pub fn set_camera_projection(&mut self, vertical_fov_radians: f32, up: Vec3) {
        if vertical_fov_radians.is_finite() && vertical_fov_radians > 0. && up.is_finite() {
            self.vertical_fov =
                vertical_fov_radians.clamp(1_f32.to_radians(), 170_f32.to_radians());
            self.camera_up = up.normalize_or(Vec3::Y);
        }
    }

    pub(super) fn output_mode(&self) -> f32 {
        if self.linear_output {
            3.
        } else {
            display_mode(self.hdr_output, self.tone_mapper)
        }
    }

    pub fn set_display_headroom(&mut self, headroom: f32) -> Result<(), crate::ComputeError> {
        if !headroom.is_finite() || headroom < 1. {
            return Err(crate::ComputeError::Validation(
                "invalid live EDR headroom".into(),
            ));
        }
        self.display_headroom = headroom;
        self.write_display_parameters();
        Ok(())
    }

    /// Photometric exposure from EV100 (Lagarde & de Rousiers 2014):
    /// exposure = 1 / (1.2 * 2^EV100). Lights are in lux / candela.
    pub fn set_exposure_ev100(&mut self, ev100: f32) -> Result<(), crate::ComputeError> {
        if !ev100.is_finite() {
            return Err(crate::ComputeError::Validation("invalid EV100".into()));
        }
        self.exposure = exposure_from_ev100(ev100);
        self.reset_temporal_history();
        Ok(())
    }

    /// Physical camera: f-number, shutter time in seconds and ISO sensitivity.
    pub fn set_camera_exposure(
        &mut self,
        aperture: f32,
        shutter_s: f32,
        iso: f32,
    ) -> Result<(), crate::ComputeError> {
        if !(aperture > 0. && shutter_s > 0. && iso > 0.) {
            return Err(crate::ComputeError::Validation(
                "invalid camera exposure".into(),
            ));
        }
        self.set_exposure_ev100(ev100_from_camera(aperture, shutter_s, iso))
    }

    pub fn exposure_ev100(&self) -> f32 {
        (1. / (1.2 * self.exposure)).log2()
    }

    /// Sets additive exposure compensation in stops (`EV`, clamped to `[-4.0, 4.0]`),
    /// scaling linear exposure by $2^{\text{bias\_ev}}$ without resetting temporal history.
    pub fn set_exposure_bias_ev(&mut self, bias_ev: f32) {
        if bias_ev.is_finite() {
            self.exposure_bias_ev = bias_ev.clamp(-4., 4.);
        }
    }

    pub fn exposure_bias_ev(&self) -> f32 {
        self.exposure_bias_ev
    }

    /// Thin-lens depth of field and shutter motion blur, applied in the
    /// temporal display pass (Temporal/MetalFx antialiasing only).
    pub fn set_camera_lens(
        &mut self,
        lens: crate::archive_temporal::ArchiveCameraLens,
    ) -> Result<(), crate::ComputeError> {
        let values = [
            lens.focal_length_m,
            lens.f_number,
            lens.focus_distance_m,
            lens.shutter_fraction,
            lens.sensor_width_m,
        ];
        if values.iter().any(|v| !v.is_finite() || *v < 0.) {
            return Err(crate::ComputeError::Validation(
                "invalid camera lens".into(),
            ));
        }
        self.camera_lens = lens;
        if let Some(temporal) = &mut self.temporal {
            temporal.lens = lens;
        }
        Ok(())
    }

    /// SDR display transform. HDR output keeps its hue-preserving highlight knee.
    pub fn set_tone_mapper(&mut self, tone_mapper: ArchiveToneMapper) {
        self.tone_mapper = tone_mapper;
        self.write_display_parameters();
    }

    pub fn tone_mapper(&self) -> ArchiveToneMapper {
        self.tone_mapper
    }

    /// Directional spatial AA, enabled by default on all backends including Metal.
    /// Disable for native-pixel comparisons or when applying an external AA pass.
    pub fn set_spatial_antialiasing(&mut self, enabled: bool) {
        self.temporal = None;
        self.antialiasing = if enabled {
            crate::ArchiveAntialiasing::Spatial
        } else {
            crate::ArchiveAntialiasing::Off
        };
        self.spatial_antialiasing = enabled;
        self.write_display_parameters();
    }

    /// Select temporal AA explicitly. Unsupported MetalFX returns an error and
    /// preserves the current mode; it never silently falls back to spatial AA.
    pub fn set_antialiasing(
        &mut self,
        mode: crate::ArchiveAntialiasing,
    ) -> Result<(), crate::ComputeError> {
        if mode == self.antialiasing {
            return Ok(());
        }
        let temporal = if matches!(
            mode,
            crate::ArchiveAntialiasing::Temporal | crate::ArchiveAntialiasing::MetalFx
        ) {
            Some(crate::archive_temporal::ArchiveTemporal::new(
                &self.device,
                self.width,
                self.height,
                self.output_format,
                mode,
            )?)
        } else {
            None
        };
        self.temporal = temporal.map(|mut t| {
            t.lens = self.camera_lens;
            t
        });
        self.antialiasing = mode;
        self.spatial_antialiasing = mode == crate::ArchiveAntialiasing::Spatial;
        self.write_display_parameters();
        Ok(())
    }

    /// Diagnostic previous-UV minus current-UV camera motion, excluding jitter.
    /// Valid only after drawing a temporal frame; immutable polygon meshes only.
    pub fn temporal_motion_texture(&self) -> Option<&wgpu::Texture> {
        self.temporal
            .as_ref()
            .map(super::super::archive_temporal::ArchiveTemporal::motion_texture)
    }

    pub fn antialiasing(&self) -> crate::ArchiveAntialiasing {
        self.antialiasing
    }

    /// Call on camera cuts or changes to scene lighting/materials/exposure.
    pub fn reset_temporal_history(&mut self) {
        if let Some(temporal) = &mut self.temporal {
            temporal.reset();
        }
        self.previous_vp = None;
    }

    pub(super) fn write_display_parameters(&self) {
        self.queue.write_buffer(
            &self.display_parameters,
            0,
            bytemuck::cast_slice(&[
                self.output_mode(),
                // Linear output bypasses display mapping, including EDR headroom.
                if self.linear_output {
                    1.
                } else {
                    self.display_headroom
                },
                f32::from(self.spatial_antialiasing),
                f32::from(self.temporal.is_some()),
            ]),
        );
    }
}
