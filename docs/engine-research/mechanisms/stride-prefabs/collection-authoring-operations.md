# Collection authoring operations and editor field binding

`CollectionEdit` provides Insert, Remove and Move operations. The collection is
selected by object ID, component schema and declared collection path; targets and
insertion anchors use item IDs. An absent anchor means the end of the collection.
`SceneDocument::edit_collection` validates a candidate before publication.
`SceneHistory::edit_collection` applies it through the existing bounded history,
preserving metadata and redo on failure or a no-op. Invalid references, duplicate
IDs, vanished anchors and invalid typed component payloads cannot modify history.
New-item data and its identity are supplied explicitly by the caller; this API
does not invent schema defaults or allocate persistent item IDs.

The generic editor now captures a field binding when editing starts. Enter commits
that binding rather than looking up the current row index again. The binding owns
object ID, schema, path and original scalar value. Declared collection fields also
capture item ID and the member path within the item. Confirmation resolves the
current item position by ID, rejects disappearing or ambiguous IDs, and rejects a
value changed since editing began. Item identity members cannot be edited in place.

On panel rebuild, typing follows the same bound field to its new row and page.
Missing fields or a changed selected object cancel the edit. Keyboard focus uses
the same binding identity for registered fields. A regression exercises typing an
angular speed while adding a material that shifts the component row indices; Enter
still edits angular speed and leaves the material unchanged. A second concurrent
speed change is rejected and does not create an extra history version.

Tests additionally exercise item insertion before a bound field, ID-based member
resolution, stable binding identity across reindexing, missing items and immutable
IDs. The history test covers Insert/Move/Remove, full undo/redo, invalid typed data,
invalid object references, missing anchors, collision rejection and no-op redo.

Remaining: native visual
acceptance. [Collection member Reset](collection-member-reset.md) now preserves
unrelated item edits and topology; ordinary and nested numeric arrays remain atomic.
The built-in codecs declare no identified collections. Applications can now install
custom codecs and declarations before scene loading using the entry points below.
Native visual acceptance of these controls remains pending.
Editor input now follows [presented-panel admission](editor-panel-presentation.md).
The surface visibility issue and native acceptance remain open.

## New-item defaults

An installed codec can now supply a default object for each declared collection
with `set_collection_default`. `new_collection_item` materializes it with a
caller-owned ID, and `CollectionEdit::InsertDefault` inserts it through the same
scene/history validation. This works with an empty collection; it does not copy an
existing item. Template IDs are removed so each insertion receives its own ID.
Templates are bounded to 64 KiB and require an object shape. Their concrete field
types and references are validated in the complete candidate scene during Insert.
Failed replacement of a template preserves the previous registered default.

The default-insertion test verifies creation in an empty typed collection,
separate supplied IDs, collision rejection, invalid typed template rejection during
insertion, undo/redo preservation, empty-ID rejection, and template shape/size
limits. The native inspector controls below allocate persisted item identities.

Verification: 64 scene library tests, 15 persistent-prefab integration tests and
81 editor library tests pass (two GPU tests ignored). The four-owner/32-package boundary check passes. The all-examples application
check fails in the unrelated existing `hair_render`, `skin_render` and `female_render` examples because they omit
`female_transmission`; this is not a passing all-examples gate. Native visual
acceptance remains pending.

## Application component installation

`editor_component_registry()` returns the editor's built-in codecs. Applications
extend that registry with typed codecs, reference visitors, identified collection
declarations and optional item defaults, then consume it with
`run_model_viewport_with_components(source, mode, scene_path, registry, ui_actions)`.
Registration happens before loading the authoring scene. The ordinary entry points
retain their built-in configuration.

The registry has one immutable shared owner for the application session. The App,
panels and AuthoringProject workers use the same `Arc<ComponentRegistry>`, including
capture, field binding and focus, history, duplication, prefab expansion/export,
save/load, Reset, audio auxiliary history and Play/Stop. There is no mutable global
registry or second per-worker schema authority. Registration does not supply custom runtime update systems or native acceptance.

A typed custom-list regression verifies generic field editing, undo/redo, scene
save/reload, Play/Stop, worker registry identity, default materialization and worker
prefab preparation with the same custom codec. Two GPU tests remain ignored and no
native custom-collection acceptance has been performed in this change.

## Inspector collection controls

The Components screen now cycles to Collections, then back to Transform. Collections
shows each explicitly declared array even when it is empty. Click the collection
heading to choose another array. Each page contains three items with Delete, Up and
Down controls; Previous/Next navigate item pages. Add is present only when the codec
has an item default. Ordinary arrays receive no topology controls.

Control targets contain hashes of the durable owner/schema/path tuple and the item
ID. Editing resolves these targets in the current document, rejects missing targets,
uses `CollectionEdit` for candidate validation and publishes through the existing
history. Moving an item does not retarget an already captured delete action. All
controls are disabled during Play. No separate collection state or history is added.

Add obtains 256 random bits from the OS and stores `item-<hex>` as the actual scene
item ID. There is no counter to reset on undo or application restart. Randomness
failure aborts insertion; current ID collisions are rejected by candidate validation.
This is probabilistic uniqueness, not a globally coordinated ID service. Saving,
loading and undo/redo preserve the stored IDs; removing an item and adding another
generates a fresh identity rather than recycling the deleted identity.

`cargo run -p voxy_editor --example collections` opens a typed Inventory example
with an initially empty list. An optional first argument chooses the scene path.
Existing files are loaded; startup never overwrites them. The default scene path is
in the OS temporary directory; its embedded quad model is created alongside the
scene only when absent. Cycle the inspector through Behavior, Audio, Mixer,
Components and Collections, then use Add and the item controls. F5 saves the scene.

The control regression builds the actual Panels geometry, clicks its hit regions,
adds to an empty list, moves both directions, uses a captured deletion after a
reorder, rejects a stale deletion without modifying the document, exercises undo
and redo, saves/reloads IDs, creates a new identity after deletion and reload, and
checks that Play prevents editing. This does not substitute for native visual QA.

Native review attempt: [2026-10-02 report](native-collection-review-2026-10-02.md).
The native window opened but its surface reported `SkippedOccluded`; the empty
client area does not prove collection controls visually. The isolated test app was
closed through its native Quit command after recording that limitation. The
focused application library/binary check and the collections example build pass.
