//! Native device proof for the saved-scene audio owner bridge.
use voxy_audio::Clip;
use voxy_audio_device::{OutputDeviceWorker, PcmPump};
use voxy_gameplay::{AudioSource, SceneAudioRuntime, extract_scene_audio};
use voxy_scene::{SceneGraph, Transform};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut worker = OutputDeviceWorker::spawn(48_000)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_mins(3);
    let connection = loop {
        if let Some(connection) = worker.poll()? {
            break connection;
        }
        if std::time::Instant::now() >= deadline {
            return Err("native audio initialization deadline exceeded".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    let rate = connection.sample_rate();
    let count = usize::try_from(rate / 10)?;
    let mut scene = SceneGraph::new(1);
    let source = scene.spawn(None, Transform::default())?;
    scene.insert_component(
        source,
        AudioSource {
            import_settings: None,
            asset: "native-tone".into(),
            bus: 0,
            gain: 0.01,
            looping: false,
            spatial: false,
            near: 1.,
            far: 5.,
        },
    )?;
    let frames = (0..count)
        .map(|index| {
            #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
            let sample =
                (index as f64 * 440. * std::f64::consts::TAU / f64::from(rate)).sin() as f32;
            [sample; 2]
        })
        .collect();
    let clip = Clip::new(rate, frames)?;
    let mut runtime = SceneAudioRuntime::new(rate, 1, 1)?;
    runtime.synchronize(&extract_scene_audio(&scene, 1)?, |_| Ok(clip.clone()))?;
    let mut pump = PcmPump::new(count)?;
    let report = pump.pump(connection.sender(), count, |output| runtime.render(output));
    if report.submitted != count || report.stop.is_some() {
        return Err("scene audio output queue failed".into());
    }
    std::thread::sleep(std::time::Duration::from_millis(250));
    let stats = connection.stats();
    if stats.supplied != u64::try_from(count)? || stats.errors != 0 {
        return Err(format!("scene output stats: {stats:?}").into());
    }
    scene.remove_subtree(source)?;
    runtime.synchronize(&extract_scene_audio(&scene, 1)?, |_| unreachable!())?;
    runtime.stop();
    worker.join()?;
    println!(
        "SCENE DEVICE PASS rate={rate} submitted={} supplied={} errors={}",
        report.submitted, stats.supplied, stats.errors
    );
    Ok(())
}
