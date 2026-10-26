# Production readiness: current evidence and first implementation

Date: 2026-10-26. The engine is not production ready. This register implements
parts of ENG-00 through ENG-03 in [the roadmap](engine-roadmap-2026-10-25.md); it does not
replace the complete engine objective. Existing dirty rendering/scene work is
preserved. Source presence below is not an execution qualification.

| Area | Current evidence | Status and next acceptance gate |
|---|---|---|
| Project bootstrap | `voxy_editor/src/project.rs`, CLI create/validate | **Partial, tested**: creation, validation, relocation and native editor entry without a model. Visible-window acceptance is pending. |
| Scenes/prefabs | `voxy_scene/src/document.rs`, `prefab_document.rs`, `history.rs`, `transaction.rs` | **Existing APIs inspected; workflow unverified**: edit/save/reopen nested instances and overrides through the editor. |
| Asset identity | `voxy_assets/src/manifest.rs`, `locations.rs` | **Reused by bootstrap**: no second asset catalog. Import, dependency rebuild and failure recovery need a combined project scenario. |
| Play/Stop | `voxy_editor/src/play_session.rs`, `voxy_scene/src/simulation.rs` | **Headless motion integration passed**: Play and standalone share initialization, fixed simulation and codecs; Stop restores authoring and invalidates old handles. Interactive gameplay/actions still need project-wide acceptance. |
| Packaging | `voxy_assets/src/package.rs`, editor `export_game_package`, `packaged_game.rs` | **Project package tested**: descriptor export and isolated loader; a referenced glTF external buffer survives source-tree deletion. Clean-machine native delivery remains pending. |
| Runtime physics/animation | Existing `physics`, `voxy_animation`, scene/gameplay adapters | **Not qualified by this change**: author and run an interactive scene with collision, character motion, animation transitions and events. |
| Rendering | Frozen near-first query/source and street HDR proofs under `artifacts/original-street-realism-2026-10-25/` | **Bounded precision gate passed**; full motion, quality and minimum 120 FPS remain unqualified. |
| UI/audio/navigation | Existing domain crates and editor adapters | **Combined scenario unverified**: authored HUD/menu, spatial audio, navigation and save/load in the package. |
| Editor authoring | Existing model-based entry in `voxy_editor/src/main.rs` | **Headless owner workflow passed**: empty initialization, object creation, undo/redo, save and relocated reopen. Native rendering and interactive controls still need acceptance. |
| Delivery/platforms | Existing platform crates and packaging tools | **Unverified here**: clean-machine desktop execution, recovery, platform-specific testing and release artifacts. |

## Implemented project boundary

A project directory contains `project.voxy` (version/name/manifest/start scene),
the EXISTING asset manifest format and the EXISTING scene document format.
An empty project has zero assets and zero scene objects; no dummy model is added.
Descriptor paths and asset sources are relative to the project root (the
descriptor directory), matching the existing editor input provider. Canonical resolution rejects paths/symlinks leaving the project root.
Opening uses a caller-supplied component registry and checks scene structure and
registered component decoding. The CLI uses built-in editor/gameplay registrations;
custom game components must supply their registry through the API. Cross-document
component-to-asset reference validation, asset decoding and build profiles remain
future work; checking file existence is not an import qualification.

Creation exclusively claims a NEW directory and writes files with `create_new`.
The descriptor is published last by a no-overwrite hard link to a flushed private
file, serving as the readiness marker. Failure removes only created files; it
never recursively deletes an occupied directory. The directory itself is visible
while creation runs: this is NOT an atomic directory rename or a claim of
crash-durable publication on every filesystem. Unsupported hard links fail
explicitly. The project command enters the existing native editor; visible-window validation
remains pending. Project standalone and descriptor-based packaging now share the
existing authoring/play owner; this does not qualify native rendering or deployment.

```sh
cargo +nightly-2026-05-27 run -p voxy_editor -- --create-project /absolute/new-game "My Game"
cargo +nightly-2026-05-27 run -p voxy_editor -- --validate-project /absolute/new-game/project.voxy
cargo +nightly-2026-05-27 run -p voxy_editor -- --project /absolute/new-game/project.voxy
cargo +nightly-2026-05-27 run -p voxy_editor -- --project /absolute/new-game/project.voxy --game-check
cargo +nightly-2026-05-27 run -p voxy_editor -- --build-project /absolute/new-game/project.voxy /absolute/game.vpak
cargo +nightly-2026-05-27 run -p voxy_editor -- --package /absolute/game.vpak --game-check
```

Create/validate return before window/GPU initialization; `--project` opens the native editor. The parent directory must
exist. Existing model/manifest editor commands remain available.

## Validation

- Nine project tests pass. Two exercise the actual editor owner without
  a window: object creation, undo/redo, save, relocated reopen, and nested manifest
  paths resolved against the project root. The original four cover: reopen after relocation, existing-directory
  preservation, malformed/missing/unsupported documents, absent source assets,
  existing-file preservation, blank-name rejection and symlink escape rejection.
- The actual editor CLI created and validated an empty project from cwd `/`,
  refused a second creation without replacement, and validated after relocation.
- `cargo clippy -p voxy_editor --lib --bin voxy_editor -- -D warnings` passes with
  the pinned nightly and isolated target. Two redundant u64 conversions in prior
  rendering diagnostics were removed without changing buffer limits.
- Full editor library suite: 201 passed, 17 ignored. Ignored checks are not
  counted as qualification.
- All-targets editor Clippy passes with `-D warnings`; the native editor binary
  builds. The real CLI validates the starter from cwd `/`.
- The empty hierarchy exposes `Create object (D)` through the existing creation
  action rather than requiring a model selection.
- Project Play and headless standalone produce identical motion after 120 fixed
  ticks. Stop restores the authoring document and rejects the former runtime handles.
- Project export includes descriptor, manifest, all declared sources and observed
  model/audio/font dependencies. The test uses a skeletal glTF with an external
  buffer not separately listed in the manifest. Its package runs after the entire
  source tree is removed; existing output is preserved and corruption rejected.
- Application-owned component codecs are exercised across editing, Play, project
  GameCheck, build and packaged GameCheck. Missing codecs fail explicitly; custom
  update callbacks/behaviors are not qualified by a custom-data component test.
- Real CLI acceptance from cwd `/`: create, validate, simulate a rotating model,
  build, reject overwrite, delete its source tree and simulate the isolated package.
  The project/package final documents are identical after 120 ticks.
- No native window, clean-machine distribution or FPS qualification is inferred
  from headless checks. No GPU performance benchmark was run for these changes.

## Next implementation, in dependency order

1. Validate native-window rendering and controls for `--project` with the empty
   starter. The constructor and actual authoring-owner roundtrip are tested; no
   visible editor window was launched in this qualification.
2. Extend the project scenario to prefab editing, import/reload failure recovery
   with useful authored components.
3. Extend the shared project runtime/package scenario to input, collision,
   UI callbacks, spatial audio and user game behavior. Existing common initialization
   and custom-data codecs are qualified only for the scenarios listed above.
4. Deliver a small interactive packaged game and verify it without its source assets.
   Only then close the first vertical slice and extend the remaining domain gates.

The generated [starter descriptor](../examples/empty-project/project.voxy) was
created and validated by the actual final CLI from cwd `/`. Logs, CLI outcomes
and source/binary SHA-256 identities are retained in
`production-readiness-evidence-2026-10-26/`. It is an empty authoring project,
openable through `--project`; visible rendering is unverified and it is not yet
a useful interactive game. The [motion example](../examples/project-motion/project.voxy)
and `artifacts/project-runtime-2026-10-26/motion.vpak` retain the CLI-verified
rotating-model sample and its exact package; `--project ... --game` and
`--package ... --game` select native launch (not visually verified here).

## Runtime/package contract and evidence

`project_runtime::prepare_game` is shared by the legacy model entry and the new
project entry. GameCheck uses offline audio and the existing fixed-step simulation;
it returns before creating an event loop, window or GPU. Native Game/GameSmoke
use the existing viewport and live-device path. `run_project_with_components`,
`export_project_package_with_components` and `run_packaged_game_with_components`
accept application codecs; executable game code is supplied by the host, never
serialized into the resource package.

The outer resource package remains unchanged. Version-1 model launch records stay
supported; version-2 launch records contain only the root project descriptor path.
Both envelopes reject traversal, nested bootstrap paths, unknown fields and
unsupported versions. The package loader materializes a private owned temporary
root and uses the ordinary project loader, with no source-directory fallback.
Export checks captured descriptor/manifest bytes and all existing observed import
revisions before creating output, and refuses existing output paths. It retains
all declared source files, including unused bindings, to preserve the current
project-open contract. This is not dead-asset pruning or an executable/app bundle.

Evidence: `project-runtime-full-tests.log`, `project-runtime-clippy.log`,
`project-runtime-build.log`, `project-runtime-cli-proof.json` and its runnable
Python script under `production-readiness-evidence-2026-10-26/`. The CLI proof
records the actual binary SHA-256, package SHA-256 and outputs. Simulation ticks
are not rendered frames or a real-time throughput measurement.
