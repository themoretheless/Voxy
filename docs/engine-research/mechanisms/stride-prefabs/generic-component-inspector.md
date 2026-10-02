# Registered component fields

The editor's field-mode control now cycles from Mixer to Component fields and
then back to Transform. Component fields enumerate registered serialized data,
including nested objects, strings, numbers, booleans, nulls and array elements.
Six rows are displayed per page, with previous/next controls. The selected row's
component schema is displayed separately. Field paths use escaped JSON pointers;
enumeration rejects more than 256 leaves or nesting deeper than 32.

Edits preserve the scalar type and use the existing component codecs and complete
scene validation before committing a candidate document to authoring history.
Invalid type, descriptor and model-resource values cannot replace the live scene.
Undo/redo uses the same history as other inspector controls. Editing is blocked
during Play. Numeric strings are not implicitly converted into numbers.

Prefab differences expose Reset controls. An array element's Reset restores its
whole containing array, preserving current atomic vector/array override semantics.
Explicit resets can split legacy whole-component overrides while retaining other
edits. Generic edits still use the existing prefab capture/save path; this adds
no second reflection registry or parallel prefab storage format. Keyboard target
identity includes object ID, component schema and field path instead of row index.

Tests cover native key-adapter input into the generic editor, validation rejection
without live mutation, undo/redo, Play rejection, escaped keys, Unicode strings,
null/type preservation and focus identity after reindexing. The existing composed
scene integration additionally exercises generic member edits, override markers,
atomic array Reset, undo, save and reload. Real panel construction includes the
new inspector mode. Native-window visual acceptance of this mode remains open;
the earlier Reset window review does not prove its layout.

Collection topology editing, stable collection-item identity, empty-container
creation, component add/remove discovery and schema-driven field names/ranges
remain necessary. Serialized array indices are positional controls, not stable
collection IDs. Only components installed in the editor's current registry can
be edited; adding a runtime component alone does not register its codec.

[Field bindings](collection-authoring-operations.md) now preserve the editing
target across row reindexing and reject concurrent scalar changes. Declared
collection bindings use item IDs; installing custom codecs and native collection
controls remain separate work.
