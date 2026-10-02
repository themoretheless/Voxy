# Atomic animation evaluation

`Animator::advance` previously advanced target/source clocks and could finish a
transition before palette generation failed. `Pose::skin_matrices` validated
local TRS values but accepted overflow in hierarchy multiplication or inverse
bind multiplication. Consequently a failed frame could consume animation time
or publish non-finite palette data.

Animation evaluation now stages the small clock/transition state, sharing the
immutable clips through Arc. Only a fully validated frame publishes that state.
Global and palette matrices must be finite; root-motion overflow returns
`AnimationError::NumericalOverflow`. Errors preserve the previous clock and
unfinished crossfade. This is animation-state atomicity, not a transaction over
later GPU submission or scene/root-motion application.

The tests cover hierarchy overflow with finite local scales, inverse-bind palette
overflow at transition completion, exact clock/source-time/elapsed preservation,
successful retry against a control animator, and overflowing loop root motion.
All 11 animation library tests pass with zero ignored in
`/tmp/voxy-animation-atomic-pose-final-tests.log`. Formatting, whitespace and
production dependency boundaries pass.
All 5 model import/sampling regression tests also pass, zero ignored, in
`/tmp/voxy-animation-atomic-model-regressions.log`.

RAG references used as navigation and architectural context:

- `wiki://voxy-voxy-adr-0001-8f968a85` (document
  `1a54f62b-5e07-41c9-8d5c-1a796143b287`): one authoritative writer and atomic
  publication. Its historical V1 scope does not redefine the current goal.
- `wiki://voxy-archive-01a0f480-80bd-77c2-8a82-5629bb37157f` (document
  `47edc881-068f-4862-86e9-10d00b663c59`): historical 54-bone hand prototype was
  not accepted; thumb grasp deformation remained visually poor. This is a
  pointer to fresh reproduction, not current runtime proof or a finished rig.

Production work remains: integrate scene-owned playback with editor Play and
GPU deformation/LOD, verify transition/root-motion application, add rig editing
and layered/masked animation, and reproduce/fix the hand prototype's deformation
on the actual model with native visual acceptance.
