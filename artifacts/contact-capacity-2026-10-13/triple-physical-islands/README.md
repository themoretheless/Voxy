# Four-guide original physical batch gate — NOT PASSED

Extended the existing four-frame/four-independent-guide physical scene gate to select the experimental single-dispatch kernel explicitly and require fewer GPU dispatches than serial solving. Both paths still compare original rod positions and orientations exactly and require zero native fallback. The initial attempt used the default backend inadvertently; the test now explicitly creates the requested triple program.

Current evidence: positions and orientations match across all four steps and dispatch sharing assertions pass. The zero-native-fallback gate FAILS: serial solving falls back once, while batching falls back for all four owners in the affected request. No tolerances or requirements were weakened.

The new opt-in VOXY_HAIR_TRIPLE_PACKING_FAILURE_DIR capture preserves exact original coordinate input bits and seeds in unique create_new JSON files. It does not contain original physical matrices/loads and is not a physical replay artifact. Two captured failures contain identical 8 x 54 coordinate operators. Four nonzero coefficients around 1e-27 cannot be reconstructed exactly with the existing column normalization (scale 2048). This is host input packing rejection, before GPU dispatch; no shader error or physical acceptance is implied.

Host packing audit: shifting the column scale down by four binary exponents permits exact three-float reconstruction, but still uses subnormal f32 components. Shifting down by 27 exponents gives exact reconstruction with only normal nonzero f32 components. This identifies an exponent-range issue and a candidate scaling strategy, not a validated fix. GPU FTZ behavior and every original physical gate must be verified before adopting it. Removing small coefficients is not acceptable.

Five regular app solver tests pass. The new opt-in physical scene gate remains failing. Full jump, production integration, realistic hair, and >160 FPS remain incomplete.

## Subsequent resolution

The original failed evidence above is retained. The normal-component column scaling change passes this four-guide physical gate with 20 batched versus 80 serial dispatches, exact poses/orientations, and zero native fallback. See ../triple-normal-component-scaling/ for current source hashes and qualification logs. This resolves this captured scene, not the full jump or FPS target.
