# Stride prefab identity and property inheritance: partial source review

Repository: stride3d/stride, pinned commit
`a7fa31ced680c7d4a919f1fe233051cf508f6060`. Four source/license files are saved
with immutable URLs, byte counts and SHA-256 in `sources.json`; all verified.
This supplements the corpus's README/root evidence without claiming a full audit.

## Source observations

- `Prefab.cs:30-32` delegates runtime instantiation to `EntityCloner.Instantiate`.
  This review does not inspect that cloner or prove runtime clone isolation.
- `AssetCompositeHierarchyPropertyGraph.cs:24-29` keeps mappings keyed by base
  part ID and instance ID, including deleted-part mappings. `DeleteParts:196`
  records source/instance pairs; `ShouldAddNewPartFromBase:454` checks tombstones.
  `UpdateAssetPartBases:604` rebuilds mappings from reachable hierarchy parts and
  manages base-asset event subscriptions. These are editor-side relationships.
- `FindBestInsertIndex:474` locates surviving instance siblings by their base IDs,
  rather than treating source list position as identity. This is not evidence of
  bounded work or transactional failure recovery.
- `EntityHierarchyPropertyGraph.cs:44` rejects duplicate component references and
  duplicate singleton types. `CloneValueFromBase:73` preserves the existing
  TransformComponent while clearing its Entity relationship for reconciliation.
  Voxy should preserve transform invariants through validated data publication,
  rather than copying these mutable object relinking steps.

The official [property override guide](https://doc.stride3d.net/latest/en/manual/game-studio/prefabs/override-prefab-properties.html)
describes an overridden property retaining its local value while inherited
properties follow the source. The [asset introspection guide](https://doc.stride3d.net/latest/en/manual/engine/asset-introspection.html)
describes collection item identities and deletion metadata that survive reorder.
These current manuals are supplementary behavior documentation, not pinned source.

## Voxy decisions and uncovered gap

Keep explicit durable object/instance identities, deletion tombstones and typed
reference remapping. Preserve the existing pure expansion -> validation ->
publication model; do not introduce a second mutable reflection graph or event
subscription network into the runtime.

At the initial review, Voxy's `ObjectOverride` stored individual node properties, but its
`components` map replaces or removes an entire serialized component.
`capture_edits` stores a full component value when any member changes. Thus editing
one component field masks future source changes to its other fields. Property
inheritance parity is incomplete even though nested round trips pass.

Implement explicit member overrides with these acceptance requirements:

1. Editing one member preserves source updates to unedited siblings, across nested
   instance expansion, save/load and source revisions.
2. Resetting a member removes its override; distinguish missing values, JSON null,
   component deletion and complete component replacement.
3. Member addresses must be schema-validated; collection indices cannot stand in
   for stable item identity. Treat unidentified collections as atomic values until
   registered stable-item codecs exist.
4. Remap only declared object references after applying overrides, then validate
   the whole component/scene. Invalid paths/schema changes retain last publication.
5. Preserve existing full-component override files and editor undo/redo metadata;
   do not silently reinterpret old files as field deltas.

No Stride build, editor runtime experiment or performance comparison was run.
The initial review required the implementation below.

## Implemented adaptation

`ObjectOverride.component_members` now stores existing object-member JSON pointers.
Capture compares codec-normalized/remapped values and emits only changed members;
arrays are atomic values. Applying edits rejects missing components/paths, invalid
escapes, array indexing, overlapping paths, whole/member conflicts and member
quotas before final component/scene publication. Null remains an explicit value.
Existing whole-component overrides retain their semantics when recaptured.

Eleven persistent-prefab integration tests pass, including nested save/JSON
roundtrip, reference remapping, inheritance after source change, reset, null,
escaped keys and stale-path rejection. All 71 editor library tests pass (two GPU
tests ignored), including real editor save/reload of a local angular speed edit
that inherits a changed source axis. Package boundaries and whitespace checks pass.
Numeric inspector rows now mark differences from the prefab baseline with `*`
and expose a Reset button through the existing pointer/keyboard target router.
Transform, physics, material, motion and audio targets share atomic vector resets.
The editor integration verifies panel hit testing, reset, undo/redo, clean save,
stale-source rejection and explicit splitting of a legacy whole-component override.
All 71 editor tests pass after this addition. Generic component-member controls,
identified collection-item edits and native-window visual acceptance remain open.

The subsequent [generic component inspector](generic-component-inspector.md)
adds paginated registered scalar fields, type-preserving edits, scene validation,
shared undo/redo, prefab markers and atomic array Reset. Collection topology and
stable item identity remain open, as does native visual acceptance of this mode.

[Identified collection inheritance](identified-collections.md) now implements
explicit codec declarations and ID-addressed collection override data. It retains
new source items, inherited sibling properties, deletion tombstones and remapped
references. Editor Add/Delete/Move controls and stable collection focus remain
open; this does not close the full collection-editor requirement.

A subsequent [native Reset review](native-reset-review-2026-10-02.md) verified
the angular-speed marker, pointer reset, undo and saved empty overrides. It also
found and fixed a clipped button label; a fresh build displayed the full label.
This closes that specific visual defect, while broader native acceptance remains
open.
