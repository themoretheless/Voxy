# Authored selected-clip events

ModelAnimation now serializes events (name, normalized phase). Old v1 components
without events deserialize with an empty list. Validation caps the marker count
at 4096, names at 1024 bytes, rejects empty/NUL names and nonfinite/out-of-range
phases. Events belong to the selected clip, including exact named selection.

ModelPlayback constructor binds authored markers. Runtime selection rebinds when
markers, selected clip, or imported source revision change, retaining pending
accepted occurrences. Marker removal clears future tracks. Unchanged ticks do
not rebuild tracks. Equivalent asset reload preserves phase and does not replay
the previous marker.

Tests: authored serialization/legacy compatibility, distinct model reload,
identity and single occurrence, invalid-marker transaction rejection, removal
across a loop. Existing real editor history/save/load/play/stop regression now
includes authored events. Initial augmentation inserted a direct scene mutation
after the history-recorded command; the fixture was corrected to include it
before that command. No history implementation change was needed.

Final qualification: release editor suite with all ignored GPU tests enabled:
174 passed, zero failed/ignored. git diff --check clean. All work local.

Pending: application/game callback integration, marker editing UX, explicit
source/fade event policy, event delivery for certified fade acceptance paths,
and qualification of changed-duration/incompatible-clip reload semantics.
The overall engine/hardware/physics objective is not complete.
