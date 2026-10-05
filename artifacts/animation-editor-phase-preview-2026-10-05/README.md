# Editor authoring pose preview

Preview beside each marker phase samples the selected clip at its authored phase
without advancing a game clock or emitting events. Repeat the same valid preview
to return to the ordinary bind-pose view. Immutable target-bound frame is stored
in AuthoringSession and sent through the existing AnimatedModels GPU/LOD owner.
No duplicate model renderer or gameplay playback instance is introduced.

The preparation supports named selection and source retarget compilation and
reuses prepare_displayed_frame for retarget/root extraction. Frame use checks
selected owner, target identity, authored settings/profile and retarget source
identity. Stale configuration drops preview requests; fresh selection replaces
only after successful preparation. Entering Play clears authoring preview.

Regression uses a real imported GLB through editor panel actions, checks sampled
pose, unchanged scene document, empty game-event queue and unchanged tick serial,
stale settings/token rejection, toggle-off and Play/Stop authoring restoration.
GPU panel test includes Preview glyphs and control hit non-overlap.

Final release editor suite with all normally ignored GPU controls: 178 passed,
zero failed/ignored. git diff --check passed. Local/uncommitted changes.

Native live verification on 2026-10-05: a separate current-binary review app
loaded the isolated saved scene and self-contained GLB staged in /tmp. Clicking
Preview at phase 0.25 visibly translated the skinned triangle to the right; a
second click restored its original position. This is UI observation, not a
quantitative pixel comparison. The earlier marker review app was preserved.
The launch using the workspace asset directory blocked before window creation;
staging the asset locally resolved this review launch, cause not established.
The small window also showed a white lower viewport; zooming the window gave
a complete frame. This rendering issue remains unqualified.

Pending: retarget-preview-specific qualification, free phase slider/timeline,
joint/foot preview policy and stable marker IDs.
Overall engine parity/hardware/research/physics goal remains active.
