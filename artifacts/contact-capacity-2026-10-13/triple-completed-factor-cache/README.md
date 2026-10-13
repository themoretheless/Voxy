# Completed QR basis cache integrated with compact readback

VOXY_HAIR_TRIPLE_COMPLETED_FACTORS=1 extends the test-only experimental triple cache to successful multi-iteration final bases. Original source coefficient identity, native physical admission, exact cache/dedup and bounded single-owner workspace/factor caches remain mandatory. Explicit row-rank hints restore the original QR order; their magnitudes never enter loads.

Full readback captures the final selected basis directly. Compact readback includes the small final-order metadata region in its initial immutable/candidate audit. A typed selected-only gather plan is constructed only from a fully validated completed candidate. The producing job remains exclusively owned and unchanged until every plan reads back selected Q/R, final row order and iteration counter. Metadata/count/duplicate IDs/positive reaction inclusion are checked; the second readback checks exact expected order/counter, length and finite QR. Warm candidate and original physical gates still determine publication; the factor snapshot itself is not a physical certificate.

All coordinate candidates decode before any additional QR gather. A single extra copy-only queue submission gathers all missing selected bases in that batch. Completed one-iteration seeded factors need no extra gather. Factor/result cache entries and pooled buffers commit only after all required reads finish. A late error publishes no new prefix; native rejection clears the caches. Profile records report selected factor bytes, extra readback submissions and wall overhead separately from equality compute counts. With host profiling disabled, functional factor gathers do not introduce timing clock reads.

Verification on the recorded current executable:
- 18 regular renderer triple tests and 5 regular app solver tests pass.
- Typed gather checks cover changed metadata/iteration, NaN QR, truncation/extension and equality with full snapshots.
- 13 ignored actual GPU checks pass: mixed heterogeneous offsets and cold basis capture/reordered reuse in full and compact modes; separate per-operator dispatches; feature disabled; GPU timestamps; host profiling disabled; independent physical scene; 91 x 882, 100 x 882 and 55 x 2142 original native physical operators in both readback modes.
- Physical replay counters in compact mode report 8, 6 and 3 basis reuse owner attempts respectively, including changed bounds and appended inactive original loads. They are attempts within these captured replays, not unique full-scene objects or a speedup ratio.

An extra attempted assertion required reuse in the four-guide scene, but this fixture supplies no positive incoming active-row hints. Its original exact-pose and zero-native-fallback checks had passed. The unsupported reuse assertion was removed; the failed log is retained. Actual reuse is instead explicitly required by the mixed reordered-basis case and the original physical operator replay gates. No original physical gate was loosened.

The new feature remains experimental/test-only. Full 469-guide jump completion and >160 FPS remain unproven. PID 55834 uses a frozen older binary and cannot qualify this cache integration. No additional full-cycle restart was made solely because it remains slow.
