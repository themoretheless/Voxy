# Reset a field within an identified prefab collection

Generic inspector override markers and Reset now use the installed component
registry to distinguish identified collection items from ordinary arrays. For a
declared collection, the field binding captures the item ID; the inherited value
is looked up by that ID in the prefab baseline, independently of array positions.
Reset replaces only that member of the current item. A vector/array nested inside
an item is still reset atomically, and undeclared component arrays retain their
existing atomic Reset semantics.

The same `ComponentField::prefab_reset_value` resolves both marker comparison and
Reset. The former duplicate generic whole-array code in the built-in Reset helpers
has been removed. Built-in transform/material/physics/audio field targeting retains
its existing rules. No parallel collection registry or independent history is added.

Reset publishes the validated candidate and recaptured prefab metadata through
`SceneHistory::commit_with_metadata`. Other changed fields, local additions,
deletion tombstones and local ordering are preserved. Undo restores both the
original field value and metadata; redo reapplies the precise member reset. After
saving, the cleared member override inherits later source changes.

An identity member has no Reset button. A locally added item has no inherited
member and cannot use field Reset. Missing/ambiguous item identities, absent
inherited members and changed prefab dependencies reject Reset without publishing
an authoring document or history version. Item lookup borrows current/baseline
payloads; it clones only the chosen replacement value.

The editor integration regression uses a typed custom collection in a persistent
prefab instance. It reorders items, adds one, deletes another and changes fields on
two inherited items. Reset on one item preserves all unrelated edits and topology,
round-trips undo/redo metadata, omits the reset member from saved overrides, then
inherits changed values after the source is reordered and updated. Reset on another
item remains precise after that reload. Added-item and identity Reset are rejected.
A separate regression covers escaped identity/member keys, atomic nested vectors,
source/current index differences and rejection of ambiguous baseline IDs.

Whole-item Reset is also available in the Collections inspector. The button is
shown only for an item whose complete payload differs from its inherited item,
or for a locally added item. Its action captures the collection owner/schema/path
and item identity, rather than a row index. Reset restores the inherited payload
at the current position; resetting a local addition removes it. Other additions,
deletion tombstones, member overrides and retained-item ordering stay intact.
Member and whole-item Reset share source validation, candidate validation and
the existing metadata/history publication path.

The persistent regression dispatches whole-item Reset through the actual panel
geometry with an explicit simulated Presented acknowledgement. It verifies full
payload restoration, removal of a local addition, undo/redo, saved override
cleanup and preservation of the unrelated deletion and ordering. A changed
external prefab rejects Reset without changing the document or history metadata.
This is headless input/geometry coverage, not native visual acceptance.

The Collections inspector also provides Reset order when the current order
differs from the implicit inherited order. It retains the current payload of
every item, sorts surviving source items by baseline position, then places local
additions at the end in identity order. It does not restore deleted items. The
same validated prefab/history transaction clears the explicit order override;
undo/redo restores document and metadata together. The regression dispatches the
button through presented panel geometry and verifies the retained deletion,
cleared order override and undo/redo. Native presentation is still unverified.
The final order-button checks pass: 81 editor library tests (two GPU tests
ignored), application library/binary compilation, formatting of changed editor
modules, whitespace and the four-owner/32-package boundary gate. Logs:
`/tmp/voxy-order-reset-final-verified.log` and `/tmp/voxy-order-reset-app.log`.

Restore deleted is now available per collection. It restores all missing baseline
items with their inherited payload, inserting each beside the nearest surviving
source sibling while keeping the relative order and payload of current items.
It also clears historical deletion tombstones whose IDs are absent from the
current source. This metadata-only change uses the same undo/redo transaction,
so a source item that returns later can be inherited again. The control is
hidden during Play and when neither missing items nor tombstones remain.

The persistent regression covers a reversed collection, a restored source item,
retained explicit order, cleared deletion metadata and undo/redo. It also removes
the deleted item from the source, clears the remaining tombstone without changing
the document, saves/reloads, then returns the source item and checks inheritance.
Presented headless panel geometry covers the Restore deleted button region.
Restore deleted restores all deletions in the chosen collection. A separate
deleted-item chooser now offers selective restoration, three entries per page.
Its heading cycles pages independently of the current-item page. Labels use a
bounded inherited name or ID; actions retain the complete durable ID hash and
collection key. Missing-source tombstones remain selectable. The list and bulk
button share one derived snapshot, not separate deletion storage. Page changes
invalidate presented input targets until the new page is rendered.
The editor action layer now additionally supports restoring one deleted item by
the durable collection key and hashed item ID. It reuses the topology transaction,
rejects unknown/ambiguous deleted targets, and clears only the selected tombstone.
Other missing source items are not inserted. The persistent regression deletes
two source items, restores one, verifies the other tombstone, rejects repeated and
unknown targets without history changes, then checks undo/redo. Its absent-source
case now uses this selective action and still round-trips save/load and later
source reappearance. The persistent regression dispatches selective restoration
through presented headless panel geometry; a separate four-entry geometry fixture
checks the second page's fourth-item target and absence of the third-item target.
The editor's initial/minimum logical height is now 680 pixels so the chooser and
existing controls fit at 1x DPI; standalone game sizing retains its existing rule.
Native visual acceptance is still pending.
Chooser verification: 81 editor library tests pass (two GPU tests ignored),
application library/binary compilation and formatting/boundary/whitespace checks
pass. Logs: `/tmp/voxy-deleted-chooser-final-verified.log` and
`/tmp/voxy-deleted-chooser-final-app.log`. These are headless geometry/action checks;
the native window presentation problem remains unresolved.
Selective-action verification: 81 editor library tests pass (two GPU tests
ignored), the application library/binary compile check and boundary/whitespace
gates pass. Logs: `/tmp/voxy-selective-restore-tests.log` and
`/tmp/voxy-selective-restore-app.log`.
The geometry regression exposed overlap with the common component actions;
Collections now moves those actions and keyboard hints below both topology
buttons. Final verification: 81 editor library tests pass (two GPU tests ignored),
application library/binary compilation and boundary/whitespace checks pass.
Logs: `/tmp/voxy-restore-deleted-final-layout.log` and
`/tmp/voxy-restore-deleted-final-app.log`. Native visual acceptance remains open.

Still pending: native visual acceptance. This change does not resolve the
surface visibility problem recorded in the native collection review.

Order capture now compares against the same implicit order used by expansion:
surviving source items followed by local additions in identity order. A local
addition at that default position no longer creates an unnecessary explicit
order override. Subsequent source reordering is therefore inherited, while item
member edits and deletion tombstones remain intact. Inserting a local item
between source items, or choosing another local ordering, still records the
explicit order and round-trips it. Existing saved explicit orders are preserved.
The regression `default_additions_do_not_freeze_inherited_order` covers both
cases, including source additions, source value updates and a deleted item.
Verification of this order-capture change: 65 scene library tests, 15 persistent
prefab tests and 81 editor library tests pass (two GPU tests ignored). Logs:
`/tmp/voxy-collection-default-order.log`,
`/tmp/voxy-collection-default-order-persistent.log` and
`/tmp/voxy-collection-default-order-editor.log`. The boundary and whitespace gates
also pass.

Verification: 81 editor library tests pass (two GPU tests ignored), including the
persistent custom-prefab and escaped-key/vector regressions. The application
library/binary compile check passes with existing warnings; the four-owner/32-package
boundary gate and diff whitespace check pass. Final logs:
`/tmp/voxy-whole-item-reset-status.log` and
`/tmp/voxy-whole-item-reset-status-app.log`. Native visual proof remains open.
