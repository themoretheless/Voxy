# Editor panel presentation and input admission

Panel construction is a candidate publication, not proof that the user saw it.
`Panels::build` revokes input readiness. Only the outcome of the editor's actual
`SceneSurface::render_scene` call can acknowledge that candidate: `Presented`
enables its pointer targets and keyboard actions. Reconfigured, timeout, occluded,
validation-skipped and suspended outcomes disable input and cancel armed keyboard
activation and pointer hover. Window resize, focus loss and explicit occlusion also
invalidate presentation. A later successful present restores input readiness.

The existing panel focus context now includes the whole authored document as well
as selected object identity and inspector mode. Keeping object IDs unchanged is not
sufficient to admit old targets after component, transform or hierarchy data has
changed. UI input requires both an acknowledged panel candidate and a matching
current context. Programmatic authoring APIs remain independent of native window
admission; headless acceptance fixtures explicitly acknowledge a simulated present.

During Play the editor panel renders the saved authoring document, while the game
updates the runtime scene. Drawing and targeting both obtain their document through
`App::panel_document`, so runtime motion cannot disable the visible Stop button by
making a different document appear to be the panel's source. Game UI retains its
own existing presented-layout admission and input owner.

Native authoring shortcuts and field commits are also blocked while the panel has
no presented candidate. Escape can still cancel an unfinished edit. Skipped frames
do not clear typed text or mutate the document/history. A captured Enter/Space
press is canceled when presentation is lost; releases and auto-repeat cannot
activate it after recovery. Editor viewport selection/drag initiation is blocked
while authoring panels lack presentation.

Regressions cover every non-presented render outcome, pointer and Tab/Enter
blocking, held-key cancellation across recovery, a fresh activation after recovery,
changes to authored data without changing IDs, rebuild invalidation, explicit
presentation invalidation used by resize/focus handlers, hidden undo and field Enter,
Escape cancellation, and Stop targeting after runtime transforms diverge. Existing
collection and prefab hit-region fixtures now explicitly acknowledge presentation.

This fixes input publication ordering. It does not cure the blank native review
surface or claim native visual acceptance. For the installed `wgpu-hal 30.0.0`
Metal backend, `metal/surface.rs::acquire_texture` returns `SurfaceError::Occluded`
when the hosting NSWindow's visibility bit is absent. A null `nextDrawable` instead
returns Timeout. The observed native `SkippedOccluded` therefore follows that
backend visibility check; why the OS reported the window invisible remains unproven.
No renderer visibility check, dependency code or OS protection was bypassed.

The previous native attempt is recorded in
[native collection review](native-collection-review-2026-10-02.md). Its frozen
binary predates this admission fix and is not evidence of the new code running.
The next native acceptance must use a newly built binary and exercise controls
only after a presented frame is observed.

Verification: 79 editor library tests pass (two GPU tests ignored), including the
native-key adapter and Play/Stop regressions; the application library/binary compile
check passes with its existing 15 warnings. The four-owner/32-package boundary gate
and diff whitespace check pass. Final logs are
`/tmp/voxy-panel-presentation-live-context.log` and
`/tmp/voxy-panel-presentation-final-app.log`. No new native visual proof is claimed.

## Shared native UI adapter

`voxy_ui_winit::WindowUi` now exposes `set_presented_regions(regions, presented)`
and `invalidate_presentation()`. A skipped frame removes targets, cancels pointer
and keyboard capture, clears hover/cursor and disables wheel routing. Held
activation keys remain tracked across frame skips: after publication resumes,
a repeat or release cannot activate a newly focused target. Resize, explicit
occlusion and native focus loss also invalidate geometry. Occluded(false) alone
does not republish it; the owner must acknowledge a newly presented layout.

The existing `set_regions` remains immediate for callers that already manage their
own admission; its documentation now makes that contract explicit. No renderer
dependency or second input router is added. `ui_window` uses the shared admission
method after the surface outcome and invalidates targets for insufficient space.

Three adapter tests pass, including capture cancellation, recovery with held keys,
fresh activation after release, resize and occlusion recovery. The UI example
compile check and architecture/whitespace gates pass. Logs:
`/tmp/voxy-ui-admission-tests.log` and `/tmp/voxy-ui-admission-example.log`.
The earlier native control experiment proves the example's unpresented-click
guard in its previous binary; this new shared API version has headless regression
and compile coverage, not a new native presented-frame acceptance result.
