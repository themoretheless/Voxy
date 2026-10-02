//! Presentation receipts are emitted by the native renderer after Presented, never by MCP writes.
use serde_json::{Value, json};
use std::path::Path;
pub fn receipt_path(parameters: &Path) -> std::path::PathBuf {
    parameters.with_extension("presented.json")
}
fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0., |d| d.as_secs_f64())
}
pub fn record(
    parameters: &Path,
    applied: &Value,
    simulation_time: f64,
    frame: u64,
    size: [u32; 2],
    measurements: &Value,
) -> Result<(), Box<dyn std::error::Error>> {
    if !applied.is_object()
        || !measurements.is_object()
        || !simulation_time.is_finite()
        || simulation_time < 0.
        || size.contains(&0)
    {
        return Err("Invalid presentation data".into());
    }
    let path = receipt_path(parameters);
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    let value = json!({"parameters":applied,"measurements":measurements,"presentedAtUnixSeconds":now(),"frame":frame,"simulationTime":simulation_time,"size":size,"viewerPid":std::process::id(),"rendererOutcome":"Presented"});
    std::fs::write(&temporary, serde_json::to_vec(&value)?)?;
    std::fs::rename(temporary, path)?;
    Ok(())
}
/// Fresh receipts certify render presentation submission, not monitor scanout or a screenshot.
pub fn status(parameters: &Path, current: &Value) -> Value {
    let receipt = std::fs::read(receipt_path(parameters))
        .ok()
        .and_then(|data| serde_json::from_slice::<Value>(&data).ok());
    let Some(receipt) = receipt else {
        return json!({"liveApplied":null,"reason":"No presentation receipt"});
    };
    let valid = receipt["rendererOutcome"] == "Presented"
        && receipt["frame"].is_u64()
        && receipt["parameters"].is_object()
        && receipt["measurements"].is_object()
        && receipt["viewerPid"].as_u64().is_some_and(|pid| pid > 0)
        && receipt["simulationTime"]
            .as_f64()
            .is_some_and(|t| t.is_finite() && t >= 0.)
        && receipt["size"].as_array().is_some_and(|size| {
            size.len() == 2
                && size.iter().all(|n| {
                    n.as_u64()
                        .is_some_and(|n| n > 0 && n <= u64::from(u32::MAX))
                })
        })
        && receipt["presentedAtUnixSeconds"]
            .as_f64()
            .is_some_and(|t| t.is_finite() && t > 0.);
    if !valid {
        return json!({"liveApplied":null,"reason":"Invalid presentation receipt"});
    }
    let age = now() - receipt["presentedAtUnixSeconds"].as_f64().unwrap();
    let fresh = (0. ..=5.).contains(&age);
    json!({"liveApplied":if fresh {Value::Bool(receipt["parameters"]==*current)}else{Value::Null},"fresh":fresh,"ageSeconds":age,"receipt":receipt,
        "reason":if !fresh {"Receipt stale; viewer may be paused, minimized or stopped"}else{"Fresh renderer presentation receipt; screenshot not captured"}})
}

fn film_request_path(parameters: &Path) -> std::path::PathBuf {
    parameters.with_extension("film-state-request.json")
}
fn film_state_path(parameters: &Path, id: &str) -> Result<std::path::PathBuf, &'static str> {
    if id.is_empty()
        || id.len() > 128
        || !id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
    {
        return Err("invalid film snapshot request id");
    }
    Ok(parameters.with_extension(format!("film-state-{id}.json")))
}
fn atomic_json(path: &Path, value: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    let result = (|| {
        std::fs::write(&temporary, serde_json::to_vec(value)?)?;
        std::fs::rename(&temporary, path)?;
        Ok::<_, Box<dyn std::error::Error>>(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result
}
/// Request one numerical checkpoint from the next acknowledged viewer frame.
pub fn request_film_state(parameters: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let id = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    );
    atomic_json(
        &film_request_path(parameters),
        &json!({"requestId":id,"requestedAtUnixSeconds":now()}),
    )?;
    Ok(json!({"requestId":id,"status":"pending","capture":"numericalFilmState"}))
}
pub fn pending_film_state(parameters: &Path) -> Option<String> {
    let request: Value =
        serde_json::from_slice(&std::fs::read(film_request_path(parameters)).ok()?).ok()?;
    let id = request["requestId"].as_str()?;
    let output = film_state_path(parameters, id).ok()?;
    (!output.exists()).then(|| id.to_owned())
}
pub fn publish_film_state(
    parameters: &Path,
    id: &str,
    applied: &Value,
    simulation_time: f64,
    frame: u64,
    state: Option<Value>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !simulation_time.is_finite()
        || simulation_time < 0.
        || !applied.is_object()
        || state.as_ref().is_some_and(|s| !s.is_object())
    {
        return Err("invalid film snapshot metadata".into());
    }
    let output = film_state_path(parameters, id)?;
    if output.exists() {
        return Ok(());
    }
    let value = json!({"requestId":id,"frame":frame,"simulationTime":simulation_time,
        "parameters":applied,"viewerPid":std::process::id(),"capturedAtUnixSeconds":now(),
        "filmEnabled":state.is_some(),"state":state,"capture":"numericalFilmState",
        "viewerImageCaptured":false,"wholeBodyCheckpoint":false});
    let temporary = output.with_extension(format!("{}.tmp", std::process::id()));
    std::fs::write(&temporary, serde_json::to_vec(&value)?)?;
    // Publish complete bytes without replacing another viewer's checkpoint.
    let result = std::fs::hard_link(&temporary, &output);
    let _ = std::fs::remove_file(&temporary);
    match result {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error.into()),
    }
}
pub fn film_state_status(parameters: &Path, id: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let output = film_state_path(parameters, id)?;
    if !output.exists() {
        let pending = pending_film_state(parameters).as_deref() == Some(id);
        return Ok(
            json!({"requestId":id,"status":if pending {"pending"} else {"unknownOrSuperseded"}}),
        );
    }
    let snapshot: Value = serde_json::from_slice(&std::fs::read(&output)?)?;
    if snapshot["requestId"] != id {
        return Err("film snapshot request id mismatch".into());
    }
    Ok(
        json!({"requestId":id,"status":"ready","path":output,"frame":snapshot["frame"],
        "simulationTime":snapshot["simulationTime"],"parameters":snapshot["parameters"],
        "filmEnabled":snapshot["filmEnabled"],"viewerPid":snapshot["viewerPid"],
        "capturedAtUnixSeconds":snapshot["capturedAtUnixSeconds"],"capture":"numericalFilmState",
        "viewerImageCaptured":false,"wholeBodyCheckpoint":false}),
    )
}
#[cfg(test)]
mod tests {
    #[test]
    fn film_request_is_correlated_and_checkpoint_is_not_overwritten() {
        let directory = std::env::temp_dir().join(format!(
            "voxy-film-request-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("body.json");
        let request = super::request_film_state(&path).unwrap();
        let id = request["requestId"].as_str().unwrap();
        assert_eq!(super::pending_film_state(&path).as_deref(), Some(id));
        assert_eq!(
            super::film_state_status(&path, id).unwrap()["status"],
            "pending"
        );
        let state = serde_json::json!({"physics":{"cellVolumesM3":[1e-9,2e-9]}});
        let parameters = serde_json::json!({"height_cm":180.});
        assert!(
            super::publish_film_state(&path, id, &parameters, f64::NAN, 9, Some(state.clone()))
                .is_err()
        );
        super::publish_film_state(&path, id, &parameters, 2., 9, Some(state.clone())).unwrap();
        assert!(super::pending_film_state(&path).is_none());
        let metadata = super::film_state_status(&path, id).unwrap();
        assert_eq!(metadata["frame"], 9);
        assert_eq!(metadata["filmEnabled"], true);
        let snapshot = super::film_state_path(&path, id).unwrap();
        let original = std::fs::read(&snapshot).unwrap();
        let saved: serde_json::Value = serde_json::from_slice(&original).unwrap();
        assert_eq!(saved["state"], state);
        super::publish_film_state(&path, id, &parameters, 3., 10, None).unwrap();
        assert_eq!(std::fs::read(&snapshot).unwrap(), original);
        let next = super::request_film_state(&path).unwrap();
        let next_id = next["requestId"].as_str().unwrap();
        assert_ne!(id, next_id);
        super::publish_film_state(&path, next_id, &parameters, 4., 11, None).unwrap();
        assert_eq!(
            super::film_state_status(&path, next_id).unwrap()["filmEnabled"],
            false
        );
        assert!(super::film_state_status(&path, "../../other").is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn saved_parameters_are_distinguished_from_presented_parameters() {
        let path = std::env::temp_dir().join(format!(
            "voxy-receipt-{}-{:?}-{}.json",
            std::process::id(),
            std::thread::current().id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let p = serde_json::json!({"height_cm":180});
        assert!(super::status(&path, &p)["liveApplied"].is_null());
        super::record(
            &path,
            &p,
            1.,
            10,
            [800, 600],
            &serde_json::json!({"fluid":null}),
        )
        .unwrap();
        assert_eq!(super::status(&path, &p)["liveApplied"], true);
        assert_eq!(
            super::status(&path, &serde_json::json!({"height_cm":185}))["liveApplied"],
            false
        );
        let receipt = super::receipt_path(&path);
        let before = std::fs::read(&receipt).unwrap();
        for (time, size) in [(f64::NAN, [800, 600]), (-1., [800, 600]), (1., [0, 600])] {
            assert!(super::record(&path, &p, time, 11, size, &serde_json::json!({})).is_err());
            assert_eq!(std::fs::read(&receipt).unwrap(), before);
        }
        let valid: serde_json::Value = serde_json::from_slice(&before).unwrap();
        for (key, invalid) in [
            ("simulationTime", serde_json::json!(-1.)),
            ("size", serde_json::json!([0, 600])),
            ("viewerPid", serde_json::json!(0)),
            ("measurements", serde_json::Value::Null),
        ] {
            let mut malformed = valid.clone();
            malformed[key] = invalid;
            std::fs::write(&receipt, malformed.to_string()).unwrap();
            assert!(super::status(&path, &p)["liveApplied"].is_null());
        }
        std::fs::write(&receipt, &before).unwrap();
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&receipt).unwrap()).unwrap();
        value["presentedAtUnixSeconds"] = serde_json::json!(super::now() - 60.);
        std::fs::write(&receipt, value.to_string()).unwrap();
        assert!(super::status(&path, &p)["liveApplied"].is_null());
        std::fs::remove_file(receipt).unwrap();
    }
}
