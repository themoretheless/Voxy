# Voxy OpenXR

Runtime discovery, graphics requirements, session/frame lifecycle, controller
bindings, swapchain ownership, stereo projection and compositor quad layers.
Graphics session creation must use the runtime-selected native device; this
crate does not substitute a desktop device or render a headset scene itself.

## Run checks

```sh
cargo test --locked --offline -p voxy_xr --lib
cargo run --locked --offline -p voxy_xr --example panel_input
cargo run --locked --offline -p voxy_xr --example xr_probe -- vulkan
```

`panel_input` runs the actual aim picking and common UI routing code with
controlled runtime-boundary inputs. It checks topmost widget capture/click,
release outside the panel, missing tracking, inactive actions and recovery
while select remains held. Its PASS marker does not prove headset rendering.
`xr_probe` needs an installed loader/runtime and checks discovery/requirements;
it does not create a graphics session or present frames. Choose `dx12`, `gl`
or `gles` explicitly when appropriate for the platform/runtime.

## Frame integration

1. Process runtime/session events; respect running and `should_render` state.
2. Wait/begin the frame and locate stereo views at predicted display time.
3. Sync actions, read the chosen hand, and locate its aim space against the
   panel's reference space at that same predicted time. Ignore unavailable
   actions and invalid locations.
4. Call `hit_test_quad` with the panel pose, dimensions in metres and range.
   Pass that hit, hand and same aim location to `XrPanelPointer::update_hand`.
   Supply UI dimensions in logical pixels matching `PointerRouter` regions.
   Dispatch returned `PointerAction` through the application's existing UI.
5. Acquire/wait runtime swapchain images, render stereo and the UI panel on
   the compatible native graphics device, finish GPU work, then release images.
6. Build `StereoSubmission` and `QuadSubmission` from released subimages.
   `end_stereo_frame_with_quad_and_history` submits both layers and commits
   combined scene history only on successful submission. When rendering or
   tracking is unavailable, submit no layers through the existing frame API.

Panel and stereo spaces can differ but must belong to the runtime instance.
Aim picking must use the panel space. Panel coordinates use local +X right,
+Y up and front +Z; aim points along local -Z. UI UV has a top-left origin.
Explicitly cancel the pointer adapter when switching controller, panel,
reference space or session. Tracking loss cancels capture and rearming requires
button release, avoiding accidental clicks from a trigger held across recovery.
An active ray leaving a panel retains normal drag-out release semantics.

Physical runtime/device/graphics interop, compositor appearance and controller
interaction require separate headset acceptance. CPU fixtures and cross-checks
are not a substitute for those results.
