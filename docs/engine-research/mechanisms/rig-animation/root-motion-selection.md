# Selecting the motion joint

Animator.set_root_motion_joint selects a validated u16 joint index. Default zero
preserves existing callers. The displacement curve is compiled when selection or
clip changes, not during frame advancement. Ordinary transitions retain their
source curve and the target curve; selection updates both without advancing time.
An invalid index changes no selection, curve, clock or transition. Transactional
Animator clones share their current compiled curves through Arc.

AnimatorFrame.root_motion_joint identifies the selected joint. root_motion is its
translation displacement in parent-local space. It excludes ancestor animation
and is not removed from the pose; consumers must not silently treat it as a world
transform. In-place conversion, rotation extraction and gameplay application are
separate requirements. Missing translation tracks produce zero displacement.

The unchanged pinned Fox asset demonstrates why selection matters: its static
container produces zero displacement, while b_Hip_01 yields nonzero local movement
in Run. Extracted motion matches sampled hip translation differences within 1e-5,
and the complete pose remains unchanged. Tests also cover selecting during a fade,
source/target curve updates, invalid selection and immediate clip switches.
255 distinct ordinary tests passed (31 animation, 92 editor, 132 renderer); the
additional imported-Fox test passed after the full regression suites. Logs:
artifacts/rig-root-selection-2026-10-03/.

Numeric editor selection is now implemented; see [authoring](root-motion-authoring.md).
Named selection, ancestor/model/world conversion, rotational extraction,
in-place conversion, collision-driven application and native-window acceptance
remain incomplete. Selected nonzero-joint caches are currently owned by Animator;
sharing them across independent owners and CPU cache budgets remain future work.
