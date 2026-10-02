# Reference rig GPU animation and prepared preflight

The unchanged Khronos glTF Sample Assets RiggedFigure (2017 Cesium, CC-BY-4.0)
is retained as a regression fixture with pinned revision, hash and attribution.
It has 19 skin bones; import maps skin indices into a 22-node palette. The test
uses 370 vertices and samples the full clip at 65 evenly spaced times. On the
host Apple M4 Max/Metal, all GPU positions match the CPU reference within
2.3841858e-7 model units; observed motion exceeds 0.335 model units.
This is sampled position evidence, not all-time quality or normal verification.

`SceneSkinPose` binds preflight to immutable instance/palette borrows. Private
fields prevent construction without validation. `prepare_pose` performs the
existing finite/domain/skin validation; `encode_prepared_pose` rejects foreign
skinners and writes the validated palette without repeating vertex arithmetic.
The ordinary `encode_pose` remains a validating convenience entrypoint.

The editor stages all resources and prepares every skin primitive before the
first palette write. Existing GPU streams therefore execute one preflight per
pose rather than preflight followed by the same validation during encoding.
Failed preparation retains existing streams and animation time. The borrow
contract's compile-fail test rejects palette mutation while a prepared pose is
in use. This is CPU preflight publication discipline, not a transaction over
arbitrary external GPU commands.

Debug host median/p95 diagnostics, in microseconds: CPU pose/skin 392.375/404.875;
preflight 328.083/341.042; prepared encoding/copy 81.75/159.375;
submission/wait/readback 406.709/1185.5. They include host validation and queue
waiting, are not GPU timestamps, and do not establish production FPS or a
measured end-to-end speedup. Release profiling on larger rigs remains pending.
The actual reduced reference-rig CPU certificate profile is now documented in
`reference-rig-reduced-lod.md`; it does not establish whole-frame performance.

Native inspector/Play/Stop also runs this actual reference GLB: two owners share
one source, moving/paused palettes differ, 77,136 logical animation bytes retire
to zero after Stop, and the authoring document is restored. Logs, source identity
and exact acceptance limits are in
`artifacts/reference-rig-preflight-2026-10-03/`.
