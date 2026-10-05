# Finite animation interpolation

Pose blending and linear vector tracks share a stable interpolation helper.
Ordinary finite f32 results retain the existing arithmetic path. Endpoints return
exact authored vectors. When the ordinary intermediate subtraction overflows,
f64 interpolation recovers a representable convex result. Invalid input/result
pose and palette admission remain intact.

The regression blends opposite 2e38 translations at endpoints and quarter/mid
weights, compares sampled tracks and poses and validates skin palettes. All 212
animation tests and 160 default editor tests pass. Of 11 default-ignored GPU
controls, the playback/CPU equivalence/failed-revision control was explicitly run
and passed. The other 10 GPU controls were not run in this increment. Total 373
passed, zero failed. Broad hardware/CUDA and full engine scope remain unfinished.
