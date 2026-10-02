# Stride content lifetime: pinned source review

Repository: stride3d/stride, commit `a7fa31ced680c7d4a919f1fe233051cf508f6060`.
The manifest records ContentManager.cs, IContentManager.cs, TestContentManager.cs
and the root license with pinned URLs and SHA-256 digests. Original notices are
retained. Stride was not built, and its tests were inspected rather than executed.

## Observations

ContentManager maintains URL-to-reference and object-to-reference dictionaries.
ResolveUrl canonicalizes aliases before lookup/loading so aliases may share a
loaded object. Load deserializes under the loaded-assets lock. Its documented
contract reuses a loaded instance and increments its reference count. Unload by
object or canonicalized URL delegates to DecrementReference with public ownership.
The reference-count implementation is in another partial file and is not reviewed
here; cycle collection, disposal ordering and complete concurrency guarantees
remain unproven by these files.

Reload finds a previously loaded object, deserializes into that object, and can
switch its URL. DeserializeObject puts old dependency references aside before
reconstruction and decrements them afterward. This prevents premature release
while reusing dependencies on a successful reload; these statements alone do not
prove transactional rollback of every mutated object on serializer failure.
SimpleReloadData tests reloading an unknown instance, reloading a loaded instance,
and switching to another saved URL while preserving the same object variable.
Other inspected tests exercise shared references and canonical/alias loading.

## Voxy decision

Adapt canonical logical asset identity before cache lookup and explicit ownership
of dependent resources. Preserve Voxy's separation of import work, CPU publication
and GPU residency. Its scene nodes reference logical resources; immutable imported
revisions and renderer-owned allocations provide stable boundaries for cancellation,
last-good fallback, Undo and device isolation.

Do not transplant in-place reload semantics into the immutable revision path.
Publish only validated replacement revisions and retain previous CPU/GPU versions
on refusal. Do not replace the render-owner boundary with dictionary locking:
worker permissions and GPU completion lifetime are separate contracts.
These are workload-specific choices, not claims that Stride's locks or object
identity model are intrinsically bad or slow.

## Next implementation and review gates

- Review the Reference partial implementation, failure cleanup and dependency cycles.
- Verify canonical identity across load/reload/rename/aliases with real dependencies.
- Inject serializer/import failure halfway through dependency replacement; confirm
  previous revisions and ownership counts remain intact.
- Keep CPU ownership distinct from GPU image/geometry budgets and submitted work.
- Expand Stride review to entity processors, render features, shader composition,
  animation and editor prefab/undo mechanisms before claiming feature coverage.

This is one partial mechanism review, not a complete Stride architecture audit.
