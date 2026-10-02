//! Short quiet tone through offline conversion, mixer and native callback.
use voxy_audio::{Clip, Mixer};
use voxy_audio_device::OutputDevice;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (device, sender) = OutputDevice::open(48_000)?;
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
    let mut frames = vec![[0.0; 2]; count];
    mixer.render(&mut frames);
    assert!(frames.iter().flatten().any(|x| x.abs() > 0.001));
    for frame in frames {
        sender.submit(frame).map_err(|e| format!("submit: {e:?}"))?;
    }
    device.start()?;
    std::thread::sleep(std::time::Duration::from_millis(250));
    let stats = device.stats();
    assert_eq!(stats.supplied, u64::try_from(count)?);
    assert_eq!(stats.errors, 0);
    println!(
        "MIXER OUTPUT PASS: source=48000Hz, device={}Hz, mixed_frames={}, submitted={}, errors={}",
        device.sample_rate(),
        count,
        stats.supplied,
        stats.errors
    );
    Ok(())
}
