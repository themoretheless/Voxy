# Original-column contact velocity full trajectory

Separate 720-frame full-density native/GPU paired run launched after source and binary freeze. Child13189 confirmed running, exec session13030; old cooperative process86606 and original historical process52287 preserved. This experiment changes the numerical velocity projection method; it is not a restart of the previous run due to slowness.

Native and GPU-QR contact velocities use original-column square-root physical projection, retaining original 1e-9 m/s velocity tolerance. Full trajectory/model admission remains unchanged. Cooperative physical/Newton/equality batching, early release, frozen hints, validated prefix reuse and power-of-two RHS normalization enabled. Phase observations target rod227 on frames4-6, including actual predicted inertial positions, linear and angular velocities. No extra same-input native audit is enabled. No rendered FPS claim.

Prelaunch actual Metal physical island test passed: four guides/four steps with velocity correction enabled, exact serial/batch pose and rotation, 219 equality dispatches in64 submissions, zero native fallback. Three phase observation invariance tests passed. Source/binary hashes and exact runtime handles preserved. Initial prelaunch build briefly saw concurrently edited phase fields; final rebuilt source and tests passed, final log retained. git diff --check passed.

Pending full720 completion, original drift/contact/strain gates, rendered secondary-motion evidence, >160 FPS, full hardware and broader engine feature goals.
