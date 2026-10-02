//! Real-window scene behavior acceptance: activity, start and shutdown cleanup.
use std::sync::{Arc, Mutex};
use voxy_scene::{Behavior, NodeId, SceneGraph, Transform};
#[derive(Debug, Default)]
struct Counts {
    enable: usize,
    disable: usize,
    start: usize,
    updates: usize,
    fixed_updates: usize,
    destroy: usize,
}
#[derive(Debug)]
struct Spin {
    angle: f32,
    counts: Arc<Mutex<Counts>>,
}
impl Behavior for Spin {
    fn on_enable(&mut self, _: &mut SceneGraph, _: NodeId) {
        self.counts.lock().unwrap().enable += 1;
    }
    fn on_disable(&mut self, _: &mut SceneGraph, _: NodeId) {
        self.counts.lock().unwrap().disable += 1;
    }
    fn start(&mut self, _: &mut SceneGraph, _: NodeId) {
        self.counts.lock().unwrap().start += 1;
    }
    fn fixed_update(&mut self, _: &mut SceneGraph, _: NodeId, delta: f64) {
        assert!((delta - 1.0 / 60.0).abs() < f64::EPSILON);
        self.counts.lock().unwrap().fixed_updates += 1;
    }
    fn update(&mut self, scene: &mut SceneGraph, owner: NodeId, delta: f64) {
        self.angle += delta as f32;
        self.counts.lock().unwrap().updates += 1;
        scene
            .set_local(
                owner,
                Transform {
                    rotation: glam::Quat::from_rotation_z(self.angle),
                    ..Transform::default()
                },
            )
            .unwrap();
    }
    fn on_destroy(&mut self, _: &mut SceneGraph, _: NodeId) {
        self.counts.lock().unwrap().destroy += 1;
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let smoke = match std::env::args().nth(1).as_deref() {
        None => false,
        Some("--smoke") => true,
        _ => return Err("lifecycle_scene [--smoke]; V toggles objects, Space pauses".into()),
    };
    let counts = Arc::new(Mutex::new(Counts::default()));
    voxy_app::SceneApp::new(smoke)?
        .with_lifecycle_smoke()
        .with_object_behavior(Spin {
            angle: 0.0,
            counts: counts.clone(),
        })?
        .run(winit::event_loop::EventLoop::new()?)?;
    let counts = counts.lock().unwrap();
    assert_eq!(counts.start, 1);
    assert_eq!(counts.destroy, 1);
    if smoke {
        assert_eq!(counts.enable, 2);
        assert_eq!(counts.disable, 2);
        assert!(counts.updates >= 100);
        assert!(counts.fixed_updates > 0);
    }
    println!("SCENE LIFECYCLE PASS: {counts:?}");
    Ok(())
}
