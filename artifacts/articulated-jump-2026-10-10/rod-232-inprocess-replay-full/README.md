# Full qualification with rod-232 failure replay

Original 720-frame request and all density/parity settings retained. Targeted operator capture is rod 232, frame 3, limit 1024. A failing frame triggers an in-process diagnostic replay before the unchanged gate rejects it. The preceding three-frame attempt rejected configuration before simulation; its log is preserved separately. This is not an FPS benchmark.

## First completed frame snapshot

Frame 1 passed original parity gates: maximum position error 7.736196887091207e-10 m; quaternion component error 3.4670877471551887e-8. Native solver wall time 10625.239209 ms, external 91592.546125 ms, with 6093 coordinate calls and 35058 submissions. No native fallback. These are instrumented physics fixture wall times, not rendered FPS. The process was live when captured; no later frame or terminal result is inferred. See frame-one-work-snapshot.json.

## Original frame-three failure localized

Original frame 3 exceeded the unchanged position/quaternion gates, with max position error 0.00022248771285753033 m. Before the in-process replay completes, original traces have been preserved separately in original-frame-three-phases.log and compared in original-phase-comparison.json. First over-gate phase: rod 232, substep 0, contact-positions iteration 3; position error 4.360276746667724e-5 m and quaternion component error 0.0013343173844616585. Replay remains a separate pending result; these files do not establish its endpoint reproducibility.

Terminal result: exit 101 after 5120.25 seconds. Original frame 3 parity failed at 0.00022248771285753033 m position drift. In-process replay reproduces original native and external endpoint pose bits exactly. Phase comparison/report equality is recorded in terminal-validation.json; replay determinism does not qualify the original parity gate, complete mutable state, whole trajectory or FPS.
