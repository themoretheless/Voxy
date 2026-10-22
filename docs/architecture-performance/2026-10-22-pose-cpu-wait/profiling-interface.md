# CPU-only pose clocks

Run the existing device-free replay with `--cpu-replay OUTPUT.csv
--cpu-replay-scenario catchup --cpu-replay-profile-pose` (arguments on one command
line). The additional `OUTPUT.pose.csv` contains one row per measured frame;
inactive no-tick frames have `active=false` and zero clock values.

- `total_wall_us`: elapsed render-pose validation, excluding snapshot publication.
- `caller_cpu_us`: scheduled CPU of the caller across the whole validation.
- `process_cpu_us`: scheduled CPU of the process, including all pose workers;
  it can exceed wall time because workers run concurrently.
- `spawn_wall_us` / `spawn_cpu_us`: caller elapsed / CPU during worker creation.
- `main_work_wall_us` / `main_work_cpu_us`: caller's actual admission section.
- `worker_work_wall_sum_us`, `worker_work_wall_max_us`, `worker_cpu_sum_us`,
  `worker_cpu_max_us`: worker loop wall / CPU statistics, excluding launch/exit.
- `launch_delay_max_us`: time from a spawn request to worker loop entry, including
  thread creation and scheduling; it is not solely ready-queue delay.
- `join_wall_us` / `join_cpu_us`: elapsed / caller CPU while joining workers.
  Their difference includes blocking, worker exit, wakeup and caller descheduling;
  it must not be described as pure synchronization implementation overhead.
- `groups`: immutable distinct influence-group count; `workers`: planned parallel
  budget including caller, or zero when the joint-family certificate avoids workers.
- `fallback`: whether the exact full-vertex scan was needed.

CPU clocks use POSIX thread/process `clock_gettime`; failures invalidate the
profile rather than substituting elapsed time. Timer overhead is included. The
ordinary entry point instantiates `PROFILE=false`, compiling out clock reads.
Whole replay `total_cpu_us` retains its previous elapsed-time meaning. It is not
the sum of scheduled CPU; use the new pose clock sidecar for that distinction.
Overlapping worker wall times must not be added to obtain frame latency.

The paired driver accepts `--profile-pose` when both variants should collect
clocks. Qualification in this investigation instead uses instrumentation disabled
for both variants. `cpu_replay_process.py` records native wait statuses, actual
whole-process user/system CPU and context switches. A stopped sample is killed
and invalidated, never automatically resumed or silently included in a p99.
