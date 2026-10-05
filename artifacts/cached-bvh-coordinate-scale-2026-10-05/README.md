# Cache conservative BVH coordinate scale

The runtime sample in ../local-contact-frame-2026-10-05/runtime-sample.txt identifies conservative BVH traversal and its repeated maximum-coordinate reduction as active costs. Nodes now cache that maximum when built/refitted, while the query maximum is computed once before recursion. Each node retains exactly the same padding formula and candidate order. The thin-film unpadded traversal still uses zero padding.

An oracle reproducing the previous uncached traversal compares candidate vectors exactly across 9,000 queries, three refitted coordinate offsets (0, +1e12, -1e12) and three margins. The existing randomized static/swept refit oracle also passes.

Manual release microbenchmark: 4096 triangles, 102400 queries, 819200 returned candidates in each run. Uncached traversal: 19.562750 and 19.110458 ms; cached traversal: 3.217000 and 3.219583 ms. This is a traversal microbenchmark, not whole-simulation speedup or realtime proof.

86 library tests pass (3 manual benchmarks ignored in the regular run). Prescribed-contact, viscoelastic and thin-film integration regressions pass; exact counts are in regressions.log. Changes remain local and uncommitted. The separately running local-contact diagnostic binary was built before this cache change.
