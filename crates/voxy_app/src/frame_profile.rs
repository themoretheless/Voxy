//! Opt-in wall-clock frame measurements shared by the native clients.
//! Empty CSV fields denote unavailable measurements, never zero GPU cost.
use std::{
    fs::File,
    io::{self, Write},
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scenario {
    Idle,
    Motion,
    Water,
    Street,
}
impl Scenario {
    pub fn name(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Motion => "motion",
            Self::Water => "water",
            Self::Street => "street",
        }
    }
}
#[derive(Debug)]
pub struct Profile {
    pub scenario: Scenario,
    output: PathBuf,
    warmup: Duration,
    duration: Duration,
    start: Option<Instant>,
    previous: Option<Instant>,
    samples: Vec<Sample>,
    gpu_samples: Vec<[Option<f64>; 3]>,
    completed: std::cell::Cell<bool>,
    cpu_stages: std::collections::BTreeMap<&'static str, Vec<f64>>,
}
#[derive(Debug)]
struct Sample {
    frame_ms: f64,
    simulation_ms: f64,
    preparation_ms: f64,
    render_ms: f64,
    acquire_ms: Option<f64>,
    presented: bool,
}
impl Profile {
    pub fn parse(
        args: impl IntoIterator<Item = String>,
        street: bool,
    ) -> Result<Option<Self>, String> {
        let mut args = args.into_iter();
        let mut output = None;
        let mut scenario = if street {
            Scenario::Street
        } else {
            Scenario::Idle
        };
        let mut warmup = 10.0;
        let mut duration = 60.0;
        let mut supplied = false;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--profile" => {
                    if output.is_some() {
                        return Err("duplicate --profile".into());
                    }
                    output = Some(PathBuf::from(
                        args.next().ok_or("--profile requires a CSV path")?,
                    ));
                }
                "--profile-scenario" => {
                    supplied = true;
                    scenario = match args.next().as_deref() {
                        Some("idle") => Scenario::Idle,
                        Some("motion") => Scenario::Motion,
                        Some("water") => Scenario::Water,
                        Some("street") => Scenario::Street,
                        _ => {
                            return Err(
                                "--profile-scenario expects idle|motion|water|street".into()
                            );
                        }
                    };
                }
                "--profile-warmup" | "--profile-seconds" => {
                    supplied = true;
                    let value: f64 = args
                        .next()
                        .ok_or("profile duration requires seconds")?
                        .parse()
                        .map_err(|_| "invalid profile duration")?;
                    if !value.is_finite() || value <= 0.0 || value > 3600.0 {
                        return Err("profile duration must be in (0, 3600] seconds".into());
                    }
                    if arg == "--profile-warmup" {
                        warmup = value;
                    } else {
                        duration = value;
                    }
                }
                _ => {}
            }
        }
        if supplied && output.is_none() {
            return Err("profile options require --profile PATH".into());
        }
        let Some(output) = output else {
            return Ok(None);
        };
        if !output
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("csv"))
        {
            return Err("--profile output must have a .csv extension".into());
        }
        if street != (scenario == Scenario::Street) {
            return Err(
                "street scenario requires street_walk; idle/motion/water require voxy_app".into(),
            );
        }
        Ok(Some(Self {
            scenario,
            output,
            warmup: Duration::from_secs_f64(warmup),
            duration: Duration::from_secs_f64(duration),
            start: None,
            previous: None,
            samples: Vec::new(),
            gpu_samples: Vec::new(),
            completed: std::cell::Cell::new(false),
            cpu_stages: std::collections::BTreeMap::new(),
        }))
    }
    /// Call once per attempted frame, including surface skips. Returns true when done.
    pub fn record(
        &mut self,
        now: Instant,
        simulation: Duration,
        preparation: Duration,
        render: Duration,
        acquire: Option<Duration>,
        presented: bool,
    ) -> bool {
        let start = *self.start.get_or_insert(now);
        let previous = self.previous.replace(now);
        // Exclude the interval crossing the warmup boundary.
        if let Some(previous) = previous.filter(|p| p.duration_since(start) >= self.warmup) {
            self.samples.push(Sample {
                frame_ms: now.duration_since(previous).as_secs_f64() * 1000.0,
                simulation_ms: simulation.as_secs_f64() * 1000.0,
                preparation_ms: preparation.as_secs_f64() * 1000.0,
                render_ms: render.as_secs_f64() * 1000.0,
                acquire_ms: acquire.map(|d| d.as_secs_f64() * 1000.0),
                presented,
            });
        }
        now.duration_since(start) >= self.warmup + self.duration
    }
    /// Delayed GPU samples are separate observations, never attributed to the CPU frame collecting them.
    pub fn gpu_sample(&mut self, now: Instant, sample: [Option<f64>; 3]) {
        if self
            .start
            .is_some_and(|start| now.duration_since(start) >= self.warmup)
            && sample.iter().flatten().all(|v| v.is_finite() && *v >= 0.0)
        {
            self.gpu_samples.push(sample);
        }
    }
    pub fn cpu_stage(&mut self, now: Instant, name: &'static str, duration: Duration) {
        if self
            .start
            .is_some_and(|start| now.duration_since(start) >= self.warmup)
        {
            self.cpu_stages
                .entry(name)
                .or_default()
                .push(duration.as_secs_f64() * 1000.0);
        }
    }
    pub fn is_complete(&self) -> bool {
        self.completed.get()
    }
    pub fn finish(&self, adapter: &str) -> io::Result<()> {
        if self.samples.is_empty() {
            return Err(io::Error::other("profile has no measured frames"));
        }
        if let Some(parent) = self.output.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let mut csv = io::BufWriter::new(File::create(&self.output)?);
        writeln!(
            csv,
            "frame,frame_ms,simulation_ms,preparation_ms,render_cpu_ms,acquire_ms,presented,gpu_ms,upload_bytes,peak_memory_bytes"
        )?;
        for (i, s) in self.samples.iter().enumerate() {
            writeln!(
                csv,
                "{i},{:.6},{:.6},{:.6},{:.6},{},{},,,",
                s.frame_ms,
                s.simulation_ms,
                s.preparation_ms,
                s.render_ms,
                s.acquire_ms.map_or_else(String::new, |v| format!("{v:.6}")),
                s.presented
            )?;
        }
        csv.flush()?;
        let mut times: Vec<_> = self.samples.iter().map(|s| s.frame_ms).collect();
        times.sort_by(f64::total_cmp);
        let p99 = percentile(&times, 0.99);
        let mut gpu_csv = io::BufWriter::new(File::create(self.output.with_extension("gpu.csv"))?);
        writeln!(
            gpu_csv,
            "sample,scene_or_primary_pass_ms,opaque_ms,transparent_ms"
        )?;
        for (i, s) in self.gpu_samples.iter().enumerate() {
            writeln!(
                gpu_csv,
                "{i},{},{},{}",
                s[0].map_or_else(String::new, |v| v.to_string()),
                s[1].map_or_else(String::new, |v| v.to_string()),
                s[2].map_or_else(String::new, |v| v.to_string())
            )?;
        }
        gpu_csv.flush()?;
        let gpu_stats: Vec<_> = (0..3).map(|i| { let mut values:Vec<_>=self.gpu_samples.iter().filter_map(|s|s[i]).collect(); values.sort_by(f64::total_cmp); if values.is_empty() { serde_json::Value::Null } else {serde_json::json!({"p50_ms":percentile(&values,0.5),"p95_ms":percentile(&values,0.95),"p99_ms":percentile(&values,0.99)})} }).collect();
        let cpu_stats: std::collections::BTreeMap<_,_> = self.cpu_stages.iter().map(|(name, samples)| {
            let mut values = samples.clone();
            values.sort_by(f64::total_cmp);
            (*name, serde_json::json!({"p50_ms":percentile(&values,0.5),"p95_ms":percentile(&values,0.95),"p99_ms":percentile(&values,0.99)}))
        }).collect();
        let report = serde_json::json!({"scenario":self.scenario.name(), "adapter":adapter,"frames":times.len(),"warmup_seconds":self.warmup.as_secs_f64(),"measurement_seconds":self.duration.as_secs_f64(),"p50_ms":percentile(&times,0.5),"p95_ms":percentile(&times,0.95),"p99_ms":p99,"max_ms":times.last(),"over_120hz":times.iter().filter(|t|**t>1000.0/120.0).count(),"over_60hz":times.iter().filter(|t|**t>1000.0/60.0).count(),"skipped_frames":self.samples.iter().filter(|s|!s.presented).count(),"frame_budget_met":p99<=1000.0/120.0 && self.samples.iter().all(|s|s.presented),"gpu_scope":if self.scenario == Scenario::Street {"archive scene span and opaque/transparent passes"} else {"primary resident voxel pass only"},"cpu_stages":cpu_stats,"gpu_samples":self.gpu_samples.len(),"gpu_pass_statistics":gpu_stats,"gpu_ms":null,"upload_bytes":null,"peak_memory_bytes":null,"limitations":["CPU render includes submission/presentation; not GPU execution time","presentation completion is not display scanout","GPU scene span excludes external postprocessing; delayed samples may cross warmup boundary","upload/memory counters unavailable"]});
        std::fs::write(
            self.output.with_extension("json"),
            serde_json::to_vec_pretty(&report)?,
        )?;
        self.completed.set(true);
        println!("FRAME PROFILE {report}");
        Ok(())
    }
}
fn percentile(sorted: &[f64], fraction: f64) -> f64 {
    sorted[((sorted.len() as f64 * fraction).ceil() as usize)
        .saturating_sub(1)
        .min(sorted.len() - 1)]
}
#[cfg(test)]
mod tests {
    use super::*;
    fn parse(args: &[&str]) -> Result<Option<Profile>, String> {
        Profile::parse(args.iter().map(ToString::to_string), false)
    }
    #[test]
    fn rejects_invalid_config() {
        for args in [
            vec!["--profile"],
            vec!["--profile", "report.json"],
            vec!["--profile", "out.csv", "--profile-seconds", "NaN"],
            vec!["--profile", "out.csv", "--profile-warmup", "0"],
            vec!["--profile-scenario", "motion"],
            vec!["--profile", "out.csv", "--profile-scenario", "street"],
        ] {
            assert!(parse(&args).is_err());
        }
        assert!(parse(&[]).unwrap().is_none());
    }
    #[test]
    fn excludes_warmup_and_counts_skips() {
        let mut p = parse(&[
            "--profile",
            "out.csv",
            "--profile-warmup",
            "1",
            "--profile-seconds",
            "1",
        ])
        .unwrap()
        .unwrap();
        let start = Instant::now();
        for ms in [0, 900, 1100, 1200, 2000] {
            let done = p.record(
                start + Duration::from_millis(ms),
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                None,
                false,
            );
            assert_eq!(done, ms == 2000);
        }
        assert_eq!(p.samples.len(), 2);
        assert_eq!(p.samples[0].frame_ms, 100.0);
        assert_eq!(percentile(&[1., 2., 3., 4.], 0.99), 4.);
    }
    #[test]
    fn report_preserves_missing_metrics_and_incomplete_state() {
        let directory = std::env::temp_dir().join(format!(
            "voxy-profile-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let output = directory.join("frames.csv");
        let mut p = Profile::parse(
            [
                "--profile".to_owned(),
                output.to_string_lossy().into_owned(),
                "--profile-warmup".to_owned(),
                "1".to_owned(),
                "--profile-seconds".to_owned(),
                "1".to_owned(),
            ],
            false,
        )
        .unwrap()
        .unwrap();
        assert!(!p.is_complete());
        assert!(p.finish("test").is_err());
        let start = Instant::now();
        for ms in [0, 1100, 1200, 2000] {
            p.record(
                start + Duration::from_millis(ms),
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                None,
                false,
            );
        }
        p.gpu_sample(start + Duration::from_secs(2), [Some(1.0), None, None]);
        p.finish("test").unwrap();
        assert!(p.is_complete());
        let report: serde_json::Value =
            serde_json::from_slice(&std::fs::read(output.with_extension("json")).unwrap()).unwrap();
        assert_eq!(report["skipped_frames"], 2);
        assert!(report["gpu_ms"].is_null());
        assert!(report["gpu_pass_statistics"][1].is_null());
        assert_eq!(report["frame_budget_met"], false);
        assert!(
            std::fs::read_to_string(output)
                .unwrap()
                .lines()
                .skip(1)
                .all(|line| line.ends_with(",,,"))
        );
        std::fs::remove_dir_all(directory).unwrap();
    }
}
