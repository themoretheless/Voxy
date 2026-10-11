# Full-density contact owner isolation candidate

Frozen binary: /tmp/voxy-contact-owner-gpu-tests. Its SHA-256 and build-source hashes are in ../contact-owner-isolation/provenance.json. Run script preserves 469 guides, 20 segments, 720 requested frames, original 1e-6 m and 5e-5 quaternion gates, and coordinate batching. Execution session 50174 was re-polled live after frame 2. No restart on an observation timeout.

Observed frame 1: position difference 7.736196887091207e-10 m, quaternion difference 3.4670877474551887e-8. Observed frame 2: position difference 1.6468706840952474e-9 m, same cumulative quaternion maximum. Both have original GPU admission and zero native fallback. The preceding candidate failed frame 2 at 3.955094080576593e-6 m and 0.0002665447427230294 quaternion difference.

The run remains in progress. live-log-summary.json is a point-in-time snapshot, not terminal evidence. Full 720-frame agreement, rendered secondary-physics playback, >160 FPS, and hardware-wide qualification remain unproven. Solver wall-clock timings are not rendered FPS.

## Per-frame workload observation

The full run was re-polled live after the two observed frames; it has not terminated. `live-frame-work.json` is produced by `tools/summarize_hair_frame_work.py` from completed timing and counter blocks. Original counters are cumulative and must be differenced: frame 2 adds 14900 coordinate calls, 277444 equality task dispatches and 103720 submissions (not the cumulative 20993 calls). Equality dispatch counts do not include every QR kernel dispatch. Frame 2 external solver wall time is 815562.422083 ms; it excludes rendering and post-solve comparison and is a loaded diagnostic qualification run, not an isolated rendering benchmark. It does not establish FPS.

The report rejects resets, inconsistent dispatch/submission counts, missing admission outcomes, noncontiguous/duplicate frames, invalid or nonfinite times, and CPU-control records. Eleven malformed synthetic cases were rejected; known per-frame deltas and one trailing live block were verified. Current workload identifies a major remaining scheduling/iteration problem despite improved trajectory agreement. No shortcut, reduced guide count, relaxed tolerance, new solver defaults or restart was introduced for this observation.
