# Elide redundant leaf transactions

The adaptive skeletal tissue wrapper previously cloned the complete continuum
owner before every trial, although `step_viscoelastic` already guarantees an
atomic successful commit or complete rollback. The wrapper now calls that atomic
solver directly. It clones only after a rejection, to keep both recursive halves
transactional together. A committed first half cannot escape a failed second half.

The new regression independently proves that the first half can succeed and
release heat, that the second half fails, and that the combined failed call
preserves the original full owner, including velocity and Maxwell history.
Existing complete-frame, last-bone and recovery tests remain applicable.

The existing `body_motion_snapshot --cpu-bench TRACE.csv` records the full
12-second cycle. It is compared byte-for-byte with the preceding predictor trace;
step counts and final work/heat receipts are compared independently. Results and
source hashes are recorded in report.json after the processes finish. Timings
are local observations with other processes active, not editor FPS proof.

The broad engine/hardware/research/material/fluid goal remains active.
