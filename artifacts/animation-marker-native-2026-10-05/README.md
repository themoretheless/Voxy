# Native interactive marker acceptance

Current release native_editor_review launched with the existing animated-triangle
skeletal fixture through /tmp/Voxy Marker Review.app. Real CUA window input and
screenshots confirmed: add animation component; Add marker; type 0.25 into phase
and Enter; Remove; Z undo restores the exact phase; Y redo removes; Z restores;
F5 saves; Remove then F9 reload restores the saved marker. F6 Play animates the
model and hides edit buttons; F6 Stop restores authoring and edit controls.

scene.json is the actual saved scene, verified to contain exactly one event
named event at phase 0.25. native.log and report.json retain authoritative review
project/PID and input evidence. Tool screenshots were observed inline; they are
not the previous offscreen panel.png. The native window remains running for
inspection. Review source scene is isolated; user project assets were not edited.

No new algorithm change or test-count increment this turn. This qualifies one
native window/model/scale. Timeline/scrubbing, stable marker IDs, full engine
parity, hardware/CUDA, broad research and physical simulation goal remain pending.
