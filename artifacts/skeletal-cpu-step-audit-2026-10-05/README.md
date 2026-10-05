# Full skeletal CPU cycle and adaptive-step predictor

The existing body_motion_snapshot now supports `--cpu-bench [TRACE.csv]`, using
exactly the displayed skeleton and continuum tissues without GPU allocation.
Transactional ledgers record accepted/rejected trials and maximum refinement.

The initial four-subdivision controller restarts at its coarse timestep in every
pose interval. The baseline cycle shows 1,295,538 rejected trials. A per-body
predictor now starts one level coarser than the preceding interval's finest
accepted step. Each attempted step retains the unchanged energy/work/heat and
volume-path guards, including the same maximum refinement depth. The predictor
and all diagnostics roll back together with a failed frame.

`baseline.txt` and `predicted.txt` each measure a 12-second cycle. Timings are
single local runs with other processes active, not controlled editor FPS proof.
The deterministic trial counts are stronger evidence for removal of wasted work.
`cpu.sample` is the one-second baseline profile; it is not a benchmark itself.

The optional final trace is compared with the previous source-qualified GPU
cycle's physical CSV in report.json. Existing rendered GIFs retain the source
state of their own render; no new GPU render is claimed in this increment.

This does not complete the engine parity, research, hardware/CUDA, destruction,
material calibration or multiphase fluids objective.
