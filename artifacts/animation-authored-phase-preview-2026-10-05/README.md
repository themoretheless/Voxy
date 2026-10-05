# Stateless authored phase preview

AnimationClip::try_sample_phase samples normalized phase in [0,1] through the
existing validated local sampler, preserving the authored endpoint for looping
clips. Ordinary time sampling retains its loop/clamp policy. Phase preview
neither advances Animator clocks nor emits events. ModelAsset::sample_pose_phase
exposes the same behavior with selected-clip index and bind-pose support.

Core regression: authored phases 0,.25,.5,1 over a two-second loop, final pose
versus ordinary loop reset, invalid/nonfinite phases, finite skin matrices and
unchanged running Animator phase. Imported model regression: real GLB phase
sampling/palette generation, bind pose and invalid phase/missing clip handling.

Verification: 220 animation library tests; one focused imported-model test;
177 editor library tests including normally ignored GPU controls. 398 unique
passed, zero failed/ignored. git diff --check clean. Local uncommitted changes.

This is the core preview API. Editor phase selection, live GPU request ownership,
timeline and retargeted preview integration remain pending. No new native preview
was rendered and the previously opened editor binary predates this source change.
The overall engine parity/hardware/research/physics goal remains active.
