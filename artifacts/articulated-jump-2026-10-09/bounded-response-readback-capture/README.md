# Shared staging for contact response waves

Response waves now use actual staging output sizes and current shared-pool byte/count capacity, instead of assuming default device budgets. All loads remain ordered and present; factors are reused across wave chunks. The existing eight-wave upper bound remains. Correction auditing checks simultaneous full/compact snapshot admission before encoding, avoiding quarantine caused by predictable shortages.

Hardware checks passed for eleven distinct loads, full and compact outputs, separately byte-limited and count-limited pools admitting two snapshots, ordered copies and gather-shader transport, fresh and reused factors, exact comparison with one-wave serial calls, original physical residual admission, zero quarantine, and pool reuse after rejected audit staging. Mixed-shape/load-order and one-storage-buffer transport regressions pass. The 65-owner equality staging stress also passes on this final binary.

Scheduling capacity is a snapshot, not an atomic reservation against concurrent device consumers; allocation failures remain errors. Numerical equations and physical tolerances are unchanged. This is not a full-model dynamics or FPS qualification. A separate 720-frame full-density experiment is running under ../bounded-readback-full-capture with immutable launch hashes and runtime handles. Older running binaries remain preserved.
