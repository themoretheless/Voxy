# Cubic quaternion underflow

Ordinary zero quaternion intermediates now use the same wide Hermite fallback
as nonfinite intermediates. A nonzero wide result is scaled before f32
normalization; a genuinely zero wide result remains invalid. Regression uses
opposite signed identity endpoints and a smallest-positive-f32 tangent: its
midpoint term is below f32 range but gives a defined normalized half-turn.
Removing that tangent yields a zero sample and remains rejected.

All 215 animation and 171 editor controls, including 11 GPU tests, pass: 386
passed, no failures or ignored cases. This is finite-precision sampling recovery,
not an exact curve singularity certificate or full hardware/engine completion.
