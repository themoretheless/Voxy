//! Offline proof of background WAV import and immutable playing versions.
//! Does not open an audio device or claim audible playback.
use voxy_assets::{
    AssetCatalog, AssetDemand, AssetError, AssetId, AssetStatus, ImportInputs, ImportedAsset,
};
use voxy_audio::{Clip, Mixer, decode_wav};
fn wav(sample: i16) -> Vec<u8> {
    let mut bytes = b"RIFF\x26\0\0\0WAVEfmt \x10\0\0\0".to_vec();
    bytes.extend_from_slice(&1_u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&1_u16.to_le_bytes()); // mono
    bytes.extend_from_slice(&48_000_u32.to_le_bytes());
    bytes.extend_from_slice(&96_000_u32.to_le_bytes());
    bytes.extend_from_slice(&2_u16.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(b"data\x02\0\0\0");
    bytes.extend_from_slice(&sample.to_le_bytes());
    bytes
}
fn import(bytes: Vec<u8>) -> std::thread::JoinHandle<Result<ImportedAsset<Clip>, String>> {
    std::thread::spawn(move || {
        // This provider owns immutable memory; a file provider must recheck disk content.
        let reader = |_: &AssetId, limit: usize| {
            if bytes.len() > limit {
                Err("WAV exceeds input budget".into())
            } else {
                Ok(bytes.clone())
            }
        };
        let mut inputs = ImportInputs::new(1, 1024);
        let source = inputs
            .read(AssetId("audio/ambience.wav".into()), reader)
            .map_err(|e| format!("{e:?}"))?;
        let clip = decode_wav(&source.bytes, 48_000).map_err(|e| e.to_string())?;
        inputs.finish(clip, reader).map_err(|e| format!("{e:?}"))
    })
}
fn build_key(asset: &ImportedAsset<Clip>) -> [u8; 32] {
    asset
        .inputs()
        .build_key("voxy.wav", "1", "offline-stereo", &48_000_u32.to_le_bytes())
        .unwrap()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let id = AssetId("audio/ambience.wav".into());
    let mut assets = AssetCatalog::new(4, 2)?;
    let AssetDemand::Started(ticket) = assets.demand(id.clone())? else {
        return Err("missing asset did not start an import".into());
    };
    let worker = import(wav(8192));
    // A second consumer sees pending work and must not spawn another worker.
    assert!(matches!(assets.demand(id.clone())?, AssetDemand::Pending));
    assert_eq!(assets.pending(), 1);
    assets.complete(&ticket, worker.join().map_err(|_| "worker panic")?)?;
    let AssetDemand::Ready(original) = assets.demand(id.clone())? else {
        return Err("completed demand did not return its clip".into());
    };
    assert_eq!(original.value().sample_rate(), 48_000);
    assert_eq!(original.value().frame_count(), 1);
    let mut mixer = Mixer::new(48_000, 2, 1)?;
    let playing = mixer.play(original.value().clone(), 0, 1.0, true)?;
    let obsolete = assets.request(id.clone())?;
    let late_worker = import(wav(16384));
    let broken = assets.request(id.clone())?;
    assets.complete(
        &broken,
        import(vec![0; 8]).join().map_err(|_| "worker panic")?,
    )?;
    assert!(matches!(assets.status(&id), Some(AssetStatus::Failed(_))));
    assert!(matches!(assets.demand(id.clone())?, AssetDemand::Failed(_)));
    assert_eq!(assets.pending(), 0);
    assert!(std::sync::Arc::ptr_eq(
        &original,
        &assets.snapshot(&id).unwrap()
    ));
    assert_eq!(
        assets.complete(&obsolete, late_worker.join().map_err(|_| "worker panic")?),
        Err(AssetError::StaleTicket)
    );
    let replacement = assets.request(id.clone())?;
    assets.complete(
        &replacement,
        import(wav(16384)).join().map_err(|_| "worker panic")?,
    )?;
    let mut output = [[0.0; 2]; 4];
    mixer.render(&mut output);
    assert!(
        output
            .iter()
            .flatten()
            .all(|x| (*x - 0.25).abs() < f32::EPSILON)
    );
    mixer.stop(playing)?;
    let current = assets.snapshot(&id).unwrap();
    assert_ne!(build_key(&original), build_key(&current));
    let source = AssetId("audio/ambience.wav".into());
    assert_eq!(
        &*original.inputs().observations()[&source]
            .as_ref()
            .unwrap()
            .bytes,
        &wav(8192)
    );
    assert_eq!(
        &*current.inputs().observations()[&source]
            .as_ref()
            .unwrap()
            .bytes,
        &wav(16384)
    );
    mixer.play(current.value().clone(), 0, 1.0, true)?;
    mixer.render(&mut output);
    assert!(
        output
            .iter()
            .flatten()
            .all(|x| (*x - 0.5).abs() < f32::EPSILON)
    );
    println!(
        "AUDIO ASSET PASS: deduplicated demand, background decode, failed/stale reload rejection, existing voice keeps original version, new voice uses replacement with distinct input provenance and build key"
    );
    Ok(())
}
