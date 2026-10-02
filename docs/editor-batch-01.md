# Priority batch 01: items 1–10

User-requested execution order: complete priority groups of ten. This batch is
implemented and verified for the bounded OBJ/identity-camera editor described
below. The acceptance is scoped to those supported objects, not full engine parity.
Overall Stride/Unity/Godot capability coverage remains tracked separately.

| Item | Acceptance | Current state |
| --- | --- | --- |
| 1. Visual transform, drag and undo | Actual window input visibly previews, commits, cancels and undoes; correlate Presented frames | Passed: OS input, Presented traces and screenshots showed numeric movement, axis drag and visible undo; cancellation/redo tests passed |
| 2. Selection highlight | Chosen instance has visible distinct decoration; inactive/deleted selection does not draw | Amber local-bounds overlay implemented; GPU pixels verified and native window screenshot viewed |
| 3. Mouse selection with overlap | Nearest actual triangle wins; holes/bounds do not hide a hit behind; inactive/singular objects skipped | Passed for identity camera: real OS clicks selected foreground triangle and background square through the triangle bounds hole; exact mesh/depth regression passed |
| 4. Translation gizmo | Visible axis handles constrain dragging, commit one undo and support cancellation | Passed: RGB GPU pixels, pointer X/Y/Z tests and real OS X-axis drag; X changed 0.200 to 0.286 while Y stayed 0, undo visibly restored X=0.200 |
| 5. Multiple model resources | Different logical models coexist with independent reload; shared resource instances remain shared | Passed: per-object model IDs and per-resource GPU table; real square/triangle window; independent corrupt/recovery reload and persistence regression |
| 6. Scene tree | Objects can be selected and managed through a visible hierarchy panel | Passed for supported model objects: visible scrollable tree, pointer selection, duplicate/delete buttons |
| 7. Transform/component inspector | Visible validated property editing participates in undo | Passed: visible numeric TRS, finite input validation, Enter commit/Escape cancel, active/model properties and undo; actual OS numeric input verified |
| 8. Editor hierarchy | Reparenting and inherited transforms/activity integrate with tree, picking and gizmo | Passed: choose parent in tree, detach, inherited activity, subtree deletion/undo; inverse-parent conversion verified under rotated/scaled parent |
| 9. Full editor scene persistence | All supported objects, hierarchy and resource references round-trip; failures preserve state | Passed for supported objects: real save/delete-subtree/load restored hierarchy; file/fresh-App tests preserve IDs, activity, TRS and distinct resource references |
| 10. Play/Stop | Runtime copy can simulate without mutating authoring; Stop restores authoring | Passed: native Play rotates a detached runtime graph and Stop visibly restores authoring; regression proves history is unchanged |

Earlier validation: eight editor tests; actual-GPU readback of selection and gizmo
on Metal, 120 submissions per pose without CPU waits. RGB handle pixel counts
were [60,60,16] per pose; amber counts [1024,894,1024]. A 0.5 translation moved
centroid approximately 64 pixels at 256-pixel width, and reset restored exact
pixels. Bounds outline is not a silhouette, and world handles are not yet a
perspective-camera gizmo. Overlay presentation is derived data, not a persisted
scene component or an imported resource.

Final batch verification (2026-10-01): ten editor regression tests, strict editor
Clippy across all targets, 40 asset tests and 41 scene tests, native reload/manifest relocation/scene harness (228
Presented frames), plus direct OS UI acceptance. The save file from the native
two-resource test had `model-1.parent = model-0`, with separate `quad` and
`triangle` references; loading after subtree deletion restored the nested tree.
Window occlusion during automation can leave a stale screenshot; only fresh
presentation/unocclusion screenshots were treated as visual proof.

Scope: max 128 supported model objects/resources, manifest identities discovered
at project open, existing bounded OBJ import limits, identity camera, local-bounds
selection outline and world translation handles. Play currently runs a rotation
demonstration; generic gameplay/physics adapters belong to the next batch. Native
accessibility, Unicode label fallback and complete text-editing widgets remain
UI work. This batch does not claim Unity/Godot/Stride feature parity.
