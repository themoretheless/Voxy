# Marker add/remove controls

Component inspector shows Add beside the atomic marker list and Remove beside
each marker name. Add inserts a valid event named event at phase 0.5; scalar
fields edit it afterward. Controls are absent while playing. All edits use the
existing component codec/document/history transaction; no runtime ownership copy.

Actions carry a BLAKE3 digest of durable object ID and observed marker list.
Changing owner/list invalidates stale deletion or insertion. The digest is
computed once per component-panel build and checked against the current document.
Add respects the existing 4096-marker capacity. Remove checks index bounds.

Regression checks actual generated panel rectangles and hit actions for both
buttons, document mutation, stale removal rejection, and undo/redo restoration.
An initial test insertion accidentally landed inside assert! and failed to compile;
the fixture was corrected before the final successful run.

Final release editor suite including normally ignored GPU controls: 176 passed,
zero failed/ignored. git diff --check passed. Local changes remain uncommitted.

Still pending: native visual marker-panel inspection, timeline dragging/scrubbing,
stable marker identity for fine-grained prefab inheritance. Full engine goal
remains active and incomplete.
