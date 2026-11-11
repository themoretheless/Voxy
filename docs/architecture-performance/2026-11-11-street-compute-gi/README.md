# Street compute GI: surface correspondence

Base: `d1b018baf`. Apple M4 Max, Metal, original street at 1920×1080 with
Temporal AA, lossless authored textures, 161 mesh batches, 3,634,960 source
triangles and 120,739 instances. No idle-window coordination. Experiments use
the same read-only material snapshot. Quality bounds are unchanged.

The baseline now uses a bilateral filter when the forward pass consumes
compute GI. Inline GI consumes one ray's radiance and coverage. The stage
crossover selects the unfiltered texel so it does not change both the
execution stage and the spatial filter. The independent compute GI mode
retains its existing filter, and the default inline path is unchanged.

The native frame-120 screen isolates the remaining error:

| Candidate | Scene-before-TAA channels outside bound | Final-output channels outside bound |
|---|---:|---:|
| Same-pixel compute result | 4,616 | 49,628 |
| Shared forward pipeline, same-pixel result | 4,616 | 49,637 |
| Same-pixel result, world-position and normal guard | 0 | 5,902 |

A separate diagnostic with GI disabled found 3 scene channels outside the
bound. It is not a timing or quality qualification for the full workload.
The compute texture and standalone raster reference agreed in all 8,294,400
Float32 words on frame 120, with 294,637 positive-coverage reference pixels.
This compares the same stored history, rather than claiming equivalence to the
full forward pass.

The guard accepts the result only when the guide is populated and its world
position and normal match the shaded fragment; otherwise it traces the
original ray inline. The large reduction in error is consistent with a
surface correspondence problem. This experiment does not separately prove
the effects of interpolation, equal-depth draw ordering or compilation.
Although the guarded scene target passes on frame 120, the TAA history and
final output still fail. No tolerance, render resolution or ray settings
were relaxed.

## Full-route timing: surface guard

Eight runs: AA, AB, BA, AB, BA, BB, BB, AA. Each run measured 2,400
1920×1080 frames after 180 warm-up frames, one fixed 120 Hz simulation step
per rendered frame. Timing waits for the exact last GPU submission.
Frame pacing is the interval between host observations of completion,
including event-loop gaps; no display scanout is measured.

Both A/A controls pass. The between-run A/A GPU mean increased by 14.0%
from start to end; this drift is retained, not normalized away.

| Pure compute route | Full-frame p99 | Completion-interval p99 |
|---|---:|---:|
| BB 0 | 35.665 ms | 35.889 ms |
| BB 1 | 35.392 ms | 35.600 ms |

Bootstrap intervals from the four counterbalanced route contrasts:
completed-frame p99 ratio [1.111, 1.153], completion-interval p99 ratio
[1.104, 1.146], and GPU timestamp-span mean ratio [1.132, 1.149].
The surface guard regresses this workload and fails the 8.33 ms gate.

The measured binary is reconstructed by applying `surface-guard.patch`
to the base revision and building `cargo build --locked --release -p voxy_app
--example street_walk`. Use the prior
`2026-10-27-gi-bottleneck/math-match/stage/run.py`, `quality.py`,
`raw_full.py` and `analyze.py` with the unchanged `quality-policy.json`.
The checked-in run manifest freezes the input and executable hashes. This
performance campaign is for the surface guard, not the narrower
same-pixel sampling change.

## Full-route native quality

The three capture routes have identical replay hashes and use the unchanged
native-stage policy. The initial frame-120 screen had zero outside-bound
scene channels with the guard; the separate full-route repetition has three.
The other four scene targets pass. The output still exceeds the bound at
three phases:

| Replay frame | Scene channels outside bound | Final channels outside bound |
|---|---:|---:|
| 120 | 3 | 5,903 |
| 600 | 0 | 0 |
| 1200 | 0 | 16 |
| 1800 | 0 | 26 |
| 2520 | 0 | 0 |

These captures do not qualify the surface guard. In particular, matching a
scene target at the capture instant does not guarantee its accumulated TAA
history is equivalent.

The standalone compute/reference check on the full route agrees in all
41,472,000 Float32 words, with finite results and nonzero coverage in each of
the five captures. This passes the raw GI gate; it does not override the
failed native image, relative performance or absolute p99 gates.
`qualification.json` can be regenerated from the checked-in `runs/`,
`quality-route/quality.json` and `raw-route/raw-proof.json`.

Validation of the narrow sampling change: release example build; 360
renderer tests, 23 street_walk tests and 28 qualification analyzer tests
pass (66 renderer and two example tests explicitly ignored). Neither the
surface guard nor the shared pipeline experiments are enabled by the
sampling change.

The follow-up combines the guard with one forward shader for both arms and
does the guarded lookup at the inline ray's original evaluation point. The
guard checks the bit patterns of the world position and normal. Frame 120
matches the retained original inline capture before TAA, but 5,903 final
channels still exceed the bound (max absolute difference 0.00634765625).
Neither a shared forward shader nor bitwise guard comparisons resolve the
remaining temporal discrepancy. See `guard-shared-forward.patch` and the
two `phase-120/guard-shared*.json` comparisons.

The pushed sampling path also completed all three native frame-120 smoke
runs. The unchanged quality check found 4,616 scene and 49,628
final channels outside the bound. The change removes the spatial-filter
confound from the crossover; it does not qualify compute GI.
