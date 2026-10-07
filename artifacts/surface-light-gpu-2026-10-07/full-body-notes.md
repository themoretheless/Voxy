Full-body static GPU diffusion gate

42342 original-body vertices; CPU f64 reference. The 512-iteration result meets max absolute error < 0.001 and squared original-equation L2 residual ratio < 1e-6. Error: 0.0000629900803. Wall time: 209.227 ms, including encoding, submission, wait, readback and CPU audit; this is not a GPU timestamp or FPS measurement.

Initial exploratory run passed at 1024 iterations, then failed map_async with BufferAsyncError at 4096. That log is retained. The gate now stops at the first qualified budget, tests 512 explicitly, and returns failure if none qualifies.

Runtime character integration is still pending. Static CSR coefficients are uploaded from CPU; deformed-geometry coefficient assembly has not moved to GPU. The displayed character still uses CPU diffusion. 120 FPS has not been achieved. This Jacobi prototype is not admitted as a performance replacement. Next: faster iterative solver and resident geometry/normal/lighting buffers, followed by runtime integration and full-frame measurement.
