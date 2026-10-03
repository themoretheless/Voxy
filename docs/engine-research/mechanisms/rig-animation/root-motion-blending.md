# Root displacement blending during fades

Animator blends source/target root displacements during the active fade interval
by integrating their translation curves against the linear transition weight. A timestep that crosses completion is
split: the remaining interval uses only target displacement. Constant-velocity
clips therefore produce the same displacement for one large step and corresponding
smaller steps, including loop crossings. LINEAR, STEP and CUBICSPLINE now use analytic weighted integration; see
[root curve integrals](root-motion-integrals.md).

A source snapshot from an interrupted fade is frozen and contributes zero root
displacement. This preserves existing snapshot semantics but does not preserve
velocity on interruption. Zero speed yields zero root displacement; fade elapsed
still uses wall-clock timestep. Root translation reads only the root channel and
its bind fallback, avoiding complete pose-vector allocations for root queries.

After a fade completes, its zero-weight source pose is no longer sampled/admitted.
This permits completion when the discarded cubic source happens to be invalid at
that instant; the target pose and palette still pass admission. Errors retain the
prior animation clock and transition through transactional staging.

248 ordinary tests passed: 25 animation, 92 editor and 131 renderer. Runtime changes
were tested across all three suites, followed by an animation-suite rerun after
adding the completion-source regression. Artifacts:
artifacts/rig-root-motion-blend-2026-10-03/.

Root motion is displacement of joint zero in clip space. Rotation extraction,
root selection, in-place pose conversion, gameplay/collision application, editor
controls and native-window acceptance remain
requirements. No new hardware acceptance or FPS claim is made by these CPU tests.
