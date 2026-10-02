# Identified collection inheritance

Collection identity is declared on an existing component codec with
`ComponentRegistry::declare_identified_collection(schema, path, identity_member)`.
Declarations use object-member JSON pointers and a string item-ID member. Arrays
without declarations retain atomic behavior, including existing vector fields.
There is no inferred `id` convention or parallel reflection registry.

`ObjectOverride.component_collections` records changes by schema and collection
path. Within a collection, durable item IDs address additions, deletion tombstones,
member overrides, atomic replacements and optional local ordering. Member edits
inherit changes to other source properties. Item shape changes use an atomic
replacement. A local order follows IDs; new source items are inserted relative to
surviving source siblings. Source insertion cannot retarget a member edit.

Recapture preserves an explicit deletion when its source item is temporarily
absent. A regression first reproduced that deletion being lost, then passed after
the fix. An absent tombstone is metadata: resetting it explicitly requires removing
that collection override, because copying an identical absent value cannot express
the user's reset intention. Ordinary recapture does not silently discard it.

Addition-ID collisions, disappearing member-edit targets, identity mutation,
overlapping member/collection paths, duplicate IDs and undeclared collection
paths fail validation. Collection application builds a candidate before replacing
the value. Final scene validation and existing reference remapping still govern
publication. Tests verify source-local reference capture for both member edits
and locally added items, then remap them to instance object IDs during expansion.
Existing whole-component overrides preserve their atomic semantics.

Limits: 16 declarations per codec, 256 items per collection, 256 item operations
per override, 256 member overrides per item, 32 pointer levels, 1024 pointer bytes
and 256 item-ID bytes. IDs are local to the declared collection. Collection paths
cannot traverse another array by position. Codec-normalized data is checked again
so a codec cannot discard the declared identity during serialization.

Verification: 62 scene library tests and 15 persistent-prefab integration tests
pass. These include source insertion/local reorder, inherited sibling properties,
deletion/disappearance/reappearance, JSON roundtrip, reference remapping, failed
application preserving the original value, invalid declarations and duplicate IDs.

Editor topology controls remain open: Add/Delete/Move for declared collections,
schema-provided new-item defaults, stable item targeting in the inspector, explicit
tombstone Reset, and native visual acceptance. The generic inspector still uses
positional array paths and atomic array Reset; it does not yet expose these
collection identity controls. This mechanism is not full Stride/Unity/Godot parity.

The subsequent [authoring operations and field binding](collection-authoring-operations.md)
provide validated ID-addressed Insert/Remove/Move through scene history and capture
editor field targets before typing. Native topology controls remain pending.
