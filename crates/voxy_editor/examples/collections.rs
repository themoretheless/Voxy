//! Run with an optional scene path. Existing scenes are never overwritten at startup.
use serde::{Deserialize, Serialize};
use voxy_editor::{
    ModelSource, ViewportMode, editor_component_registry, run_model_viewport_with_components,
};
#[derive(Serialize, Deserialize)]
struct Inventory {
    items: Vec<Item>,
}
#[derive(Serialize, Deserialize)]
struct Item {
    id: String,
    name: String,
    quantity: u32,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = editor_component_registry()?;
    registry.register::<Inventory>("demo.inventory.v1")?;
    registry.declare_identified_collection("demo.inventory.v1", "/items", "id")?;
    registry.set_collection_default(
        "demo.inventory.v1",
        "/items",
        serde_json::json!({"name":"New item","quantity":1}),
    )?;
    let scene = std::env::args_os()
        .nth(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("voxy-collection-demo/scene.json"));
    let root = scene
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    std::fs::create_dir_all(root)?;
    let model = root.join("quad.obj");
    if !model.try_exists()? {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&model)?;
        std::io::Write::write_all(
            &mut file,
            include_bytes!("../../voxy_render/examples/assets/quad.obj"),
        )?;
        file.sync_all()?;
    }
    if !scene.try_exists()? {
        if let Some(parent) = scene
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)?;
        }
        let document = voxy_scene::SceneDocument::from_json(&serde_json::json!({
            "version":1,"objects":[{"id":"inventory","parent":null,"name":"Inventory","active":true,
            "translation":[0,0,0.5],"rotation":[0,0,0,1],"scale":[1,1,1],
            "components":{"editor.model.v1":"quad.obj","demo.inventory.v1":{"items":[]}}}]
        }).to_string())?;
        document.load(&registry, 128)?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&scene)?;
        std::io::Write::write_all(&mut file, document.to_json()?.as_bytes())?;
        file.sync_all()?;
    }
    run_model_viewport_with_components(
        &ModelSource::File(model),
        ViewportMode::Interactive,
        Some(&scene),
        registry,
        None,
    )
}
