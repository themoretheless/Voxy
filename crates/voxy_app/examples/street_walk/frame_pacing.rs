//! Monitor-aware frame pacing for `street_walk`: phase-locked `Immediate` presentation
//! at `monitor_refresh_hz` by default (avoiding Metal `Fifo` VBlank quantization and DVFS
//! stalls), explicit `--present fifo` hardware VSync, and uncapped `--present immediate`.

use std::time::{Duration, Instant};
use winit::{
    event_loop::{ActiveEventLoop, ControlFlow},
    window::Window,
};

const MIN_REFRESH_HZ: f64 = 30.0;
const MAX_REFRESH_HZ: f64 = 500.0;
const DEFAULT_REFRESH_HZ: f64 = 60.0;
const WAKE_LEAD: Duration = Duration::from_micros(1500);
const POLL_LEAD: Duration = Duration::from_micros(1200);
const SLEEP_THRESHOLD: Duration = Duration::from_micros(350);
const SLEEP_STEP: Duration = Duration::from_micros(150);

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MonitorRefresh {
    pub name: String,
    pub hz: f64,
}

impl MonitorRefresh {
    pub(crate) fn detect(window: &Window, el: &ActiveEventLoop) -> Self {
        let monitor = window.current_monitor().or_else(|| el.primary_monitor());
        let Some(monitor) = monitor else {
            return Self {
                name: "Unknown".into(),
                hz: DEFAULT_REFRESH_HZ,
            };
        };
        let name = monitor.name().unwrap_or_else(|| "Unknown".into());
        let hz = resolve_refresh_hz(
            monitor.refresh_rate_millihertz(),
            monitor.video_modes().map(|m| m.refresh_rate_millihertz()),
        );
        Self { name, hz }
    }

    pub(crate) fn period(&self) -> Duration {
        Duration::from_secs_f64(1.0 / self.hz.clamp(MIN_REFRESH_HZ, MAX_REFRESH_HZ))
    }
}

pub(crate) fn resolve_refresh_hz(
    current_millihertz: Option<u32>,
    mode_millihertz: impl Iterator<Item = u32>,
) -> f64 {
    let from_current = current_millihertz.map(|mh| f64::from(mh) / 1000.0);
    let max_mode = mode_millihertz
        .map(|mh| f64::from(mh) / 1000.0)
        .filter(|hz| hz.is_finite() && *hz >= MIN_REFRESH_HZ)
        .max_by(f64::total_cmp);
    let raw = match (from_current, max_mode) {
        (Some(cur), Some(max)) if cur >= MIN_REFRESH_HZ => cur.max(max),
        (Some(cur), _) if cur >= MIN_REFRESH_HZ => cur,
        (_, Some(max)) => max,
        _ => DEFAULT_REFRESH_HZ,
    };
    raw.clamp(MIN_REFRESH_HZ, MAX_REFRESH_HZ)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PresentPolicy {
    MonitorPaced,
    HardwareVsync,
    Uncapped,
}

impl PresentPolicy {
    pub(crate) fn detect() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let present_env = std::env::var("VOXY_STREET_PRESENT").ok();
        let uncapped_env = std::env::var("VOXY_STREET_UNCAPPED").ok();
        parse_present_policy(&args, present_env.as_deref(), uncapped_env.as_deref())
    }

    pub(crate) fn select_mode(self, supported: &[wgpu::PresentMode]) -> wgpu::PresentMode {
        match self {
            Self::MonitorPaced => {
                if supported.contains(&wgpu::PresentMode::Immediate) {
                    wgpu::PresentMode::Immediate
                } else if supported.contains(&wgpu::PresentMode::AutoNoVsync) {
                    wgpu::PresentMode::AutoNoVsync
                } else {
                    supported
                        .first()
                        .copied()
                        .unwrap_or(wgpu::PresentMode::Fifo)
                }
            }
            Self::HardwareVsync => {
                if supported.contains(&wgpu::PresentMode::Fifo) {
                    wgpu::PresentMode::Fifo
                } else {
                    wgpu::PresentMode::AutoVsync
                }
            }
            Self::Uncapped => {
                if supported.contains(&wgpu::PresentMode::Immediate) {
                    wgpu::PresentMode::Immediate
                } else {
                    wgpu::PresentMode::AutoNoVsync
                }
            }
        }
    }
}

pub(crate) fn parse_present_policy(
    args: &[String],
    present_env: Option<&str>,
    uncapped_env: Option<&str>,
) -> PresentPolicy {
    let arg_mode = args
        .windows(2)
        .find_map(|w| (w[0] == "--present").then_some(w[1].as_str()))
        .or_else(|| args.iter().find_map(|a| a.strip_prefix("--present=")));
    let requested = arg_mode.or(present_env);
    if uncapped_env == Some("1") || requested.is_some_and(|v| v.eq_ignore_ascii_case("immediate")) {
        PresentPolicy::Uncapped
    } else if requested.is_some_and(|v| v.eq_ignore_ascii_case("fifo")) {
        PresentPolicy::HardwareVsync
    } else {
        PresentPolicy::MonitorPaced
    }
}

#[derive(Debug)]
pub(crate) struct FramePacer {
    policy: PresentPolicy,
    monitor: MonitorRefresh,
    last_monitor_check: Instant,
    next_deadline: Option<Instant>,
    occluded_until: Option<Instant>,
}

impl FramePacer {
    pub(crate) fn new() -> Self {
        let now = Instant::now();
        Self {
            policy: PresentPolicy::detect(),
            monitor: MonitorRefresh {
                name: "Unknown".into(),
                hz: DEFAULT_REFRESH_HZ,
            },
            last_monitor_check: now,
            next_deadline: None,
            occluded_until: None,
        }
    }

    pub(crate) const fn policy(&self) -> PresentPolicy {
        self.policy
    }

    pub(crate) fn monitor(&self) -> &MonitorRefresh {
        &self.monitor
    }

    pub(crate) fn update_monitor(&mut self, window: &Window, el: &ActiveEventLoop) {
        self.update_monitor_at(MonitorRefresh::detect(window, el), Instant::now());
    }

    pub(crate) fn maybe_refresh_monitor(
        &mut self,
        window: Option<&Window>,
        el: &ActiveEventLoop,
        now: Instant,
    ) {
        if now.duration_since(self.last_monitor_check) >= Duration::from_secs(1) {
            if let Some(window) = window {
                self.update_monitor_at(MonitorRefresh::detect(window, el), now);
            } else {
                self.last_monitor_check = now;
            }
        }
    }

    pub(crate) fn update_monitor_at(&mut self, detected: MonitorRefresh, now: Instant) {
        if detected != self.monitor {
            self.monitor = detected;
            self.next_deadline = None;
            self.occluded_until = None;
        }
        self.last_monitor_check = now;
    }

    pub(crate) fn ready_to_draw(&self, now: Instant) -> bool {
        if let Some(occ) = self.occluded_until {
            return now >= occ;
        }
        match self.next_deadline {
            Some(due) => due.saturating_duration_since(now) <= WAKE_LEAD,
            None => true,
        }
    }

    pub(crate) fn wait_for_deadline(&mut self) {
        if self.occluded_until.is_some_and(|occ| Instant::now() >= occ) {
            self.occluded_until = None;
        }
        if let Some(due) = self.next_deadline {
            while due.saturating_duration_since(Instant::now()) > SLEEP_THRESHOLD {
                std::thread::sleep(SLEEP_STEP);
            }
            while Instant::now() < due {
                std::hint::spin_loop();
            }
        }
    }

    pub(crate) fn control_flow(&self, now: Instant) -> ControlFlow {
        if let Some(occ) = self.occluded_until
            && occ > now
        {
            return ControlFlow::WaitUntil(occ);
        }
        if let Some(due) = self.next_deadline
            && due > now + WAKE_LEAD
        {
            return ControlFlow::WaitUntil(due.checked_sub(POLL_LEAD).unwrap_or(now));
        }
        ControlFlow::Poll
    }

    pub(crate) fn on_present(&mut self, frame_start: Instant, presented_at: Instant) {
        self.occluded_until = None;
        if self.policy != PresentPolicy::MonitorPaced {
            self.next_deadline = None;
            return;
        }
        let period = self.monitor.period();
        let candidate = self
            .next_deadline
            .map_or(frame_start + period, |due| due + period);
        self.next_deadline = (candidate > presented_at).then_some(candidate);
    }

    pub(crate) fn on_occluded_or_timeout(&mut self, now: Instant) {
        self.next_deadline = None;
        if self.policy != PresentPolicy::Uncapped {
            self.occluded_until = Some(now + self.monitor.period());
        } else {
            self.occluded_until = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pacer_with(policy: PresentPolicy, hz: f64) -> FramePacer {
        let mut pacer = FramePacer::new();
        pacer.policy = policy;
        pacer.update_monitor_at(
            MonitorRefresh {
                name: "Color LCD".into(),
                hz,
            },
            Instant::now(),
        );
        pacer
    }

    #[test]
    fn resolve_refresh_hz_handles_missing_fractional_and_extreme_rates() {
        assert_eq!(resolve_refresh_hz(None, std::iter::empty()), 60.0);
        assert!((resolve_refresh_hz(Some(120_000), std::iter::empty()) - 120.0).abs() < 1e-6);
        assert!((resolve_refresh_hz(Some(60_000), [120_000].into_iter()) - 120.0).abs() < 1e-6);
        assert!((resolve_refresh_hz(Some(59_940), std::iter::empty()) - 59.94).abs() < 1e-6);
        assert_eq!(resolve_refresh_hz(Some(1_000), std::iter::empty()), 60.0);
        assert_eq!(resolve_refresh_hz(Some(30_000), std::iter::empty()), 30.0);
        assert_eq!(resolve_refresh_hz(Some(999_000), std::iter::empty()), 500.0);
    }

    #[test]
    fn present_policy_supports_monitor_paced_hardware_vsync_and_uncapped() {
        assert_eq!(
            parse_present_policy(&["street_walk".into()], None, None),
            PresentPolicy::MonitorPaced
        );
        assert_eq!(
            parse_present_policy(&["street_walk".into(), "--present=fifo".into()], None, None),
            PresentPolicy::HardwareVsync
        );
        assert_eq!(
            parse_present_policy(&["street_walk".into()], Some("fifo"), None),
            PresentPolicy::HardwareVsync
        );
        assert_eq!(
            parse_present_policy(
                &["street_walk".into(), "--present".into(), "immediate".into()],
                None,
                None
            ),
            PresentPolicy::Uncapped
        );
        assert_eq!(
            parse_present_policy(&["street_walk".into()], Some("immediate"), None),
            PresentPolicy::Uncapped
        );
        assert_eq!(
            parse_present_policy(&["street_walk".into()], None, Some("1")),
            PresentPolicy::Uncapped
        );

        let caps = [wgpu::PresentMode::Fifo, wgpu::PresentMode::Immediate];
        assert_eq!(
            PresentPolicy::MonitorPaced.select_mode(&caps),
            wgpu::PresentMode::Immediate
        );
        assert_eq!(
            PresentPolicy::HardwareVsync.select_mode(&caps),
            wgpu::PresentMode::Fifo
        );
        assert_eq!(
            PresentPolicy::Uncapped.select_mode(&caps),
            wgpu::PresentMode::Immediate
        );
    }

    #[test]
    fn monitor_paced_phase_locks_without_catchup_burst_and_sleeps_when_occluded() {
        let mut pacer = pacer_with(PresentPolicy::MonitorPaced, 120.0);
        let period = pacer.monitor().period();
        let t0 = Instant::now();

        // Fast frame: sets phase-locked deadline at t0 + period.
        pacer.on_present(t0, t0 + Duration::from_millis(4));
        assert!(!pacer.ready_to_draw(t0 + Duration::from_millis(5)));
        assert!(pacer.ready_to_draw((t0 + period).checked_sub(Duration::from_millis(1)).unwrap()));
        assert!(matches!(
            pacer.control_flow(t0 + Duration::from_millis(4)),
            ControlFlow::WaitUntil(_)
        ));

        // Second fast frame advances phase from previous deadline: t0 + 2 * period.
        let t1 = t0 + period;
        pacer.on_present(t1, t1 + Duration::from_millis(4));
        assert!(!pacer.ready_to_draw(t1 + Duration::from_millis(5)));
        assert!(pacer.ready_to_draw(t1 + period));

        // Slow frame (> period): resets deadline so there is zero catch-up burst.
        let t2 = t1 + period;
        pacer.on_present(t2, t2 + period + Duration::from_millis(2));
        assert!(pacer.ready_to_draw(t2 + period + Duration::from_millis(2)));
        assert_eq!(
            pacer.control_flow(t2 + period + Duration::from_millis(2)),
            ControlFlow::Poll
        );

        // Occluded swapchain sleeps for a full refresh period.
        pacer.on_occluded_or_timeout(t2);
        assert!(!pacer.ready_to_draw(t2 + Duration::from_millis(1)));
        assert!(pacer.ready_to_draw(t2 + period));
    }
}
