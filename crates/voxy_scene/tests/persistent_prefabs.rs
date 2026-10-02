use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use voxy_scene::{
    ComponentRegistry, DocumentError, ObjectId, ObjectOverride, PrefabInstance, PrefabLimits,
    PrefabSceneDocument, SceneObject, instance_object_id, read_prefab_scene_file,
    save_prefab_scene_file,
};
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
struct Link {
    target: ObjectId,
    label: String,
}
fn registry(remapping: bool) -> ComponentRegistry {
    let mut registry = ComponentRegistry::default();
    if remapping {
        registry
            .register_with_reference_remap::<Link>(
                "link.v1",
                |link| vec![link.target.clone()],
                |link, ids| {
                    if let Some(target) = ids.get(&link.target) {
                        link.target = target.clone();
                    }
                },
            )
            .unwrap();
    } else {
        registry
            .register_with_references::<Link>("link.v1", |link| vec![link.target.clone()])
            .unwrap();
    }
    registry
}
fn id(value: &str) -> ObjectId {
    ObjectId(value.into())
}

#[test]
fn identified_collection_edits_survive_source_insertion_and_reordering() {
    let mut registry = ComponentRegistry::default();
    registry.register::<serde_json::Value>("list.v1").unwrap();
    registry
        .declare_identified_collection("list.v1", "/items", "id")
        .unwrap();
    let mut leaf = object("root");
    leaf.components.insert("list.v1".into(), serde_json::json!({"items":[
        {"id":"a","label":"A","gain":1}, {"id":"b","label":"B","gain":1}, {"id":"c","label":"C","gain":1}
    ],"enabled":true}));
    let mut assets = BTreeMap::from([("leaf".into(), document(vec![leaf]))]);
    let source = document(vec![]);
    let mut source = source;
    source.instances.push(instance("outer", "leaf"));
    let baseline = source
        .instance_baseline(&registry, PrefabLimits::default(), |asset| {
            resolve(&assets, asset)
        })
        .unwrap();
    let mut edited = baseline.clone();
    edited.objects[0].components.get_mut("list.v1").unwrap()["items"] = serde_json::json!([
        {"id":"c","label":"local C","gain":1}, {"id":"a","label":"A","gain":1}, {"id":"local","label":"added","gain":2}
    ]);
    let captured = source
        .capture_edits(&baseline, &edited, &registry, 128)
        .unwrap();
    let changes =
        &captured.instances[0].overrides[&id("root")].component_collections["list.v1"]["/items"];
    assert_eq!(
        changes.removed,
        std::collections::BTreeSet::from(["b".into()])
    );
    assert_eq!(changes.members["c"]["/label"], "local C");
    assert!(changes.added.contains_key("local"));
    let captured: PrefabSceneDocument =
        serde_json::from_str(&serde_json::to_string(&captured).unwrap()).unwrap();
    assets.get_mut("leaf").unwrap().objects[0]
        .components
        .get_mut("list.v1")
        .unwrap()["items"] = serde_json::json!([
        {"id":"new","label":"new source","gain":3}, {"id":"a","label":"A","gain":1},
        {"id":"b","label":"B","gain":9}, {"id":"c","label":"source C","gain":7}
    ]);
    let expanded = captured
        .expand(&registry, PrefabLimits::default(), |asset| {
            resolve(&assets, asset)
        })
        .unwrap();
    let items = expanded.document.objects[0].components["list.v1"]["items"]
        .as_array()
        .unwrap();
    assert_eq!(
        items
            .iter()
            .map(|item| item["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["c", "new", "a", "local"]
    );
    assert_eq!(items[0]["label"], "local C");
    assert_eq!(items[0]["gain"], 7);
    // Source disappearance must not erase the instance's deletion tombstone.
    assets.get_mut("leaf").unwrap().objects[0]
        .components
        .get_mut("list.v1")
        .unwrap()["items"]
        .as_array_mut()
        .unwrap()
        .retain(|item| item["id"] != "b");
    let base = captured
        .instance_baseline(&registry, PrefabLimits::default(), |asset| {
            resolve(&assets, asset)
        })
        .unwrap();
    let current = captured
        .expand(&registry, PrefabLimits::default(), |asset| {
            resolve(&assets, asset)
        })
        .unwrap();
    let recaptured = captured
        .capture_edits(&base, &current.document, &registry, 128)
        .unwrap();
    assets.get_mut("leaf").unwrap().objects[0]
        .components
        .get_mut("list.v1")
        .unwrap()["items"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"id":"b","label":"returned","gain":4}));
    let expanded = recaptured
        .expand(&registry, PrefabLimits::default(), |asset| {
            resolve(&assets, asset)
        })
        .unwrap();
    assert!(
        !expanded.document.objects[0].components["list.v1"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["id"] == "b")
    );
}

#[test]
fn identified_collection_tombstone_survives_recapture_with_no_other_changes() {
    let mut registry = ComponentRegistry::default();
    registry.register::<serde_json::Value>("list.v1").unwrap();
    registry
        .declare_identified_collection("list.v1", "/items", "id")
        .unwrap();
    let mut root = object("root");
    root.components
        .insert("list.v1".into(), serde_json::json!({"items":[{"id":"a"}]}));
    let mut assets = BTreeMap::from([("leaf".into(), document(vec![root]))]);
    let mut source = document(vec![]);
    source.instances.push(instance("outer", "leaf"));
    let base = source
        .instance_baseline(&registry, PrefabLimits::default(), |asset| {
            resolve(&assets, asset)
        })
        .unwrap();
    let mut edited = base.clone();
    edited.objects[0].components.get_mut("list.v1").unwrap()["items"] = serde_json::json!([]);
    let captured = source
        .capture_edits(&base, &edited, &registry, 128)
        .unwrap();
    let original = assets["leaf"].clone();
    assets.get_mut("leaf").unwrap().objects[0]
        .components
        .get_mut("list.v1")
        .unwrap()["items"] = serde_json::json!([]);
    let base = captured
        .instance_baseline(&registry, PrefabLimits::default(), |asset| {
            resolve(&assets, asset)
        })
        .unwrap();
    let recaptured = captured
        .capture_edits(&base, &base, &registry, 128)
        .unwrap();
    assets.insert("leaf".into(), original);
    let result = recaptured
        .expand(&registry, PrefabLimits::default(), |asset| {
            resolve(&assets, asset)
        })
        .unwrap();
    assert_eq!(
        result.document.objects[0].components["list.v1"]["items"],
        serde_json::json!([])
    );
}

#[test]
fn collection_declarations_reject_unstable_paths_and_duplicate_ids() {
    let mut registry = ComponentRegistry::default();
    registry.register::<serde_json::Value>("list.v1").unwrap();
    assert!(
        registry
            .declare_identified_collection("missing", "/items", "id")
            .is_err()
    );
    assert!(
        registry
            .declare_identified_collection("list.v1", "/bad~2", "id")
            .is_err()
    );
    registry
        .declare_identified_collection("list.v1", "/items", "id")
        .unwrap();
    assert!(
        registry
            .declare_identified_collection("list.v1", "/items/nested", "id")
            .is_err()
    );
    let mut root = object("root");
    root.components.insert(
        "list.v1".into(),
        serde_json::json!({"items":[{"id":"a"},{"id":"a"}]}),
    );
    assert!(
        document(vec![root])
            .expand(&registry, PrefabLimits::default(), |_| Err(
                DocumentError::Invalid("unused".into())
            ))
            .is_err()
    );
}

#[test]
fn identified_collection_members_and_additions_remap_object_references() {
    let mut registry = ComponentRegistry::default();
    registry
        .register_with_reference_remap::<serde_json::Value>(
            "list.v1",
            |value| {
                value["items"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|item| ObjectId(item["target"].as_str().unwrap().into()))
                    .collect()
            },
            |value, ids| {
                for item in value["items"].as_array_mut().unwrap() {
                    let old = ObjectId(item["target"].as_str().unwrap().into());
                    if let Some(new) = ids.get(&old) {
                        item["target"] = serde_json::json!(new.0);
                    }
                }
            },
        )
        .unwrap();
    registry
        .declare_identified_collection("list.v1", "/items", "id")
        .unwrap();
    let mut root = object("root");
    root.components.insert(
        "list.v1".into(),
        serde_json::json!({"items":[{"id":"a","target":"root"}]}),
    );
    let assets = BTreeMap::from([("leaf".into(), document(vec![root, object("other")]))]);
    let mut source = document(vec![]);
    source.instances.push(instance("outer", "leaf"));
    let base = source
        .instance_baseline(&registry, PrefabLimits::default(), |asset| {
            resolve(&assets, asset)
        })
        .unwrap();
    let mut edited = base.clone();
    let other = instance_object_id(&id("outer"), &id("other"));
    edited
        .objects
        .iter_mut()
        .find(|object| object.components.contains_key("list.v1"))
        .unwrap()
        .components
        .get_mut("list.v1")
        .unwrap()["items"] = serde_json::json!([
        {"id":"a","target":other.0}, {"id":"local","target":other.0}
    ]);
    let captured = source
        .capture_edits(&base, &edited, &registry, 128)
        .unwrap();
    let changes =
        &captured.instances[0].overrides[&id("root")].component_collections["list.v1"]["/items"];
    assert_eq!(changes.members["a"]["/target"], "other");
    assert_eq!(changes.added["local"]["target"], "other");
    let result = captured
        .expand(&registry, PrefabLimits::default(), |asset| {
            resolve(&assets, asset)
        })
        .unwrap();
    let component = &result
        .document
        .objects
        .iter()
        .find(|object| object.components.contains_key("list.v1"))
        .unwrap()
        .components["list.v1"];
    assert!(
        component["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["target"] == other.0)
    );
}
fn object(value: &str) -> SceneObject {
    SceneObject {
        id: id(value),
        parent: None,
        name: value.into(),
        active: true,
        translation: [0.0; 3],
        rotation: [0.0, 0.0, 0.0, 1.0],
        scale: [1.0; 3],
        components: BTreeMap::new(),
    }
}
fn document(objects: Vec<SceneObject>) -> PrefabSceneDocument {
    PrefabSceneDocument {
        version: 1,
        objects,
        instances: vec![],
    }
}
fn instance(name: &str, asset: &str) -> PrefabInstance {
    PrefabInstance {
        id: id(name),
        asset: asset.into(),
        parent: None,
        overrides: BTreeMap::new(),
    }
}
fn fixture() -> BTreeMap<String, PrefabSceneDocument> {
    let mut follower = object("follower");
    follower.parent = Some(id("root"));
    follower.components.insert(
        "link.v1".into(),
        serde_json::to_value(Link {
            target: id("root"),
            label: "root".into(),
        })
        .unwrap(),
    );
    let leaf = document(vec![object("root"), follower]);
    let mut assembly = document(vec![object("assembly")]);
    let mut nested = instance("nested", "leaf");
    nested.parent = Some(id("assembly"));
    assembly.instances.push(nested);
    BTreeMap::from([("leaf".into(), leaf), ("assembly".into(), assembly)])
}
fn resolve(
    assets: &BTreeMap<String, PrefabSceneDocument>,
    asset: &str,
) -> Result<PrefabSceneDocument, DocumentError> {
    assets
        .get(asset)
        .cloned()
        .ok_or_else(|| DocumentError::Invalid(format!("missing prefab asset {asset}")))
}
#[test]
fn malformed_source_identity_cannot_be_hidden_by_instance_framing() {
    let registry = registry(true);
    let mut scene = document(vec![]);
    scene.instances.push(instance("outer", "invalid"));
    for objects in [vec![object("")], vec![object("same"), object("same")]] {
        let source = document(objects);
        assert!(
            scene
                .expand(&registry, PrefabLimits::default(), |_| Ok(source.clone()))
                .is_err()
        );
        assert!(
            scene
                .instance_baseline(&registry, PrefabLimits::default(), |_| Ok(source.clone()))
                .is_err()
        );
        // Baseline topology may remain repairable, but identity cannot be ambiguous.
        assert!(
            source
                .instance_baseline(&registry, PrefabLimits::default(), |_| unreachable!())
                .is_err()
        );
    }
}

#[test]
fn member_override_inherits_source_siblings_after_nested_roundtrip_and_reset() {
    let registry = registry(true);
    let mut assets = fixture();
    let mut source = document(vec![]);
    source.instances.push(instance("outer", "assembly"));
    let baseline = source
        .instance_baseline(&registry, PrefabLimits::default(), |asset| {
            resolve(&assets, asset)
        })
        .unwrap();
    let local = instance_object_id(&id("nested"), &id("follower"));
    let global = instance_object_id(&id("outer"), &local);
    let mut edited = baseline.clone();
    let follower = edited
        .objects
        .iter_mut()
        .find(|object| object.id == global)
        .unwrap();
    follower.components.get_mut("link.v1").unwrap()["label"] = serde_json::json!("local label");
    let captured = source
        .capture_edits(&baseline, &edited, &registry, 128)
        .unwrap();
    assert!(
        captured.instances[0].overrides[&local]
            .components
            .is_empty()
    );
    assert_eq!(
        captured.instances[0].overrides[&local].component_members["link.v1"].len(),
        1
    );
    let captured: PrefabSceneDocument =
        serde_json::from_slice(&serde_json::to_vec(&captured).unwrap()).unwrap();
    assets.get_mut("leaf").unwrap().objects[1]
        .components
        .insert(
            "link.v1".into(),
            serde_json::json!({"target":"follower", "label":"new source label"}),
        );
    let expanded = captured
        .expand(&registry, PrefabLimits::default(), |asset| {
            resolve(&assets, asset)
        })
        .unwrap()
        .document;
    let link: Link = serde_json::from_value(
        expanded
            .objects
            .iter()
            .find(|object| object.id == global)
            .unwrap()
            .components["link.v1"]
            .clone(),
    )
    .unwrap();
    assert_eq!(link.label, "local label");
    assert_eq!(link.target, global);
    let current_base = captured
        .instance_baseline(&registry, PrefabLimits::default(), |asset| {
            resolve(&assets, asset)
        })
        .unwrap();
    let reset = captured
        .capture_edits(&current_base, &current_base, &registry, 128)
        .unwrap();
    assert!(reset.instances[0].overrides.is_empty());
}

#[test]
fn legacy_whole_component_override_retains_its_atomic_semantics() {
    let registry = registry(true);
    let assets = fixture();
    let mut source = document(vec![]);
    let mut instance = instance("outer", "leaf");
    instance.overrides.insert(
        id("follower"),
        ObjectOverride {
            components: BTreeMap::from([(
                "link.v1".into(),
                Some(serde_json::json!({"target":"root","label":"legacy"})),
            )]),
            ..ObjectOverride::default()
        },
    );
    source.instances.push(instance);
    let baseline = source
        .instance_baseline(&registry, PrefabLimits::default(), |asset| {
            resolve(&assets, asset)
        })
        .unwrap();
    let edited = source
        .expand(&registry, PrefabLimits::default(), |asset| {
            resolve(&assets, asset)
        })
        .unwrap()
        .document;
    let recaptured = source
        .capture_edits(&baseline, &edited, &registry, 128)
        .unwrap();
    assert!(
        recaptured.instances[0].overrides[&id("follower")]
            .component_members
            .is_empty()
    );
    assert_eq!(
        recaptured.instances[0].overrides[&id("follower")].components,
        source.instances[0].overrides[&id("follower")].components
    );
}

#[test]
fn member_paths_escape_names_keep_null_and_arrays_atomic_and_reject_stale_paths() {
    let mut registry = ComponentRegistry::default();
    registry.register::<serde_json::Value>("data.v1").unwrap();
    let mut node = object("node");
    node.components.insert(
        "data.v1".into(),
        serde_json::json!({
            "arr":[1,2], "nested":{"a/b~c":1,"other":2}, "nullable":1
        }),
    );
    let mut asset = document(vec![node]);
    let mut source = document(vec![]);
    source.instances.push(instance("outer", "data"));
    let baseline = source
        .instance_baseline(&registry, PrefabLimits::default(), |_| Ok(asset.clone()))
        .unwrap();
    let mut edited = baseline.clone();
    *edited.objects[0].components.get_mut("data.v1").unwrap() = serde_json::json!({
        "arr":[1,3], "nested":{"a/b~c":4,"other":2}, "nullable":null
    });
    let captured = source
        .capture_edits(&baseline, &edited, &registry, 128)
        .unwrap();
    let members = &captured.instances[0].overrides[&id("node")].component_members["data.v1"];
    assert_eq!(members["/arr"], serde_json::json!([1, 3]));
    assert_eq!(members["/nested/a~1b~0c"], serde_json::json!(4));
    assert!(members["/nullable"].is_null());
    *asset.objects[0].components.get_mut("data.v1").unwrap() = serde_json::json!({
        "arr":[5,6], "nested":{"a/b~c":8,"other":9}, "nullable":7
    });
    let expanded = captured
        .expand(&registry, PrefabLimits::default(), |_| Ok(asset.clone()))
        .unwrap();
    assert_eq!(
        expanded.document.objects[0].components["data.v1"],
        serde_json::json!({
            "arr":[1,3], "nested":{"a/b~c":4,"other":9}, "nullable":null
        })
    );
    for path in ["/arr/0", "/missing", "/nested/~2bad", ""] {
        let mut invalid = captured.clone();
        invalid.instances[0]
            .overrides
            .get_mut(&id("node"))
            .unwrap()
            .component_members
            .get_mut("data.v1")
            .unwrap()
            .insert(path.into(), serde_json::json!(0));
        assert!(
            invalid
                .expand(&registry, PrefabLimits::default(), |_| Ok(asset.clone()))
                .is_err()
        );
    }
    let mut missing = asset.clone();
    missing.objects[0].components.clear();
    assert!(
        captured
            .expand(&registry, PrefabLimits::default(), |_| Ok(missing.clone()))
            .is_err()
    );
}
#[test]
fn nested_instances_override_remap_and_roundtrip_preserve_durable_identity() {
    let assets = fixture();
    let registry = registry(true);
    let mut scene = document(vec![object("world")]);
    let mut a = instance("a", "assembly");
    a.parent = Some(id("world"));
    let local_root = instance_object_id(&id("nested"), &id("root"));
    a.overrides.insert(
        local_root.clone(),
        ObjectOverride {
            name: Some("edited".into()),
            translation: Some([5.0, 0.0, 0.0]),
            ..ObjectOverride::default()
        },
    );
    scene.instances = vec![a, instance("b", "assembly")];
    let expanded = scene
        .expand(&registry, PrefabLimits::default(), |asset| {
            resolve(&assets, asset)
        })
        .unwrap();
    assert_eq!(expanded.dependencies.len(), 2);
    assert_eq!(expanded.document.objects.len(), 7);
    let loaded = expanded.document.load(&registry, 7).unwrap();
    let a_root_id = instance_object_id(&id("a"), &local_root);
    let b_root_id = instance_object_id(&id("b"), &local_root);
    let a_root = loaded.resolve(&a_root_id).unwrap();
    let b_root = loaded.resolve(&b_root_id).unwrap();
    assert_ne!(a_root, b_root);
    assert_eq!(loaded.graph.name(a_root).unwrap(), "edited");
    assert_eq!(loaded.graph.name(b_root).unwrap(), "root");
    let follower = loaded
        .resolve(&instance_object_id(
            &id("a"),
            &instance_object_id(&id("nested"), &id("follower")),
        ))
        .unwrap();
    let link = loaded.graph.component::<Link>(follower).unwrap().unwrap();
    assert_eq!(link.target, a_root_id);
    assert_eq!(link.label, "root"); // An equal ordinary string is not remapped.
    let folder = std::env::temp_dir().join(format!("voxy-prefab-test-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("scene.json");
    save_prefab_scene_file(
        &path,
        &scene,
        &registry,
        PrefabLimits::default(),
        64 * 1024,
        |asset| resolve(&assets, asset),
    )
    .unwrap();
    assert_eq!(read_prefab_scene_file(&path, 64 * 1024).unwrap(), scene);
    let before = std::fs::read(&path).unwrap();
    let mut invalid = scene.clone();
    invalid.instances[0]
        .overrides
        .insert(id("missing"), ObjectOverride::default());
    assert!(
        save_prefab_scene_file(
            &path,
            &invalid,
            &registry,
            PrefabLimits::default(),
            64 * 1024,
            |asset| resolve(&assets, asset)
        )
        .is_err()
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let reloaded = scene
        .load(&registry, PrefabLimits::default(), |asset| {
            resolve(&assets, asset)
        })
        .unwrap();
    assert!(reloaded.resolve(&a_root_id).is_some());
    assert_ne!(reloaded.resolve(&a_root_id).unwrap(), a_root);
    std::fs::remove_dir_all(folder).unwrap();
}
#[test]
fn cycles_quotas_collisions_unknown_overrides_and_missing_remappers_are_rejected() {
    let mut assets = fixture();
    let registry = registry(true);
    let mut scene = document(vec![]);
    scene.instances.push(instance("one", "assembly"));
    for limits in [
        PrefabLimits {
            max_depth: 1,
            ..PrefabLimits::default()
        },
        PrefabLimits {
            max_instances: 1,
            ..PrefabLimits::default()
        },
        PrefabLimits {
            max_objects: 2,
            ..PrefabLimits::default()
        },
    ] {
        assert!(
            scene
                .expand(&registry, limits, |asset| resolve(&assets, asset))
                .is_err()
        );
    }
    assert!(
        scene
            .expand(&self::registry(false), PrefabLimits::default(), |asset| {
                resolve(&assets, asset)
            })
            .is_err()
    );
    let mut duplicate = scene.clone();
    duplicate.instances.push(instance("one", "assembly"));
    assert!(
        duplicate
            .expand(&registry, PrefabLimits::default(), |asset| resolve(
                &assets, asset
            ))
            .is_err()
    );
    let collision = instance_object_id(&id("one"), &id("assembly"));
    scene.objects.push(object(&collision.0));
    assert!(
        scene
            .expand(&registry, PrefabLimits::default(), |asset| resolve(
                &assets, asset
            ))
            .is_err()
    );
    scene.objects.clear();
    assets
        .get_mut("leaf")
        .unwrap()
        .instances
        .push(instance("cycle", "assembly"));
    let error = scene
        .expand(&registry, PrefabLimits::default(), |asset| {
            resolve(&assets, asset)
        })
        .unwrap_err();
    assert!(error.to_string().contains("cycle"));
}
#[test]
fn component_overrides_can_remove_add_and_preserve_external_references() {
    let assets = fixture();
    let registry = registry(true);
    let mut scene = document(vec![object("external")]);
    let mut instance = instance("one", "leaf");
    instance.overrides.insert(
        id("follower"),
        ObjectOverride {
            components: BTreeMap::from([("link.v1".into(), None)]),
            ..ObjectOverride::default()
        },
    );
    instance.overrides.insert(
        id("root"),
        ObjectOverride {
            components: BTreeMap::from([(
                "link.v1".into(),
                Some(
                    serde_json::to_value(Link {
                        target: id("external"),
                        label: "external".into(),
                    })
                    .unwrap(),
                ),
            )]),
            ..ObjectOverride::default()
        },
    );
    scene.instances.push(instance);
    let loaded = scene
        .load(&registry, PrefabLimits::default(), |asset| {
            resolve(&assets, asset)
        })
        .unwrap();
    let root = loaded
        .resolve(&instance_object_id(&id("one"), &id("root")))
        .unwrap();
    assert_eq!(
        loaded
            .graph
            .component::<Link>(root)
            .unwrap()
            .unwrap()
            .target,
        id("external")
    );
    let follower = loaded
        .resolve(&instance_object_id(&id("one"), &id("follower")))
        .unwrap();
    assert!(loaded.graph.component::<Link>(follower).unwrap().is_none());
}

#[test]
fn source_reorder_keeps_identity_and_broken_source_or_remapper_rejects_publication() {
    let mut assets = fixture();
    let registry = registry(true);
    let mut scene = document(vec![]);
    scene.instances.push(instance("one", "leaf"));
    let original = scene
        .expand(&registry, PrefabLimits::default(), |asset| {
            resolve(&assets, asset)
        })
        .unwrap();
    assets.get_mut("leaf").unwrap().objects.reverse();
    let reordered = scene
        .expand(&registry, PrefabLimits::default(), |asset| {
            resolve(&assets, asset)
        })
        .unwrap();
    let mut original_ids: Vec<_> = original
        .document
        .objects
        .iter()
        .map(|object| &object.id)
        .collect();
    let mut reordered_ids: Vec<_> = reordered
        .document
        .objects
        .iter()
        .map(|object| &object.id)
        .collect();
    original_ids.sort();
    reordered_ids.sort();
    assert_eq!(original_ids, reordered_ids);
    let mut broken_registry = ComponentRegistry::default();
    broken_registry
        .register_with_reference_remap::<Link>(
            "link.v1",
            |link| vec![link.target.clone()],
            |_, _| {},
        )
        .unwrap();
    assert!(
        scene
            .expand(&broken_registry, PrefabLimits::default(), |asset| resolve(
                &assets, asset
            ))
            .unwrap_err()
            .to_string()
            .contains("did not remap")
    );
    assets
        .get_mut("leaf")
        .unwrap()
        .objects
        .retain(|object| object.id != id("root"));
    assert!(
        scene
            .expand(&registry, PrefabLimits::default(), |asset| resolve(
                &assets, asset
            ))
            .is_err()
    );
    // Previously returned immutable authoring data is unaffected by failed staging.
    assert_eq!(original.document.objects.len(), 2);
    assert!(original.document.load(&registry, 2).is_ok());
}

#[test]
fn capture_edits_preserves_instances_and_reverses_only_declared_object_links() {
    let assets = fixture();
    let registry = registry(true);
    let mut source = document(vec![]);
    source.instances = vec![instance("first", "leaf"), instance("second", "leaf")];
    let baseline = source
        .expand(&registry, PrefabLimits::default(), |asset| {
            resolve(&assets, asset)
        })
        .unwrap()
        .document;
    let mut edited = baseline.clone();
    let follower_id = instance_object_id(&id("first"), &id("follower"));
    let follower = edited
        .objects
        .iter_mut()
        .find(|object| object.id == follower_id)
        .unwrap();
    follower.translation[0] = 0.000_001;
    follower.components.insert(
        "link.v1".into(),
        serde_json::to_value(Link {
            target: follower_id.clone(),
            label: follower_id.0.clone(),
        })
        .unwrap(),
    );
    let captured = source
        .capture_edits(&baseline, &edited, &registry, 128)
        .unwrap();
    assert!(captured.objects.is_empty());
    assert!(captured.instances[1].overrides.is_empty());
    let members = &captured.instances[0].overrides[&id("follower")].component_members["link.v1"];
    assert_eq!(members["/target"], serde_json::json!("follower"));
    assert_eq!(members["/label"], serde_json::json!(follower_id.0));
    assert_eq!(
        captured
            .expand(&registry, PrefabLimits::default(), |asset| resolve(
                &assets, asset
            ))
            .unwrap()
            .document,
        edited
    );
    edited.objects.retain(|object| object.id != follower_id);
    let deleted = source
        .capture_edits(&baseline, &edited, &registry, 128)
        .unwrap();
    assert!(deleted.instances[0].overrides[&id("follower")].deleted);
    assert_eq!(
        deleted
            .expand(&registry, PrefabLimits::default(), |asset| resolve(
                &assets, asset
            ))
            .unwrap()
            .document,
        edited
    );
}

#[test]
fn structural_overrides_reparent_delete_and_resurrect_without_baking() {
    let assets = fixture();
    let registry = registry(true);
    let mut source = document(vec![object("world")]);
    source.instances.push(instance("first", "leaf"));
    let baseline = source
        .instance_baseline(&registry, PrefabLimits::default(), |asset| {
            resolve(&assets, asset)
        })
        .unwrap();
    let original = source
        .expand(&registry, PrefabLimits::default(), |asset| {
            resolve(&assets, asset)
        })
        .unwrap()
        .document;
    let mut edited = original.clone();
    let follower_id = instance_object_id(&id("first"), &id("follower"));
    let follower = edited
        .objects
        .iter_mut()
        .find(|object| object.id == follower_id)
        .unwrap();
    follower.parent = Some(id("world"));
    let reparented = source
        .capture_edits(&baseline, &edited, &registry, 128)
        .unwrap();
    assert_eq!(reparented.objects.len(), 1);
    assert_eq!(
        reparented.instances[0].overrides[&id("follower")]
            .parent
            .as_ref()
            .unwrap()
            .parent,
        Some(id("world"))
    );
    assert_eq!(
        reparented
            .expand(&registry, PrefabLimits::default(), |asset| resolve(
                &assets, asset
            ))
            .unwrap()
            .document,
        edited
    );
    edited.objects.retain(|object| object.id == id("world"));
    let deleted = reparented
        .capture_edits(&baseline, &edited, &registry, 128)
        .unwrap();
    assert!(
        deleted.instances[0]
            .overrides
            .values()
            .all(|value| value.deleted)
    );
    assert_eq!(
        deleted
            .expand(&registry, PrefabLimits::default(), |asset| resolve(
                &assets, asset
            ))
            .unwrap()
            .document,
        edited
    );
    let clean_baseline = deleted
        .instance_baseline(&registry, PrefabLimits::default(), |asset| {
            resolve(&assets, asset)
        })
        .unwrap();
    let restored = deleted
        .capture_edits(&clean_baseline, &original, &registry, 128)
        .unwrap();
    assert!(restored.instances[0].overrides.is_empty());
    assert_eq!(
        restored
            .expand(&registry, PrefabLimits::default(), |asset| resolve(
                &assets, asset
            ))
            .unwrap()
            .document,
        original
    );
    let mut dangling = original.clone();
    dangling
        .objects
        .retain(|object| object.id != instance_object_id(&id("first"), &id("root")));
    assert!(
        source
            .capture_edits(&baseline, &dangling, &registry, 128)
            .is_err()
    );
}

#[test]
fn external_parent_with_same_id_as_source_object_is_not_remapped() {
    let assets = fixture();
    let registry = registry(true);
    let mut source = document(vec![object("root")]);
    source.instances.push(instance("first", "leaf"));
    let baseline = source
        .instance_baseline(&registry, PrefabLimits::default(), |asset| {
            resolve(&assets, asset)
        })
        .unwrap();
    let mut edited = source
        .expand(&registry, PrefabLimits::default(), |asset| {
            resolve(&assets, asset)
        })
        .unwrap()
        .document;
    let follower = edited
        .objects
        .iter_mut()
        .find(|object| object.id == instance_object_id(&id("first"), &id("follower")))
        .unwrap();
    follower.parent = Some(id("root"));
    let captured = source
        .capture_edits(&baseline, &edited, &registry, 128)
        .unwrap();
    assert!(
        captured.instances[0].overrides[&id("follower")]
            .parent
            .as_ref()
            .unwrap()
            .external
    );
    assert_eq!(
        captured
            .expand(&registry, PrefabLimits::default(), |asset| resolve(
                &assets, asset
            ))
            .unwrap()
            .document,
        edited
    );
}
