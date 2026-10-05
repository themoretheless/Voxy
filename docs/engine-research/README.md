# Engine architecture research

User scope: find 500 engine repositories, study the strongest mechanisms, and
cover UE (Unreal Engine)/Stride3D/Unity/Godot features with an architecture suited to Voxy.

`collect.py` discovers public GitHub repositories and saves query URLs, fetch
timestamps and repository IDs. `candidates.json` is discovery evidence, not a
claim that every entry is an engine or that its source was reviewed. Topic tags
include games, templates and tools; these must be excluded before the final 500
engine corpus is accepted. Forks are excluded by query and response checks;
independent engines sharing ancestry still require manual classification.

## Acceptance process

1. Discover more than 500 candidates; retain provenance and excluded candidates.
2. Confirm engine ownership from README, build manifests and source structure.
   Separate complete engines, engine frameworks and subsystem libraries.
3. Pin a commit for each source review. Record concrete paths, ownership,
   lifetime, synchronization and failure behavior; stars are discovery ordering,
   never architecture evidence.
4. Compare mechanisms against Voxy contracts and measured workloads.
5. Record adopt/adapt/reject decisions with tests and limitations. Do not import
   code without checking its actual license and compatibility.

No single architecture can be ideal for every workload. Voxy decisions should
state their workload, complexity cost and measurable acceptance criteria.

## Next source-review questions

- UE: object/resource ownership, runtime/editor boundaries, animation/physics
  update order and import/build dependencies; compare contracts before adoption.
- Godot: scene/server/platform boundaries, editor resources and import pipeline.
- Bevy: declared system access, scheduling and change detection costs.
- WickedEngine / Filament: render resource lifetime and pass dependencies.
- Fyrox / Stride: editor commands, serialization and prefab overrides.
- O3DE / Open 3D Engine: modules, asset processing and runtime ownership.
- Flecs / EnTT: entity lifetime and storage tradeoffs (subsystem libraries).

These are questions to inspect, not verified recommendations or completed ports.

## Reproducible commands

```sh
python3 tools/engine-research/collect.py --target 1400
python3 tools/engine-research/collect_evidence.py --limit 700 --workers 6
python3 tools/engine-research/report.py
```

Evidence collection resumes from completed records. It fetches public metadata
and README text at pinned commits and never builds or executes fetched projects.
The public API is attempted first; configured `gh api` is the fallback when
anonymous API access is limited. No credentials are saved in evidence.
`report.py` verifies unique repository identities, commit syntax and README
SHA-256 digests, then emits a queue and counts by category. Verified content
identity does not prove engine identity or architectural quality.

[Initial mechanism decisions](decisions.md) contain two source-file reviews with
pinned links and Voxy-specific adoption criteria. [Review queue](review-queue.md)
tracks collected, classified and pending entries. Complete engine counts exclude
render-only engines, frameworks, subsystem libraries and resource lists.

Explicit classifications are applied with `classify.py <decision-file.json>`.
The input maps repository names to `[category, evidence-based reason]`; every
entry needs a previously pinned record. Automatic keyword acceptance is not used.
Game-engine repository counts include derived runtime implementations; the
separate non-derived count is also reported. These are repository counts, not
proof of distinct architecture families or completed source-mechanism reviews.

A description may disagree with the pinned README or current source state.
Claw3D, for example, currently describes itself in its README as an AI-office
application; SiliconStudio/xenko currently contains only a successor redirect.
Neither is accepted as an implemented engine at the pinned revision merely
because the discovery description says engine.

## Reproducible corpus integrity audit

Run `python3 tools/engine-research/audit.py` for a read-only check of candidate
identity uniqueness, evidence directory identity, pinned source URLs, README
SHA-256 digests, classification mirror agreement and current status counts.
Validation also runs under `python3 -O`; it does not rely on removable assertions.
The optional `--root` argument supports isolated corruption checks without editing
the saved corpus. The audit does not execute repository code or refresh remote
state, and it does not decide engine classification from source semantics.

Current audited snapshot: 1,428 candidates and pinned README/root records; 500
classified engine identities, including 52 derived engines (448 non-derived).
All 1,428 corpus records currently have `root_and_readme_only` review depth.
The separate mechanism decisions document contains partial pinned source reviews;
these do not prove 500 architectural reviews, runtime verification, or full
Stride3D/Unity/Godot feature coverage. Completion of the broad goal remains open.

Validation: normal and optimized Python runs produced identical audit results.
An isolated fixture rejected README corruption, a missing classification mirror
entry and an unpinned source URL. Original evidence was not modified.

Audit refresh (2026-10-05): normalized the Bevy command mechanism manifest to the shared repository/sources/file schema without changing pinned commit, source URLs or digests. Normal and optimized audits agree: 1,428 pinned records, 500 accepted engine identities (52 derived), and 24 verified source files across five mechanism manifests. Evidence: artifacts/engine-corpus-audit-2026-10-05/audit.json. Review depth and broad feature coverage remain incomplete.
