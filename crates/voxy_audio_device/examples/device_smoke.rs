//! Native callback smoke with silence; no audible waveform assertion.
use voxy_audio_device::OutputDevice;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (device, sender) = OutputDevice::open(1024)?;
    for _ in 0..1024 {
        sender
            .submit([0.0; 2])
            .map_err(|e| format!("submit: {e:?}"))?;
    }
    device.start()?;
    std::thread::sleep(std::time::Duration::from_millis(200));
    let stats = device.stats();
    assert_eq!(stats.supplied, 1024);
    assert_eq!(stats.errors, 0);
    assert!(stats.missing > 0);
    println!(
        "DEVICE PASS: rate={}, supplied={}, silence_frames={}, errors={}",
        device.sample_rate(),
        stats.supplied,
        stats.missing,
        stats.errors
    );
    Ok(())
}
