# Target clip event preview

ClipEvents binds authored normalized markers to a clip and previews the target
Animator phase interval without changing time. Events use (start,end] semantics;
phase zero does not auto-fire at initial playback. Loop seams return prior end
events before next start events. Equal-phase authored order is stable. Paused
ticks emit none. Foreign clips, invalid ticks/markers and event budget overflow
reject before returning a partial list. Preview is to be delivered only after
the corresponding animation tick commits.

All 217 animation release tests pass. New controls cover multi-loop seam order,
pause, budget, invalid time, foreign clip, malformed phase and clamp completion
without repeated endpoint emission. First fixture construction used an empty
track list and failed the existing constructor invariant; it was corrected to
one default track per joint before the final run.

This is a core API foundation. Editor dispatch, audio/game callbacks and policy
for source events during crossfades remain unfinished; engine goal remains active.
