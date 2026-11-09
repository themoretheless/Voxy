//! Source-tree ratchets (`docs/refactor/ARCHITECTURE.md` §1).
//!
//! Two rules, each with a list of known exceptions that may only shrink:
//! every `.rs` file under a crate's `src` is part of a module tree, and no
//! production file exceeds the line budget.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use voxy_depgraph::source::{
    Reach, blank_non_code, classify_sources, normalize, production_lines, workspace_crates,
};

/// Files the compiler never sees, with the plan step that removes them.
const KNOWN_ORPHANS: &[(&str, &str)] = &[];

/// Production lines a source file may have (tests excluded).
const LINE_BUDGET: usize = 1500;

/// A listed file may sit this far below its ceiling before the ceiling has to
/// be lowered.
const CEILING_SLACK: usize = 200;

/// Files over [`LINE_BUDGET`] with the ceiling each must stay under. Splitting
/// a file lowers its ceiling or removes the entry; nothing is added here
/// without a reason recorded in `docs/refactor/PLAN.md`.
const KNOWN_OVERSIZED: &[(&str, usize)] = &[
    ("crates/physics/src/astrophysics/spherical.rs", 1650),
    (
        "crates/physics/src/astrophysics/spherical_radiation.rs",
        1700,
    ),
    ("crates/physics/src/biomechanics/inertia/implicit.rs", 2050),
    ("crates/physics/src/liquid.rs", 1550),
    ("crates/physics/src/liquid/multi_body.rs", 2000),
    ("crates/physics/src/lymph.rs", 1550),
    ("crates/voxy_animation/src/lib.rs", 2250),
    ("crates/voxy_animation/src/root_rigid/enclosure.rs", 1900),
    (
        "crates/voxy_animation/src/root_rigid/enclosure/compilation.rs",
        2150,
    ),
    ("crates/voxy_animation/src/root_rotation.rs", 1550),
    ("crates/voxy_app/src/main.rs", 3000),
    ("crates/voxy_demos/src/face_parameters.rs", 2750),
    ("crates/voxy_demos/src/female_demo.rs", 2800),
    ("crates/voxy_demos/src/female_rig.rs", 2550),
    ("crates/voxy_demos/src/tissue_demo.rs", 2150),
    ("crates/voxy_editor/src/animation_runtime.rs", 1650),
    ("crates/voxy_editor/src/lib.rs", 2000),
    ("crates/voxy_editor/src/panels.rs", 1700),
    ("crates/voxy_gameplay/src/lib.rs", 1850),
    ("crates/voxy_gameplay/src/liquid_runtime.rs", 2450),
    ("crates/voxy_ray_probe/src/shared_producer_audit.rs", 2600),
    ("crates/voxy_render/src/archive_scene.rs", 3900),
    ("crates/voxy_render/src/renderer.rs", 2150),
    ("crates/voxy_render/src/scene.rs", 3150),
    ("crates/voxy_render/src/surface.rs", 1650),
    ("crates/voxy_render/src/world_nearest_job.rs", 1850),
    ("crates/voxy_shell/src/scene_app.rs", 2750),
];

fn workspace() -> PathBuf {
    normalize(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../..")))
}

fn relative(workspace: &Path, file: &Path) -> String {
    file.strip_prefix(workspace)
        .unwrap_or(file)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Workspace-relative path of every source file with its reach.
fn sources() -> BTreeMap<String, Option<Reach>> {
    let workspace = workspace();
    workspace_crates(&workspace)
        .iter()
        .flat_map(|(_, src)| classify_sources(src))
        .map(|(file, reach)| (relative(&workspace, &file), reach))
        .collect()
}

/// Production line count of every production file over [`LINE_BUDGET`].
fn oversized() -> BTreeMap<String, usize> {
    let workspace = workspace();
    sources()
        .into_iter()
        .filter(|(_, reach)| *reach == Some(Reach::Production))
        .filter_map(|(path, _)| {
            let text = fs::read_to_string(workspace.join(&path)).ok()?;
            let lines = production_lines(&blank_non_code(&text));
            (lines > LINE_BUDGET).then_some((path, lines))
        })
        .collect()
}

#[test]
fn every_source_file_belongs_to_a_module_tree() {
    let orphans: Vec<String> = sources()
        .into_iter()
        .filter(|(path, reach)| {
            reach.is_none() && !KNOWN_ORPHANS.iter().any(|(known, _)| known == path)
        })
        .map(|(path, _)| path)
        .collect();
    assert!(
        orphans.is_empty(),
        "these files are not declared by any `mod`, `#[path]` or `include!`, so they are never \
         compiled; declare them or delete them:\n{}",
        orphans.join("\n")
    );
}

#[test]
fn known_orphans_are_still_orphans() {
    let sources = sources();
    let stale: Vec<String> = KNOWN_ORPHANS
        .iter()
        .filter(|(path, _)| sources.get(*path) != Some(&None))
        .map(|(path, step)| format!("{path} ({step})"))
        .collect();
    assert!(
        stale.is_empty(),
        "these files are compiled again or gone; remove them from KNOWN_ORPHANS:\n{}",
        stale.join("\n")
    );
}

#[test]
fn production_files_stay_within_their_line_budget() {
    let ceilings: BTreeMap<&str, usize> = KNOWN_OVERSIZED.iter().copied().collect();
    let over: Vec<String> = oversized()
        .into_iter()
        .filter(|(path, lines)| ceilings.get(path.as_str()).is_none_or(|max| lines > max))
        .map(|(path, lines)| format!("    (\"{path}\", {}),", lines.next_multiple_of(50)))
        .collect();
    assert!(
        over.is_empty(),
        "production files over the {LINE_BUDGET}-line budget or over their recorded ceiling; \
         split them (see docs/refactor/PLAN.md) instead of raising the ceiling:\n{}",
        over.join("\n")
    );
}

#[test]
fn oversized_ceilings_only_shrink() {
    let current = oversized();
    let stale: Vec<String> = KNOWN_OVERSIZED
        .iter()
        .filter_map(|(path, ceiling)| match current.get(*path) {
            None => Some(format!("{path}: within the budget now, remove the entry")),
            Some(lines) if lines + CEILING_SLACK < *ceiling => Some(format!(
                "{path}: {lines} lines, lower the ceiling to {}",
                lines.next_multiple_of(50)
            )),
            Some(_) => None,
        })
        .collect();
    assert!(
        stale.is_empty(),
        "KNOWN_OVERSIZED is out of date; tighten it so the ratchet holds:\n{}",
        stale.join("\n")
    );
}
