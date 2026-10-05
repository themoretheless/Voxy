# Local-displacement tissue solve and neutral animation

The implicit contact unknown is now displacement from the start of a substep. Inertia and its residual use local displacement; material and contact geometry still evaluate world positions. Final impulse/drift, continuous collision admission, energy budgets and atomic commit remain unchanged. The tiny-step impulse regression now uses a stricter 1e-18 J budget, requiring equilibrium refinement below world-coordinate resolution.

Validation: 21 prescribed-contact tests, 8 viscoelastic tests, 18 tissue tests and the tightened tiny-step test passed. The imported contact regression still FAILS at step 52 (implicit contact line search failed). A diagnostic rerun reproduces it with initial nearest gap 4.0866e-8 m; the rejected trial preserves state. Full character-surface contact is not qualified.

A fresh non-contact render completed all 480 simulation steps / two seconds, producing 41 frames on Apple M4 Max / Metal. The four-pose strip was visually inspected. motion.gif is assembled from those actual rendered frames. These are neutral diagnostic FEM pads driven by imported skeletal supports, not integrated anatomical skin or calibrated physiology. Simulation plus rendering took 9.83255 seconds in this run; realtime is not established.

All source changes remain local, uncommitted and unpushed.
