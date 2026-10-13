# Owned expanded coordinate transport

The f64-only coordinate boundary discarded precision required by the 100-row coordinate constraints. Added ExperimentalTripleExpansion and ExperimentalTripleExpandedCandidate so all three original GPU components of BOTH coordinates and reactions are retained with their separate positive finite binary scale factors. There is no implicit scalar conversion or rounded-value field. into_parts transfers ownership; clones preserve every component and scale. retained_bytes reports variable-size contents for future cache accounting.

Full and compact expanded decoders share original immutable-header/status/iteration validation with the legacy scalar decoder. They validate every component as finite and check the exact sign of reaction sums using an error-free f64 grow expansion. Three finite f32 values cannot overflow/underflow f64 summation, allowing even wide cancellation to retain the correct sign. The new decoder does not require the scaled coordinate to have a representable f64 preview. Original physical admission remains mandatory.

Batch expanded_candidates validates table/length and returns no result prefix when a later candidate fails. The existing scalar API and its numerical conversion remain unchanged; no default precision allocation was added to that path.

Current-source verification:
- 21 regular triple renderer checks pass, covering sub-f64 component retention, full/compact equivalence, shared immutable gates, finite values, negative reactions hidden by wide cancellation, ownership after input release, range independence and atomic batch failure.
- Eight actual saved GPU snapshots replay bit-for-bit for every coordinate/reaction component and scale. Actual API coefficient inputs recreate the saved packed input exactly. Omitted mutable QR scratch is padded only for the full-decoder comparison; every decoded word comes from the actual gather.
- Compensated original-coordinate checking of retained components passes all eight captured snapshots. The legacy f64-only views still fail in four snapshots, matching the independent exact rational audit in ../triple-decode-quantization/.
- Nine existing actual GPU triple regressions pass after the shared header refactor. Five regular app solver checks pass.

Scope: checked, owned renderer transport. It is not a physical certificate. App result caches, aliases, the physics proposal interface and native triangular backtransform still use the old scalar route and must be migrated together. No side channel, hidden lookup or duplicate physical solver was introduced. The original 100-row physical case remains unresolved; full jump completion and >160 FPS are unproven.
