# Per-channel STEP animation

The glTF importer now accepts STEP and LINEAR channels in the same clip.
Interpolation is stored separately for translation, rotation and scale, with
joint indices remapped through the same hierarchy mapping as the key streams.
Existing AnimationClip::new callers retain LINEAR behavior; the additive
new_with_interpolation constructor validates one mode descriptor per joint.

STEP returns the previous key inside a segment, the new value at an exact key,
and the nearest endpoint outside the channel range. Quaternion STEP values are
returned directly without interpolation. Playback looping/clamping remains a
clip-level clock policy. Missing channel data retains the joint's bind property.

Primary specification:
https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#appendix-c-animation-sampler-interpolation-modes

Regression coverage includes adjacent floating-point times before/after keys,
nonzero initial key time, final channel key, mixed STEP translation/rotation and
LINEAR scale, negative loop time, loop endpoint, bind fallback and invalid mode
counts. A parsed synthetic glTF distinguishes STEP geometry from LINEAR geometry.
The upstream RiggedFigure and Fox GPU fixtures change only sampler metadata to
STEP and use the existing CPU/GPU position and normal comparison plus visible
rendering. Fox passes 195 poses across three clips (position error <=1.53e-5).
RiggedFigure passes 65 frames (position error <=1.20e-7); its two keys are at
0 and duration, so loop STEP must hold the first pose and remain motionless.
The initial generic motion assertion failed on this legitimate static loop;
the final regression explicitly checks that it remains static.

12 animation, 90 editor and 128 renderer ordinary tests passed. The renderer
final run supersedes an obsolete test that rejected STEP. Two GPU regressions
passed on Apple M4 Max / Metal. Evidence is retained under
artifacts/rig-step-interpolation-2026-10-03/. No native STEP screenshot or
non-Metal hardware acceptance is claimed.

CUBICSPLINE was subsequently implemented; see cubic-interpolation.md for its
validation, GPU evidence and limits. Morph animation, editor blend controls and
retargeting remain incomplete.
