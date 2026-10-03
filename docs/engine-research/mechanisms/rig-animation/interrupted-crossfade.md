# Interrupted crossfades

Previously an interrupted fade restarted from its target clip, discarding the
already blended pose. Animator.transition_to now samples that current blend and
admits its palette before changing state. The new transition holds the captured
source while its target plays. Ordinary uninterrupted transitions continue playing
their source clip. A zero-duration switch remains immediate.

The captured pose uses Arc ownership, so transactional Animator clones share its
storage instead of copying the joint vector each frame. Only one snapshot belongs
to each active transition. Replacement/completion releases it; errors retain the
prior clock and transition. Binding and numerical admission remain enforced.

Regressions cover repeated interruption, exact translation continuity at zero
timestep, shared snapshot storage, release on replacement/completion, failed
advancement and subsequent recovery. Rotated/scaled hierarchical palettes remain
continuous within 2e-6. All 22 animation, 92 editor and 131 renderer ordinary tests
passed; the final additional hierarchy test was followed by a complete animation
suite rerun. Logs: artifacts/rig-interrupted-fade-2026-10-03/.

This guarantees pose continuity in Animator, not velocity continuity. The source
snapshot freezes during the new fade. Inertial blending, foot/contact locking,
root-motion blending, downstream IK capture, editor transition controls and
native-window interrupted-fade acceptance remain incomplete. Root displacement now blends across active fades; see
[root motion blending](root-motion-blending.md) for policy and remaining limits.
