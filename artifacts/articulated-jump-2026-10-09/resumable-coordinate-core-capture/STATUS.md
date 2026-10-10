# Resumable contact coordinate owner

The synchronous active-set API now drives an internal continuation. Each owner retains its immutable operator, active rows, original-coordinate state, reactions and QR storage while waiting for an equality proposal. Repeated polling does not mutate a waiting owner. An invalid reply poisons that owner; release-only proposals cannot publish a solution.

Validation: 339 physics library tests passed, 39 ignored. The new regression interleaves two independent owners in reverse order, exercises negative-reaction release, compares final states exactly against the synchronous path and rejects a NaN reply without rescheduling it.

This is preparation for cooperative independent-island GPU batching. Actual batched GPU submissions are not implemented by this change. Full 720-frame articulated jump and >160 FPS remain unqualified. Existing full captures were launched from older binaries and are not evidence for this source revision.

Current-source Metal replay passed on Apple M4 Max: complete 47-row / 1512-coordinate original operator, 52 equality submissions, zero native fallbacks; maximum translation difference from native 1.1167282376600696e-17 m. Changed/restored bounds and cache-disabled paths passed. Jump launch/flight/landing continuity and analytic ballistic derivatives test also passed. These isolated checks do not qualify a complete trajectory or frame rate.
