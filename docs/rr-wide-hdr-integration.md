# Wide HDR scene inputs for DX12 Ray Reconstruction

Enable `voxy_streamline/scene-dx12` on Windows. Initialize the native SDK, registered DX12 device, serialized queue and packed HDR RR viewport before recording candidates. Reuse one `voxy_render::HdrHalfResolvePipeline` created on that registered device.

For each temporal candidate, produce ray-traced linear radiance, primary depth, two-channel motion and matching material guides for the same camera, jitter and pixel grid. The radiance may use RGBA32Float; the SDK color input uses RGBA16Float. Conversion clips RGB to `[0, 65504]`, preserves alpha at half precision and reads mip zero. It applies neither exposure nor a display transfer function. Source contents must be finite and alpha must be in `[0, 1]`.

The integration inside a caller that already owns the temporal frame, matching textures and encoder is:

```rust
// SAFETY: All resources and the resolve pipeline share the registered DX12
// device; guide contents match this exact temporal candidate. Queue access and
// native resource states are serialized through GPU/SDK completion.
let candidate = unsafe {
    voxy_streamline::scene_dx12::SceneRayReconstruction::import_wide_radiance(
        &temporal_frame,
        composed_radiance,
        &resolve_pipeline,
        &material_guides,
        rr_output,
    )
}?;

// Record primary/guide/radiance producer commands into this encoder first.
// This call then records linear conversion followed by resource transitions.
candidate.prepare(&mut encoder)?;
registered_wgpu_queue.submit([encoder.finish()]);
```

Import rejects mismatched depth/motion dimensions and non-RG16Float motion before allocating the intermediate texture. Existing RR import additionally validates guides, formats, usages and native aliases. The candidate retains the conversion job; native texture leases retain the converted color through submission completion.

After preparation submission, call `submit_with_reconciliation` with the same viewport/token and matching camera constants. If the candidate requests history reset, `constants.reset` must be nonzero. The reconciliation callback must restore prepared states and order UAV writes using the actual SDK exit states before later wgpu use. Retain the resulting native submission until its fence completes and preserve SDK lifetime throughout. Commit temporal history only after the actual presentation succeeds; retries/skipped presentation must follow the renderer's candidate lifecycle.

This is an integration recipe, not a standalone runtime initialization example. Windows cross-compilation validates the API. HDR conversion readback is verified on Metal and software Vulkan/OpenGL; physical NVIDIA RR execution with this complete path still needs validation.
