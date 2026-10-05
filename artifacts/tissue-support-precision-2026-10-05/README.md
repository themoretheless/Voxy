# FEM support precision

Attachment coordinates remain f64 when transformed by the f32 animation palette promoted to f64. This removes reference-coordinate rounding at the render/physics boundary; it does not improve the precision of the imported palette itself.

The identity-palette regression verifies exact preservation of all three supports per region, including coordinates not representable as f32. The rotating-bone regression verifies transformed support positions and secondary free-node response.

Validation: release voxy_app tissue_demo::tests, 14 passed, no failures. git diff --check clean for tissue_demo.rs.

The imported CesiumMan fixture still rejects step 11 with a support-work defect and preserves the complete prior state. Admission tolerance is unchanged. Character/material-frame placement and a successful imported-rig tissue animation remain unresolved. No new native or GPU visual verification was performed for this precision change.
