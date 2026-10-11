# Contact support across a tessellated surface

The preceding finite-face fix retained support only on the single triangle selected by the minimum-distance query. On a square made from two triangles it omits an outer support endpoint (fraction 0.25). The saved before log reproduces that failure. Production discovery now inspects every overlapping near-parallel face and retains its supported interval endpoints after the existing unexpanded-face, query-radius and global-nearest-triangle checks. Original minimum-distance contacts remain. No physics admission tolerance changes.

The regression checks both triangle orders, two world orientations, three scales (0.01, 1, 100), both velocity directions, and independent surface/velocity support at both outer endpoints. A further moving-collider case checks face-dependent affine surface velocity, including a 2 m/s support velocity on one side of the shared edge. Discovery preserves all rod positions.

Three face regressions pass; the additional moving-surface regression passes. The broader physics unit run completed 351 passed, 39 ignored, zero failures before adding the moving test. Hair integration: 21 passed, zero failures. The current compiled Metal physical-scene regression passes (4 guides, 4 frames, exact serial pose/rotation, zero native fallback).

All 24 captured transformed static-surface velocity operators pass same-input Metal/native and cooperative-batch admission with zero fallback; maximum linear response difference 1.3100631690576847e-14 m/s. Both additional moving-surface operators pass with zero fallback; maximum linear difference 1.5987211554602254e-14 m/s. Their stored operator tolerance is approximately 2e-9 after existing operator preparation; the independent physical moving-surface test uses the unchanged 1e-9 endpoint velocity allowance. Replay binary provenance and explicit limits are in same-input-gpu-results.json.

The existing full-density 720-frame run in ../flat-face-manifold-batched-full-capture/ remains live but predates this cross-triangle fix. It must not qualify this later source. No full jump or rendered >160 FPS result is established.

A separate full-density qualification containing the cross-triangle fix is now launched in ../tessellated-manifold-batched-full-capture/. Its launch is not evidence of successful physical admission or FPS. The previous single-face full run is preserved independently.
