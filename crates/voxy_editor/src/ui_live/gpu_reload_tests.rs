//! Exercises the production native publication path on a real offscreen device.
use super::*;
use voxy_gameplay::{UiElement, UiText};
use voxy_scene::{SceneGraph, Transform};

#[test]
#[ignore = "requires a real GPU and two local TrueType fonts"]
#[allow(clippy::too_many_lines)]
fn gpu_font_reload_retains_last_good_and_recovers_through_actual_worker_and_watcher() {
    let first = std::fs::read("/System/Library/Fonts/Supplemental/Arial.ttf").unwrap();
    let second = std::fs::read("/System/Library/Fonts/Supplemental/Courier New.ttf").unwrap();
    assert_ne!(first, second);
    let root = std::env::temp_dir().join(format!(
        "voxy-gpu-font-reload-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("font.ttf"), &first).unwrap();
    let project = AuthoringProject::new(&root, &crate::InputRecipe::Direct).unwrap();
    let mut scene = SceneGraph::new(1);
    let owner = scene.spawn(None, Transform::default()).unwrap();
    scene
        .insert_component(
            owner,
            UiElement {
                origin: [0.0; 2],
                size: [1.0; 2],
                color: [0.1, 0.2, 0.3, 1.0],
                layer: 0,
                enabled: true,
                action: Some("menu".into()),
                text: Some(UiText {
                    font: "font.ttf".into(),
                    content: "Hello 123".into(),
                    size: 28.0,
                    color: [1.0; 4],
                }),
            },
        )
        .unwrap();
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .unwrap();
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = voxy_render::SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let mut ui = UiLive::default();
    let mut cache = crate::gpu_model::ResidencyCache::with_budget(32 * 1024 * 1024);
    let mut draws = Vec::<GpuDraw>::new();
    let mut presented = false;
    let mut failures = Vec::<String>::new();
    macro_rules! until {
        ($condition:expr) => {{
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
            loop {
                cache.geometry_live = draws
                    .iter()
                    .map(|draw| draw.geometry.allocation_bytes())
                    .sum();
                if let Err(error) = ui.poll_gpu(
                    &scene,
                    &project,
                    [128.0; 2],
                    &renderer,
                    &device,
                    &queue,
                    &mut cache,
                    &mut draws,
                    &mut presented,
                ) {
                    failures.push(error.to_string());
                }
                if $condition {
                    break;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "UI reload timed out: {failures:?}"
                );
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }};
    }
    until!(draws.len() == 2);
    assert!(failures.is_empty());
    ui.frame_presented();
    let visible = ui.presented().unwrap().clone();
    let original = Arc::clone(draws[1].texture.as_ref().unwrap());
    std::fs::write(root.join("font.ttf"), b"broken font").unwrap();
    until!(!failures.is_empty());
    assert!(failures[0].contains("font"), "{failures:?}");
    assert!(Arc::ptr_eq(&original, draws[1].texture.as_ref().unwrap()));
    assert_eq!(ui.presented(), Some(&visible));
    assert!(draws[1].geometry.index_count() > 0);
    std::fs::write(root.join("font.ttf"), &second).unwrap();
    until!(!Arc::ptr_eq(&original, draws[1].texture.as_ref().unwrap()));
    assert_eq!(ui.published.as_ref(), Some(&visible));
    std::fs::write(root.join("font.ttf"), &first).unwrap();
    until!(Arc::ptr_eq(&original, draws[1].texture.as_ref().unwrap()));
    for handle in ui.close() {
        handle.join().unwrap();
    }
    assert!(pollster::block_on(scope.pop()).is_none());
    std::fs::remove_dir_all(root).unwrap();
}
