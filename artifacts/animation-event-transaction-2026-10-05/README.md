# Accepted animation frame and event delivery

Animator::advance_wall_with_events previews target-clip markers and then uses
the existing atomic frame advance. It returns the frame and events together
only after success. EventTickError preserves event versus animation failure
identity. No extra animator clone or independent clock system is introduced.

All 219 animation release controls pass. New regressions prove budget failure
preserves phase, successful subsequent steps emit once, a preview containing
a marker is discarded after a late singular pose, and a repaired shorter tick
recovers that marker. Caller callbacks must wait for acceptance of any enclosing
world transaction. Editor/audio hookup and source-crossfade policy remain.
