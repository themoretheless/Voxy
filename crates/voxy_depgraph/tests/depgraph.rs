//! Crate-graph ratchet (refactoring plan step A1).
//!
//! Reads `cargo metadata` for the workspace and checks every edge between
//! workspace crates. An edge is either on the documented allow-list, or a
//! known violation that the plan removes step by step. A new undocumented edge
//! fails the test; so does a known violation that has disappeared, because the
//! list must shrink with it (the ratchet only tightens).

use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

/// Edges that match the layering of `docs/architecture.md` §4: platform/GPU
/// never below world/storage/mesher, composition roots on top, leaf utility
/// crates at the bottom.
const ALLOWED: &[(&str, &[&str])] = &[
    ("physics_voxel", &["physics", "voxy_core", "voxy_world"]),
    ("voxy_audio_device", &["voxy_audio"]),
    ("voxy_cuda", &["physics"]),
    (
        "voxy_demos",
        &[
            "physics",
            "physics_voxel",
            "voxy_animation",
            "voxy_core",
            "voxy_cuda",
            "voxy_gpu",
            "voxy_render",
            "voxy_world",
        ],
    ),
    (
        "voxy_editor",
        &[
            "physics",
            "voxy_animation",
            "voxy_assets",
            "voxy_audio",
            "voxy_audio_device",
            "voxy_gameplay",
            "voxy_input",
            "voxy_render",
            "voxy_scene",
            "voxy_text",
            "voxy_ui",
        ],
    ),
    (
        "voxy_gameplay",
        &[
            "physics",
            "voxy_animation",
            "voxy_assets",
            "voxy_audio",
            "voxy_input",
            "voxy_scene",
            "voxy_text",
            "voxy_ui",
        ],
    ),
    (
        "voxy_gpu",
        &[
            "physics",
            "physics_voxel",
            "voxy_core",
            "voxy_cuda",
            "voxy_world",
        ],
    ),
    ("voxy_lighting", &["voxy_core", "voxy_world"]),
    ("voxy_mesher", &["voxy_core", "voxy_world"]),
    ("voxy_ray_probe", &["voxy_demos", "voxy_render"]),
    (
        "voxy_render",
        &["voxy_core", "voxy_lighting", "voxy_mesher"],
    ),
    (
        "voxy_runtime",
        &[
            "physics_voxel",
            "voxy_core",
            "voxy_lighting",
            "voxy_mesher",
            "voxy_storage",
            "voxy_time",
            "voxy_world",
        ],
    ),
    ("voxy_rush", &["physics", "voxy_input", "voxy_scene"]),
    ("voxy_scene", &["voxy_core", "voxy_time"]),
    ("voxy_storage", &["voxy_core", "voxy_world"]),
    ("voxy_streamline", &["voxy_render"]),
    ("voxy_ui_winit", &["voxy_ui"]),
    ("voxy_vulkan", &["voxy_cuda"]),
    (
        "voxy_web",
        &[
            "physics",
            "physics_voxel",
            "voxy_core",
            "voxy_gpu",
            "voxy_mesher",
            "voxy_render",
            "voxy_runtime",
            "voxy_world",
        ],
    ),
    ("voxy_world", &["voxy_core"]),
    ("voxy_xr", &["voxy_ui"]),
];

/// Composition roots may depend on anything in the workspace.
const COMPOSITION_ROOTS: &[&str] = &["voxy_app"];

/// Normal-dependency edges the plan still has to remove, with the step that
/// removes them. Remove an entry here in the same change that removes the edge.
const KNOWN_VIOLATIONS: &[(&str, &str, &str)] = &[
    (
        "voxy_mobile",
        "voxy_app",
        "C2b: the mobile shell needs a shared SceneApp crate, not the app binary",
    ),
    (
        "voxy_gpu",
        "voxy_render",
        "D3: shared device utilities move below render",
    ),
    (
        "voxy_render",
        "voxy_animation",
        "D5: render sees only a skin palette",
    ),
];

/// Dev-dependency cycles (a crate's tests build a crate that depends on it).
const KNOWN_DEV_CYCLES: &[(&str, &str, &str)] = &[];

/// Crates that must stay free of the platform/GPU shell (invariant 9).
const PLATFORM_FREE: &[&str] = &[
    "voxy_core",
    "voxy_time",
    "voxy_world",
    "voxy_storage",
    "voxy_mesher",
    "voxy_lighting",
    "voxy_runtime",
    "physics",
    "physics_voxel",
    "voxy_scene",
    "voxy_animation",
];
const PLATFORM_CRATES: &[&str] = &["wgpu", "winit"];

struct Graph {
    /// crate -> normal dependencies inside the workspace
    normal: BTreeMap<String, BTreeSet<String>>,
    /// crate -> dev dependencies inside the workspace
    dev: BTreeMap<String, BTreeSet<String>>,
    /// crate -> external normal dependencies
    external: BTreeMap<String, BTreeSet<String>>,
}

fn load_graph() -> Graph {
    let output = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .output()
        .expect("cargo metadata runs");
    assert!(output.status.success(), "cargo metadata failed");
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let packages = metadata["packages"].as_array().unwrap();
    let members: BTreeSet<String> = packages
        .iter()
        .map(|p| p["name"].as_str().unwrap().to_owned())
        .collect();
    let mut graph = Graph {
        normal: BTreeMap::new(),
        dev: BTreeMap::new(),
        external: BTreeMap::new(),
    };
    for package in packages {
        let name = package["name"].as_str().unwrap().to_owned();
        for dependency in package["dependencies"].as_array().unwrap() {
            let target = dependency["name"].as_str().unwrap().to_owned();
            let kind = dependency["kind"].as_str();
            match (members.contains(&target), kind) {
                (true, None) => {
                    graph.normal.entry(name.clone()).or_default().insert(target);
                }
                (true, Some("dev")) => {
                    graph.dev.entry(name.clone()).or_default().insert(target);
                }
                (false, None) => {
                    graph
                        .external
                        .entry(name.clone())
                        .or_default()
                        .insert(target);
                }
                _ => {}
            }
        }
    }
    graph
}

fn reaches(graph: &BTreeMap<String, BTreeSet<String>>, from: &str, to: &str) -> bool {
    let mut stack = vec![from.to_owned()];
    let mut seen = BTreeSet::new();
    while let Some(current) = stack.pop() {
        if current == to {
            return true;
        }
        if let Some(next) = graph.get(&current) {
            for dependency in next {
                if seen.insert(dependency.clone()) {
                    stack.push(dependency.clone());
                }
            }
        }
    }
    false
}

#[test]
fn every_workspace_edge_is_documented_or_a_known_violation() {
    let graph = load_graph();
    let allowed: BTreeMap<&str, BTreeSet<&str>> = ALLOWED
        .iter()
        .map(|(from, to)| (*from, to.iter().copied().collect()))
        .collect();
    let mut undocumented = Vec::new();
    for (from, targets) in &graph.normal {
        if COMPOSITION_ROOTS.contains(&from.as_str()) {
            continue;
        }
        for to in targets {
            let documented = allowed
                .get(from.as_str())
                .is_some_and(|set| set.contains(to.as_str()));
            let known = KNOWN_VIOLATIONS
                .iter()
                .any(|(f, t, _)| f == from && t == to);
            if !documented && !known {
                undocumented.push(format!("{from} -> {to}"));
            }
        }
    }
    assert!(
        undocumented.is_empty(),
        "undocumented workspace edges (add to ALLOWED in tests/depgraph.rs and to \
         docs/architecture.md §4, or to KNOWN_VIOLATIONS with the plan step that removes them):\n{}",
        undocumented.join("\n")
    );
}

#[test]
fn known_violations_still_exist_until_their_plan_step_lands() {
    let graph = load_graph();
    let stale: Vec<String> = KNOWN_VIOLATIONS
        .iter()
        .filter(|(from, to, _)| !graph.normal.get(*from).is_some_and(|set| set.contains(*to)))
        .map(|(from, to, step)| format!("{from} -> {to} ({step})"))
        .collect();
    assert!(
        stale.is_empty(),
        "these violations are gone; remove them from KNOWN_VIOLATIONS so the ratchet tightens:\n{}",
        stale.join("\n")
    );
}

#[test]
fn dev_dependency_cycles_are_only_the_known_ones() {
    let graph = load_graph();
    let mut cycles = Vec::new();
    for (from, targets) in &graph.dev {
        for to in targets {
            if reaches(&graph.normal, to, from) {
                let known = KNOWN_DEV_CYCLES
                    .iter()
                    .any(|(f, t, _)| f == from && t == to);
                if !known {
                    cycles.push(format!("{from} (dev) -> {to} -> ... -> {from}"));
                }
            }
        }
    }
    assert!(
        cycles.is_empty(),
        "new dev-dependency cycles:\n{}",
        cycles.join("\n")
    );
    for (from, to, step) in KNOWN_DEV_CYCLES {
        assert!(
            graph.dev.get(*from).is_some_and(|set| set.contains(*to))
                && reaches(&graph.normal, to, from),
            "{from} -> {to} is no longer a dev cycle; remove it from KNOWN_DEV_CYCLES ({step})"
        );
    }
}

#[test]
fn world_storage_mesher_and_simulation_crates_stay_platform_free() {
    let graph = load_graph();
    let mut leaks = Vec::new();
    for crate_name in PLATFORM_FREE {
        if let Some(external) = graph.external.get(*crate_name) {
            for platform in PLATFORM_CRATES {
                if external.contains(*platform) {
                    leaks.push(format!("{crate_name} -> {platform}"));
                }
            }
        }
    }
    assert!(
        leaks.is_empty(),
        "platform crates below the render layer:\n{}",
        leaks.join("\n")
    );
}
