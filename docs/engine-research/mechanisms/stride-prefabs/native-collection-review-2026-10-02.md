# Native collection inspector review — 2026-10-02

Status: not accepted visually. Backend and panel hit-region regressions pass, but
no native collection control has yet been observed or clicked on a presented frame.

The `collections` example installs a typed Inventory codec, its identified `/items`
array and a new-item default through the public application registry entry point.
It now writes its embedded quad model next to the selected scene, only when absent,
so the default temporary example project does not depend on runtime access to the
repository under Documents. Existing model and scene files are retained.

The first review binary stopped in a directory-open operation while discovering
prefabs in the repository's example asset directory. A process sample showed
`AuthoringProject::asset_choices -> read_dir -> __open_nocancel`. No root cause or
OS permission diagnosis has been established. That first review process was not
forcibly terminated or restarted. It used
`/tmp/Voxy Collections Review-2026-10-02.app` and the fixture
`/tmp/voxy-collections-native-review-2026-10-02/scene.json`.

The isolated review app was compiled and frozen at:

- App: `/tmp/Voxy Collections Local Review-2026-10-02.app`
- Executable SHA-256: `dec880576f33f44c8f177973e714ed018baa700e9ad6e2c663eef8d4ded499c4`
- Project: `/tmp/voxy-collections-local-native-review-2026-10-02`
- Log: `/tmp/voxy-collections-local-native-review.log`

CUA observed the native window titled `Voxy — model loaded`. Its screenshot showed
an empty dark client area, in ordinary, zoomed and full-screen states. Raising and
activating the window did not produce a visible editor frame. After restarting the
same frozen binary with `VOXY_EDITOR_TRACE_INPUT=1`, the runtime reported:

```
EDITOR PRESENT frames=0 draws=1 outcome=SkippedOccluded
MODEL PUBLISHED frames=0 first_x=-0.5 asset=quad.obj
```

Those lines show import publication but no presented frame. They do not prove why
the surface was occluded or prove native Add/Delete/Move, save/reload or undo/redo.
CUA screenshots were inspected inline; no screenshot file was archived. The
isolated review application was then closed with its native Quit menu command,
and the app inventory confirmed it was no longer running. The older prefab review
application and the first collection startup process were left untouched.

Verification of the current source:

- 78 editor library tests pass; two GPU tests remain ignored.
- The collection regression builds real Panels geometry and uses its hit regions;
  it covers empty-list Add, ID-targeted deletion after reorder, Up/Down, page
  navigation, cross-page moves, selected-owner mismatch, stale target failure,
  undo/redo, persisted IDs, fresh IDs after deletion and Play edit blocking.
- `cargo check -p voxy_app --lib --bin voxy_app` passes with existing warnings.
- `cargo build -p voxy_editor --example collections` passes.
- Four-owner/32-package dependency boundary gate and `git diff --check` pass.
- Checking every application example fails in `hair_render`, `skin_render` and
  `female_render`, which refer to `crate::female_transmission` without providing
  that module. This broader gate is not passing and those unrelated sources were
  not changed in this collection patch.

Next native work: diagnose presentation/occlusion and gate editor interactions on
presented targets, then visually exercise Collections and save/reload with the
same installed codec. Item-specific prefab Reset and tombstone Reset also remain.

## Current source diagnostic review

The current Collections example was rebuilt after member/item/order Reset and
Restore deleted were implemented. Presented-panel input admission is also now
implemented; those backend changes do not establish native visual acceptance.

New isolated bundle: `/tmp/Voxy Collections Current Review-2026-10-02.app`.
Fixture: `/tmp/voxy-collections-current-native-review-2026-10-02/scene.json`.
Log: `/tmp/voxy-collections-current-native-review.log`.
Initial executable SHA-256:
`b29d44a6cbbf1fc3aeba890c58ddcd6309a433b7c4a0066cbf570191a3e58faa`.
It again loaded the model but displayed a blank dark client after Raise and a
title-bar click, reporting `frames=0` and `SkippedOccluded`.

An opt-in trace was added for Focused/Occluded/Resized window events, using the
existing `VOXY_EDITOR_TRACE_INPUT` flag and reporting winit visibility/focus.
Rebuilt diagnostic executable SHA-256:
`9d997defa8010cea9b1074fba4ae61ba009854a88785451d23343157dd10c68f`.
CUA raised it and clicked its client. The screenshot showed active coloured
window controls with an empty dark client. The trace reported:

```
EDITOR WINDOW event=Focused(true) visible=Some(true) focused=Some(true)
```

The last render outcome remained `SkippedOccluded`, with zero presented frames.
No Occluded event appeared in this captured log. This establishes a mismatch
between observed activation/winit visibility and surface acquisition's occlusion
result; it does not establish the cause or justify bypassing occlusion handling.
The installed wgpu Metal backend reads the hosting NSWindow's occlusionState
before acquiring a drawable. A live process sample is saved at
`/tmp/voxy-native-current-sample.txt`; the main thread was executing the event/draw
loop, rather than blocked in the earlier repository directory-open path.

Both review launches were closed through their native Quit menu, and the process
inventory then showed no matching review executable. The build with diagnostics
and the architecture boundary/whitespace gates pass. Next: reconcile window and
surface ownership/visibility and demonstrate a presented native frame before
claiming acceptance of the collection controls.

## Direct Metal window observation

Cargo's current `[patch.crates-io]` resolves wgpu and wgpu-hal to local `vendor`
sources (30.0.1), not the registry copies inspected earlier. The hosting-window
and occlusion guard code agree, but further diagnosis uses the authoritative
`vendor/wgpu-hal/src/metal/surface.rs`. An opt-in, state-change-only trace under
`VOXY_EDITOR_TRACE_INPUT` observes the exact window used by acquisition, without
changing the occlusion decision or requesting a drawable while occluded.

Diagnostic binary SHA-256:
`ced5b2d898b6516a5ddc52baa730cbea17f5d878815faa693d4cbf3a31800d13`.
Build log: `/tmp/voxy-metal-window-trace-build.log` (success).
Runtime log: `/tmp/voxy-collections-current-native-review.log`.
After CUA Raise and client activation:

```
METAL WINDOW number=24984 occlusion=8192 visible=true miniaturized=false key=true
```

The active-window screenshot remained blank. Zoom generated normal resize events
up to 4112x2516 physical pixels without changing the Metal window state or producing
a presented frame. The current SDK's NSWindow.h defines Visible as `1UL << 1`,
consistent with the guard; 8192 does not include that bit. Thus isVisible/key-window
state is not sufficient proof of the AppKit occlusion bit. The root cause of this
state remains unproven; do not label it a bit-mask bug or bypass the guard.
Apple's property contract is documented at
https://developer.apple.com/documentation/appkit/nswindow/occlusionstate-swift.property?language=objc
and the motivating acquisition stall is https://github.com/gfx-rs/wgpu/issues/8309.

The diagnostic app was closed through native Quit; the executable process then
disappeared. Boundary and whitespace gates pass. This observation improves the
diagnosis but does not fix presentation or accept the native controls.

## Minimal UI control experiment

The existing `voxy_render/examples/ui_window.rs` was built and run without a font,
scene, editor panels or asset loading, using the same patched surface stack.
Bundle: `/tmp/Voxy Surface Control-2026-10-02.app`.
Initial executable SHA-256:
`d417bf09d63170b6d4c3ca485893d7374a3fbcbdaa206bdffcdb9f7513cc4cba`.
Build log: `/tmp/voxy-control-ui-build.log`.
It reproduced the blank dark client and Metal `occlusion=8192` even as a key,
visible, nonminiaturized window. Thus the failure reproduces independently of
editor/prefab/asset code; this does not distinguish a common surface defect from
the native window/display environment.

The control experiment also exposed premature UI hit-region publication: clicking
the blank client changed the title to `Voxy UI toggles: [true, false, false]`.
The example now publishes candidate hit regions only after RenderOutcome::Presented
and clears them after skipped/reconfigured outcomes. It uses the existing WindowUi
router; no additional history or input system is introduced.
Rebuilt executable SHA-256:
`1ec62741dcdd8803e285cb08782526f09588b09f8709eb875219f3fdab438263`.
Build log: `/tmp/voxy-control-ui-admission-build.log`.
After Raise, the same client click, Tab and Return, CUA observed the title remain
`Voxy UI — mouse, Tab, Enter/Space`; the client still remained blank. This proves
the unpresented-control activation regression in the reproduced native case,
not rendering acceptance or behaviour after a presented frame.
Runtime log: `/tmp/voxy-surface-control.log`.
Both launches were closed through native Quit and the matching process disappeared.
Boundary/whitespace checks pass. The presentation root cause remains unresolved.
