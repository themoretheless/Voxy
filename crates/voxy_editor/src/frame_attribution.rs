//! Bounded per-domain CPU and presentation timing attribution across editor, Play, and HeadlessGame.

use serde::{Deserialize, Serialize};

const WINDOW_CAPACITY: usize = 120;

/// Microsecond duration attributed to each engine domain over a frame or headless step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainTimingUs {
    pub asset_io_poll_us: u64,
    pub physics_motion_us: u64,
    pub animation_us: u64,
    pub triggers_behaviors_us: u64,
    pub audio_us: u64,
    pub scene_extraction_us: u64,
    pub ui_layout_us: u64,
    pub render_submit_us: u64,
    pub surface_acquire_us: u64,
    pub total_cpu_us: u64,
}

impl DomainTimingUs {
    /// Accumulates another domain timing measurement into `self` with saturating addition.
    pub fn accumulate(&mut self, other: Self) {
        self.asset_io_poll_us = self.asset_io_poll_us.saturating_add(other.asset_io_poll_us);
        self.physics_motion_us = self
            .physics_motion_us
            .saturating_add(other.physics_motion_us);
        self.animation_us = self.animation_us.saturating_add(other.animation_us);
        self.triggers_behaviors_us = self
            .triggers_behaviors_us
            .saturating_add(other.triggers_behaviors_us);
        self.audio_us = self.audio_us.saturating_add(other.audio_us);
        self.scene_extraction_us = self
            .scene_extraction_us
            .saturating_add(other.scene_extraction_us);
        self.ui_layout_us = self.ui_layout_us.saturating_add(other.ui_layout_us);
        self.render_submit_us = self.render_submit_us.saturating_add(other.render_submit_us);
        self.surface_acquire_us = self
            .surface_acquire_us
            .saturating_add(other.surface_acquire_us);
        self.total_cpu_us = self.total_cpu_us.saturating_add(other.total_cpu_us);
    }

    fn named_fields(self) -> [(&'static str, u64); 9] {
        [
            ("physics_motion", self.physics_motion_us),
            ("animation", self.animation_us),
            ("triggers_behaviors", self.triggers_behaviors_us),
            ("audio", self.audio_us),
            ("scene_extraction", self.scene_extraction_us),
            ("ui_layout", self.ui_layout_us),
            ("render_submit", self.render_submit_us),
            ("surface_acquire", self.surface_acquire_us),
            ("asset_io_poll", self.asset_io_poll_us),
        ]
    }

    /// Returns the domain consuming the highest time in this sample, or `"idle"` if all are zero.
    #[must_use]
    pub fn dominant_domain(&self) -> &'static str {
        self.named_fields()
            .into_iter()
            .filter(|(_, us)| *us > 0)
            .max_by_key(|(_, us)| *us)
            .map_or("idle", |(name, _)| name)
    }

    fn map_domains(self, f: impl Fn(usize, u64) -> u64) -> Self {
        Self {
            asset_io_poll_us: f(0, self.asset_io_poll_us),
            physics_motion_us: f(1, self.physics_motion_us),
            animation_us: f(2, self.animation_us),
            triggers_behaviors_us: f(3, self.triggers_behaviors_us),
            audio_us: f(4, self.audio_us),
            scene_extraction_us: f(5, self.scene_extraction_us),
            ui_layout_us: f(6, self.ui_layout_us),
            render_submit_us: f(7, self.render_submit_us),
            surface_acquire_us: f(8, self.surface_acquire_us),
            total_cpu_us: f(9, self.total_cpu_us),
        }
    }

    fn as_array(self) -> [u64; 10] {
        [
            self.asset_io_poll_us,
            self.physics_motion_us,
            self.animation_us,
            self.triggers_behaviors_us,
            self.audio_us,
            self.scene_extraction_us,
            self.ui_layout_us,
            self.render_submit_us,
            self.surface_acquire_us,
            self.total_cpu_us,
        ]
    }

    fn from_array(values: [u64; 10]) -> Self {
        Self {
            asset_io_poll_us: values[0],
            physics_motion_us: values[1],
            animation_us: values[2],
            triggers_behaviors_us: values[3],
            audio_us: values[4],
            scene_extraction_us: values[5],
            ui_layout_us: values[6],
            render_submit_us: values[7],
            surface_acquire_us: values[8],
            total_cpu_us: values[9],
        }
    }
}

/// Summary statistics over a rolling window of domain timings.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameDomainAttribution {
    pub frames_recorded: u64,
    pub dominant_domain: String,
    pub last: DomainTimingUs,
    pub mean: DomainTimingUs,
    pub p95: DomainTimingUs,
    pub p99: DomainTimingUs,
    pub max: DomainTimingUs,
}

/// Fixed-capacity ring buffer tracking recent per-domain frame timings without heap growth.
#[derive(Clone, Debug)]
pub struct FrameAttributionWindow {
    samples: Box<[DomainTimingUs; WINDOW_CAPACITY]>,
    cursor: usize,
    len: usize,
    frames_recorded: u64,
    last: DomainTimingUs,
}

impl Default for FrameAttributionWindow {
    fn default() -> Self {
        Self {
            samples: Box::new([DomainTimingUs::default(); WINDOW_CAPACITY]),
            cursor: 0,
            len: 0,
            frames_recorded: 0,
            last: DomainTimingUs::default(),
        }
    }
}

impl FrameAttributionWindow {
    /// Records one completed frame or headless step sample into the rolling window.
    pub fn record(&mut self, sample: DomainTimingUs) {
        self.samples[self.cursor] = sample;
        self.cursor = (self.cursor + 1) % WINDOW_CAPACITY;
        self.len = (self.len + 1).min(WINDOW_CAPACITY);
        self.frames_recorded = self.frames_recorded.saturating_add(1);
        self.last = sample;
    }

    /// Clears recorded samples while retaining allocated buffer storage.
    pub fn reset(&mut self) {
        self.cursor = 0;
        self.len = 0;
        self.frames_recorded = 0;
        self.last = DomainTimingUs::default();
    }

    /// Total frames/steps recorded since construction or last [`Self::reset`].
    #[must_use]
    pub fn frames_recorded(&self) -> u64 {
        self.frames_recorded
    }

    fn active_samples(&self) -> impl Iterator<Item = &DomainTimingUs> {
        self.samples[..self.len].iter()
    }

    fn percentile_index(count: usize, numerator: usize, denominator: usize) -> usize {
        if count <= 1 {
            return 0;
        }
        (count * numerator)
            .div_ceil(denominator)
            .saturating_sub(1)
            .min(count - 1)
    }

    /// Computes mean, p95, p99, and maximum per-domain durations over the rolling window.
    #[must_use]
    pub fn snapshot(&self) -> FrameDomainAttribution {
        if self.len == 0 {
            return FrameDomainAttribution {
                frames_recorded: self.frames_recorded,
                dominant_domain: "idle".to_owned(),
                last: self.last,
                mean: DomainTimingUs::default(),
                p95: DomainTimingUs::default(),
                p99: DomainTimingUs::default(),
                max: DomainTimingUs::default(),
            };
        }
        let mut sums = [0_u128; 10];
        let mut columns: [Vec<u64>; 10] = std::array::from_fn(|_| Vec::with_capacity(self.len));
        let mut max_vals = [0_u64; 10];
        for sample in self.active_samples() {
            for (slot, val) in sample.as_array().into_iter().enumerate() {
                sums[slot] = sums[slot].saturating_add(u128::from(val));
                columns[slot].push(val);
                max_vals[slot] = max_vals[slot].max(val);
            }
        }
        let count = self.len as u128;
        let idx_95 = Self::percentile_index(self.len, 95, 100);
        let idx_99 = Self::percentile_index(self.len, 99, 100);
        let mut p95_vals = [0_u64; 10];
        let mut p99_vals = [0_u64; 10];
        for (slot, col) in columns.iter_mut().enumerate() {
            col.sort_unstable();
            p95_vals[slot] = col[idx_95];
            p99_vals[slot] = col[idx_99];
        }
        let mean = DomainTimingUs::default()
            .map_domains(|slot, _| u64::try_from(sums[slot] / count).unwrap_or(u64::MAX));
        FrameDomainAttribution {
            frames_recorded: self.frames_recorded,
            dominant_domain: mean.dominant_domain().to_owned(),
            last: self.last,
            mean,
            p95: DomainTimingUs::from_array(p95_vals),
            p99: DomainTimingUs::from_array(p99_vals),
            max: DomainTimingUs::from_array(max_vals),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_window_reports_idle_and_zero_percentiles() {
        let window = FrameAttributionWindow::default();
        let snap = window.snapshot();
        assert_eq!(snap.frames_recorded, 0);
        assert_eq!(snap.dominant_domain, "idle");
        assert_eq!(snap.mean, DomainTimingUs::default());
        assert_eq!(snap.p99, DomainTimingUs::default());
    }

    #[test]
    fn rolling_window_wraps_cleanly_and_computes_percentiles_and_dominant_domain() {
        let mut window = FrameAttributionWindow::default();
        for frame in 1_u64..=140 {
            window.record(DomainTimingUs {
                asset_io_poll_us: 10,
                physics_motion_us: frame * 10,
                animation_us: 50,
                triggers_behaviors_us: 25,
                audio_us: 15,
                scene_extraction_us: 30,
                ui_layout_us: 20,
                render_submit_us: 40,
                surface_acquire_us: 5,
                total_cpu_us: frame * 10 + 195,
            });
        }
        let snap = window.snapshot();
        assert_eq!(window.frames_recorded(), 140);
        assert_eq!(snap.frames_recorded, 140);
        assert_eq!(snap.dominant_domain, "physics_motion");
        // Last 120 samples are physics_motion = 210..=1400 in steps of 10.
        assert_eq!(snap.last.physics_motion_us, 1400);
        assert_eq!(snap.max.physics_motion_us, 1400);
        // 120 * 0.95 = 114th sorted item (0-indexed 113): (21 + 113) * 10 = 1340.
        assert_eq!(snap.p95.physics_motion_us, 1340);
        // ceil(120 * 0.99) = 119th item (0-indexed 118): (21 + 118) * 10 = 1390.
        assert_eq!(snap.p99.physics_motion_us, 1390);
        assert_eq!(snap.mean.physics_motion_us, 805);
        window.reset();
        assert_eq!(window.frames_recorded(), 0);
        assert_eq!(window.snapshot().dominant_domain, "idle");
    }
}
