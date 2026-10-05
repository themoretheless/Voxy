# Arbitrary authoring phase seek

Component inspector has a phase track independent of event markers. Pointer X
maps continuously to authored [0,1] phase. Left press captures owner/track bounds;
move samples the same existing immutable pose preview, release/resize/cursor
leave/occlusion/focus loss cancels capture. Owner change/Play rejects continued
capture. Keyboard activation selects midpoint. Markers retain toggle behavior;
free seek does not toggle off when repeating a phase. Preview invalidates panel
cache so the current phase and track cursor redraw.

Tests drive generated inspector hit regions at arbitrary fractions, marker-free
phase selection including endpoint and repeated endpoint, unchanged authored
document, invalid phase retention. GPU readback checks seek track glyphs and no
field-hit overlap alongside existing marker controls. Retarget preview tests
remain enabled. Release suite: 179 passed, 0 failed/ignored; diff check passed.

Native drag/DPI/focus lifecycle not yet qualified in a live window. This is a
pose seek track, not a complete animation editing timeline. Local uncommitted
work; broad engine/hardware/research/physical realism objective remains active.

Capture follow-up: pointer movement now checks current owner, inspector mode,
settings, current target asset identity, retarget profile and source identity
before resampling. Stale capture cancels instead of driving new configuration.
A new press clears the old capture and failed activation clears capture. Tests
exercise 1x/1.5x/2x logical mapping, beyond-end clamping, unchanged document,
settings-change cancellation and invalid pointer cancellation. Latest full
release suite including GPU tests: 179 passed, zero failed/ignored (2.09s).
Native event delivery and pointer lifecycle remain pending live qualification.

Native seek verification: current rebuilt example in separate Phase Seek Review
app, staged self-contained asset and seed. Native pointer drag changed caption
from 0.000 to 0.494, reverse drag to 0.006 visibly restored the skinned triangle
near initial location. F5 saved scene compared as JSON equal to original seed.
This proves native pointer routing/resampling and scene preservation; it does
not qualify every focus-loss or multi-monitor DPI transition. Earlier native
review processes preserved. PID 79507 left open for review.
