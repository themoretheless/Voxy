# Fluid format and enabled-device feature admission

`ScreenSpaceFluidRenderer::new_with_adapter` checks actual adapter format
capabilities before GPU allocation. Format usages and flags are intersected
with the device's guaranteed format features unless
`TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES` is enabled. Adapter metadata mismatch
is rejected; matching metadata alone is not proof of physical adapter identity.
The caller must pass the adapter that created the device.

Required roles:

- Output format: float color, render attachment, texture binding and blending.
  The shared `SceneRenderer` alpha-blends the background scene even though the
  fluid composition pass replaces color. This output-blending requirement was
  missing and is now checked.
- Depth32Float: render attachment and texture binding.
- Rg32Float: render attachment, texture binding and copy source.
- R16Float / Rgba16Float optical targets: the same usages plus blending.

The CPU regression checks integer and depth outputs, disabled optional format
support, missing attachment usage and missing blending. Rgba32Float without the
required enabled support now fails early rather than reaching background
pipeline validation.

The physical Metal film test requests zero storage buffers per shader stage and
zero storage-buffer binding size. It rejects unsupported Rgba32Float through the
public constructor under a GPU validation scope before successfully rendering
and reading the ordinary fluid fixture. Existing analytic coverage includes
film thickness, RGB optical-depth sums, overlap, near clipping, opaque clipping,
dry pixels and final RGB absorption. The sphere integration covers path scale,
additivity and opaque occlusion.

Runtime `SceneApp`, `liquid_snapshot` and the sphere integration use the checked
constructor. The legacy `new` API remains compatible and does not query actual
adapter format capabilities. Actual execution here covers Apple M4 Max / Metal;
GLSL ES translation evidence in the separate vertex-instance artifact is not
proof of execution on WebGL, Windows, NVIDIA or CUDA.

Commands:

```sh
cargo test -p voxy_render --lib
cargo test -p voxy_render --lib film_gpu_integrates -- --ignored --nocapture
cargo test -p voxy_render --test fluid_optics -- --nocapture
cargo run -p voxy_app --example liquid_snapshot -- artifacts/fluid-format-admission-2026-10-05/impacts-optical.png --impacts --optical
```

The impact snapshot contains initial, active and deposited-film stages. Material
coefficients and lighting are illustrative. The renderer does not reconstruct a
continuous liquid density field or internal interface refraction. Changes are
local and unpushed.

## Verification result

138 renderer library tests passed (17 hardware tests ignored by the default
run). The targeted physical film test and sphere integration each passed on
Metal. The film test's final GPU validation scope was empty after the rejected
constructor and healthy fixture. The final impact snapshot passed with film
cell counts `[0, 0, 161, 232]` and was visually inspected. Logs and source hashes
are preserved alongside this report. This is scoped format-admission evidence,
not completion of full hardware support or complete liquid physics.
