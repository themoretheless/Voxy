# Authoring the motion joint

ModelAnimation adds root_motion_joint (u16) to editor.model-animation.v1, with
serde default zero so existing component JSON remains readable. The ordinary
component inspector displays Motion bone. Generic scene persistence and history
include this value. Scene admission checks the global joint limit before import,
and checks the published CPU rig count when available. Runtime admission repeats
validation against its accepted model revision.

ModelPlayback applies selection at construction and exposes a setter that keeps
its clock. AnimatedModels applies it to a staged playback clone before publication;
invalid settings cannot replace the current owner. Bind-only playback reports the
configured joint with zero displacement. Existing clip and speed behavior remains.

93 editor tests and all 6 editor GPU tests pass. Inspector tests cover invalid
indices, rejected edits without history mutation, selection undo/redo and document
round-trip. Old JSON loads with zero selection; owner selection keeps playback time.
Release native smoke edits the motion joint through ordinary inspector input,
checks undo/redo, switches three Fox clips, runs two owners sharing GPU sources and
stops with zero animation bytes and restored authoring state. A separate native
CUA pointer/keyboard edit visually confirms 1 Motion bone in the reviewed screenshot.
The final label-only build was visually checked after the CPU/GPU/native checks.
Artifacts: artifacts/rig-root-authoring-2026-10-03/.

This is numeric selection, not a named bone picker or stable reference across
reordered reimports. Root displacement still has parent-local coordinates and is
not applied to the character or removed from the pose. World conversion, named
selection, rotation extraction, in-place conversion and collision application remain.
