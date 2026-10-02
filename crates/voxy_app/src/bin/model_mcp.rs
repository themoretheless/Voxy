//! Local stdio MCP bridge for the imported body model; stdout is protocol only.
use serde_json::{Value, json};
use std::{
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
};
use voxy_app::body_parameters::BodyParameters;

const PROTOCOL: &str = "2025-06-18";
fn parameter_schema() -> Value {
    let mut properties = serde_json::Map::new();
    for (key, value) in BodyParameters::default().to_json().as_object().unwrap() {
        if key == "body_model" || key == "nipple_type" || key == "navel_shape" {
            let options = if key == "body_model" {
                json!(["female", "male"])
            } else if key == "nipple_type" {
                json!(["reference", "projecting", "flat", "inverted"])
            } else {
                json!(["reference", "round", "vertical", "horizontal"])
            };
            properties.insert(
                key.clone(),
                json!({"type":"string","enum":options,"default":value}),
            );
            continue;
        }
        let (min, max, unit) = match key.as_str() {
            "height_cm" => (130., 210., "cm"),
            "weight_kg" => (35., 180., "kg"),
            "areola_radius_mm" => (8., 40., "mm in reference space"),
            "areola_pigmentation" => (0., 1., "illustrative pigmentation strength"),
            "nipple_cold_response" => (0., 1., "normalized visual cold stimulus; uncalibrated"),
            "nipple_radius_mm" => (3., 25., "mm"),
            "nipple_projection_mm" => (0., 12., "mm"),
            "navel_width_mm" | "navel_height_mm" => (8., 40., "mm"),
            "navel_depth_mm" => (-8., 12., "mm; positive recess, negative protrusion"),
            _ => (0.6, 1.6, "reference ratio"),
        };
        properties.insert(
            key.clone(),
            json!({"type":"number","minimum":min,"maximum":max,"default":value,"description":unit}),
        );
    }
    json!({"type":"object","properties":properties,"additionalProperties":false})
}
fn tools() -> Value {
    let empty = json!({"type":"object","properties":{},"additionalProperties":false});
    let mut tools = Vec::new();
    for (name, description) in [
        (
            "model_get",
            "Read current body proportions and model metadata",
        ),
        (
            "model_schema",
            "Read allowed parameters, units, defaults and ranges",
        ),
        ("model_reset", "Restore reference body proportions"),
        ("view_get", "Read saved surface diagnostic mode"),
        (
            "film_state_request",
            "Request a numerical film checkpoint from a future presented viewer frame; returns a requestId",
        ),
        (
            "film_state_get",
            "Get the checkpoint path and captured frame metadata by requestId; not a viewer image",
        ),
        (
            "film_get",
            "Read saved film properties and independent source rate",
        ),
        (
            "model_measurements",
            "Read simulation measurements from the latest presented frame, with freshness and exact applied parameters",
        ),
        (
            "model_snapshot",
            "Render a PNG of the current saved parameters in a separate offscreen renderer; this is not a capture of the live viewer",
        ),
        (
            "model_status",
            "Check fresh renderer acknowledgement of saved body, view and enabled film settings",
        ),
    ] {
        let schema = if name == "film_state_get" {
            json!({"type":"object","properties":{"requestId":{"type":"string"}},"required":["requestId"],"additionalProperties":false})
        } else if name == "model_snapshot" {
            json!({"type":"object","properties":{"includeFilm":{"type":"boolean","default":false,"description":"Render saved film settings with a fresh 0.05 ml demo deposit; does not capture live liquid state"}},"additionalProperties":false})
        } else {
            empty.clone()
        };
        tools.push(json!({"name":name,"description":description,"inputSchema":schema}));
    }
    tools.push(json!({"name":"view_update","description":"Save surface mode; compare measurements.view with saved settings in a fresh presented frame","inputSchema":{"type":"object","properties":{"settings":{"type":"object","properties":{"mode":{"type":"string","enum":["material","displacement","strain"]}},"additionalProperties":false}},"required":["settings"],"additionalProperties":false}}));
    tools.push(json!({"name":"model_update","description":"Atomically apply partial body proportions; unspecified fields are preserved. A preview watching this file reloads on its next simulation advance.","inputSchema":{"type":"object","properties":{"parameters":parameter_schema()},"required":["parameters"],"additionalProperties":false}}));
    tools.push(json!({"name":"film_update","description":"Set film physical properties and independent demo source rate; enabled film preview watches this sidecar. Density editing conserves existing mass.","inputSchema":{"type":"object","properties":{"settings":{"type":"object","properties":{
        "self_contact_enabled":{"type":"boolean","description":"Same-body film exchange; static contact, no tissue collision resolution"},
        "self_contact_max_gap_m":{"type":"number","minimum":1e-6,"maximum":0.01,"description":"metres; films must span actual gap"},
        "self_contact_transfer_speed_m_s":{"type":"number","minimum":0,"maximum":0.1,"description":"phenomenological transfer speed in m/s, not calibrated physiology"},
        "precursor_wetting_enabled":{"type":"boolean","description":"Enable small-angle wetting potential without depositing precursor liquid"},
        "contact_angle_rad":{"type":"number","minimum":0,"maximum":0.5,"description":"radians; long-wave model validity limit"},
        "precursor_thickness_m":{"type":"number","minimum":1e-9,"maximum":1e-5,"description":"metres; regularization thickness, no hidden liquid injection"},
        "display_mode":{"type":"string","enum":["material","thickness"],"description":"Thickness colour scale: blue 0, green 50, red 100 micrometres; saturates above 100"},
        "refractive_index":{"type":"number","minimum":1,"maximum":2.5,"description":"optical index of refraction"},
        "absorption_r_per_m":{"type":"number","minimum":0,"maximum":10000,"description":"red absorption per metre"},
        "absorption_g_per_m":{"type":"number","minimum":0,"maximum":10000,"description":"green absorption per metre"},
        "absorption_b_per_m":{"type":"number","minimum":0,"maximum":10000,"description":"blue absorption per metre"},
        "density":{"type":"number","minimum":100,"maximum":20000,"description":"kg/m3"},
        "viscosity":{"type":"number","minimum":0.0001,"maximum":100,"description":"Pa s"},
        "surface_tension":{"type":"number","minimum":0,"maximum":1,"description":"N/m"},
        "wetting":{"type":"number","minimum":0,"maximum":0.001,"description":"phenomenological m2/s"},
        "sources":{"type":"array","maxItems":16,"description":"Additional independent sources; replaces the entire list, primary scalar source is preserved","items":{"type":"object","properties":{
            "id":{"type":"string","minLength":1,"maxLength":64,"pattern":"^[A-Za-z0-9_-]+$"},
            "x_m":{"type":"number","minimum":-2,"maximum":2},"y_m":{"type":"number","minimum":-2,"maximum":2},"z_m":{"type":"number","minimum":-2,"maximum":2},
            "radius_m":{"type":"number","minimum":0.001,"maximum":0.5},"rate_m3_s":{"type":"number","minimum":0,"maximum":0.000001}},"required":["id","x_m","y_m","z_m","radius_m","rate_m3_s"],"additionalProperties":false}},
        "source_x_m":{"type":"number","minimum":-2,"maximum":2,"description":"canonical model-space metres"},
        "source_y_m":{"type":"number","minimum":-2,"maximum":2,"description":"canonical model-space metres"},
        "source_z_m":{"type":"number","minimum":-2,"maximum":2,"description":"canonical model-space metres"},
        "source_radius_m":{"type":"number","minimum":0.001,"maximum":0.5,"description":"source selection radius in metres"},
        "source_rate_m3_s":{"type":"number","minimum":0,"maximum":0.000001,"description":"explicit demo source volume flow m3/s"}},"additionalProperties":false}},"required":["settings"],"additionalProperties":false}}));
    json!({"tools":tools})
}
fn read(path: &Path) -> Result<BodyParameters, Box<dyn std::error::Error>> {
    BodyParameters::from_json(&std::fs::read_to_string(path)?)
}
fn application_status(path: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let saved = read(path)?;
    let mut status = voxy_app::model_presentation::status(path, &saved.to_json());
    let view = voxy_app::view_settings::ViewSettings::read(&voxy_app::view_settings::path(path))?
        .to_json();
    let film = voxy_app::film_settings::FilmSettings::read(&voxy_app::film_settings::path(path))?
        .to_json();
    let fresh = status["fresh"] == true;
    let applied_view = &status["receipt"]["measurements"]["view"];
    let applied_film = &status["receipt"]["measurements"]["fluid"]["settings"];
    let view_applied = if fresh && applied_view.is_object() {
        json!(*applied_view == view)
    } else {
        Value::Null
    };
    let film_applied = if fresh && applied_film.is_object() {
        json!(*applied_film == film)
    } else {
        Value::Null
    };
    status["viewApplied"] = view_applied;
    status["filmSettingsApplied"] = film_applied;
    status["filmEnabled"] = if fresh {
        json!(status["receipt"]["measurements"]["fluid"].is_object())
    } else {
        Value::Null
    };
    status["savedView"] = view;
    status["savedFilm"] = film;
    let response = status["receipt"]["measurements"]["coldResponse"].clone();
    let valid_response = fresh
        && response["enabled"].is_boolean()
        && response["solverCoupled"].is_boolean()
        && ["current", "target"].iter().all(|key| {
            response[*key]
                .as_f64()
                .is_some_and(|value| value.is_finite() && (0. ..=1.).contains(&value))
        });
    status["coldResponse"] = if valid_response {
        response.clone()
    } else {
        Value::Null
    };
    status["coldTargetApplied"] = if valid_response {
        json!(
            status["liveApplied"] == true
                && response["target"].as_f64().unwrap() == f64::from(saved.nipple_cold_response)
        )
    } else {
        Value::Null
    };
    status["coldResponseError"] = if valid_response {
        json!((response["current"].as_f64().unwrap() - f64::from(saved.nipple_cold_response)).abs())
    } else {
        Value::Null
    };
    Ok(status)
}
fn save(path: &Path, p: BodyParameters) -> Result<(), Box<dyn std::error::Error>> {
    // The server owns one chosen preset file. Validate before creating or replacing it.
    p.validate()?;
    save_json(path, &p.to_json())
}
fn save_json(path: &Path, value: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(serde_json::to_string_pretty(value)?.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(&temporary, path)?;
        Ok::<_, Box<dyn std::error::Error>>(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result
}
fn call(path: &Path, params: &Value) -> Result<Value, Box<dyn std::error::Error>> {
    let name = params["name"].as_str().ok_or("missing tool name")?;
    let args = params.get("arguments").cloned().unwrap_or(json!({}));
    let object = args.as_object().ok_or("arguments must be an object")?;
    if object.keys().any(|k| {
        !((name == "model_update" && k == "parameters")
            || ((name == "film_update" || name == "view_update") && k == "settings")
            || (name == "model_snapshot" && k == "includeFilm"))
            && !(name == "film_state_get" && k == "requestId")
    }) {
        return Err("unknown tool argument".into());
    }
    let value = match name {
        "film_state_request" => voxy_app::model_presentation::request_film_state(path)?,
        "film_state_get" => voxy_app::model_presentation::film_state_status(
            path,
            args["requestId"].as_str().ok_or("requestId is required")?,
        )?,
        "view_get" => {
            voxy_app::view_settings::ViewSettings::read(&voxy_app::view_settings::path(path))?
                .to_json()
        }
        "view_update" => {
            let path = voxy_app::view_settings::path(path);
            let next = voxy_app::view_settings::ViewSettings::read(&path)?
                .patched(args.get("settings").ok_or("missing settings")?)?;
            save_json(&path, &next.to_json())?;
            next.to_json()
        }
        "film_get" => {
            voxy_app::film_settings::FilmSettings::read(&voxy_app::film_settings::path(path))?
                .to_json()
        }
        "film_update" => {
            let path = voxy_app::film_settings::path(path);
            let next = voxy_app::film_settings::FilmSettings::read(&path)?
                .patched(args.get("settings").ok_or("missing settings")?)?;
            save_json(&path, &next.to_json())?;
            next.to_json()
        }
        "model_measurements" => {
            let status = application_status(path)?;
            let receipt = &status["receipt"];
            json!({"fresh":status["fresh"],"reason":status["reason"],"liveApplied":status["liveApplied"],
                "frame":receipt["frame"],"simulationTime":receipt["simulationTime"],
                "parameters":receipt["parameters"],"measurements":receipt["measurements"],
                "ageSeconds":status["ageSeconds"],"viewApplied":status["viewApplied"],
                "filmSettingsApplied":status["filmSettingsApplied"],"filmEnabled":status["filmEnabled"],
                "coldResponse":status["coldResponse"],"coldTargetApplied":status["coldTargetApplied"],
                "coldResponseError":status["coldResponseError"]})
        }
        "model_snapshot" => {
            let include_film = match args.get("includeFilm") {
                Some(value) => value.as_bool().ok_or("includeFilm must be boolean")?,
                None => false,
            };
            return snapshot(path, include_film);
        }
        "model_schema" => parameter_schema(),
        "model_get" => {
            let parameters = read(path)?.to_json();
            let presentation = application_status(path)?;
            json!({"model":format!("blender-{}",read(path)?.body_model.as_str()),"parameters":parameters,"parameterFile":path,"liveApplied":presentation["liveApplied"],"presentation":presentation})
        }
        "model_status" => application_status(path)?,
        "model_update" => {
            let next = read(path)?.patched(args.get("parameters").ok_or("missing parameters")?)?;
            save(path, next)?;
            next.to_json()
        }
        "model_reset" => {
            let next = BodyParameters::default();
            save(path, next)?;
            next.to_json()
        }
        _ => return Err("unknown tool".into()),
    };
    Ok(
        json!({"content":[{"type":"text","text":value.to_string()}],"structuredContent":value,"isError":false}),
    )
}
fn snapshot(path: &Path, include_film: bool) -> Result<Value, Box<dyn std::error::Error>> {
    let parameters = read(path)?;
    let view = voxy_app::view_settings::ViewSettings::read(&voxy_app::view_settings::path(path))?;
    let renderer = std::env::current_exe()?
        .parent()
        .ok_or("missing executable directory")?
        .join("examples/female_render");
    if !renderer.is_file() {
        return Err("Build the snapshot renderer with cargo build -p voxy_app --release --example female_render".into());
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "voxy-model-snapshot-{}-{stamp}",
        std::process::id()
    ));
    std::fs::create_dir(&directory)?;
    let preset = directory.join("parameters.json");
    let output = directory.join("snapshot.png");
    save(&preset, parameters)?;
    let film_settings = if include_film {
        Some(voxy_app::film_settings::FilmSettings::read(
            &voxy_app::film_settings::path(path),
        )?)
    } else {
        None
    };
    let film_preset = directory.join("film.json");
    let mut command = std::process::Command::new(renderer);
    command.args([
        "--body-preset",
        preset.to_str().ok_or("invalid preset path")?,
        "--snapshot",
        output.to_str().ok_or("invalid output path")?,
    ]);
    match view.mode.as_str() {
        "strain" => {
            command.arg("--strain");
        }
        "displacement" => {
            command.arg("--displacement");
        }
        _ => {}
    }
    if let Some(settings) = &film_settings {
        save_json(&film_preset, &settings.to_json())?;
        command.args([
            "--film",
            "--film-preset",
            film_preset.to_str().ok_or("invalid film path")?,
        ]);
    }
    let mut child = command
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    let started = std::time::Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed().as_secs() >= 60 {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Snapshot renderer exceeded 60 seconds".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    };
    if !status.success() {
        return Err(format!("Snapshot renderer failed: {status}").into());
    }
    let bytes = std::fs::read(&output)?;
    if bytes.len() < 24
        || &bytes[..8] != b"\x89PNG\r\n\x1a\n"
        || u32::from_be_bytes(bytes[16..20].try_into()?) != 576
        || u32::from_be_bytes(bytes[20..24].try_into()?) != 768
    {
        return Err("Unexpected snapshot format or dimensions".into());
    }
    let value = json!({"path":output,"parameters":parameters.to_json(),"width":576,"height":768,"capture":"offscreen","view":view.to_json(),"liveViewerCapture":false,"poseTimeSeconds":0,"film":film_settings.map(|s| json!({"settings":s.to_json(),"state":"freshDemoDeposit","initialMassKg":5e-5,"initialVolumeM3":5e-5/s.material.density,"depositCenterM":[0.,0.2,0.13],"depositRadiusM":0.04,"liveLiquidCapture":false}))});
    Ok(json!({"content":[{"type":"text","text":value.to_string()},
        {"type":"resource_link","uri":format!("file://{}",output.display()),"name":"snapshot.png","mimeType":"image/png"}],
        "structuredContent":value,"isError":false}))
}
fn dispatch(path: &Path, request: &Value, initialized: &mut bool) -> Value {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let error = |code, message: &str| json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}});
    if request["jsonrpc"] != "2.0" || !request["method"].is_string() {
        return error(-32600, "Invalid Request");
    }
    let method = request["method"].as_str().unwrap();
    let params = request.get("params").cloned().unwrap_or(json!({}));
    let result = match method {
        "initialize" => {
            let requested = params["protocolVersion"].as_str().unwrap_or(PROTOCOL);
            let version = if ["2024-11-05", "2025-03-26", PROTOCOL].contains(&requested) {
                requested
            } else {
                PROTOCOL
            };
            *initialized = true;
            json!({"protocolVersion":version,"capabilities":{"tools":{}},"serverInfo":{"name":"voxy-models","version":"0.1.0"}})
        }
        "ping" => json!({}),
        "tools/list" if *initialized => tools(),
        "tools/call" if *initialized => match call(path, &params) {
            Ok(v) => v,
            Err(e) => json!({"content":[{"type":"text","text":e.to_string()}],"isError":true}),
        },
        "tools/list" | "tools/call" => return error(-32000, "Initialize first"),
        _ => return error(-32601, "Method not found"),
    };
    json!({"jsonrpc":"2.0","id":id,"result":result})
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("usage: model_mcp /absolute/path/to/body.json")?,
    );
    if !path.is_absolute() {
        return Err("parameter file path must be absolute".into());
    }
    if !path.exists() {
        save(&path, BodyParameters::default())?;
    }
    read(&path)?;
    let mut initialized = false;
    let mut stdout = io::stdout().lock();
    for line in io::stdin().lock().lines() {
        let response = match serde_json::from_str::<Value>(&line?) {
            Ok(request) => {
                // Client notifications and responses require no server response.
                if request.get("method").is_none()
                    || (request.get("id").is_none() && request["method"].is_string())
                {
                    continue;
                }
                dispatch(&path, &request, &mut initialized)
            }
            Err(_) => {
                json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"Parse error"}})
            }
        };
        writeln!(stdout, "{response}")?;
        stdout.flush()?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn film_snapshot_tools_require_a_correlated_viewer_response() {
        let directory = std::env::temp_dir().join(format!(
            "voxy-mcp-film-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("body.json");
        save(&path, BodyParameters::default()).unwrap();
        let request = call(&path, &json!({"name":"film_state_request"})).unwrap();
        let id = request["structuredContent"]["requestId"].as_str().unwrap();
        assert!(call(&path, &json!({"name":"film_state_get"})).is_err());
        let arguments = json!({"name":"film_state_get","arguments":{"requestId":id}});
        assert_eq!(
            call(&path, &arguments).unwrap()["structuredContent"]["status"],
            "pending"
        );
        voxy_app::model_presentation::publish_film_state(
            &path,
            id,
            &BodyParameters::default().to_json(),
            2.,
            12,
            None,
        )
        .unwrap();
        let response = call(&path, &arguments).unwrap();
        assert_eq!(response["structuredContent"]["status"], "ready");
        assert_eq!(response["structuredContent"]["filmEnabled"], false);
        assert_eq!(response["structuredContent"]["frame"], 12);
        assert!(
            call(
                &path,
                &json!({"name":"film_state_get","arguments":{"requestId":"../escape"}})
            )
            .is_err()
        );
        std::fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn nipple_side_schema_and_atomic_updates_match_dimension_bounds() {
        for name in [
            "left_nipple_radius_scale",
            "right_nipple_radius_scale",
            "left_nipple_projection_scale",
            "right_nipple_projection_scale",
        ] {
            let schema = parameter_schema();
            assert_eq!(schema["properties"][name]["minimum"], 0.6);
            assert_eq!(schema["properties"][name]["maximum"], 1.6);
            assert_eq!(schema["properties"][name]["default"], 1.);
        }
        let path = std::env::temp_dir().join(format!(
            "voxy-nipple-side-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        save(&path, BodyParameters::default()).unwrap();
        call(&path,&json!({"name":"model_update","arguments":{"parameters":{"left_nipple_radius_scale":1.5,"right_nipple_projection_scale":0.6}}})).unwrap();
        let saved = read(&path).unwrap();
        assert_eq!(saved.left_nipple_radius_scale, 1.5);
        assert_eq!(saved.right_nipple_radius_scale, 1.);
        let previous = std::fs::read(&path).unwrap();
        assert!(
            call(
                &path,
                &json!({"name":"model_update","arguments":{"parameters":{"nipple_radius_mm":25.}}})
            )
            .is_err()
        );
        assert_eq!(std::fs::read(&path).unwrap(), previous);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn cold_ack_separates_target_from_current_and_rejects_invalid_or_stale_state() {
        let path = std::env::temp_dir().join(format!(
            "voxy-cold-ack-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let parameters = BodyParameters::default()
            .patched(&json!({"nipple_cold_response":1.}))
            .unwrap();
        save(&path, parameters).unwrap();
        assert!(application_status(&path).unwrap()["coldTargetApplied"].is_null());
        let measurement = json!({"coldResponse":{"enabled":true,"target":1.,"current":0.4,"solverCoupled":false}});
        voxy_app::model_presentation::record(
            &path,
            &parameters.to_json(),
            2.,
            3,
            [800, 600],
            &measurement,
        )
        .unwrap();
        let status = application_status(&path).unwrap();
        assert_eq!(status["coldTargetApplied"], true);
        assert!((status["coldResponseError"].as_f64().unwrap() - 0.6).abs() < 1e-12);
        assert_eq!(
            call(&path, &json!({"name":"model_get"})).unwrap()["structuredContent"]["presentation"]
                ["coldResponse"],
            measurement["coldResponse"]
        );
        assert_eq!(
            call(&path, &json!({"name":"model_measurements"})).unwrap()["structuredContent"]["coldTargetApplied"],
            true
        );
        save(
            &path,
            parameters.patched(&json!({"height_cm":180.})).unwrap(),
        )
        .unwrap();
        assert_eq!(
            application_status(&path).unwrap()["coldTargetApplied"],
            false
        );
        let receipt_path = voxy_app::model_presentation::receipt_path(&path);
        let valid: Value = serde_json::from_slice(&std::fs::read(&receipt_path).unwrap()).unwrap();
        for (key, value) in [
            ("current", json!(1.1)),
            ("target", json!(-0.1)),
            ("enabled", json!("yes")),
            ("solverCoupled", Value::Null),
        ] {
            let mut invalid = valid.clone();
            invalid["measurements"]["coldResponse"][key] = value;
            std::fs::write(&receipt_path, invalid.to_string()).unwrap();
            let status = application_status(&path).unwrap();
            assert!(status["coldTargetApplied"].is_null());
            assert!(status["coldResponse"].is_null());
        }
        let mut stale = valid;
        stale["presentedAtUnixSeconds"] = json!(1.);
        std::fs::write(&receipt_path, stale.to_string()).unwrap();
        let status = application_status(&path).unwrap();
        assert!(status["coldResponse"].is_null());
        assert!(status["coldResponseError"].is_null());
        std::fs::remove_file(path).unwrap();
        std::fs::remove_file(receipt_path).unwrap();
    }
    #[test]
    fn settings_acknowledgement_requires_matching_fresh_frame() {
        let path = std::env::temp_dir().join(format!(
            "voxy-mcp-ack-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let parameters = BodyParameters::default();
        save(&path, parameters).unwrap();
        assert!(application_status(&path).unwrap()["viewApplied"].is_null());
        let view = voxy_app::view_settings::ViewSettings::default().to_json();
        let film = voxy_app::film_settings::FilmSettings::default().to_json();
        let measurements = json!({"view":view,"fluid":{"settings":film}});
        voxy_app::model_presentation::record(
            &path,
            &parameters.to_json(),
            1.,
            10,
            [800, 600],
            &measurements,
        )
        .unwrap();
        let status = application_status(&path).unwrap();
        assert_eq!(status["viewApplied"], true);
        assert_eq!(status["filmSettingsApplied"], true);
        assert_eq!(status["filmEnabled"], true);
        let view_path = voxy_app::view_settings::path(&path);
        let film_path = voxy_app::film_settings::path(&path);
        save_json(&view_path, &json!({"mode":"strain"})).unwrap();
        save_json(&film_path, &json!({"density":2000.})).unwrap();
        let status = application_status(&path).unwrap();
        assert_eq!(status["liveApplied"], true);
        assert_eq!(status["viewApplied"], false);
        assert_eq!(status["filmSettingsApplied"], false);
        voxy_app::model_presentation::record(
            &path,
            &parameters.to_json(),
            2.,
            11,
            [800, 600],
            &json!({"view":view,"fluid":null}),
        )
        .unwrap();
        let status = application_status(&path).unwrap();
        assert_eq!(status["filmEnabled"], false);
        assert!(status["filmSettingsApplied"].is_null());
        let receipt_path = voxy_app::model_presentation::receipt_path(&path);
        let mut receipt: Value =
            serde_json::from_slice(&std::fs::read(&receipt_path).unwrap()).unwrap();
        receipt["presentedAtUnixSeconds"] = json!(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64()
                - 60.
        );
        std::fs::write(&receipt_path, receipt.to_string()).unwrap();
        let status = application_status(&path).unwrap();
        for key in [
            "liveApplied",
            "viewApplied",
            "filmSettingsApplied",
            "filmEnabled",
        ] {
            assert!(status[key].is_null());
        }
        for file in [path, view_path, film_path, receipt_path] {
            std::fs::remove_file(file).unwrap();
        }
    }
    #[test]
    fn update_is_partial_atomic_and_persistent() {
        let path = std::env::temp_dir().join(format!("voxy-mcp-test-{}.json", std::process::id()));
        save(&path, BodyParameters::default()).unwrap();
        call(
            &path,
            &json!({"name":"model_update","arguments":{"parameters":{"height_cm":182}}}),
        )
        .unwrap();
        call(
            &path,
            &json!({"name":"model_update","arguments":{"parameters":{"breast_size":1.3}}}),
        )
        .unwrap();
        assert_eq!(read(&path).unwrap().height_cm, 182.);
        assert!(
            call(
                &path,
                &json!({"name":"model_snapshot","arguments":{"includeFilm":"true"}})
            )
            .is_err()
        );
        let before = std::fs::read(&path).unwrap();
        assert!(
            call(
                &path,
                &json!({"name":"model_update","arguments":{"parameters":{"weight_kg":0}}})
            )
            .is_err()
        );
        assert_eq!(before, std::fs::read(&path).unwrap());
        call(&path,&json!({"name":"film_update","arguments":{"settings":{"density":2000,"source_rate_m3_s":1e-9}}})).unwrap();
        let film_path = voxy_app::film_settings::path(&path);
        let film_before = std::fs::read(&film_path).unwrap();
        assert!(
            call(
                &path,
                &json!({"name":"film_update","arguments":{"settings":{"viscosity":0}}})
            )
            .is_err()
        );
        assert_eq!(film_before, std::fs::read(&film_path).unwrap());
        std::fs::remove_file(film_path).unwrap();
        let mut initialized = false;
        assert_eq!(
            dispatch(
                &path,
                &json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
                &mut initialized
            )["error"]["code"],
            -32000
        );
        let response = dispatch(
            &path,
            &json!({"jsonrpc":"2.0","id":2,"method":"initialize","params":{"protocolVersion":PROTOCOL}}),
            &mut initialized,
        );
        assert_eq!(response["result"]["protocolVersion"], PROTOCOL);
        assert_eq!(
            dispatch(
                &path,
                &json!({"jsonrpc":"2.0","id":3,"method":"tools/list"}),
                &mut initialized
            )["result"]["tools"]
                .as_array()
                .unwrap()
                .len(),
            13
        );
        std::fs::remove_file(path).unwrap();
    }
}
