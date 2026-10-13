# Exact column scaling without uploaded subnormal components

The experimental triple input codec now rejects nonzero subnormal f32 expansion components, which FTZ GPU arithmetic cannot preserve. It tries the original binary column scale first and then at most 32 exact binary downshifts. Every normalized coefficient must round-trip exactly through the complete triple expansion and original scale. No nonzero coefficient is discarded and no physical tolerance changes.

The maximum normalized column coefficient remains below 2^33. With the existing codec coefficient-count bound (<2^23 values), a complete vector squared norm stays below 2^89, inside f32 range. This bounds this input-norm calculation, not every possible solver intermediate. Unsupported dynamic ranges remain explicit errors. Bounds and tolerance keep their existing independent scale; their uploaded nonzero components must also be normal.

Verification:
- 17 regular renderer triple tests and 5 regular app solver tests pass.
- Nine actual GPU renderer tests pass across the broad gpu_triple filter and separate original-column-norm replay. The initial broad invocation omitted the VQC1 fixture variable for the column-norm test; that test was rerun successfully with the required fixture. Both logs are retained.
- The four-frame/four-guide physical scene now passes exact serial-versus-batch positions AND orientations with zero CPU/native fallback. Single dispatch batches: 20 dispatches / 20 submissions versus serial 80 dispatches. Full and compact readback modes both pass.
- App exact cache, duplicate sharing, failed-batch atomicity, cold retry, factors and workspace gates pass in full and compact readback modes.
- All three captured native physical families (91 x 882, 100 x 882, 55 x 2142) pass with changed bounds and appended inactive loads in compact mode.

The earlier four-guide NumericRange failure is resolved for this scene. This does not prove full 469-guide jump completion, all floating-point ranges, realistic visual hair, production default readiness or >160 FPS. Long older full-cycle processes use frozen older binaries.
