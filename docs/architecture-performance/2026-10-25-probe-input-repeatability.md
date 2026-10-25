# Original street input repeatability gate

BVH8 remains opt-in. Minimum 120 FPS and the full motion cycle are not qualified.
Current complete geometry has 117857 instances and 3480232 instanced triangles.

## Observed original-control failure

`artifacts/original-street-realism-2026-10-25/wide-full-street-v1/qualification/`
contains two successful original-path processes. All native camera, lighting and
material uniform words matched, but four SH words of probe 82 differed before any
source query. This is not evidence of BVH8 precision loss: no wide candidate ran.
It also cannot be ignored when qualifying a complete rendered frame.

`artifacts/original-street-realism-2026-10-25/projection-operands-v1/` retains
lightweight, in-projector observations from two further complete original scene
runs. All 2232 face reductions and subsequent SH accumulations in each run agree
bit for bit with independent IEEE-f32 reconstruction. For probe 82, all 109350
sample records reconstruct all 9216 thread-partial RGB words exactly. Between
runs, thread partials changed at probe/face 197/2, 264/3 and 297/3. Uniform records
matched. Main image words also changed at frames 3 and 4. The cause is not fixed.

The observer does not add a GPU dispatch or readback between faces. It preserves
the canonical production shader file and inserts observation statements into an
explicit diagnostic variant. A single readback occurs after baking completes.
Replay of six complete historical F16 source faces, repeated 20 times, produced
all 120 original coefficient results bit for bit. Expected coefficients remain
CPU-only. These are correctness checks, not frame-rate measurements.

## Diagnostic interface

Set both `VOXY_STREET_PROBE_PROJECTOR_TRACE_INDEX` (an in-grid probe index) and
`VOXY_STREET_PROBE_PROJECTOR_TRACE_OUTPUT` (a fresh directory) before beginning
baking. The optional storage allocation is charged to the existing device memory
budget and rejected if it exceeds device binding limits. It never truncates a
bake. With these variables absent, the canonical shader and three bindings apply;
no trace storage is allocated.

Schema 2 additionally records ordered integer hashes of each thread's actual
radiance RGB operands and weight/normal operands for every probe. Different hashes
prove different observed operand sequences; equal hashes are not a bitwise input
identity proof. The selected probe still retains all actual sample words. The
independent auditor verifies its hashes against those complete native records.

```sh
python3 tools/scene-research/audit_probe_projection_operands.py CAPTURE_DIR \
  --compare OTHER_CAPTURE_DIR --output proof.json
```

The auditor reconstructs thread sums from recorded operands for the selected
probe, and reductions/SH accumulation for every probe. It applies no epsilon and
explicitly rejects unsupported non-normal sample domains rather than silently
assuming a floating-point flush policy. Capture directories and frozen executable
identities are retained under `projection-operands-v2/`; consult its `status.json`
and terminal `process.json` records for current progress.

## Remaining gates

1. Identify and fix the original scene's varying inputs; diagnostics alone are
   not a fix, and additional stable runs would not prove one.
2. Compare current original and admitted BVH8 full-frame native outputs with
   identical verified inputs, including all probe coefficients and source data.
3. Measure isolated, alternating original/candidate runs without observations or
   source audits; retain rejected controls and separate GPU stages from whole
   frame completion.
4. Qualify the complete motion cycle and the actual 120-Hz workload. The existing
   offscreen fixed-30-step stress batch is not one ordinary 120-Hz frame. No
   presented-frame-rate claim may be derived from that diagnostic workload.
