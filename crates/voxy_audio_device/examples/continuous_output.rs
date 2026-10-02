//! Short quiet tone through offline conversion, mixer and native callback.
use voxy_audio::{Clip, Mixer};
use voxy_audio_device::{MixerPump, OutputDevice, SubmitError};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (device, sender) = OutputDevice::open(512)?;
    let samples: Vec<_> = (0..4800)
        .map(|i| {
            #[allow(clippy::cast_possible_truncation)] // Normalized quiet waveform.
            let value =
                (0.01 * (f64::from(i) * 440.0 * std::f64::consts::TAU / 48_000.0).sin()) as f32;
            [value; 2]
        })
        .collect();
    let clip =
        Clip::new(48_000, samples)?.resample_filtered(device.sample_rate(), 48_000, 3_120_000)?;
    let count = clip.frame_count();
    let mut mixer = Mixer::new(device.sample_rate(), 1, 1)?;
    let id = mixer.play(clip, 0, 1.0, false)?;
    mixer.ramp_voice_gain(id, 0.0, u32::try_from(count)?)?;
    let mut pump = MixerPump::new(mixer, 128)?;
    let mut submitted = pump.pump(&sender, 512).submitted;
    device.start()?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let mut backpressure = 0;
    while submitted < count {
        let report = pump.pump(&sender, (count - submitted).min(512));
        submitted += report.submitted;
        match report.stop {
            Some(SubmitError::Full) => backpressure += 1,
            Some(error) => return Err(format!("pump: {error:?}").into()),
            None => {}
        }
        if std::time::Instant::now() >= deadline {
            return Err("device pump timeout".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert!(backpressure > 0);
    let stats = device.stats();
    assert_eq!(stats.supplied, u64::try_from(count)?);
    assert_eq!(stats.errors, 0);
    println!(
        "CONTINUOUS OUTPUT PASS: source=48000Hz, device={}Hz, mixed_frames={}, submitted={}, errors={}",
        device.sample_rate(),
        count,
        stats.supplied,
        stats.errors
    );
    Ok(())
}
