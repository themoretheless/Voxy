# App single-dispatch triple qualification

The existing test-only triple solver supports VOXY_HAIR_TRIPLE_SINGLE_DISPATCH=1. The missing unique operators share one checked payload and one dispatch, with one workgroup per independent operator. Inputs move into the batch without cloning. Canonical complete-payload deduplication and exact result cache lookup happen before packing; original input ownership and native physical admission remain unchanged.

The batch codec supplies immutable offset-table plus per-operator compact gather ranges. Exact total readback length and offset-table equality are checked before splitting readback. Each original decoder then checks every original immutable coefficient and result. The app delays result/factor cache publication and buffer retention until every missing candidate decodes. Seeded failure retries the complete batch cold. Workspace reuse, compact factor keys, aliases and physical-rejection cache clearing use the existing owners. Preflight charges the offset table and rejects more than 65535 workgroups before operator packing. Host dispatch counters now count one dispatch, while submitted owner counts still count independent unique operators.

Verification on the current source:
- 16 regular renderer triple tests pass, including compact/full batch readback equality, offset table mutation and truncated/extended buffers.
- 5 regular app solver tests pass.
- Actual GPU transport gate passes with full and compact readback, warm hints, factor reuse, exact cache, dedup and pooled buffers. It covers a late contradictory operator, no prefix cache publication, cold retry, and smaller payload in a larger reused buffer.
- Native original physical admission passes all three captured families (91 rows x 882 coordinates, 100 x 882, 55 x 2142) in both readback modes, including changed bounds and appended inactive original loads. These are individual captured-operator replays, not a simultaneous complete scene or trajectory.
- Existing non-batched app GPU gate also passes with the same reuse flags.

Scope: test-only opt-in integration, not production default. No isolated timing comparison or rendered FPS claim. The older frozen long full-cycle processes do not qualify this source. Full jump completion, realistic visual hair and >160 FPS remain open.
