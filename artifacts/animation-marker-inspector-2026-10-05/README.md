# Inspector editing of animation marker lists

The generic component inspector exposes editor.model-animation.v1 /events as
an atomic JSON array field, including the empty list. This permits adding,
renaming, moving and removing markers through the existing field editor and
component validation/history transaction. No separate playback/editor owner is
introduced. Example: [{"name":"step","phase":0.25}]. [] clears markers.

The actual panel Field action, text input and Enter path are tested for adding
two markers. Undo/redo restores complete documents. Out-of-range phase 1.1 is
rejected without a document mutation. Clearing the array keeps it accessible.

Release editor library suite, including ignored GPU tests: 176 passed, zero
failed/ignored. git diff --check passed. Changes local/uncommitted.

This is a baseline JSON list editor, not a visual animation timeline. Dedicated
marker rows, dragging/scrubbing and visual native-editor acceptance remain
pending. Broad engine parity/hardware/physics goal remains active.
