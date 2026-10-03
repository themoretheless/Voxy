# Rig binding and compatibility

Clips and poses retain an Arc reference to the immutable ordered Joint layout.
Compatibility uses Arc pointer identity first, then exact structural equality:
ordered bone names, parent links, local bind TRS and inverse-bind matrices. This
allows exact independently reconstructed rigs without hashes, process-local IDs
or copying the joint array into every clip/pose. Different layouts of the same
joint count are rejected; this is not automatic retargeting.

AnimationClip.try_sample, Pose.skin_matrices and Pose.blend enforce binding.
Animator.transition_to rejects a foreign target before changing the current clip,
clock or transition, including immediate switches. Animator.advance checks both
active clips against the supplied rig before clock advancement. ModelPlayback
rejects an incompatible selected clip before creating the owner. The legacy
infallible sampler retains source rig defaults/binding even if given a foreign
skeleton argument; checked runtime APIs report SkeletonMismatch.

Tests cover exact independent reconstruction, changed names/order, changed parents,
local binds and inverse binds, blend weights 0/0.5/1, unchecked sampler binding,
foreign transitions and failed advance with active crossfade retention/recovery,
and editor rejection before owner creation. Numerical palette overflow is still
tested inside a compatible rig with an unfinished transition, rather than by
silently replacing the source skeleton. 242 ordinary tests pass (19 animation,
92 editor, 131 renderer). Artifacts: artifacts/rig-binding-2026-10-03/.

Exact comparison intentionally does not equate differently signed quaternion
representations, differently named bones or approximate bind values. Retargeting,
external clip import/remapping and persistent asset identity remain separate
requirements. GPU palette APIs accept matrices and cannot recover originating
clip identity; binding is enforced before matrix publication by the animation
owner. Structural fallback costs O(joints); shared owners use the pointer fast path.

GPU regression: 4 selected editor and all 15 renderer GPU tests passed on Metal.
