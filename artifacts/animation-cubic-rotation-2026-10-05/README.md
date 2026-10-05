# Finite cubic quaternion sampling

Nonfinite ordinary Hermite quaternion samples are recomputed with shared f64
coefficients and scaled before f32 normalization. The vector cubic fallback
uses the same wide evaluator, avoiding duplicate polynomial implementations.
Finite ordinary samples keep the existing path; zero samples remain invalid.

Independent controls check large equal tangents cancelling to identity, large
non-cancelling amplitude normalizing to a half-turn and opposite signed identity
keys producing a true zero quaternion. All 214 animation and 171 editor tests,
including the 11 real GPU acceptance controls, pass. Total 385, no failures or
ignored tests. This does not qualify CUDA or complete the broad engine objective.
