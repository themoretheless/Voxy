//! Bounded background preparation for the full secondary-motion example.
//! The render thread never waits for a physics result or builds a model mesh.
use crate::female_demo::FemaleDemo;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use voxy_render::SceneMesh;
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Controls {
    pub view: [f32; 3],
    pub paused: bool,
    pub pressing: bool,
    pub probe: bool,
    pub skin: bool,
    pub complexion: bool,
    pub strain: bool,
    pub hair: bool,
}
impl Controls {
    pub fn capture(f: &FemaleDemo, paused: bool) -> Self {
        Self {
            view: [f.yaw, f.pitch, f.distance],
            paused,
            pressing: f.pressing,
            probe: f.probe_enabled,
            skin: f.show_skin,
            complexion: f.show_complexion,
            strain: f.show_strain,
            hair: f.show_hair,
        }
    }
}
#[derive(Debug)]
pub(crate) struct Frame {
    pub mesh: SceneMesh,
    pub body_indices: Vec<u32>,
    pub hair_indices: Vec<u32>,
    pub title: String,
    pub work_ms: f64,
    pub phase_ms: [f64; 4],
}
#[derive(Debug)]
pub(crate) struct FullModelWorker {
    controls: Arc<Mutex<Controls>>,
    stopped: Arc<AtomicBool>,
    receiver: mpsc::Receiver<Result<Frame, String>>,
    thread: Option<std::thread::JoinHandle<()>>,
    pub title: String,
}
impl FullModelWorker {
    pub fn new(mut simulation: FemaleDemo, controls: Controls) -> std::io::Result<Self> {
        let shared = Arc::new(Mutex::new(controls));
        let stopped = Arc::new(AtomicBool::new(false));
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker_controls = shared.clone();
        let stop = stopped.clone();
        let title = simulation.title();
        let thread = std::thread::Builder::new()
            .name("voxy-full-model".into())
            .spawn(move || {
                let mut last = std::time::Instant::now();
                let mut previous = None;
                while !stop.load(Ordering::Acquire) {
                    let settings = *worker_controls.lock().unwrap();
                    if settings.paused && previous == Some(settings) {
                        last = std::time::Instant::now();
                        std::thread::park_timeout(std::time::Duration::from_millis(4));
                        continue;
                    }
                    let started = std::time::Instant::now();
                    let elapsed = started.duration_since(last).as_secs_f64();
                    last = started;
                    [simulation.yaw, simulation.pitch, simulation.distance] = settings.view;
                    simulation.pressing = settings.pressing;
                    simulation.probe_enabled = settings.probe;
                    simulation.show_skin = settings.skin;
                    simulation.show_complexion = settings.complexion;
                    simulation.show_strain = settings.strain;
                    simulation.show_hair = settings.hair;
                    let advance_started = std::time::Instant::now();
                    let advance = if settings.paused {
                        Ok(())
                    } else {
                        simulation.advance(elapsed)
                    };
                    let advance_ms = advance_started.elapsed().as_secs_f64() * 1000.;
                    let mesh_started = std::time::Instant::now();
                    let result = advance
                    .map_err(str::to_owned)
                    .and_then(|()| simulation.mesh().map_err(|e| e.to_string()))
                    .and_then(|mesh| {
                        let mesh_ms = mesh_started.elapsed().as_secs_f64() * 1000.;
                        let partition_started = std::time::Instant::now();
                        let (body_indices, hair_indices) = partition_hair(&mesh)?;
                        let partition_ms = partition_started.elapsed().as_secs_f64() * 1000.;
                        let streams_started = std::time::Instant::now();
                        let mesh = mesh.with_prepared_upload_streams();
                        Ok(Frame {
                            mesh,
                            body_indices,
                            hair_indices,
                            title: simulation.title(),
                            work_ms: started.elapsed().as_secs_f64() * 1000.,
                            phase_ms: [advance_ms, mesh_ms, partition_ms, streams_started.elapsed().as_secs_f64() * 1000.],
                        })
                    });
                    previous = Some(settings);
                    let failed = result.is_err();
                    // One queued snapshot bounds memory; backpressure retains every solved state.
                    if sender.send(result).is_err() || failed {
                        break;
                    }
                }
            })?;
        Ok(Self {
            controls: shared,
            stopped,
            receiver,
            thread: Some(thread),
            title,
        })
    }
    pub fn poll(&mut self, controls: Controls) -> Result<Option<Frame>, String> {
        *self.controls.lock().unwrap() = controls;
        match self.receiver.try_recv() {
            Ok(Ok(frame)) => {
                self.title = frame.title.clone();
                Ok(Some(frame))
            }
            Ok(Err(error)) => Err(error),
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => Err("full-model worker stopped".into()),
        }
    }
}
impl Drop for FullModelWorker {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        // Release a producer blocked by its bounded output before joining it.
        let (_, empty) = mpsc::sync_channel(1);
        drop(std::mem::replace(&mut self.receiver, empty));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub(crate) fn partition_hair(mesh: &SceneMesh) -> Result<(Vec<u32>, Vec<u32>), String> {
    let mut body = Vec::new();
    let mut hair = Vec::new();
    for tri in mesh.indices().chunks_exact(3) {
        let n = tri
            .iter()
            .filter(|i| mesh.vertices()[**i as usize].uv[0] == -7.)
            .count();
        match n {
            0 => body.extend_from_slice(tri),
            3 => hair.extend_from_slice(tri),
            _ => return Err("mixed hair material triangle".into()),
        }
    }
    Ok((body, hair))
}
