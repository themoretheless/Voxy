use super::*;
use voxy_gameplay::{UiElement, UiText};
use voxy_scene::{NodeId, ObjectId, SceneGraph, Transform};

fn limits() -> UiPublicationLimits {
    UiPublicationLimits {
        max_bytes: 256 * 1024 * 1024,
        max_layout_bytes: 100000,
        max_elements: 128,
        max_input_bytes: 32 * 1024 * 1024,
        max_pixels: 32 * 1024 * 1024,
        max_glyphs: 16384,
        max_runs: 256,
        text: ui_text::checkpoint::UiTextArchiveLimits {
            imported: voxy_assets::ImportedArchiveLimits {
                max_inputs: 9,
                max_input_bytes: 16 * 1024 * 1024,
                max_archive_bytes: 128 * 1024 * 1024,
            },
            max_payload_bytes: 64 * 1024 * 1024,
            max_fonts: 8,
            max_runs: 128,
            max_pixels: 16 * 1024 * 1024,
            max_glyphs_per_run: 1024,
            max_total_glyphs: 8192,
            font: voxy_text::FontLimits {
                max_font_bytes: 4 * 1024 * 1024,
                max_glyph_pixels: 16384,
                max_size: 128.,
            },
        },
    }
}
struct Fixture {
    root: std::path::PathBuf,
    scene: SceneGraph,
    nodes: Vec<NodeId>,
    layout: SceneUiSnapshot,
    text: Text,
}
impl Fixture {
    fn new() -> Self {
        static SERIAL: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "voxy-ui-publications-{}-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let sources = root.join("sources");
        std::fs::create_dir(&sources).unwrap();
        let bytes = [
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "C:/Windows/Fonts/arial.ttf",
        ]
        .iter()
        .find_map(|p| std::fs::read(p).ok())
        .expect("Unicode TrueType font fixture");
        for file in ["font.ttf", "other.ttf"] {
            std::fs::write(sources.join(file), &bytes).unwrap();
        }
        let project = AuthoringProject::new(&sources, &crate::InputRecipe::Direct).unwrap();
        let mut scene = SceneGraph::new(8);
        let mut nodes = Vec::new();
        for i in 0..3 {
            let owner = scene.spawn(None, Transform::default()).unwrap();
            scene
                .insert_component(
                    owner,
                    UiElement {
                        origin: [0., i as f32 * 0.25],
                        size: [1., 0.25],
                        color: [0.1, 0.2, 0.3, 1.],
                        layer: i,
                        enabled: true,
                        action: Some(format!("action-{i}")),
                        text: Some(UiText {
                            font: if i < 2 { "font.ttf" } else { "other.ttf" }.into(),
                            content: "office e\u{301} café".into(),
                            size: 18.,
                            color: [1.; 4],
                        }),
                    },
                )
                .unwrap();
            nodes.push(owner);
        }
        let layout = voxy_gameplay::extract_scene_ui(&scene, [320.; 2], 128).unwrap();
        let text = ui_text::UiTextPreparation::prepare_import(&scene, &project, [320.; 2]).unwrap();
        Self {
            root,
            scene,
            nodes,
            layout,
            text,
        }
    }
    fn view(&self) -> SceneCheckpointView<'_> {
        SceneCheckpointView::new(
            &self.scene,
            self.nodes
                .iter()
                .enumerate()
                .map(|(i, node)| (ObjectId(format!("label-{i}")), *node)),
        )
        .unwrap()
    }
    fn variant(&self, shared: bool) -> Text {
        let view = self.view();
        let bytes = ui_text::checkpoint::encode(
            &self.text,
            |node| view.reserved_identity(node).cloned(),
            limits().text,
        )
        .unwrap();
        let mut pool = if shared {
            vec![
                self.text.value().runs[0].run.clone(),
                self.text.value().runs[2].run.clone(),
            ]
        } else {
            Vec::new()
        };
        Arc::new(
            ui_text::checkpoint::decode_historical_interned(
                &bytes,
                &view,
                limits().text,
                if shared { &[0, 2] } else { &[0, 1] },
                &mut pool,
            )
            .unwrap(),
        )
    }
    fn ui(&self, text: Text) -> UiLive {
        let mut published = self.layout.clone();
        published.viewport = [400., 320.];
        published.elements[1].descriptor.color = [0.7, 0.2, 0.3, 1.];
        UiLive {
            attempted: Some(SceneUiSnapshot {
                viewport: [640.; 2],
                elements: Vec::new(),
            }),
            published: Some(published),
            published_text: Some(text),
            presented: Some(self.layout.clone()),
            presented_text: Some(self.text.clone()),
            ..UiLive::default()
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn geometry(mesh: &voxy_render::SceneMesh) -> Vec<u32> {
    mesh.vertices()
        .iter()
        .flat_map(|v| {
            v.position
                .into_iter()
                .chain(v.uv)
                .chain(v.color)
                .map(f32::to_bits)
        })
        .collect()
}
fn same_draws(
    left: &[ui_draw::UiDrawMesh],
    right: &[ui_draw::UiDrawMesh],
    a: &SceneCheckpointView<'_>,
    b: &SceneCheckpointView<'_>,
) {
    assert_eq!(left.len(), right.len());
    for (left, right) in left.iter().zip(right) {
        assert_eq!(
            a.reserved_identity(left.owner),
            b.reserved_identity(right.owner)
        );
        assert_eq!(geometry(&left.mesh), geometry(&right.mesh));
        assert_eq!(left.mesh.indices(), right.mesh.indices());
        assert_eq!(
            left.atlas.as_ref().map(|run| run.checkpoint()),
            right.atlas.as_ref().map(|run| run.checkpoint())
        );
    }
}

#[test]
fn file_bundle_preserves_two_versions_shared_runs_and_retired_owners_for_120_draw_cycles() {
    let mut fixture = Fixture::new();
    let ui = fixture.ui(fixture.variant(true));
    fixture.scene.remove_subtree(fixture.nodes[0]).unwrap();
    let view = fixture.view();
    let saved = ui.publication_checkpoint(&view, limits()).unwrap();
    assert_eq!(saved.wire.publications.len(), 2);
    // Published first: its second run is distinct from the equal presented run.
    assert_eq!(saved.wire.publications[0].run_ids, [0, 1]);
    assert_eq!(saved.wire.publications[1].run_ids, [0, 2]);
    let bytes = saved.to_bytes(limits()).unwrap();
    let path = fixture.root.join("ui.bin");
    std::fs::write(&path, &bytes).unwrap();
    let mut registry = voxy_scene::ComponentRegistry::default();
    voxy_gameplay::register_components(&mut registry).unwrap();
    let policy = voxy_scene::SimulationLimits {
        fixed_step: 0.01,
        max_steps: 4,
        max_behaviors: 4,
        max_commands: 4,
    };
    let simulation = voxy_scene::SceneSimulation::new(&fixture.scene, policy).unwrap();
    let scene_record = simulation
        .checkpoint_view(&view, &registry, 100000)
        .unwrap();
    let (target, _) = scene_record
        .prepare(
            &registry,
            &voxy_scene::BehaviorCheckpointRegistry::default(),
            policy,
            8,
            100000,
        )
        .unwrap();
    let target_view = target.checkpoint_view();
    std::fs::remove_dir_all(fixture.root.join("sources")).unwrap();
    let decoded =
        UiPublicationsCheckpoint::from_bytes(&std::fs::read(path).unwrap(), limits()).unwrap();
    let prepared = decoded.prepare(&target_view, limits()).unwrap();
    assert_eq!(
        prepared
            .checkpoint(&target_view, limits())
            .unwrap()
            .to_bytes(limits())
            .unwrap(),
        bytes
    );
    assert_eq!(target.graph.len(), 2);
    let retired = target.reserved_node(&ObjectId("label-0".into())).unwrap();
    assert!(target.graph.local(retired).is_err());
    assert!(target.resolve(&ObjectId("label-0".into())).is_none());
    assert_ne!(prepared.attempted, prepared.published);
    assert_ne!(prepared.published, prepared.presented);
    let published = prepared.published_text.as_ref().unwrap();
    let presented = prepared.presented_text.as_ref().unwrap();
    assert!(!Arc::ptr_eq(published, presented));
    assert!(Arc::ptr_eq(
        &published.value().runs[0].run,
        &presented.value().runs[0].run
    ));
    assert!(!Arc::ptr_eq(
        &published.value().runs[2].run,
        &presented.value().runs[2].run
    ));
    assert_eq!(
        published.value().runs[2].run.checkpoint(),
        presented.value().runs[2].run.checkpoint()
    );
    for i in 0..120 {
        for (a, at, b, bt) in [
            (
                ui.published.as_ref().unwrap(),
                ui.published_text.as_ref().unwrap(),
                prepared.published.as_ref().unwrap(),
                published,
            ),
            (
                ui.presented.as_ref().unwrap(),
                ui.presented_text.as_ref().unwrap(),
                prepared.presented.as_ref().unwrap(),
                presented,
            ),
        ] {
            let mut left = a.clone();
            let mut right = b.clone();
            for layout in [&mut left, &mut right] {
                layout.viewport = [400. + i as f32, 320.];
                for element in &mut layout.elements {
                    element.clip.as_mut().unwrap()[2] = 20. + (i % 40) as f32;
                    element.descriptor.text.as_mut().unwrap().color[0] = (i % 10) as f32 * 0.1;
                }
            }
            same_draws(
                &ui_draw::build(&left, &at.value().runs).unwrap(),
                &ui_draw::build(&right, &bt.value().runs).unwrap(),
                &view,
                &target_view,
            );
        }
    }
    let mut resumed = UiLive {
        attempted: prepared.attempted,
        published: prepared.published,
        presented: prepared.presented,
        published_text: prepared.published_text,
        presented_text: prepared.presented_text,
        ..UiLive::default()
    };
    assert!(resumed.focus_snapshot().is_none());
    resumed.frame_presented();
    assert!(resumed.focus_snapshot().is_some());
    assert!(Arc::ptr_eq(
        resumed.published_text.as_ref().unwrap(),
        resumed.presented_text.as_ref().unwrap()
    ));
}

#[test]
fn shared_publication_and_distinct_equal_publications_remain_distinguishable() {
    let fixture = Fixture::new();
    let view = fixture.view();
    for shared in [true, false] {
        let ui = fixture.ui(if shared {
            fixture.text.clone()
        } else {
            fixture.variant(false)
        });
        let saved = ui.publication_checkpoint(&view, limits()).unwrap();
        assert_eq!(saved.archives.len(), if shared { 1 } else { 2 });
        let restored =
            UiPublicationsCheckpoint::from_bytes(&saved.to_bytes(limits()).unwrap(), limits())
                .unwrap()
                .prepare(&view, limits())
                .unwrap();
        assert_eq!(
            Arc::ptr_eq(
                restored.published_text.as_ref().unwrap(),
                restored.presented_text.as_ref().unwrap()
            ),
            shared
        );
        assert_eq!(
            Arc::ptr_eq(
                &restored.published_text.as_ref().unwrap().value().runs[0].run,
                &restored.presented_text.as_ref().unwrap().value().runs[0].run
            ),
            shared
        );
        assert_eq!(
            restored
                .checkpoint(&view, limits())
                .unwrap()
                .to_bytes(limits())
                .unwrap(),
            saved.to_bytes(limits()).unwrap()
        );
    }
    let empty = UiLive::default()
        .publication_checkpoint(&view, limits())
        .unwrap();
    let restored = empty.prepare(&view, limits()).unwrap();
    assert!(restored.published.is_none());
    assert!(restored.presented.is_none());
    assert!(restored.attempted.is_none());
    assert!(restored.meshes().unwrap().0.is_empty());
}

#[test]
fn corruption_aliases_late_failures_and_aggregate_limits_preserve_the_source() {
    let fixture = Fixture::new();
    let view = fixture.view();
    let ui = fixture.ui(fixture.variant(true));
    let saved = ui.publication_checkpoint(&view, limits()).unwrap();
    let before = saved.to_bytes(limits()).unwrap();
    for cut in [0, 7, 55, before.len() - 1] {
        assert!(UiPublicationsCheckpoint::from_bytes(&before[..cut], limits()).is_err());
    }
    for offset in [0, 8, 16, 24, before.len() - 1] {
        let mut bytes = before.clone();
        bytes[offset] ^= 1;
        assert!(UiPublicationsCheckpoint::from_bytes(&bytes, limits()).is_err());
    }
    let mut trailing = before.clone();
    trailing.push(0);
    assert!(UiPublicationsCheckpoint::from_bytes(&trailing, limits()).is_err());
    for case in 0..8 {
        let mut bad = saved.clone();
        match case {
            0 => bad.wire.presented.as_mut().unwrap().text = Some(2),
            1 => bad.wire.presented.as_mut().unwrap().text = Some(0),
            2 => bad.wire.published.as_mut().unwrap().text = Some(1),
            3 => bad.wire.publications[1].run_ids = vec![0, 0],
            4 => bad.wire.publications[1].run_ids = vec![0, 3],
            5 => bad.wire.publications[1].run_ids = vec![0],
            6 => {
                let archive = &mut bad.archives[1];
                let last = archive.len() - 1;
                archive[last] ^= 1;
            }
            7 => {
                let mut json: serde_json::Value = serde_json::from_str(
                    &bad.wire
                        .presented
                        .as_ref()
                        .unwrap()
                        .layout
                        .to_json(100000)
                        .unwrap(),
                )
                .unwrap();
                json["layout"]["elements"][0]["owner"] = serde_json::json!("missing-late-owner");
                bad.wire.presented.as_mut().unwrap().layout =
                    SceneUiSnapshotCheckpoint::from_json(&json.to_string(), 100000).unwrap();
            }
            _ => unreachable!(),
        }
        let encoded = bad.to_bytes(limits()).unwrap();
        let bad = UiPublicationsCheckpoint::from_bytes(&encoded, limits()).unwrap();
        assert!(bad.prepare(&view, limits()).is_err(), "case {case}");
    }
    for case in 0..7 {
        let mut tight = limits();
        match case {
            0 => tight.max_bytes = before.len() - 1,
            1 => tight.max_input_bytes = 0,
            2 => tight.max_pixels = 0,
            3 => tight.max_glyphs = 0,
            4 => tight.max_runs = 1,
            5 => tight.max_elements = 2,
            6 => tight.max_layout_bytes = 1,
            _ => unreachable!(),
        }
        assert!(saved.prepare(&view, tight).is_err(), "budget {case}");
        assert!(
            ui.publication_checkpoint(&view, tight).is_err(),
            "capture budget {case}"
        );
    }
    assert_eq!(
        ui.publication_checkpoint(&view, limits())
            .unwrap()
            .to_bytes(limits())
            .unwrap(),
        before
    );
    assert_eq!(fixture.scene.len(), 3);
    let text_bytes = ui_text::checkpoint::encode(
        &fixture.text,
        |node| view.reserved_identity(node).cloned(),
        limits().text,
    )
    .unwrap();
    let mut pool = vec![fixture.text.value().runs[0].run.clone()];
    assert!(
        ui_text::checkpoint::decode_historical_interned(
            &text_bytes,
            &view,
            limits().text,
            &[1, 3],
            &mut pool
        )
        .is_err()
    );
    assert_eq!(pool.len(), 1);
    assert!(Arc::ptr_eq(&pool[0], &fixture.text.value().runs[0].run));
}

#[test]
fn a_shared_run_must_match_exact_data_and_the_current_restore_policy() {
    let fixture = Fixture::new();
    let view = fixture.view();
    let bytes = ui_text::checkpoint::encode(
        &fixture.text,
        |node| view.reserved_identity(node).cloned(),
        limits().text,
    )
    .unwrap();
    let mut changed = fixture.text.value().runs[0].run.checkpoint();
    changed.advance[0] = (f32::from_bits(changed.advance[0]) + 1.).to_bits();
    let different = Arc::new(changed.prepare(16 * 1024 * 1024, 1024).unwrap());
    let mut pool = vec![different.clone()];
    assert!(
        ui_text::checkpoint::decode_historical_interned(
            &bytes,
            &view,
            limits().text,
            &[0, 1],
            &mut pool,
        )
        .is_err()
    );
    assert_eq!(pool.len(), 1);
    assert!(Arc::ptr_eq(&pool[0], &different));
    pool[0] = fixture.text.value().runs[0].run.clone();
    let mut tight = limits().text;
    tight.max_glyphs_per_run = 0;
    assert!(
        ui_text::checkpoint::decode_historical_interned(&bytes, &view, tight, &[0, 1], &mut pool,)
            .is_err()
    );
    assert_eq!(pool.len(), 1);
    assert!(Arc::ptr_eq(&pool[0], &fixture.text.value().runs[0].run));
}
