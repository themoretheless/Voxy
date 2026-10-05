# Actual GPU marker inspector evidence

A new ignored-by-default real-GPU test builds the existing Panels geometry and
512x128 glyph atlas, renders through SceneRenderer, reads native device pixels,
and validates each Add/Remove button contains glyph pixels. It checks field hit
rectangles do not overlap marker buttons. No adapter fallback/skip is permitted.

Native Apple M4 Max / Metal qualified. First snapshot showed schema footer
collision with the last row, truncated Remove and noisy raw field paths. Fixed
footer baseline (270->284), widened marker controls, human-readable name/phase
labels, compact marker-count display and Clip name/Animation labels.

panel.png is the final 1024x640 offscreen GPU image of the real native panel code,
not a screenshot of an interactive window. PPM is raw readback for reproducibility.
No game viewport content was included. Original compile attempt used nonexistent
upload_geometry; corrected to existing upload_mesh before successful validation.

Final editor release suite including all ignored GPU controls: 177 passed,
zero failed/ignored. git diff --check passed. All work local/uncommitted.

Remaining: visual timeline, scrubbing/dragging, stable marker IDs and native
interactive acceptance across window sizes. Overall engine goal remains active.
