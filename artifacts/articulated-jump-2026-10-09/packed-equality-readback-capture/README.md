# Packed equality readback qualification

65 ready equality tasks use one submission and one staging lease when their total snapshot bytes fit. Byte-limited chunks still split; owner results retain serial order and exact scalar agreement. Mixed-source gather validates all ranges before leasing staging storage.

Saved checks cover real GPU gather, 65-owner single-buffer and byte-budget limits, heterogeneous seeded/dependent retry, original physical captured operators, strict managed storage and production compilation. They do not qualify the complete 720-frame jump, rendering throughput or 160 FPS. Host stage timings include queue waits and callbacks and are not GPU timestamps. The earlier full bounded-readback trajectory failed at frame 6; transport changes do not establish a numerical correction.
