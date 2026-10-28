use super::*;
use voxy_assets::{AssetId, FileInputs, ImportInputs, ImportOutcome};

struct Temp(std::path::PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "voxy-ui-reload-barrier-{}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn until<T>(mut f: impl FnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(value) = f() {
            return value;
        }
        assert!(Instant::now() < deadline, "UI barrier timed out");
        std::thread::yield_now();
    }
}
fn freeze(ui: &mut UiLive) -> UiReloadCheckpoint {
    until(|| {
        ui.poll_sources().unwrap();
        ui.advance_checkpoint().unwrap()
    })
}
fn watching(root: &std::path::Path) -> UiLive {
    let provider = FileInputs::new(root).unwrap();
    let mut success = ImportInputs::new(2, 100);
    success
        .read(AssetId("font.ttf".into()), |id, max| provider.read(id, max))
        .unwrap();
    let mut failure = ImportInputs::new(2, 100);
    assert!(
        failure
            .read(AssetId("missing.ttf".into()), |id, max| provider
                .read(id, max))
            .is_err()
    );
    let mut sources = voxy_assets::SourceDependencies::new(1, 32);
    sources
        .record(
            AssetId("game-ui-text".into()),
            &success,
            ImportOutcome::Published,
        )
        .unwrap();
    sources
        .record(
            AssetId("game-ui-text".into()),
            &failure,
            ImportOutcome::Failed,
        )
        .unwrap();
    let mut observations = success.observations().clone();
    observations.extend(failure.observations().clone());
    let watcher = voxy_assets::SourcePollWorker::new_with_observations(
        provider,
        32,
        16 * 1024 * 1024,
        &observations,
    )
    .unwrap();
    UiLive {
        attempted: Some(SceneUiSnapshot {
            viewport: [128.; 2],
            elements: Vec::new(),
        }),
        sources: Some(sources),
        watcher: Some(watcher),
        deferred: Some((3, 1000, 20, 2000)),
        ..UiLive::default()
    }
}
fn start_scan(ui: &mut UiLive) {
    assert!(!ui.scanning);
    ui.watcher
        .as_mut()
        .unwrap()
        .request(ui.sources.as_ref().unwrap(), 32)
        .unwrap();
    ui.scanning = true;
    ui.last_scan = Some(Instant::now());
}
fn close(ui: &mut UiLive) {
    for handle in ui.close() {
        handle.join().unwrap();
    }
}

#[test]
fn scan_is_drained_failed_dependencies_are_saved_and_restored_baseline_observes_later_changes() {
    let temp = Temp::new();
    std::fs::write(temp.0.join("font.ttf"), b"first").unwrap();
    let mut original = watching(&temp.0);
    let source_before = original.sources.as_ref().unwrap().checkpoint();
    std::fs::write(temp.0.join("font.ttf"), b"second").unwrap();
    std::fs::write(temp.0.join("missing.ttf"), b"recovered").unwrap();
    start_scan(&mut original);
    original.begin_checkpoint().unwrap();
    assert!(original.begin_checkpoint().is_err());
    let saved = freeze(&mut original);
    assert!(!original.scanning);
    assert!(original.worker.is_none());
    assert!(original.retired.is_empty());
    assert!(original.attempted.is_none());
    assert!(original.deferred.is_none());
    assert_eq!(saved.sources.as_ref().unwrap().0, source_before);
    let json = saved.to_json(100000).unwrap();
    let saved = UiReloadCheckpoint::from_json(&json, 100000).unwrap();
    assert_eq!(saved.to_json(100000).unwrap(), json);
    assert_eq!(freeze(&mut original).to_json(100000).unwrap(), json);
    // No new scan while held, even if a file changes again.
    std::fs::write(temp.0.join("font.ttf"), b"third").unwrap();
    for _ in 0..20 {
        original.poll_sources().unwrap();
    }
    assert!(!original.scanning);
    assert_eq!(
        original
            .advance_checkpoint()
            .unwrap()
            .unwrap()
            .to_json(100000)
            .unwrap(),
        json
    );
    let project = AuthoringProject::new(&temp.0, &crate::InputRecipe::Direct).unwrap();
    let PreparedUiReload {
        sources,
        watcher,
        last_scan,
        deferred,
    } = saved.prepare(&project, (19, 2000, 40, 4000)).unwrap();
    let mut restored = UiLive {
        deferred,
        sources,
        watcher,
        last_scan,
        ..UiLive::default()
    };
    assert!(original.resume_after_checkpoint().unwrap());
    for ui in [&mut original, &mut restored] {
        ui.attempted = Some(SceneUiSnapshot {
            viewport: [128.; 2],
            elements: Vec::new(),
        });
        start_scan(ui);
        until(|| {
            ui.poll_sources().unwrap();
            (!ui.scanning).then_some(())
        });
        assert!(
            ui.attempted.is_none(),
            "restored worker must not silently rebaseline changed files"
        );
        ui.begin_checkpoint().unwrap();
    }
    let a = freeze(&mut original);
    let b = freeze(&mut restored);
    assert_eq!(a.sources.as_ref().unwrap().0, b.sources.as_ref().unwrap().0);
    assert_eq!(a.watcher.as_ref().unwrap().0, b.watcher.as_ref().unwrap().0);
    assert_eq!(a.deferred, b.deferred);
    close(&mut original);
    close(&mut restored);
}

#[test]
fn cancellation_consumes_the_pending_watcher_fence_and_reopens_dispatch() {
    let temp = Temp::new();
    std::fs::write(temp.0.join("font.ttf"), b"first").unwrap();
    let mut ui = watching(&temp.0);
    ui.begin_checkpoint().unwrap();
    // Enter the real requested state without racing advance_checkpoint's eager
    // receive against the OS scheduler. Cancellation must consume this response.
    ui.watcher.as_mut().unwrap().request_checkpoint().unwrap();
    ui.checkpoint = Some(Barrier::Requested);
    until(|| ui.resume_after_checkpoint().unwrap().then_some(()));
    assert!(ui.checkpoint.is_none());
    start_scan(&mut ui); // Must not be Busy due to an orphan checkpoint request.
    until(|| {
        ui.poll_sources().unwrap();
        (!ui.scanning).then_some(())
    });
    ui.begin_checkpoint().unwrap();
    freeze(&mut ui);
    close(&mut ui);
    assert!(ui.checkpoint.is_none());
}

#[test]
fn a_retired_worker_is_owned_until_finished_and_empty_state_roundtrips() {
    let (tx, rx) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        rx.recv().unwrap();
    });
    let mut ui = UiLive {
        retired: vec![worker],
        ..UiLive::default()
    };
    assert!(ui.advance_checkpoint().is_err());
    ui.begin_checkpoint().unwrap();
    assert!(ui.advance_checkpoint().unwrap().is_none());
    assert_eq!(ui.retired.len(), 1);
    tx.send(()).unwrap();
    let saved = freeze(&mut ui);
    assert!(ui.retired.is_empty());
    assert!(saved.sources.is_none() && saved.watcher.is_none());
    assert!(saved.delay.is_none() && saved.deferred.is_none());
    let prepared = saved
        .prepare_using((1, 2, 3, 4), |_| panic!("no watcher for empty state"))
        .unwrap();
    assert!(prepared.watcher.is_none() && prepared.sources.is_none());
    assert!(prepared.deferred.is_none() && prepared.last_scan.is_none());
    assert!(prepared.close().is_none());
    assert!(ui.resume_after_checkpoint().unwrap());
    close(&mut ui);
}

#[test]
fn invalid_records_reject_before_worker_creation_and_deferred_retry_rebinds_explicitly() {
    let temp = Temp::new();
    std::fs::write(temp.0.join("font.ttf"), b"first").unwrap();
    let mut ui = watching(&temp.0);
    ui.last_scan = Some(Instant::now());
    ui.begin_checkpoint().unwrap();
    let saved = freeze(&mut ui);
    assert_eq!(saved.deferred, Some([3, 1000, 20, 2000]));
    assert!(saved.delay.is_some());
    let json = saved.to_json(100000).unwrap();
    assert!(saved.to_json(json.len() - 1).is_err());
    assert!(UiReloadCheckpoint::from_json(&json, json.len() - 1).is_err());
    for case in 0..7 {
        let mut bad = saved.clone();
        match case {
            0 => bad.version = 2,
            1 => bad.delay = Some(PERIOD + Duration::from_nanos(1)),
            2 => bad.sources = None,
            3 => bad.sources.as_mut().unwrap().0.outputs[0].output = AssetId("other".into()),
            4 => bad.watcher.as_mut().unwrap().0.max_sources = 33,
            5 => bad.watcher.as_mut().unwrap().0.cursor = usize::MAX,
            6 => bad.sources.as_mut().unwrap().0.max_edges = 33,
            _ => unreachable!(),
        }
        let called = std::cell::Cell::new(false);
        assert!(
            bad.prepare_using((99, 8, 7, 6), |_| {
                called.set(true);
                Err("factory must not run".into())
            })
            .is_err(),
            "case {case}"
        );
        assert!(!called.get(), "case {case}");
    }
    let project = AuthoringProject::new(&temp.0, &crate::InputRecipe::Direct).unwrap();
    let prepared = saved.prepare(&project, (99, 8, 7, 6)).unwrap();
    assert_eq!(prepared.deferred, Some((99, 8, 7, 6)));
    assert!(prepared.last_scan.is_some());
    assert_eq!(saved.deferred, Some([3, 1000, 20, 2000]));
    prepared.close().unwrap().join().unwrap();
    assert_eq!(ui.deferred, Some((3, 1000, 20, 2000)));
    assert!(ui.resume_after_checkpoint().unwrap());
    close(&mut ui);
}
