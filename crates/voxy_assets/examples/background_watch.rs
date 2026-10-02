//! Real-file worker proof. Only this offline harness waits for scan completion.
use std::time::{Duration, Instant};
use voxy_assets::{
    AssetId, FileInputs, ImportInputs, ImportOutcome, PollWorkerError, SourceDependencies,
    SourcePollWorker,
};
fn wait(worker: &mut SourcePollWorker) -> Result<Vec<AssetId>, Box<dyn std::error::Error>> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(changes) = worker.try_result()? {
            return Ok(changes);
        }
        if Instant::now() >= deadline {
            return Err("scan timeout".into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!(
        "voxy-background-watch-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    std::fs::create_dir(&root)?;
    let source = AssetId("include.txt".into());
    let output = AssetId("compiled".into());
    let mut inputs = ImportInputs::new(1, 16);
    let _ = inputs.read(source.clone(), |_, _| Err("missing include".into()));
    let mut index = SourceDependencies::new(1, 1);
    index.record(output.clone(), &inputs, ImportOutcome::Failed)?;
    let mut worker = SourcePollWorker::new(FileInputs::new(&root)?, 1, 16)?;
    assert!(worker.try_result()?.is_none());
    worker.request(&index, 1)?;
    assert_eq!(worker.request(&index, 1), Err(PollWorkerError::Busy));
    assert_eq!(index.affected(wait(&mut worker)?), vec![output.clone()]);
    worker.request(&index, 1)?;
    assert!(wait(&mut worker)?.is_empty());
    for bytes in [b"first", b"other"] {
        std::fs::write(root.join(&source.0), bytes)?;
        worker.request(&index, 1)?;
        assert_eq!(index.affected(wait(&mut worker)?), vec![output.clone()]);
    }
    std::fs::remove_file(root.join(&source.0))?;
    worker.request(&index, 1)?;
    assert_eq!(wait(&mut worker)?, vec![source.clone()]);
    std::fs::write(root.join(&source.0), b"back")?;
    worker.request(&index, 1)?;
    assert_eq!(wait(&mut worker)?, vec![source]);
    worker.close().join().map_err(|_| "poll worker panic")?;
    let mut too_small = SourcePollWorker::new(FileInputs::new(&root)?, 0, 16)?;
    assert_eq!(too_small.request(&index, 1), Err(PollWorkerError::Capacity));
    too_small
        .close()
        .join()
        .map_err(|_| "capacity worker panic")?;
    let mut closing = SourcePollWorker::new(FileInputs::new(&root)?, 1, 16)?;
    closing.request(&index, 1)?;
    closing
        .close()
        .join()
        .map_err(|_| "pending shutdown panic")?;
    std::fs::remove_dir_all(root)?;
    println!(
        "BACKGROUND WATCH PASS: nonblocking owner API, one outstanding scan, missing include recovery, same-length edit, deletion/recovery, joined shutdown"
    );
    Ok(())
}
