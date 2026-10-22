//! Optional clocks for CPU pose diagnostics. Normal admission uses const PROFILE=false.
use std::time::Instant;

/// CPU clocks sum actual scheduled execution; worker CPU can exceed elapsed time.
/// Join wall time includes waiting and scheduling, not just synchronization overhead.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Default)]
pub struct RenderPoseProfile {
    pub cpu_clock_available: bool,
    pub groups: usize,
    pub workers: usize,
    pub fallback: bool,
    pub total_wall_us: f64,
    pub caller_cpu_us: f64,
    pub process_cpu_us: f64,
    pub spawn_wall_us: f64,
    pub spawn_cpu_us: f64,
    pub main_work_wall_us: f64,
    pub main_work_cpu_us: f64,
    pub worker_work_wall_sum_us: f64,
    pub worker_work_wall_max_us: f64,
    pub worker_cpu_sum_us: f64,
    pub worker_cpu_max_us: f64,
    pub launch_delay_max_us: f64,
    pub join_wall_us: f64,
    pub join_cpu_us: f64,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Clock {
    wall: Instant,
    cpu: Option<f64>,
}
impl Clock {
    pub fn start() -> Self {
        Self {
            wall: Instant::now(),
            cpu: cpu_us(false),
        }
    }
    pub fn elapsed(self) -> (f64, f64) {
        let cpu = cpu_us(false)
            .zip(self.cpu)
            .map_or(f64::NAN, |(end, start)| (end - start).max(0.));
        (self.wall.elapsed().as_secs_f64() * 1e6, cpu)
    }
}

// The only FFI boundary: libc initializes a valid, writable timespec. A failed or
// unsupported clock is reported explicitly; elapsed time is never used as CPU time.
#[cfg(unix)]
#[allow(unsafe_code)]
pub(crate) fn cpu_us(process: bool) -> Option<f64> {
    let mut value = std::mem::MaybeUninit::<libc::timespec>::uninit();
    let clock = if process {
        libc::CLOCK_PROCESS_CPUTIME_ID
    } else {
        libc::CLOCK_THREAD_CPUTIME_ID
    };
    // SAFETY: clock_gettime writes one timespec through this aligned live pointer;
    // the value is read only when the call reports successful initialization.
    if unsafe { libc::clock_gettime(clock, value.as_mut_ptr()) } != 0 {
        return None;
    }
    // SAFETY: successful clock_gettime initialized every timespec field.
    let value = unsafe { value.assume_init() };
    Some(value.tv_sec as f64 * 1e6 + value.tv_nsec as f64 / 1e3)
}
#[cfg(not(unix))]
pub(crate) fn cpu_us(_: bool) -> Option<f64> {
    None
}

impl RenderPoseProfile {
    pub(crate) fn work(&mut self, wall: f64, cpu: f64, worker: bool) {
        if worker {
            self.worker_work_wall_sum_us += wall;
            self.worker_work_wall_max_us = self.worker_work_wall_max_us.max(wall);
            self.worker_cpu_sum_us += cpu;
            self.worker_cpu_max_us = self.worker_cpu_max_us.max(cpu);
        } else {
            self.main_work_wall_us += wall;
            self.main_work_cpu_us += cpu;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    #[test]
    fn thread_clock_excludes_sleep() {
        let clock = Clock::start();
        assert!(cpu_us(false).is_some() && cpu_us(true).is_some());
        std::thread::sleep(std::time::Duration::from_millis(20));
        let (wall, cpu) = clock.elapsed();
        assert!(wall >= 15_000.);
        assert!(wall > cpu + 10_000.);
    }
}
