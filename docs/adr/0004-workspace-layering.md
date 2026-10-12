# ADR 0004: Workspace layering is a test, not a convention

Status: accepted; ratchet in place, violations being removed.
Date: 2026-10-12

## Context

`docs/architecture.md` §4 described a seven-crate graph while the workspace
had grown to 33 crates. Four edges contradict the documented direction
(`voxy_ray_probe`/`voxy_mobile -> voxy_app`, `voxy_gpu -> voxy_render`,
`voxy_render -> voxy_animation`) and `voxy_render`'s tests build
`voxy_editor`, which depends on `voxy_render`. Nothing enforced invariant 9
("platform/GPU are not imported by world/storage/mesher"), and examples in
`voxy_app` compiled library sources through `#[path]`, hiding dead code.

## Decision

1. `crates/voxy_depgraph` reads `cargo metadata` and asserts, in CI, that
   every edge between workspace crates is on the documented allow-list
   (`tests/depgraph.rs::ALLOWED`), except a composition root (`voxy_app`),
   which may depend on anything.
2. Known violations are listed with the plan step that removes them. The
   test fails both when a new undocumented edge appears and when a listed
   violation disappears without being removed from the list: the list only
   shrinks.
3. Dev-dependency cycles are listed the same way. Crates below the render
   layer (`voxy_core`, `voxy_time`, `voxy_world`, `voxy_storage`,
   `voxy_mesher`, `voxy_lighting`, `voxy_runtime`, `voxy_scene`,
   `voxy_animation`, `physics`, `physics_voxel`) must not depend on `wgpu`
   or `winit`.
4. Every crate inherits `[workspace.lints]`; `unsafe_code` is `deny` so that
   the few FFI/GPU-interop sites can opt in locally with `#[allow]`.
5. Examples use the library API (`use voxy_app::module`), never `#[path]`
   includes of library sources.

## Status of the graph after the first refactoring batch (2026-10-12)

- `voxy_demos` holds the character/tissue/liquid demos; `voxy_app` is the
  shell and binary only; `voxy_ray_probe` depends on `voxy_demos`.
- `voxy_gpu_core` (compute program, memory budget, readback pool, backend
  options, indirect mesh handle) sits below `voxy_render`; `voxy_gpu` no
  longer depends on `voxy_render`.
- `voxy_render` no longer dev-depends on `voxy_editor`.
- Remaining listed violations: `voxy_mobile -> voxy_app` (needs a shared
  shell crate for `SceneApp`) and `voxy_render -> voxy_animation` (render's
  model module evaluates poses; a narrower boundary is a design decision,
  not a move).

## Consequences

- Adding a dependency edge is a reviewed change to `ALLOWED` and to §4.
- `voxy_app` stops being a library for other crates once C2 of the plan
  lands; until then that edge is a listed violation.
- Shared primitives that several layers need (padded-chunk constants, the
  deterministic hasher) live in `voxy_core`, which stays dependency-free.
