//! Exact accepted UI text publication, including original input observations.
use super::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use voxy_assets::{ImportedArchiveLimits, ImportedAsset};
use voxy_scene::{NodeId, ObjectId};
use voxy_text::{
    FontLimits, GlyphAtlasCheckpoint, PlacedGlyphCheckpoint, TextFontCheckpoint, TextRun,
    TextRunCheckpoint,
};
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct UiTextArchiveLimits {
    pub imported: ImportedArchiveLimits,
    pub max_payload_bytes: usize,
    pub max_fonts: usize,
    pub max_runs: usize,
    pub max_pixels: usize,
    pub max_glyphs_per_run: usize,
    pub max_total_glyphs: usize,
    pub font: FontLimits,
}
macro_rules! primitive_record {
    ($native:ident,$wire:ident {$($field:ident:$ty:ty),* $(,)?})=>{
        #[derive(Clone,Debug,Serialize,Deserialize)]
        #[serde(deny_unknown_fields)]
        struct $wire{$($field:$ty),*}
        impl From<$native> for $wire {
            fn from(value:$native)->Self {let $native{$($field),*}=value;Self{$($field),*}}
        }
        impl From<$wire> for $native {
            fn from(value:$wire)->Self {let $wire{$($field),*}=value;Self{$($field),*}}
        }
    };
}
primitive_record!(TextFontCheckpoint,FontWire{version:u32,bytes:Vec<u8>,max_font_bytes:usize,max_glyph_pixels:usize,max_size:u32});
primitive_record!(GlyphAtlasCheckpoint,AtlasWire{version:u32,dimensions:[usize;2],alpha:Vec<u8>,cursor:[usize;3],entries:usize,max_entries:usize});
primitive_record!(
    PlacedGlyphCheckpoint,
    GlyphWire {
        origin: [u32; 2],
        region_origin: [usize; 2],
        region_size: [usize; 2],
        cluster: u32
    }
);
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RunWire {
    version: u32,
    atlas: AtlasWire,
    glyphs: Vec<GlyphWire>,
    advance: [u32; 2],
}
impl From<&TextRun> for RunWire {
    fn from(value: &TextRun) -> Self {
        let TextRunCheckpoint {
            version,
            atlas,
            glyphs,
            advance,
        } = value.checkpoint();
        Self {
            version,
            atlas: atlas.into(),
            glyphs: glyphs.into_iter().map(Into::into).collect(),
            advance,
        }
    }
}
impl From<RunWire> for TextRunCheckpoint {
    fn from(value: RunWire) -> Self {
        let RunWire {
            version,
            atlas,
            glyphs,
            advance,
        } = value;
        Self {
            version,
            atlas: atlas.into(),
            glyphs: glyphs.into_iter().map(Into::into).collect(),
            advance,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TextWire {
    font: String,
    content: String,
    size: u32,
    color: [u32; 4],
}
impl From<&voxy_gameplay::UiText> for TextWire {
    fn from(value: &voxy_gameplay::UiText) -> Self {
        let voxy_gameplay::UiText {
            font,
            content,
            size,
            color,
        } = value;
        Self {
            font: font.clone(),
            content: content.clone(),
            size: size.to_bits(),
            color: color.map(f32::to_bits),
        }
    }
}
impl From<TextWire> for voxy_gameplay::UiText {
    fn from(value: TextWire) -> Self {
        Self {
            font: value.font,
            content: value.content,
            size: f32::from_bits(value.size),
            color: value.color.map(f32::from_bits),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Label {
    owner: ObjectId,
    origin: [u32; 2],
    clip: [u32; 4],
    source: TextWire,
    run: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    version: u32,
    fonts: Vec<(String, FontWire)>,
    runs: Vec<RunWire>,
    labels: Vec<Label>,
}
fn invalid(message: &str) -> InputError {
    InputError::Read(message.into())
}
#[allow(dead_code)]
pub(crate) fn encode(
    publication: &ImportedAsset<UiTextPreparation>,
    identify: impl Fn(NodeId) -> Option<ObjectId>,
    limits: UiTextArchiveLimits,
) -> Result<Vec<u8>, EditorError> {
    let UiTextPreparation {
        fonts,
        runs,
        observations,
    } = publication.value();
    if observations != publication.inputs().observations() {
        return Err("UI publication observations disagree".into());
    }
    if fonts.len() > limits.max_fonts || runs.len() > limits.max_runs {
        return Err("UI publication count budget".into());
    }
    let mut archive = Vec::<Arc<TextRun>>::new();
    let mut owners = BTreeSet::new();
    let labels = runs
        .iter()
        .map(|label| {
            let PreparedUiText {
                owner,
                origin,
                clip,
                source,
                run,
            } = label;
            let owner = identify(*owner)
                .filter(|id| !id.0.is_empty())
                .ok_or("unidentified accepted text owner")?;
            if !owners.insert(owner.clone()) {
                return Err("duplicate accepted text owner".into());
            }
            let index = archive
                .iter()
                .position(|old| Arc::ptr_eq(old, run))
                .unwrap_or_else(|| {
                    archive.push(run.clone());
                    archive.len() - 1
                });
            Ok(Label {
                owner,
                origin: origin.map(f32::to_bits),
                clip: clip.map(f32::to_bits),
                source: source.into(),
                run: index,
            })
        })
        .collect::<Result<Vec<_>, EditorError>>()?;
    if !limits.font.max_size.is_finite() || limits.font.max_size <= 0. {
        return Err("invalid UI font policy".into());
    }
    let encoded_runs: Vec<_> = archive
        .iter()
        .map(|run| RunWire::from(run.as_ref()))
        .collect();
    let pixels = encoded_runs
        .iter()
        .try_fold(0usize, |total, run| {
            total.checked_add(run.atlas.alpha.len())
        })
        .ok_or("UI pixel budget overflow")?;
    let glyphs = runs
        .iter()
        .try_fold(0usize, |total, label| {
            total.checked_add(label.run.glyphs().len())
        })
        .ok_or("UI glyph budget overflow")?;
    if pixels > limits.max_pixels
        || glyphs > limits.max_total_glyphs
        || encoded_runs.iter().any(|run| {
            run.glyphs.len() > limits.max_glyphs_per_run
                || run.atlas.max_entries > limits.max_glyphs_per_run
        })
    {
        return Err("UI accepted run budget".into());
    }
    let fonts = fonts
        .iter()
        .map(|(id, font)| {
            let checkpoint = font.checkpoint();
            if checkpoint.bytes.len() > limits.font.max_font_bytes
                || checkpoint.max_font_bytes > limits.font.max_font_bytes
                || checkpoint.max_glyph_pixels > limits.font.max_glyph_pixels
                || f32::from_bits(checkpoint.max_size) > limits.font.max_size
            {
                return Err("UI accepted font policy".into());
            }
            if !observations
                .values()
                .filter_map(|input| input.as_ref().ok())
                .any(|input| input.bytes.as_ref() == checkpoint.bytes.as_slice())
            {
                return Err("UI font has no original input observation".into());
            }
            Ok((id.0.clone(), FontWire::from(checkpoint)))
        })
        .collect::<Result<Vec<_>, EditorError>>()?;
    let wire = Wire {
        version: 1,
        fonts,
        runs: encoded_runs,
        labels,
    };
    let payload = serde_json::to_vec(&wire)?;
    if payload.len() > limits.max_payload_bytes {
        return Err("UI publication payload budget".into());
    }
    publication
        .to_checkpoint_bytes(|_| Ok(payload), limits.imported)
        .map_err(|error| format!("UI publication archive: {error:?}").into())
}
#[allow(dead_code)]
pub(crate) fn decode(
    bytes: &[u8],
    scene: &SceneGraph,
    resolve: impl Fn(&ObjectId) -> Option<NodeId>,
    limits: UiTextArchiveLimits,
) -> Result<ImportedAsset<UiTextPreparation>, EditorError> {
    decode_inner(bytes, scene, resolve, limits, false)
}
#[allow(dead_code)]
pub(crate) fn decode_historical(
    bytes: &[u8],
    view: &voxy_scene::SceneCheckpointView<'_>,
    limits: UiTextArchiveLimits,
) -> Result<ImportedAsset<UiTextPreparation>, EditorError> {
    decode_inner(
        bytes,
        view.graph,
        |id: &ObjectId| view.reserved_node(id),
        limits,
        true,
    )
}
fn decode_inner(
    bytes: &[u8],
    scene: &SceneGraph,
    resolve: impl Fn(&ObjectId) -> Option<NodeId>,
    limits: UiTextArchiveLimits,
    history: bool,
) -> Result<ImportedAsset<UiTextPreparation>, EditorError> {
    decode_inner_with_runs(bytes, scene, resolve, limits, history, |saved| {
        saved
            .prepare(limits.max_pixels, limits.max_glyphs_per_run)
            .map(Arc::new)
            .map_err(|_| invalid("invalid accepted text run"))
    })
}

/// Bundle-local allocation identities. Equal records do not imply shared runs.
/// On any failure neither the caller's pool nor any live publication is changed.
#[allow(dead_code)]
pub(crate) fn decode_historical_interned(
    bytes: &[u8],
    view: &voxy_scene::SceneCheckpointView<'_>,
    limits: UiTextArchiveLimits,
    run_ids: &[usize],
    pool: &mut Vec<Arc<TextRun>>,
) -> Result<ImportedAsset<UiTextPreparation>, EditorError> {
    if run_ids.len() > limits.max_runs
        || run_ids.iter().copied().collect::<BTreeSet<_>>().len() != run_ids.len()
    {
        return Err("invalid UI bundle run identities".into());
    }
    let mut candidate = pool.clone();
    let mut index = 0;
    let publication = decode_inner_with_runs(
        bytes,
        view.graph,
        |id| view.reserved_node(id),
        limits,
        true,
        |saved| {
            let id = *run_ids
                .get(index)
                .ok_or_else(|| invalid("missing UI bundle run identity"))?;
            index += 1;
            if id == candidate.len() {
                let run = saved
                    .prepare(limits.max_pixels, limits.max_glyphs_per_run)
                    .map_err(|_| invalid("invalid accepted text run"))?;
                candidate.push(Arc::new(run));
            } else if candidate
                .get(id)
                .is_none_or(|run| run.checkpoint() != saved)
            {
                return Err(invalid("UI bundle shared run disagrees"));
            }
            Ok(candidate[id].clone())
        },
    )?;
    if index != run_ids.len() {
        return Err("unused UI bundle run identities".into());
    }
    *pool = candidate;
    Ok(publication)
}

fn decode_inner_with_runs(
    bytes: &[u8],
    scene: &SceneGraph,
    resolve: impl Fn(&ObjectId) -> Option<NodeId>,
    limits: UiTextArchiveLimits,
    history: bool,
    mut restore_run: impl FnMut(TextRunCheckpoint) -> Result<Arc<TextRun>, InputError>,
) -> Result<ImportedAsset<UiTextPreparation>, EditorError> {
    ImportedAsset::from_checkpoint_bytes_with_inputs(
        bytes,
        |payload, inputs| {
            if payload.len() > limits.max_payload_bytes {
                return Err(InputError::Capacity);
            }
            let wire: Wire = serde_json::from_slice(payload)
                .map_err(|_| invalid("invalid UI publication encoding"))?;
            if wire.version != 1 {
                return Err(invalid("unsupported UI publication version"));
            }
            if wire.fonts.len() > limits.max_fonts
                || wire.runs.len() > limits.max_runs
                || wire.labels.len() > limits.max_runs
            {
                return Err(InputError::Capacity);
            }
            let mut fonts = BTreeMap::new();
            let mut previous = None;
            for (id, font) in wire.fonts {
                if id.is_empty()
                    || id.len() > 1024
                    || id.chars().any(char::is_control)
                    || previous.as_ref().is_some_and(|old: &String| old >= &id)
                {
                    return Err(invalid("invalid UI font identity"));
                }
                if !inputs
                    .observations()
                    .values()
                    .filter_map(|input| input.as_ref().ok())
                    .any(|input| input.bytes.as_ref() == font.bytes.as_slice())
                {
                    return Err(invalid("archived font has no original input"));
                }
                let font = TextFontCheckpoint::from(font)
                    .prepare(limits.font)
                    .map_err(|_| invalid("invalid archived UI font"))?;
                previous = Some(id.clone());
                fonts.insert(AssetId(id), font);
            }
            let mut pixels = 0_usize;
            let mut archive = Vec::new();
            for run in wire.runs {
                if run.glyphs.len() > limits.max_glyphs_per_run
                    || run.atlas.max_entries > limits.max_glyphs_per_run
                {
                    return Err(InputError::Capacity);
                }
                pixels = pixels
                    .checked_add(run.atlas.alpha.len())
                    .ok_or(InputError::Capacity)?;
                if pixels > limits.max_pixels {
                    return Err(InputError::Capacity);
                }
                archive.push(restore_run(TextRunCheckpoint::from(run))?);
            }
            let mut glyphs = 0_usize;
            let mut text_bytes = 0_usize;
            let mut owners = BTreeSet::new();
            let mut ids = BTreeSet::new();
            let mut used = BTreeSet::new();
            let mut runs = Vec::new();
            for label in wire.labels {
                if label.owner.0.is_empty() || !ids.insert(label.owner.clone()) {
                    return Err(invalid("duplicate accepted UI identity"));
                }
                let owner =
                    resolve(&label.owner).ok_or_else(|| invalid("missing accepted UI owner"))?;
                if history {
                    if !scene.owns_handle(owner) {
                        return Err(invalid("foreign historical text owner"));
                    }
                } else {
                    scene
                        .local(owner)
                        .map_err(|_| invalid("foreign accepted UI owner"))?;
                }
                if !owners.insert(owner) {
                    return Err(invalid("aliased accepted UI owner"));
                }
                let run = archive
                    .get(label.run)
                    .ok_or_else(|| invalid("invalid accepted run reference"))?
                    .clone();
                if used.insert(label.run) && label.run != used.len() - 1 {
                    return Err(invalid("noncanonical accepted run order"));
                }
                glyphs = glyphs
                    .checked_add(run.glyphs().len())
                    .ok_or(InputError::Capacity)?;
                text_bytes = text_bytes
                    .checked_add(label.source.content.len())
                    .ok_or(InputError::Capacity)?;
                if glyphs > limits.max_total_glyphs || text_bytes > 65536 {
                    return Err(InputError::Capacity);
                }
                let source: voxy_gameplay::UiText = label.source.into();
                let origin = label.origin.map(f32::from_bits);
                let clip = label.clip.map(f32::from_bits);
                let descriptor = UiElement {
                    origin: [0.; 2],
                    size: [1.; 2],
                    color: [1.; 4],
                    layer: 0,
                    enabled: true,
                    action: None,
                    text: Some(source.clone()),
                };
                if !descriptor.valid()
                    || !fonts.contains_key(&AssetId(source.font.clone()))
                    || origin.iter().chain(&clip).any(|v| !v.is_finite())
                    || clip[2] <= 0.
                    || clip[3] <= 0.
                    || !(clip[0] + clip[2]).is_finite()
                    || !(clip[1] + clip[3]).is_finite()
                    || run.glyphs().iter().any(|glyph| {
                        glyph.cluster as usize >= source.content.len()
                            || !source.content.is_char_boundary(glyph.cluster as usize)
                    })
                {
                    return Err(invalid("invalid accepted UI label"));
                }
                runs.push(PreparedUiText {
                    owner,
                    origin,
                    clip,
                    source,
                    run,
                });
            }
            if used.len() != archive.len() {
                return Err(invalid("unreferenced accepted UI run"));
            }
            Ok(UiTextPreparation {
                fonts,
                runs,
                observations: inputs.observations().clone(),
            })
        },
        limits.imported,
    )
    .map_err(|error| format!("UI publication restore: {error:?}").into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxy_scene::Transform;
    fn limits() -> UiTextArchiveLimits {
        UiTextArchiveLimits {
            imported: ImportedArchiveLimits {
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
            font: FontLimits {
                max_font_bytes: 4 * 1024 * 1024,
                max_glyph_pixels: 16384,
                max_size: 128.,
            },
        }
    }
    struct Temp(std::path::PathBuf);
    impl Temp {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "voxy-ui-text-archive-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
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
    fn fixture() -> (
        Temp,
        SceneGraph,
        Vec<NodeId>,
        Arc<ImportedAsset<UiTextPreparation>>,
    ) {
        let temp = Temp::new();
        let root = temp.0.join("source");
        std::fs::create_dir(&root).unwrap();
        let font = [
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "C:/Windows/Fonts/arial.ttf",
        ]
        .iter()
        .find_map(|path| std::fs::read(path).ok())
        .expect("Unicode font fixture required");
        std::fs::write(root.join("font.ttf"), &font).unwrap();
        std::fs::write(root.join("font-alias.ttf"), &font).unwrap();
        std::fs::write(root.join("assets.json"),r#"{"version":1,"assets":[{"asset":"interface","source":"font.ttf"},{"asset":"alias","source":"font-alias.ttf"}]}"#).unwrap();
        let project = AuthoringProject::new(
            &root,
            &crate::InputRecipe::Manifest(AssetId("assets.json".into())),
        )
        .unwrap();
        let mut scene = SceneGraph::new(8);
        let mut nodes = Vec::new();
        for i in 0..4 {
            let node = scene.spawn(None, Transform::default()).unwrap();
            scene
                .insert_component(
                    node,
                    UiElement {
                        origin: [0., i as f32 * 0.25],
                        size: [1., 0.25],
                        color: [0.1, 0.2, 0.3, 1.],
                        layer: 0,
                        enabled: true,
                        action: None,
                        text: Some(voxy_gameplay::UiText {
                            font: if i < 2 { "interface" } else { "alias" }.into(),
                            content: "office e\u{301} café".into(),
                            size: 18.,
                            color: [1.; 4],
                        }),
                    },
                )
                .unwrap();
            nodes.push(node);
        }
        let publication = UiTextPreparation::prepare_import(&scene, &project, [512.; 2]).unwrap();
        (temp, scene, nodes, publication)
    }
    fn identify(nodes: &[NodeId], node: NodeId) -> Option<ObjectId> {
        nodes
            .iter()
            .position(|old| *old == node)
            .map(|i| ObjectId(format!("owner-{i}")))
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
    #[test]
    fn real_file_restores_original_font_observations_arc_sharing_and_120_draws_without_sources() {
        let (temp, scene, nodes, original) = fixture();
        assert!(Arc::ptr_eq(
            &original.value().runs[0].run,
            &original.value().runs[1].run
        ));
        assert!(Arc::ptr_eq(
            &original.value().runs[2].run,
            &original.value().runs[3].run
        ));
        assert!(!Arc::ptr_eq(
            &original.value().runs[0].run,
            &original.value().runs[2].run
        ));
        assert_eq!(
            original.value().runs[0].run.checkpoint(),
            original.value().runs[2].run.checkpoint()
        );
        let bytes = encode(&original, |node| identify(&nodes, node), limits()).unwrap();
        let path = temp.0.join("publication.bin");
        std::fs::write(&path, &bytes).unwrap();
        std::fs::remove_dir_all(temp.0.join("source")).unwrap();
        let bytes = std::fs::read(path).unwrap();
        let mut target = SceneGraph::new(8);
        let mut fresh = vec![nodes[0]; 4];
        for i in (0..4).rev() {
            let owner = target.spawn(None, Transform::default()).unwrap();
            target
                .insert_component(
                    owner,
                    scene
                        .component::<UiElement>(nodes[i])
                        .unwrap()
                        .unwrap()
                        .clone(),
                )
                .unwrap();
            fresh[i] = owner;
        }
        let resolve = |id: &ObjectId| {
            fresh
                .iter()
                .enumerate()
                .find(|(i, _)| id.0 == format!("owner-{i}"))
                .map(|(_, node)| *node)
        };
        let restored = decode(&bytes, &target, resolve, limits()).unwrap();
        assert_eq!(
            restored.inputs().observations(),
            original.inputs().observations()
        );
        assert_eq!(restored.value().observations, original.value().observations);
        for (id, input) in &restored.value().observations {
            assert!(Arc::ptr_eq(
                &input.as_ref().unwrap().bytes,
                &restored.inputs().observations()[id].as_ref().unwrap().bytes
            ));
        }
        let runs = &restored.value().runs;
        assert!(Arc::ptr_eq(&runs[0].run, &runs[1].run));
        assert!(Arc::ptr_eq(&runs[2].run, &runs[3].run));
        assert!(!Arc::ptr_eq(&runs[0].run, &runs[2].run));
        assert_eq!(
            encode(&restored, |node| identify(&fresh, node), limits()).unwrap(),
            bytes
        );
        let snapshot = voxy_gameplay::extract_scene_ui(&scene, [512.; 2], 128).unwrap();
        let snapshot_record = snapshot
            .checkpoint(|node| identify(&nodes, node), 100000)
            .unwrap();
        let mapped = snapshot_record
            .prepare(&target, resolve, 128, 100000)
            .unwrap();
        for i in 0..120 {
            let mut a = snapshot.clone();
            let mut b = mapped.clone();
            for view in [&mut a, &mut b] {
                view.viewport = [512. + i as f32, 512.];
                for element in &mut view.elements {
                    element.descriptor.color[0] = (i % 10) as f32 * 0.1;
                    element.clip.as_mut().unwrap()[2] = 20. + (i % 40) as f32;
                }
            }
            let left = crate::ui_draw::build(&a, &original.value().runs).unwrap();
            let right = crate::ui_draw::build(&b, runs).unwrap();
            assert_eq!(left.len(), right.len());
            for (a, b) in left.iter().zip(&right) {
                assert_eq!(geometry(&a.mesh), geometry(&b.mesh));
                assert_eq!(a.mesh.indices(), b.mesh.indices());
                assert_eq!(identify(&nodes, a.owner), identify(&fresh, b.owner));
                if let (Some(a), Some(b)) = (&a.atlas, &b.atlas) {
                    assert_eq!(a.checkpoint(), b.checkpoint());
                }
            }
        }
        let before = encode(&original, |node| identify(&nodes, node), limits()).unwrap();
        assert!(decode(&bytes, &target, |_| None, limits()).is_err());
        assert!(decode(&bytes, &target, |_| Some(fresh[0]), limits()).is_err());
        assert_eq!(
            encode(&original, |node| identify(&nodes, node), limits()).unwrap(),
            before
        );
    }
    #[test]
    fn retired_visual_owners_restore_as_history_without_reviving_scene_objects() {
        let (temp, mut scene, nodes, original) = fixture();
        let snapshot = voxy_gameplay::extract_scene_ui(&scene, [512.; 2], 128).unwrap();
        scene.remove_subtree(nodes[0]).unwrap();
        scene.remove_subtree(nodes[2]).unwrap();
        let replacement = scene.spawn(None, Transform::default()).unwrap();
        let mut bindings: Vec<_> = nodes
            .iter()
            .enumerate()
            .map(|(i, node)| (ObjectId(format!("owner-{i}")), *node))
            .collect();
        bindings.push((ObjectId("replacement".into()), replacement));
        let view = voxy_scene::SceneCheckpointView::new(&scene, bindings).unwrap();
        let identify = |node| view.reserved_identity(node).cloned();
        let bytes = encode(&original, identify, limits()).unwrap();
        let layout = snapshot.checkpoint(identify, 100000).unwrap();
        let mut registry = voxy_scene::ComponentRegistry::default();
        voxy_gameplay::register_components(&mut registry).unwrap();
        let policy = voxy_scene::SimulationLimits {
            fixed_step: 0.01,
            max_steps: 4,
            max_behaviors: 4,
            max_commands: 4,
        };
        let simulation = voxy_scene::SceneSimulation::new(&scene, policy).unwrap();
        let saved = simulation
            .checkpoint_view(&view, &registry, 100000)
            .unwrap();
        let (target, _) = saved
            .prepare(
                &registry,
                &voxy_scene::BehaviorCheckpointRegistry::default(),
                policy,
                8,
                100000,
            )
            .unwrap();
        std::fs::remove_dir_all(temp.0.join("source")).unwrap();
        let target_view = target.checkpoint_view();
        assert!(
            decode(
                &bytes,
                &target.graph,
                |id| target.reserved_node(id),
                limits()
            )
            .is_err()
        );
        assert!(
            layout
                .prepare(&target.graph, |id| target.reserved_node(id), 128, 100000)
                .is_err()
        );
        let restored = decode_historical(&bytes, &target_view, limits()).unwrap();
        let accepted = layout
            .prepare_historical(&target_view, 128, 100000)
            .unwrap();
        assert_eq!(
            encode(
                &restored,
                |node| target_view.reserved_identity(node).cloned(),
                limits()
            )
            .unwrap(),
            bytes
        );
        let old = target.reserved_node(&ObjectId("owner-0".into())).unwrap();
        let other = target.reserved_node(&ObjectId("owner-2".into())).unwrap();
        assert_ne!(old, other);
        assert!(target.graph.local(old).is_err());
        assert!(target.resolve(&ObjectId("owner-0".into())).is_none());
        let left = crate::ui_draw::build(&snapshot, &original.value().runs).unwrap();
        let right = crate::ui_draw::build(&accepted, &restored.value().runs).unwrap();
        for (a, b) in left.iter().zip(&right) {
            assert_eq!(geometry(&a.mesh), geometry(&b.mesh));
            assert_eq!(a.mesh.indices(), b.mesh.indices());
        }
        assert_eq!(left.len(), right.len());
        assert_eq!(target.graph.len(), 3);
        let mut actions = voxy_gameplay::UiActionHandlers::new(&target.graph, 4, 4, 4);
        assert!(
            actions
                .enqueue(
                    &target.graph,
                    voxy_gameplay::UiActionEvent {
                        owner: old,
                        action: "not-live".into()
                    }
                )
                .is_err()
        );
    }

    fn payload(bytes: &[u8]) -> Wire {
        let start = 56 + u64::from_le_bytes(bytes[8..16].try_into().unwrap()) as usize;
        serde_json::from_slice(&bytes[start..]).unwrap()
    }
    #[test]
    fn malformed_payloads_and_tight_policies_reject_without_reinterpreting_accepted_runs() {
        let (_temp, scene, nodes, original) = fixture();
        let bytes = encode(&original, |node| identify(&nodes, node), limits()).unwrap();
        let resolve = |id: &ObjectId| {
            nodes
                .iter()
                .enumerate()
                .find(|(i, _)| id.0 == format!("owner-{i}"))
                .map(|(_, node)| *node)
        };
        for cap in [
            UiTextArchiveLimits {
                max_fonts: 1,
                ..limits()
            },
            UiTextArchiveLimits {
                max_runs: 1,
                ..limits()
            },
            UiTextArchiveLimits {
                max_pixels: 1,
                ..limits()
            },
            UiTextArchiveLimits {
                max_glyphs_per_run: 1,
                ..limits()
            },
            UiTextArchiveLimits {
                max_total_glyphs: 1,
                ..limits()
            },
            UiTextArchiveLimits {
                max_payload_bytes: 1,
                ..limits()
            },
        ] {
            assert!(decode(&bytes, &scene, resolve, cap).is_err());
            assert!(encode(&original, |node| identify(&nodes, node), cap).is_err());
        }
        for n in [0, 7, 55, 56, bytes.len() / 2, bytes.len() - 1] {
            assert!(decode(&bytes[..n], &scene, resolve, limits()).is_err());
        }
        let mut corrupt = bytes.clone();
        let midpoint = corrupt.len() / 2;
        corrupt[midpoint] ^= 1;
        assert!(decode(&corrupt, &scene, resolve, limits()).is_err());
        for mode in 0..8 {
            let mut wire = payload(&bytes);
            match mode {
                0 => wire.version = 2,
                1 => wire.labels[0].run = usize::MAX,
                2 => wire.labels[1].owner = wire.labels[0].owner.clone(),
                3 => wire.runs[0].glyphs[0].origin[0] = f32::NAN.to_bits(),
                4 => wire.runs[0].glyphs[0].cluster = u32::MAX,
                5 => wire.fonts[0].1.bytes.push(0),
                6 => wire.runs.push(wire.runs[0].clone()),
                _ => {
                    wire.labels[0].clip[0] = f32::MAX.to_bits();
                    wire.labels[0].clip[2] = f32::MAX.to_bits();
                }
            }
            let bad = original
                .to_checkpoint_bytes(
                    |_| Ok(serde_json::to_vec(&wire).unwrap()),
                    limits().imported,
                )
                .unwrap();
            assert!(
                decode(&bad, &scene, resolve, limits()).is_err(),
                "accepted malformed case {mode}"
            );
        }
    }
}
