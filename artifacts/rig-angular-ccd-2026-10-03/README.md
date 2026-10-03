# Angular character CCD acceptance

Saved verification covers bounded animation clocks, scene/owner budgets, oriented
character translation and the angular sweep API. Angular integration tests include
mid-arc collision with equal endpoints, retained orientation/velocity, grounded
turning and atomic budget/invalid-request failure.

The GPU regression runs all seven device tests, including 128 logical animation
owners sharing one GPU source. Native oriented regression uses the committed
constant-parent rig fixture and checks collision-limited translation, preserved
orientation, in-place animation and Stop restoration. It does not exercise angular
CCD through animation: root rotation extraction and authored character angular
motion routing remain unfinished.

The release build completes with existing warnings. Logs retain the actual checks
and counts; this is not a complete Unity/Godot feature coverage certificate.
