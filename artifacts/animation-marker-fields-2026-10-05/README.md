# Individual marker inspector fields

The atomic /events list stays available for insertion/removal, and its item
name/phase fields now appear separately through the existing generic inspector.
Their scalar edits use existing component validation, history and scene storage.
Bound scalar marker edits retain the observed marker list; a changed list rejects
the commit and cancels stale editing during reconciliation. This conservatively
rejects changed lists rather than inventing durable marker identities.

Expanded inspector admission to 16384 fields so the permitted 4096-marker list
can expose all 8192 scalar fields plus its atomic list and other component fields.
The existing depth bound remains. The actual panel input/Enter path edits a
marker phase and undo restores the full document. Reordered markers with equal
phases cannot receive an old edit. All 4096 markers are enumerated in regression.

Final release editor suite with all normally ignored GPU controls: 176 passed,
zero failed/ignored. git diff --check passed. Local/uncommitted work.

Adding/removing still uses the atomic list field. Timeline dragging/scrubbing,
dedicated add/remove buttons, stable marker identity and native visual UI
qualification remain pending. Overall engine objective remains active.
