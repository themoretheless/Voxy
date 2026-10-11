# Workspace capacity and explicit memory retirement

Completed workspaces now select the smallest sufficient backing allocation. Each task records its current packed payload size; complete snapshots copy that payload, not the capacity of a reused larger allocation. Every payload byte is uploaded before reuse, including headers and work areas; extra capacity is outside the shader ABI. Original physical admission and shader arithmetic are unchanged.

A MemoryBudget error preserves cached jobs and their charges. The former eviction/retry moved storage into configured retirement without freeing its charge, destroying a useful reuse opportunity. Retirement remains explicit under the device owner; the solver does not destroy unrelated resources or insert global cleanup waits.

Hardware tests passed: strict 220-byte managed storage budget with one allocation / five reinitializations; oversized request rejected without pool/accounting changes; smaller retry succeeds; charge retained until explicit retirement confirmation; 24 changing operators and equal-size different layouts with exact storage equality; 65-owner staging limits; four-guide/four-step contact serial equality; captured original 21-system / 61-row / 2646-coordinate physical projection at 1e-14 tolerance, cooperative serial equivalence and zero native fallback; full and validated-tail readback variants produce identical responses/reactions. Production remains full readback because concurrent timing did not establish a tail advantage.

The storage budget is component accounting, not total VRAM. No full trajectory, rendered FPS, CUDA or multi-device hardware claim. The bounded-readback full run retains its earlier pre-pooling executable and is a separate experiment.
