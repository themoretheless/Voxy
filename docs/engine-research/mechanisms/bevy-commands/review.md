# Bevy deferred commands: focused source review

Reviewed Bevy v0.19.1, commit `b56fc29d3016e641754765244b5ba3f9cc504671`.
Saved files, upstream immutable URLs, sizes and SHA-256 hashes are in sources.json.
MIT/Apache licenses accompany the source snapshots. These snapshots are reference
material; no Bevy code is compiled into Voxy. This is one partial mechanism review,
not a whole-engine audit, upstream runtime verification or a new engine identity.

## Source findings

- `system/commands/mod.rs:205`: Commands implements ReadOnlySystemParam because
  immediate parameter access reads entity metadata; eventual commands can mutate
  the world. This label does not mean the command stream is semantically read-only.
- `system/commands/mod.rs:266`: rebound_to redirects commands to another queue.
  Its preceding documentation warns that allocated entity IDs may never be spawned
  or freed if that queue is never applied. This differs from Voxy's post-barrier
  spawned-result handles.
- `world/command_queue.rs:35`: heterogeneous commands occupy a packed byte vector
  with metadata/function pointers and unsafe implementation internals.
- `world/command_queue.rs:100`: apply takes exclusive World access and flushes its
  internal commands first; append at line 111 concatenates byte buffers.
- `world/command_queue.rs:314`: successful application resets vector length,
  retaining its allocation. This is a useful storage-lifecycle property independently
  of the unsafe packed representation.
- `schedule/executor/mod.rs:135`: ApplyDeferred describes visibility after deferred
  systems and between dependency-ordered systems. This review inspected that marker
  and documentation, not the complete automatic barrier-insertion algorithm.
- `system/commands/mod.rs:694`: queue_handled supplies per-command error policy;
  queue_silenced explicitly discards errors. Public Commands documentation describes
  a configurable fallback that defaults to panic.

## Decisions for Voxy

Keep deferred structural writes and an explicit publication boundary. Do not infer
parallel safety from a system's ability to enqueue writes: direct scene access and
captured external state still need independent ownership rules. Preserve command
order, surfaced errors, generational handles and destination SceneId admission.
The low-level Bevy queue has different world-binding responsibilities; this review
is not evidence of an upstream defect.

Retain storage across Voxy command applications using safe Vec draining. The previous
mem::take/IntoIter path released the queue allocation at each barrier. This is now
changed in voxy_scene/src/commands.rs without adopting byte packing, raw pointers or
Bevy internals. Existing count bounds and typed component operations remain intact.
No measured throughput or latency improvement is claimed.

Keep Voxy's current post-apply spawn handles until reserve/cancel semantics and
resource ownership are designed explicitly. Keep append admission atomic on scene
identity and command count; UI dispatch preflights the destination before invoking
application state. Command application and captured callback state remain outside
that admission transaction. A queue count bound does not bound arbitrary component
payload allocation or caller-captured resources.

## Acceptance limits

The local scene regressions must preserve creation order, component/name operations,
failed-command continuation, foreign-scene rejection, batched local writes and
append rejection. Separate benchmarking is required before selecting byte packing
or claiming an allocation/latency advantage. No Bevy build, benchmark or full ECS
safety audit was performed. Broader scheduling, scene/prefab and engine parity work
remains open.

Validation checkpoint: 57 voxy_scene library tests passed after the drain change.
Engine package-boundary gate and diff checks passed. All five saved reference files
matched their recorded byte lengths, SHA-256 hashes and commit-pinned URLs.
