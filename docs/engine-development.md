# General 2D/3D engine development

The requested engine extends the existing voxel runtime rather than replacing it.
Required scope: a full 2D/3D engine with DirectX, Metal, RTX, CUDA, OpenGL,
WebGL and shaders, plus VR, iOS, Android, Linux, Windows and macOS (user addition
on 2026-09-30). These are completion requirements, not optional future work.
`meta` remains ambiguous: retain Apple Metal support and include Meta Quest in
the VR target matrix; do not silently treat one as covering the other.

## Current increment

`voxy_render::GraphicsOptions` selects a strict graphics backend. Existing callers
keep using `Renderer::new`; new callers can use `Renderer::new_with_options`.
The default permits native primary APIs and GL, but excludes the noop test device.
`GraphicsCapabilities::discover` reports adapter support; ray-query support does
not imply that the device enabled the feature or that a ray tracing pass exists.

Run `cargo run -p voxy_render --example gpu_probe -- metal` to create an actual
GPU device and submit a command buffer. Other choices: auto, dx12, vulkan, gl,
webgl, webgpu. WebGL and OpenGL share wgpu's GL backend; WebGL requires a wasm
browser target. The native probe is not a browser application.

## General scene renderer

`SceneRenderer` renders validated indexed `SceneMesh` resources with RGBA8 sRGB
textures and linear vertex colors. `SceneTransform` owns one MVP uniform per draw
and supports updates without re-uploading geometry. `SceneCamera` supports finite
perspective and orthographic frusta with a right-handed view and 0..1 depth.
`SceneDraw::overlay` draws in painter order after the world and ignores depth.
World draws currently write depth and use straight alpha blending; translucent
3D sorting, lighting, batching and asset file loaders remain pending.

Run `cargo run -p voxy_render --example scene_smoke` for a real GPU offscreen
render and pixel readback. It verifies perspective geometry, rejection of a later
farther mesh, texture tint in linear space, orthographic overlay and alpha blend.
The output is `/tmp/voxy-scene-smoke.ppm`. Metal on Apple M4 Max passed this test.
Workspace tests, strict Clippy and formatting checks also pass. Native general scene presentation is now verified on Metal; browser WebGPU and WebGL2 rendering is also now verified in the local preview.

`cargo run -p voxy_app --example scene_demo` opens an animated textured cube with
a textured 2D overlay. Space pauses animation and Escape closes the window.
`--backend auto|metal|dx12|vulkan|gl` selects the API strictly; `--fallback` requests
a fallback adapter explicitly. Unknown flags/backends are errors. `SceneApp` exposes
`with_graphics(GraphicsOptions)` so platform entrypoints can select their backend
without duplicating initialization. Options persist across surface recreation.
For example: `cargo run -p voxy_app --example scene_demo -- --smoke --backend metal`.
A missing API/adapter fails initialization rather than switching to another API.
Verified locally: explicit Metal completed the 120-frame smoke; DX12 on macOS
returned a surface initialization error without fallback; help and invalid backend
arguments behaved as specified. Two earlier Metal runs timed out, so native
presentation stability is not fully established. Timeout diagnostics now include
frame count, resize stages and surface outcome; the subsequent run passed.
`--smoke` verifies 120 presented frames, two actual resize events and zero-size
surface suspension/resumption. The demo uses `SceneGraph` parent/object transforms,
separate perspective/orthographic cameras and `SceneSurface`, which requests
WebGL2-compatible limits without creating voxel storage-buffer pipelines. It passes
the event loop display handle to wgpu for native GLES presentation. The shared
`voxy_app::SceneApp` now also serves the mobile entrypoints; VR sessions remain pending.

## Native mobile shell

`voxy_mobile` exposes Android's Rust ABI `android_main` for NativeActivity and
`voxy_ios_main` through a C ABI wrapper in `mobile/ios/main.c`. Both run the shared
2D/3D `SceneApp`. `mobile/android/AndroidManifest.xml` declares the native library,
launcher and GLES3 baseline. On suspension GPU resources are released and recreated
on resume. Touch tap pauses, drag rotates; cancellation, focus loss and secondary
fingers are handled by the tested shared input state.

Verified: strict Clippy checks for `aarch64-apple-ios` and
`aarch64-linux-android`, host workspace tests/Clippy, and the shared Metal window
smoke with 120 frames, resize and suspension/resumption. Cross-target checking
is not linking or a device run. This environment has neither the iPhoneOS SDK
nor Android NDK. Xcode application configuration/signing, Android packaging execution,
orientation/safe-area behavior and actual device lifecycle tests remain required.

To check the Rust boundary:
```sh
rustup target add aarch64-apple-ios aarch64-linux-android
cargo clippy -p voxy_mobile --target aarch64-apple-ios -- -D warnings
cargo clippy -p voxy_mobile --target aarch64-linux-android -- -D warnings
```
With native SDKs, build `voxy_mobile` for the target using its platform linker;
link the iOS static library with the supplied C main, or package the Android
`libvoxy_mobile.so` under `lib/arm64-v8a` with the supplied manifest. The Android pipeline is now supplied below; its complete execution remains
unverified without SDK/NDK. An iOS Xcode application project is now supplied, with SDK linking/device proof pending.

### iOS Xcode application

Open `mobile/ios/Voxy.xcodeproj`, choose the shared Voxy scheme, select a simulator
or device and configure your signing team for device builds. `Info.plist` declares
portrait/landscape orientations and an empty launch screen. Xcode compiles the C
main and invokes `build-rust.sh` before linking the shared scene static library
with the native frameworks. The script uses device `aarch64-apple-ios`, arm64
simulator `aarch64-apple-ios-sim` or Intel simulator `x86_64-apple-ios`; install the
needed Rust target first. Only the active architecture is built. Build products
and Cargo caches remain in ignored `target` or Xcode derived data.

Example with an installed Xcode/iOS SDK:
```sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
xcodebuild -project mobile/ios/Voxy.xcodeproj -scheme Voxy \
  -sdk iphonesimulator -configuration Debug \
  -derivedDataPath target/ios-xcode CODE_SIGNING_ALLOWED=NO build
```

`VOXY_CARGO_BIN` can point to the directory containing Cargo. Script sandboxing
is disabled for this target because Cargo accesses the workspace and its global
registry/toolchain caches. Signing credentials and team IDs are not committed.
Local evidence: plist/project syntax, object references, shared scheme references
and shell syntax passed. `xcodebuild` cannot run with the installed Command Line
Tools; the build phase also correctly rejects the absent iphoneos SDK. No iOS
app has been linked, signed or run here. Orientation/safe-area/lifecycle proof is
still pending. Target mapping follows [Rust's iOS target documentation](https://doc.rust-lang.org/stable/rustc/platform-support/apple-ios.html).

### Android APK pipeline

`python3 mobile/android/build.py --check` validates prerequisites without building.
Set `ANDROID_HOME` to an installed SDK, `ANDROID_NDK_HOME` to an installed NDK,
and install platform android-35 plus build-tools 35.0.0. Override the latter with
`VOXY_ANDROID_BUILD_TOOLS` when necessary. Run `python3 mobile/android/build.py`
to compile the arm64 library and produce `target/android/voxy-debug.apk`.
`--install` explicitly installs and launches on the adb-selected device.

The pipeline uses a locked Cargo build and the NDK API-26 linker, verifies ELF64
AArch64 load segments for 16 KiB alignment, stores the library uncompressed,
aligns the APK, signs with a local development key under ignored `target/android`,
and verifies signature and ZIP alignment. It does not create a production key or
publish anything. Five artifact validation tests reject wrong architecture,
4 KiB alignment, incongruent segments and truncated headers. Run them with
`PYTHONDONTWRITEBYTECODE=1 python3 mobile/android/test_build.py`.

Locally verified: help, missing-SDK prerequisite rejection and artifact tests.
No APK was produced or installed in this environment. The implementation follows
[AAPT2 packaging](https://developer.android.com/tools/aapt2),
[NDK linker setup](https://developer.android.com/ndk/guides/other_build_systems), and
[ZIP alignment](https://developer.android.com/tools/zipalign).

## Texture sampling

`TextureSampling` configures independent U/V addressing (clamp, repeat, mirror)
and minification/magnification filtering (nearest or linear).
`upload_texture_with_sampling` applies these settings; `upload_texture` preserves
the existing nearest/clamp defaults. Textures still have one mip level; mipmap
creation and anisotropy remain pending. UVs outside 0..1 are permitted for repeat.
No optional border-color/device feature is requested by this API.

All six wrap/filter combinations pass resource validation tests. The Metal smoke
uses UV 1.25 and a two-texel red-compatible/black texture on the near mesh: repeat
samples the first texel, while clamp would sample black and fail the existing
red pixel checks. This verifies repeat behavior on Metal; mirror/filter pixel
comparisons and other GPU backends remain pending.

## Dynamic geometry updates

`SceneGeometry::update` replaces vertices and indices in existing COPY_DST buffers
and adjusts the draw index count. Geometry capacity is fixed by its initial upload or explicit reservation;
updates may shrink and later regrow within that capacity. Validation and capacity
checks complete before either queue write, so invalid/oversized updates preserve
the previous draw state. `SceneMesh` exposes read-only vertices/indices for asset
processing. `SceneRenderer::reserve_geometry` creates an empty allocation with explicit
vertex/index capacity. `capacity` exposes its bounds and `clear` sets the draw
count to zero without freeing buffers. All allocation byte sizes are checked
against enabled device limits and u32 draw/index capacity, including ordinary
mesh uploads. Zero capacities and arithmetic overflow are rejected before GPU
allocation. No automatic buffer growth or throughput claim is implied.

Submit previously encoded draws before updating a shared geometry: queue writes
are applied before the next submission and do not provide per-draw snapshots.
Use distinct geometry allocations when multiple versions must coexist. Tests
verify smaller index counts and unchanged state after index-capacity/NaN rejection.
The Metal smoke reserves space for two sprites, fills one sprite, clears and
then fills two; pixel checks verify both final sprite regions. The same smoke
uploads a white OBJ mesh, updates it to red in the existing
allocation, rejects an oversized replacement and verifies the red pixels afterward.

## PNG/JPEG texture assets

`ImageAsset::decode` accepts PNG/JPEG byte slices and yields tightly packed RGBA8
with private dimensions and pixel storage exposed through read-only accessors.
Source bytes, image dimensions and combined decoded/conversion pixel buffers have
explicit limits. Decoder scratch allocation limits are best-effort, as documented
by [image::Limits](https://docs.rs/image/0.25.10/image/struct.Limits.html); the API
does not promise a hard process-memory ceiling. No filesystem or network reads
occur implicitly. Only PNG/JPEG codecs are enabled in the dependency.

PNG alpha is retained; JPEG receives opaque alpha. Current uploads treat pixels
as sRGB, preserve row order, and do not apply EXIF orientation or ICC transforms.
APNG animation, mip generation, texture compression and color-profile handling
remain pending. Tests exercise exact PNG pixels, JPEG conversion, corrupt files,
and all three budgets. The real GPU scene smoke decodes the file
`examples/assets/tint.png` before upload and verifies tint/blending pixels.

## OBJ asset import

`ObjAsset::parse` loads triangulated Wavefront OBJ text into `SceneMesh`, retaining
UVs and per-vertex optional normals. Position/UV/normal corner tuples share indices
only when all three agree, preserving UV/normal seams. Both positive and relative
negative indices are resolved against the attributes available at that face.
UV coordinates are retained without an implicit V flip. Vertex tint defaults to
linear white. The current scene shader does not use the retained normals yet.

`ObjLimits` bounds source bytes, total attributes, output vertices and triangles;
errors include line numbers. Non-finite numbers, invalid indices, unsupported
attribute arity and non-triangle faces are rejected. Triangulate assets before
import. Object/group/smoothing/material labels are accepted but do not create
material partitions or implicitly read MTL files. General polygon triangulation,
MTL/image loading, glTF and normal-based lighting remain required asset work.

Parser tests cover negative indices, normals, UV seams, shared corners and budgets.
The real scene pixel smoke uploads the imported `examples/assets/quad.obj` as its
near world mesh; depth, texture tint, overlay and shader-reload pixel checks also
exercise the imported geometry. This does not prove arbitrary production assets.

## Browser client

`voxy_web` supplies a WASM platform shell around the shared `SceneRenderer`.
It creates the canvas surface/device, requests WebGL-compatible resource limits,
uses sRGB attachment views, handles resize and temporary surface events, and
presents a rotating 3D tetrahedron plus a 2D overlay. `web/index.html` owns the
requestAnimationFrame loop and keyboard pause, with explicit auto/webgpu/webgl
selection. Auto uses wgpu's asynchronous WebGPU detection before choosing GL.

Prerequisites: wasm32-unknown-unknown Rust target and wasm-bindgen-cli 0.2.127.
Run `web/build.sh`, then `python3 -m http.server 8766 --bind 127.0.0.1 --directory web`.
Open `http://127.0.0.1:8766/?backend=webgl` or `?backend=webgpu`.
`VOXY_WEB_PROFILE=dev` selects a development build; `VOXY_WASM_BINDGEN` can select
a matching CLI executable without replacing the globally installed CLI.
Generated `web/pkg` files are ignored build artifacts.

Verified in Codex's local browser: WASM development build, strict WASM Clippy,
both explicit backends showing animated 3D geometry and overlay, over 1500
presented frames per backend, no captured warning/error console entries, and
Space freezing animation time while frames continue. This does not prove mobile
browser/device compatibility, context-loss recovery, touch input, voxel-world
browser rendering or the complete platform acceptance matrix.

## Optional CUDA ownership boundary

`voxy_cuda` is a separate opt-in crate. Default builds have no CUDA dependency;
feature `cuda` uses dynamically loaded cudarc driver bindings, currently targeting
CUDA 12.0 APIs. `CudaCompute::new` selects an explicit device ordinal and rejects
zero memory budgets. Missing driver libraries return `DriverUnavailable` before
cudarc driver entrypoints are used. The localized unsafe availability probe is
isolated here; shared engine crates still forbid unsafe code.

`roundtrip_u32` bounds the device allocation, uploads values to private CUDA memory,
reads them back and synchronizes before releasing it. It does not borrow/import
wgpu buffers or claim graphics interop. `affine_u32` now loads embedded PTX and launches a fixed in-place wrapping
u32 multiply/add kernel with 256 threads per block and a count guard. Typed buffer
and scalar arguments match the fixed kernel signature. The localized unsafe launch
is documented at the call site; no arbitrary kernel is exposed through this safe
API. Module/function/buffer lifetimes extend through synchronized readback.
PTX targets sm_52 with ISA 6.0; a compatible driver/GPU must actually accept it.
The PTX source follows [NVIDIA's ISA specification](https://docs.nvidia.com/cuda/parallel-thread-execution/).

This supplies context/memory ownership, transfers and fixed-kernel dispatch code;
asynchronous jobs, generic kernel ABI, renderer synchronization and NVIDIA hardware
acceptance remain required work. NVIDIA driver execution is unverified locally.
The compiler verification described below now assembles this PTX with NVIDIA PTXAS.

Run on a machine with an appropriate NVIDIA driver/device:
```sh
cargo run -p voxy_cuda --features cuda --example cuda_probe -- 0
```
The probe compares 4097 u32 values exactly after transfer and kernel dispatch, including a partial final block and arithmetic overflow. Local compilation
and strict all-feature Clippy passed; default-feature budget/disabled-path tests
passed. This Mac cannot establish NVIDIA execution. cudarc API semantics follow
[its driver documentation](https://docs.rs/cudarc/0.19.10/cudarc/driver/index.html).

## Hardware ray-query foundation

`RayScene` builds an opaque triangle BLAS and an identity-instance TLAS from
finite triangle vertices. It rejects devices without enabled ray-query features,
invalid geometry and exceeded acceleration-structure/buffer limits. Builds are
encoded explicitly before passes binding the TLAS. The engine library retains
`unsafe_code = forbid` and does not automatically request experimental features.

The isolated `voxy_ray_probe` executable accepts `--experimental`, enables wgpu's
experimental token at one documented unsafe call site and requests ray-query
features. `cargo run -p voxy_ray_probe -- --experimental` passed on Apple M4 Max /
Metal: acceleration structures built, center ray hit the triangle at distance 1,
barycentrics matched and off-axis ray missed. This is real hardware ray-query
execution, not evidence of NVIDIA RTX execution. NVIDIA/DX12/Vulkan validation,
scene instances/updates, renderer lighting/shadows and raster fallback remain
required. CUDA ownership/transfer integration is now supplied separately; kernel execution and renderer interop remain pending.

## 2D sprite batching

`SpriteBatch` combines sprites sharing a texture/atlas into one indexed mesh and
one `SceneDraw`. Each sprite supports position, size, rotation, RGBA tint and UV
rectangle (including mirrored UV endpoints). Appending preserves painter order;
invalid input/capacity/overflow rejects the entire sprite before modifying the
batch. `clear` reuses CPU allocations. Upload the batch mesh once for static UI;
animated batches can update an existing GPU geometry allocation when their vertex/index counts fit its initial capacity.

The real Metal pixel smoke now renders two sprites in a single overlay draw,
checks both pixel regions and verifies that shader replacement still works for
the batch. Atlas packing, GPU instancing, automatic buffer growth and performance
measurements remain pending; no throughput claim follows from this smoke test.

## OpenXR runtime discovery

`voxy_xr::XrRuntime::discover` loads the standard OpenXR loader, negotiates a strict
graphics extension (Vulkan enable2, desktop GL, or DX12 on Windows), creates an
instance and selects a head-mounted display with two nonzero PRIMARY_STEREO view
sizes. The object owns the instance so its system handle remains valid. Loader,
runtime, missing extension and invalid stereo view errors remain distinct.
Before returning, discovery now calls the selected API's graphics-requirements
function and retains the typed result. Vulkan/GL/GLES expose runtime-supported API
version ranges; DX12 on Windows exposes adapter LUID and minimum feature level.
`graphics_requirements()` provides read-only access and the probe prints it. No
fallback API or independent desktop adapter is created. Vulkan enable2 integration
must next create the graphics instance/device through runtime functions; legacy
extension queries are not used without the legacy extension being enabled.

Run `cargo run -p voxy_xr --example xr_probe -- vulkan` on a configured OpenXR
machine. The desktop probe uses empty platform info. Android now has
`discover_android(graphics, AndroidApp)`: it initializes the loader with valid
VM/activity pointers and retains the activity alongside the instance. Android
instance creation requires `khr_android_create_instance`; GLES selection requires
`khr_opengl_es_enable`, distinct from desktop GL. Calling desktop `discover` on
Android returns `AndroidPlatformRequired` before loading. Strict Android-target
Clippy passed, but no Quest runtime/device execution is available. The existing
mobile scene entrypoint has not yet been connected to an XR session. Session, graphics-device binding,
swapchain images, timing, controller actions/haptics and headset rendering are
not yet implemented. Safe accessors expose the instance/system for the subsequent
binding integration; the unsafe dynamic loader call is isolated in this crate.

Local proof covers compilation, strict Clippy and extension negotiation tests.
A missing-loader failure is not evidence of real headset support. API behavior
follows [openxr Entry documentation](https://docs.rs/openxr/0.22.0/openxr/struct.Entry.html).

## Vulkan XR session ownership

`XrRuntime::create_vulkan_session` now forwards runtime-selected native Vulkan
handles to OpenXR and returns a session/frame-waiter/frame-stream bundle borrowing
the runtime. This is explicitly unsafe: the caller must supply graphics resources
created according to Vulkan enable2 requirements, keep them owned by the provided
Send+Sync guard and respect external synchronization. A wrong selected backend is
rejected before the runtime call. The guard is retained by session clones; Android
adds an activity clone to keep VM/activity ownership alive until session teardown.

Host and Android compilation/strict Clippy pass. No native Vulkan device creation,
OpenXR session creation or presentation was executed here. This factory is an
ownership boundary, not a working VR render loop. Runtime-created Vulkan
instance/device, wgpu resource import, image swapchains, begin/end lifecycle,
frame timing and projection layer submission still need integration.

## DX12 and Android GLES XR session factories

Windows `create_dx12_session` accepts the typed DX12 device/queue binding and
requires its device to match the runtime's queried adapter LUID and minimum
feature level. Android `create_gles_session` accepts an EGL display/config/context
binding; the context and threading must meet GLES extension requirements.
Both are explicitly unsafe native-handle boundaries with an owning Send+Sync
resource guard. Wrong graphics selection fails before the runtime call. Vulkan,
DX12 and GLES factories share the guard/lifetime handling, including Android
activity retention. All return the same typed lifecycle/frame API.

Strict Clippy passed for Windows GNU-target (DX12), Android arm64-target (GLES)
and host (Vulkan path). These are compile checks, not session creation or GPU
presentation proof. Device creation/import, native synchronization and real
headset execution remain required for every backend. Desktop `create_gl_session`
now accepts the typed OpenGL binding on Linux/Windows and retains its ownership
guard through all session clones. GLX/WGL are preferred; the OpenXR Wayland
binding is deprecated. The caller must meet the queried GL version requirements
and context-current/threading rules. Strict Linux and Windows GNU-target Clippy
passed for this factory; no native GL XR session has been executed.

## XR session lifecycle and frame timing

`XrSession::handle_state` applies READY (begin PRIMARY_STEREO), STOPPING (end),
and EXITING/LOSS_PENDING (request teardown) events belonging to its session.
`begin_frame` waits for runtime-predicted display timing, begins the frame and
retains its FrameState. `end_frame` uses that exact predicted time and clears the
pending frame only after successful runtime submission. It rejects absent frames,
layer-count overflow and submitted layers when should_render is false.

The frame waiter/stream are private so this owner controls sequencing. Do not
mix its lifecycle calls with raw Session begin/end methods. Duplicate READY,
frame begin while stopped/another frame is unfinished, and STOPPING with a pending
frame return typed lifecycle errors. Release runtime swapchain images after GPU
completion before end_frame. Image
ownership, stereo projection layers and device-loss recovery still need integration.
`locate_stereo_views` obtains both eyes at the pending frame's predicted display
time. It rejects calls outside a running frame and spaces from foreign instances,
returns None on skipped frames or invalid position/orientation, and rejects
non-stereo output. Valid inferred poses do not require TRACKED flags. The base
space must belong to the session; the wrapper can check instance identity, while
the runtime validates the space/session relationship. Tests cover missing valid
flags and malformed eye counts. All five XR unit tests and strict Clippy on
Linux/Windows GNU and Android arm64 passed. These methods have not been run
against an OpenXR runtime.

## XR swapchain ownership

`XrSwapchain` creates/enumerates a runtime swapchain and permits one outstanding
image. `acquire`, timeout-aware `wait`, `ready_image` and `release` enforce
Idle → Acquired → Ready → Idle. Timeout retains Acquired for retry. Negative
timeouts and invalid call ordering are rejected. Image handles are exposed through
ready_image only after successful waiting; GPU completion remains the caller's
responsibility before release. No Drop implementation automatically releases
potentially in-flight GPU resources.

The wrapper uses isolated raw wait/release calls because openxr 0.22's high-level
wait converts positive TIMEOUT_EXPIRED to success and marks its waited flag. Raw
wait explicitly preserves timeout semantics; raw release avoids mixing ownership
with that high-level flag. Mutable access serializes these operations. Creation
and image acquisition are still delegated to the typed OpenXR bindings.
`sub_image` builds a typed composition-layer image reference only after successful
release. It validates nonnegative offsets, positive extents, viewport bounds and
array layer against the swapchain's original dimensions. i64 extent arithmetic
avoids signed overflow. The returned reference borrows the owned swapchain,
preventing another mutable acquisition while that layer is still used. Zero
swapchain dimensions/array size are rejected before runtime creation.
Tests check edge-aligned rectangles, invalid layers, negative/zero dimensions and
overflow-shaped offsets without requiring a runtime.
Compilation/strict host+Android Clippy is the current evidence; no runtime image
acquire/wait/release has executed here. Texture import and stereo projection layer
submission remain required integration, and timeout behavior needs runtime proof.

## VR controller actions

`XrActions::new` creates one action set with left/right subaction paths and typed
input/output actions: grip/aim poses, select button, trigger/squeeze floats,
thumbstick vector and haptic output. Choose a Simple Controller or Oculus Touch
profile explicitly. Simple binds poses/select/haptic while analog channels remain
unbound/inactive; Touch binds poses, stick click as select, analog inputs and
haptic. No platform menu button is required or intercepted.

`attach` attaches the set once to a session; `sync` synchronizes both hands before
state reads. Typed OpenXR actions expose state queries, pose-space creation and
haptic feedback through the owning session. Read active flags before using values,
locate pose spaces at predicted display time and handle loss of focus/tracking.
`read_hand` returns complete action states for select/trigger/squeeze/stick,
including `is_active`, changed flags and last-change time, plus aim/grip activity.
`Hand` selects the left/right subaction path without exposing an unchecked index.
`vibrate` validates amplitude 0..1, finite nonnegative frequency (zero requests
runtime default) and positive duration fitting OpenXR's signed nanoseconds before
sending feedback. `stop_vibration` cancels feedback for one hand. Tests cover NaN,
infinity, amplitude/frequency ranges, zero and overflowing duration without a
runtime. These methods do not create pose spaces or establish session lifecycle.

The runtime probe creates Simple actions after discovery. No real action creation,
input synchronization or vibration was exercised here because no loader/session
is available. Strict host/Android Clippy passed. Profiles follow the
[OpenXR interaction specification](https://registry.khronos.org/OpenXR/specs/1.1/html/xrspec.html).
Quest Touch Plus/Pro extensions and actual session integration remain pending.

## Stereo temporal motion

The optional `voxy_render/openxr` feature adds `XrView::from_openxr(view, flags)`.
It copies the runtime pose/FOV directly and retains `POSITION_VALID` and
`ORIENTATION_VALID` from the same locate call. It does not normalize bad poses
or treat `TRACKED` as a substitute for `VALID`. Estimated but valid runtime poses
remain usable. Rendering world coordinates must use the same reference space.
The feature uses OpenXR types without loading a runtime or changing the default
renderer dependencies. Five XR unit tests and strict host Clippy passed with
this feature enabled. Windows GNU and Android ARM64 `cargo check --lib` passed;
these compilation checks do not prove headset execution. The feature-enabled
stereo motion probe also passed on Metal.

`XrMotionHistory` keeps independent unjittered left/right view-projection matrices.
`prepare` validates both eyes and does not advance history. `presented` validates
and commits the stereo pair atomically; call it only after successful
`XrSession::end_stereo_frame` submission using the exact rendered views. A failed
submission or skipped frame must not call `presented`. Invalid data during
preparation resets both eyes, so tracking recovery produces zero motion. Reset
explicitly on session/reference-space changes, teleports and swapchain
recreation. Clipping-plane changes automatically reset both eye histories.
Successful OpenXR submission is not proof of physical headset display.

`cargo run -p voxy_render --example xr_motion` rasterizes a world plane into
independent eye depth textures and reads back RG16 motion for every pixel. Its
expected UV displacement comes from eye translation, plane distance and frustum
width, independent of the motion shader. The fixture checks asymmetric frusta,
opposite eye motion, skipped-frame history and zero motion after tracking loss.
Metal on Apple M4 Max and Linux Vulkan/OpenGL on Mesa llvmpipe passed.
`tools/linux/temporal-motion-smoke.sh` runs this probe alongside the existing
primary, relative, composition and resident-skinned motion checks; all five
passed on both Linux backends. llvmpipe is CPU software rendering. This test
does not create an OpenXR session or import
headset swapchain images.

## XR view foundation

`XrView` accepts a tracked per-eye pose and `XrFov` accepts four runtime angles in
radians. The projection supports asymmetry and flipped frusta, with right-handed
-Z forward and 0..1 depth. Tracking validity, non-unit poses, degenerate frusta and
non-finite results are rejected before GPU upload. Tests check all four frustum
edges, near/far depth, stereo disparity and mirrored view mapping.

The view module is camera math only. Separate runtime discovery is now available below; OpenXR session, runtime device binding, swapchain
image import, predicted display timing, headset presentation and controller input
remain required implementation. It does not establish VR support by itself.
Angle semantics follow the [OpenXR specification](https://registry.khronos.org/OpenXR/specs/1.1-khr/html/xrspec.html#fundamentals-angles).

## Shared scene hierarchy

`voxy_scene` owns a renderer-independent `SceneGraph` with bounded object count,
validated translation/rotation/scale and generational `NodeId` handles. Handles
also identify their scene, so a node from another scene cannot alias an object.
Children inherit parent transforms; reparenting preserves the local transform
and rejects cycles before mutation. Removing a subtree invalidates every handle
and recycles slots with new generations. Traversal/removal are iterative for deep
hierarchies. Matrix composition reports overflow instead of uploading NaN/Inf.

The general GPU smoke scene now obtains both world matrices from a shared parent
in `SceneGraph`, demonstrating integration rather than only standalone math.
Scene components, persistence, asset handles and cached transform propagation
remain required work; node handles are in-process identities, not a save format.

## Custom scene shaders

`DEFAULT_SCENE_SHADER` documents the initial WGSL program. Applications load
shader assets as UTF-8 and call `SceneRenderer::reload_shader(device, source)`.
Shader compilation and creation of both world/overlay pipelines occur inside
validation scopes. Pipelines are installed together only after both succeed;
syntax, entrypoint and binding errors return `SceneShaderError` and retain the
last working program, its revision and existing material/transform bind groups.
Identical source is a cache hit. The cache retains only the active source and
pipeline pair; this is not a general multi-material pipeline cache.

The shader ABI is:
- Vertex attributes: location 0 `vec3<f32>` position, 1 `vec2<f32>` UV,
  2 `vec4<f32>` linear RGBA; entrypoints `vs_main` and `fs_main`.
- Group 0 binding 0: vertex-visible uniform `mat4x4<f32>` MVP (64 bytes).
- Group 1 bindings 0/1: fragment-visible filterable 2D texture and filtering sampler.
- One color attachment; world depth is Depth32Float, 0..1, less-or-equal.

Call reload from the device-owner task without interleaving other validation
scope operations. A host file watcher or browser asset event may trigger reload;
filesystem watching is not yet implemented in the platform shell.

`cargo run -p voxy_render --example scene_smoke -- --shader-test` creates resources
before reload, installs a channel-swapping shader, rejects three invalid shaders
and verifies the retained custom shader through real Metal pixel readback.
This test passed on Apple M4 Max.

## Portable compute shaders

`ComputeProgram::new` validates custom WGSL and a compute pipeline with entrypoint
`cs_main`. The initial compute ABI is one read/write storage buffer at group 0,
binding 0. `create_job` validates buffer byte counts and device limits before
upload. The application supplies shader-compatible bytes and workgroup counts;
the shader must guard invocations outside its array bounds.

A `ComputeJob` owns independent GPU storage and readback. Its storage buffer can
be used in subsequent passes on the same device. `encode` consumes the job,
records dispatch and readback copy; the application submits the encoder before
`begin_read`. `PendingComputeReadback::try_read` is nonblocking and takes the data
once. The native shell drives device polling; browser code yields to its event
loop. Dropping pending/completed readback destroys its private buffer and cancels
unused mapping safely. This is portable GPU compute, not CUDA integration.

`cargo run -p voxy_render --example compute_smoke` passed on Metal / Apple M4 Max:
1042 exact integer results in two independent jobs, a partially filled workgroup,
invalid WGSL/entrypoint/binding rejection, invalid buffer/dispatch rejection,
once-only readback and cancellation before/after mapping. WebGL devices without
compute limits return `ComputeError::Unsupported`; a WebGL CPU alternative is
still required for applications that mandate those calculations.

## Required implementation milestones

1. Backend policy and real adapter diagnostics, preserving existing games.
2. General indexed meshes, perspective cameras, colored materials and transforms;
   2D sprites, textures, alpha blending, orthographic camera and batching.
3. Scene hierarchy, resource handles, asset loading, input and fixed-step game API;
   a single example combining an animated 3D scene with a 2D overlay.
4. Validated custom WGSL shaders, pipeline caching, shader reload with error
   recovery, compute jobs and readback with explicit feature negotiation.
5. Browser shell and a separate WebGL-compatible renderer. Existing voxel storage
   buffers and texture arrays need an alternative path, not just a backend flag.
6. Hardware ray queries/acceleration structures on supported GPUs with raster
   fallback. RTX denotes NVIDIA hardware; it is not a graphics API.
7. Separate optional CUDA compute integration on NVIDIA systems, with explicit
   buffer ownership/synchronization and independent tests. CUDA is not provided
   by wgpu and is unavailable on this Apple Metal path.
8. Windows DX12, Linux Vulkan/GL, macOS Metal, iOS Metal, Android Vulkan/GLES
   and browser WebGPU/WebGL build/render tests, installable packages, reproducible
   examples and performance evidence.
9. VR runtime integration: OpenXR on Windows/Linux and Android headsets including
   Meta Quest; stereo rendering from runtime-supplied per-eye poses and asymmetric
   frusta, predicted display time, swapchain image acquire/wait/release, session
   lifecycle, tracking validity, controller actions and haptics. Prove rendering
   and input on actual headsets. A desktop stereo preview is not VR support.

## Linux Mesa runtime validation

`tools/linux/Dockerfile` supplies Vulkan/GL Mesa libraries and Xvfb in an isolated
Linux test image. `scene_smoke` executed successfully inside the Linux arm64 VM
on Mesa 25.0.7 llvmpipe (LLVM 19.1.7), backend Vulkan, device_type Cpu. Its pixel
checks verified indexed OBJ geometry, PNG texture decode/upload, dynamic vertex
updates, depth occlusion, repeated UV sampling and sprite overlay alpha.
This is a real Vulkan driver/render/readback path using a CPU adapter, not
hardware acceleration or NVIDIA RTX proof. The strict OpenGL probe also passed
the same pixel checks on Mesa 4.5 Core Profile / llvmpipe under Xvfb. The initial
Xvfb wrapper waited for its readiness signal before launching the renderer;
delivering that signal resumed the existing process and it completed successfully.
The native arm64 `scene_demo --smoke` also passed on both Vulkan and OpenGL
under X11/Xvfb: 120 presented frames, both resize stages and zero-size
suspension/resumption. Run containers with `--init` so Xvfb readiness signaling
is handled correctly. Wayland, interactive input, packaging and actual Linux
hardware devices remain unverified.

`VOXY_SCENE_BACKEND=auto|vulkan|gl|metal|dx12` now makes the offscreen smoke's API
selection strict; an unknown value errors before graphics initialization.
Build image: `docker build -t voxy-linux-smoke -f tools/linux/Dockerfile tools/linux`.
Container builds use source/registry read-only mounts and isolated target output;
Mesa installation changes only the test image. The latest-slim image tag is not
an immutable CI pin; the tested image/toolchain versions are recorded as evidence.

## Linux cross-target validation

`x86_64-unknown-linux-gnu` checks passed for all voxy_app targets (including
X11/Wayland/wgpu dependencies), and strict Clippy passed for voxy_app and voxy_xr.
No Linux executable or compositor/GPU run is implied by those cross-checks.
An existing Docker/Colima Linux arm64 VM is available; a separate native Linux
build uses source and registry read-only mounts and isolated
`target/linux-docker` output. Native Linux arm64 scene_smoke linked with Rust
nightly 2026-09-30 and was rebuilt with the image's bundled nightly 2026-09-17.
Vulkan and OpenGL execution results are recorded above. No host source toolchain
was changed by the container's toolchain installation.

## Windows cross-target validation

Installed `x86_64-pc-windows-gnu` Rust standard library and verified:
```sh
cargo check -p voxy_app --all-targets --target x86_64-pc-windows-gnu
cargo clippy -p voxy_xr --all-targets --target x86_64-pc-windows-gnu -- -D warnings
cargo clippy -p voxy_cuda --all-targets --features cuda --target x86_64-pc-windows-gnu -- -D warnings
```
These compile Windows-specific winit/wgpu dependencies and DX12 OpenXR requirements
that host macOS checks exclude. They are metadata/type checks, not linked Windows
executables. Native Windows rendering/input/recovery, NVIDIA CUDA/RTX kernel
execution and PC headset presentation remain unverified. No Windows artifact was
produced or published from these checks.

## Platform and VR acceptance matrix

| Target | Intended graphics path | Required platform proof | Current evidence |
| --- | --- | --- | --- |
| Windows | DirectX 12; Vulkan/GL where available | Installed app, rendering, input, resize and device recovery | GNU-target app check plus XR/CUDA Clippy passed; linked EXE/runtime/package proof pending |
| Linux | Vulkan; OpenGL/GLES where available | X11 and Wayland window/display lifecycle, rendering and input | GNU-target app/XR Clippy passed; Linux arm64 scene_smoke passed Vulkan and OpenGL pixels on Mesa llvmpipe; X11 window smoke passed both APIs (120 frames/resize/suspension); Wayland and hardware proof pending |
| macOS | Metal | App package, window/input lifecycle and GPU execution | Metal offscreen rendering on Apple M4 Max; packaging pending |
| iOS | Metal | Signed device build, touch, orientation, safe areas, background/resume and surface recreation | Rust entrypoint and strict cross-target check passed; SDK/package/device proof pending |
| Android | Vulkan; GLES fallback | APK/AAB device build, touch, orientation, activity/surface lifecycle and GPU fallback | NativeActivity entrypoint, manifest and strict cross-target check passed; NDK/package/device proof pending |
| Browser | WebGPU; WebGL2 fallback | Browser build and real render/input tests on both paths | WASM dev build and both backend previews verified; extended lifecycle/device matrix pending |
| PC VR | OpenXR with runtime-compatible graphics binding | Real headset stereo, predicted-time tracking, controllers, haptics and session recovery | Pending |
| Android VR / Meta Quest | OpenXR with Vulkan/GLES runtime binding | Installable headset package and real stereo/tracking/input/session tests | Pending |

GPU API support alone does not establish OS support. Mobile entrypoints must be
separate platform shells around the shared engine; the current desktop `main`
and synchronous initialization are not proof of iOS/Android compatibility.

The VR runtime chooses the graphics device and owns presentation images. Extend
resource import and submission synchronization explicitly; do not create an
independent desktop adapter/swapchain and assume it can present to a headset.
Two eyes require distinct transforms and asymmetric projections supplied by the
runtime, rather than two copies of the desktop camera. Optional multiview and
foveation need capability checks and a correct two-pass baseline.

Apple Vision Pro/visionOS is a separate possible XR target, not implied by iOS
or macOS support; if included later it needs its own Compositor Services/Metal
platform integration and acceptance evidence.

Platform references:
- [wgpu backend matrix](https://docs.rs/crate/wgpu/30.0.1/source/README.md)
- [OpenXR specification](https://registry.khronos.org/OpenXR/specs/1.0-khr/pdf/xrspec.pdf)
- [Android XR OpenXR integration](https://developer.android.com/develop/xr/openxr)
- [Apple immersive Metal rendering](https://developer.apple.com/documentation/CompositorServices/drawing-fully-immersive-content-using-metal)

Completion requires runnable 2D/3D examples and verified execution for every
claimed platform feature. A compiled backend selector alone is not completion.

## XR environment composition negotiation

Discovery now retains the two runtime view configurations (recommended image
sizes and sample counts) and enumerates PRIMARY_STEREO environment blend modes.
`environment_blend_modes` preserves runtime preference order.
`select_environment_blend_mode` selects the first supported application preference;
an empty preference list explicitly accepts the runtime's preferred mode. An
explicit request for OPAQUE never falls back silently to additive/alpha composition.
No available mode or no intersection returns `UnsupportedBlendMode`.

`end_frame` rejects a mode not enumerated for this runtime/system/view configuration
before submitting to OpenXR, retaining the pending frame so the caller can retry.
The probe reports both eye configurations, composition modes and whether opaque
VR is available. Six XR tests pass, including preference ordering, explicit mode
rejection and empty runtime capabilities. This negotiation does not implement
Meta passthrough extensions or prove mixed-reality rendering on a headset.
Reference: [OpenXR rendering and environment blend modes](https://github.com/KhronosGroup/OpenXR-Docs/blob/main/specification/sources/chapters/rendering.adoc).

## XR tracking origins and controller spaces

`XrSession::create_tracking_space` enumerates runtime-supported spaces and creates
an identity-offset reference space using the first supported application preference.
`[STAGE, LOCAL]` explicitly permits LOCAL fallback; `[STAGE]` requires STAGE.
Empty preferences and no intersection return `UnsupportedReferenceSpace` rather
than silently changing the world's origin. The selected type is returned alongside
the owning Space. A unit test verifies preference ordering, explicit fallback,
strict rejection and empty capabilities. Applications must still handle reference
space changes/recentering; no world compensation is performed automatically.

`XrActions::create_hand_spaces` creates grip and aim action spaces for each hand.
Locate these against the same tracking origin as the eyes at the current frame's
predicted time, after syncing actions. Both action activity and location validity
must be respected. Space ownership retains the underlying session/actions through
the OpenXR crate. Creation failures release already-created local resources.
Seven XR tests and strict Clippy for Linux/Windows GNU and Android arm64 passed.
Actual reference/action space creation and tracking remain unverified on a runtime.
Reference: [OpenXR spaces](https://github.com/KhronosGroup/OpenXR-Docs/blob/main/specification/sources/chapters/spaces.adoc).

## XR event routing

`XrSession::handle_event` accepts an event polled from the owning runtime instance
and matches session handles before applying state changes. Foreign session-state,
reference-space and interaction-profile events return Ignored. The instance event
queue stays with the application so multiple sessions can receive the same event.
READY/STOPPING/EXITING/LOSS_PENDING use the existing lifecycle owner. Instance
loss stops new frames and returns its loss time for teardown/recreation.

Reference-space changes return an owned type/time/optional previous-origin pose,
so callers can retain the effect after reusing EventDataBuffer and apply recentering
at its effective time. The pose accessor is called only when pose_valid is true.
Profile changes and lost-event counts are also surfaced. Extensions remain
available through the original event and are not treated as handled by this API.
Application world compensation and device/session recreation are still required.
Seven existing XR tests pass and strict Clippy passes on host, Linux/Windows GNU
and Android arm64. Event routing itself has not been exercised against a runtime.
Reference: [OpenXR session lifecycle](https://github.com/KhronosGroup/OpenXR-Docs/blob/main/specification/sources/chapters/session.adoc).

## XR stereo projection submission

`XrSession::end_stereo_frame` builds two projection views and one projection layer,
then submits through the existing predicted-time `end_frame` owner. Eye poses,
FOVs and subimages are explicitly ordered left/right. It accepts either separate
eye swapchains or two array layers of one swapchain. Obtain subimages through
`XrSwapchain::sub_image` after rendering, GPU completion and release; their borrows
remain live until submission returns. The caller must supply poses located for
this frame in the same reference space. This API cannot prove frame freshness of
arbitrary externally supplied views or release state of raw OpenXR subimages.

The method rejects non-finite pose/FOV data, rotations whose squared norm differs
from one by more than 1e-4, and a foreign reference-space instance before submission.
It forwards explicit layer flags for the application's alpha convention. The
runtime retains responsibility for additional native-layer validity. Use an empty
`end_frame` when rendering or valid eye tracking is unavailable. Eight XR unit tests
pass (including invalid projection data), and Linux/Windows GNU and Android arm64
strict Clippy pass. Native swapchain image import, GPU synchronization and an actual
headset projection submission remain unverified and required for VR completion.

## Scope update: ray tracing and NVIDIA DLSS 5

The user explicitly requires ray tracing and NVIDIA DLSS 5 (confirmed after the
initial spelling “dlls 5”), in addition to the existing platforms and VR scope.
Existing ray-query/BLAS/TLAS smoke evidence is not a complete ray-traced lighting
renderer. DLSS 5 neural rendering must be integrated and verified separately from
Super Resolution, Ray Reconstruction and Frame Generation. No DLSS integration
or DLSS 5 support is currently implemented or claimed.

NVIDIA's September 22, 2026 developer article describes a final neural-rendering
stage using frame color and motion vectors, plus developer intensity/model/masking
controls, running locally on RTX 50-series GPUs. The current general renderer has
no motion-vector render target or previous-frame transform history. Those inputs,
frame-aligned resource ownership, a native SDK bridge and hardware execution must
be implemented. The inspected public Streamline include listing has SR/RR/FG
headers but did not expose a DLSS-NR header; this is an observation of that listing,
not proof that authorized development packages are unavailable. Obtain and inspect
the matching official feature SDK before defining an FFI ABI.

Sources checked September 30, 2026:
- [NVIDIA DLSS 5 developer announcement](https://developer.nvidia.com/blog/whats-new-for-game-developers-dlss-5-with-3d-guided-neural-rendering-nvidia-ace-updates-and-new-rtx-kit-capabilities)
- [Official Streamline includes](https://github.com/NVIDIA-RTX/Streamline/tree/main/include)
- [Streamline 2.14.1 release](https://github.com/NVIDIA-RTX/Streamline/releases/tag/v2.14.1)

## XR swapchain format negotiation

`XrSession::select_swapchain_format` enumerates native runtime formats and selects
the first renderer preference that is supported. It preserves graphics-specific
format types and rejects empty lists/no intersection with
`UnsupportedSwapchainFormat`, avoiding implicit color-space/format conversion.
Nine XR unit tests pass, including preference ordering and no-intersection cases;
strict Clippy passes for Linux/Windows GNU and Android arm64. Actual runtime
enumeration, image import and headset display still need device verification.

## Temporal transform history for motion vectors

`MotionHistory` now stores the previous presented unjittered MVP per object/eye.
`prepare` returns current/previous matrices without advancing history; skipped
frames therefore retain the last presented transform. `presented` commits after
successful presentation, while `reset` invalidates history for cuts, teleports,
resize or resource recreation. The initial/reset pair uses identical matrices
and marks history invalid, enabling zero motion and temporal reset. Non-finite
matrices are rejected atomically. Tests verify skipped frames, resets and invalid
updates. This transform history is now connected to the native scene shell and uploaded
to the GPU transform uniform. A GPU motion-vector target and NVIDIA SDK integration
remain pending. Deforming meshes
will additionally need previous vertex/bone state rather than MVP alone.

## Native temporal transform upload

The scene transform uniform now contains current MVP at byte 0 and previous MVP
at byte 64 (128 bytes total). Default WGSL declares both. `update_motion` validates
both matrices before a single queue write and substitutes current for previous
when history is invalid. Legacy `update` writes identical current/previous values
for zero motion. Existing custom shaders using the first matrix remain compatible
with the larger binding; Metal shader-reload/pixel smoke passed.

`SceneApp` prepares the cube's motion pair, uploads it, and commits its current MVP
only on RenderOutcome::Presented. Resize, suspension, initialization and the
zero-size smoke transition reset history. The overlay remains zero-motion. This
does not yet generate velocity pixels; the default vertex shader currently uses
only current MVP. Metal native smoke passed 120 frames with resize/suspension,
strict app/render Clippy passed, and the browser WASM shell still type-checks.

## GPU motion-vector shader and readback

`MOTION_SCENE_SHADER` runs as a separate scene-renderer pass into RGBA16Float with
Depth32Float. Its interpolated current/previous clip coordinates produce backward
motion (previous minus current) in normalized top-left UV units; scale by output
width/height for pixel units. Both matrices must be unjittered. Invalid history is
handled by `update_motion` substituting current for previous. Nonpositive clip W
produces zero rather than a division singularity. Encode opaque world draws only;
this initial pass does not implement translucent/cutout motion or deformation.

`cargo run -p voxy_render --example motion_smoke` passed on Metal/Apple M4 Max:
RGBA16Float pixel readback verified stationary zero, a translated object's exact
(-0.125, +0.125) backward UV motion and reset-history zero. Error scopes found no
validation failures. Strict renderer all-target Clippy passed. This verifies an
offscreen velocity pass, not the complete temporal pipeline: the native shell
still needs a persistent motion target/pass, depth/color alignment and SDK resource
interop. Perspective/disocclusion/occlusion cases and other backend runtime tests
remain required. DLSS 5 itself is still unimplemented.

## Motion vectors: perspective and cross-backend evidence

The motion smoke now uses independent current/previous MVPs and adds a real
90-degree perspective camera with clip W=2. Translation produces the expected
(-0.0625,+0.0625) UV motion rather than the orthographic (-0.125,+0.125), proving
perspective division in this case. Stationary perspective geometry gives zero;
previous geometry behind the camera gives zero instead of a singular velocity.
The six readback cases passed on Metal/Apple M4 Max and on Linux arm64 Mesa
llvmpipe for both strict Vulkan and OpenGL (Xvfb). Mesa is CPU software rendering,
not NVIDIA GPU or DLSS proof. Clippy passed on the pinned host toolchain.

`VOXY_MOTION_BACKEND=auto|metal|vulkan|gl|dx12` makes probe backend selection strict.
Linux build uses isolated target output and source/registry read-only mounts.
Perspective-varying depth across one triangle, occlusion/disocclusion, deforming
geometry and actual SDK motion conventions still need further validation.

## Optional windowed motion-vector pass

`SceneSurface::enable_motion_vectors` now creates a persistent RGBA16Float velocity
texture and dedicated motion shader renderer after checking required format usages
and blend support. Activation is atomic after successful shader validation. Resize
recreates the velocity texture; zero-size suspension avoids submission. During a
successful frame acquisition the opaque world velocity pass and color/overlay pass
share one encoder/submission. Each pass clears the shared depth independently.
Overlay draws are excluded from velocities. Translucent world geometry still needs
an explicit motion policy; this pass currently treats all world draws as opaque.

`motion_texture` exposes the latest target for subsequent integration. Access is unavailable after creation/resize until a frame presents; skipped frames
retain previous contents and their previous presentation ID. External consumers must preserve GPU synchronization and frame
identity. The API does not provide native SDK image/resource-state interop yet.
The feature is opt-in so baseline platforms without float attachment support keep
their original render path. An explicit unsupported request returns an error.

Run `cargo run -p voxy_app --example scene_demo -- --smoke --motion --backend metal`.
Metal/Apple M4 Max passed 120 presented frames, resize and zero-size suspension;
the shell checks target dimensions against each rendered window size. Strict
app/render Clippy passed. DLSS 5 integration and the complete aligned SDK color,
depth/motion input pipeline remain pending.

## Motion frame identity and validity

`SceneSurface::motion_frame` now returns the velocity texture together with its
surface-local presentation ID, starting at one. Successful presentation advances
the ID; skipped frames preserve the old ID. Creation, resize and zero-size
suspension invalidate access until the next presented frame. `motion_texture`
also respects this validity gate, so uninitialized resized contents are not exposed.
Counter exhaustion returns FrameIdExhausted before acquisition rather than wrapping.
A recreated surface starts a new ID namespace; consumers must track surface/device
generation as well. GPU completion and native SDK resource barriers remain external.

The native smoke checks monotonically matching motion/presentation IDs and texture
dimensions on every presented frame. Its zero-size transition verifies no stale
frame is exposed during suspension or after resize before submission. Metal/M4 Max
passed all 120 frames with these checks. Strict app/render Clippy and browser WASM
check passed. This is frame ownership infrastructure, not DLSS 5 execution proof.

## Window resize race found by Linux motion validation

The first native Linux Vulkan/GL window smoke with `--motion` failed the velocity
size check: `Window::inner_size` had advanced before the queued Resized event
updated the configured surface. The shell now compares the current physical size
with `SceneSurface::configured_size` before deriving projection matrices, resizes
all targets if necessary and resets temporal history. Suspended nonzero surfaces
are also restored through this synchronization path. This keeps camera, surface
and velocity dimensions aligned even when redraw precedes the resize callback.

After the fix both Linux arm64 X11/Xvfb Vulkan and OpenGL passed 120-frame native
motion smoke, including two resize stages, suspension/invalidation and matching
motion/presentation IDs. Both used Mesa 25.0.7 llvmpipe software rendering. Windows
GNU all-target app Clippy passed before the resize edit. The subsequent strict
host Clippy run was blocked by 14 diagnostics in the independently changing
`voxy_render/src/model.rs` (panic documentation, conversions and related lints),
so this turn does not claim a green full Clippy run for the final worktree.
Metal/M4 Max also passed the final 120-frame motion smoke after this resize fix.

## Loader checks restored

The current worktree's earlier model-loader Clippy diagnostics were reduced to
one redundant Node::index closure; replacing it restored strict all-target Clippy
for voxy_app and voxy_render. Added explicit glTF triangle import tests verify
vertex/index data, no implicit URI opening, missing/truncated buffers and combined
source+buffer/vertex/index budgets. Exact source f32 values are compared by bits.
The new import tests pass. Rendering and animation unit suites were rerun after
the final lint fix; this does not prove complete glTF material/skin/animation coverage.

## Aligned depth and velocity inputs

`SceneSurface::temporal_frame` now borrows the retained Depth32Float texture and
velocity texture with one presentation ID. The depth texture supports sampling
and readback as well as attachment use. Both are unavailable before presentation
or after resize/suspension, sharing the velocity validity gate. Borrowing prevents
surface rendering/resizing while the inputs are in use through this owner.
The final color pass rewrites depth; overlays neither write nor change that world
depth. Depth is non-reversed device-space [0,1], cleared to 1. Native resource-state
transitions, GPU completion and SDK consumption are still not implemented.

Metal/M4 Max native smoke passed 120 frames with shared ID/dimension checks for
these inputs, resize and suspension. Strict app/render Clippy passed. The API does
not yet retain HUD-less color, implement translucent depth conventions, or execute
DLSS 5; these aligned inputs are only part of the required temporal pipeline.

## Models and skeletal animation

`voxy_render::ModelAsset::parse(source, buffers, ModelLimits::default())` imports
GLB with an embedded binary buffer or glTF with explicitly supplied buffers in
buffer-index order. The importer never opens URIs, files or network resources.
It preserves the complete node hierarchy, triangle primitives, UVs, base colors,
skin joint mappings, inverse bind matrices and LINEAR translation/rotation/scale
clips. Float weights are normalized into four UNORM16 influences with an exact
65535 sum. Byte, geometry and animation-key budgets are enforced before reading
large arrays, and accessor ranges are checked before buffer readers run.

Use `Animator::new(Arc::clone(&asset.animations[index]))` and
`advance(&asset.skeleton, dt)` for playback and crossfades. For the voxel GPU
renderer, upload `ModelGeometry::Skinned` with `asset.skin_matrices(&frame.pose)`
and the instance transform; the palette already includes glTF node transforms.
The existing GPU renderer currently holds one skinned primitive. Set up its
material layer from `ModelPrimitive::color` separately.

For the general renderer, `asset.scene_meshes(&frame.pose)` produces CPU-skinned
`SceneMesh` primitives including node transforms. Upload each primitive once and
update its `SceneGeometry` each frame. This path uses ordinary scene vertices,
so it does not require vertex storage buffers on WebGL/mobile. It allocates and
skins on the CPU each call; it is a compatibility path, not a performance claim.
Apply only the instance transform to these meshes. For a static imported mesh,
`asset.mesh_transform(&pose)` supplies its animated node transform directly.

Current import bounds: one mesh instance, at most one skin, up to 256 total
nodes, positive TRS scales, at most four influences per vertex. Required
extensions, sparse accessors, morph targets, texture-bearing materials, STEP and
CUBICSPLINE animation return errors. FBX and glTF PBR shading are not implemented.
Materials use base color only. The CPU scene path handles multiple primitives;
the existing GPU skin path still has one resident mesh. Animation sample callers
must use the asset's skeleton; `Animator::advance` rejects joint-count mismatches
before changing playback state.

Run `cargo run -p voxy_render --example model_smoke` for real GPU proof: an imported
GLB triangle is animated, CPU-skinned and drawn at two times. Pixel readback must
show a 16-pixel displacement. The two frames are saved side by side to
`/tmp/voxy-model-smoke.png`. Unit tests also cover explicit buffers, embedded GLB,
static geometry, malformed/bounded inputs, weight quantization and palette motion.

## Temporal world color without overlay

The optional motion path now retains world color before overlay in the scene
renderer's sRGB surface format. `TemporalFrame::color`, depth and motion share one
presentation ID, dimensions and submission. The world-color pass uses the active
scene shader and filters overlay draws; its target is sampleable/readable and is
recreated on resize with the same validity gate. The shell validates dimensions
for all three inputs. This currently repeats world rendering before the normal
surface pass; later composition should reuse this color to avoid duplicate work.
It is LDR/sRGB rather than a linear HDR SDK input, and native SDK interop remains
pending. No DLSS execution is claimed.

Metal/M4 Max passed the 120-frame native smoke on retry. The first run terminated
with zero frames and SkippedOccluded until timeout; the cause of that intermittent
window visibility condition is not established. Strict app/render Clippy and the
browser WASM check passed. No pixel-level proof of HUD exclusion was added here.
Linux arm64 X11/Xvfb Vulkan and OpenGL also passed all 120 frames with the new
color/depth/motion dimension checks on Mesa llvmpipe (CPU software rendering).

## Ray instance transforms

`RayScene::set_transform` now updates the existing instance with a native row-major
3x4 affine object-to-world matrix. Validation rejects non-finite/singular linear
transforms before mutating the TLAS; mirrored nonsingular scales are accepted.
`build_instances` rebuilds only TLAS after an initial full BLAS/TLAS build, avoiding
BLAS rebuild for rigid object movement. Submit prior queries before mutation and
encode the instance rebuild before subsequent queries. No GPU completion fence is
implied. Unit tests cover translated/scaled/mirrored, singular, NaN and overflow
transforms. Runtime transformed-ray readback remains pending, as do multiple
instances, animated geometry, ray-traced lighting and NVIDIA RTX execution.

## Transformed ray-query runtime proof

The isolated experimental ray probe now performs four sequential GPU submissions:
initial BLAS/TLAS build; TLAS-only translation backward in Z (hit distance changes
1 to 1.5); TLAS-only translation to X=3 (center ray misses, second ray hits); and
rejected singular transform (previous hit/miss state remains unchanged). Every
phase checks hit kind, distance and barycentrics through GPU readback. Previous
submissions complete before mutation. Metal/Apple M4 Max passed all four phases,
and strict probe Clippy passed. This verifies rigid instance transforms and TLAS
rebuilds, not a full lighting renderer or NVIDIA RTX execution.

## Multiple shared-BLAS ray instances

`RayScene::with_instance_capacity` allocates a bounded TLAS for multiple instances
of its geometry; the legacy constructor retains capacity one. `set_instance`
validates slot, 24-bit custom index and affine matrix before mutation; instances
have independent transforms and 8-bit visibility masks. `remove_instance` clears a
slot. The GPU probe now uses seven sequential readback phases: rigid transforms,
two simultaneous shared-BLAS instances, mask-zero exclusion, removal and rejected
capacity/index/transform updates. Metal/M4 Max passed all seven phases; renderer
library and probe strict Clippy passed. Separate geometry BLASes, animated meshes,
lighting/shadow rendering and NVIDIA RTX hardware validation remain pending.

## Opaque light-segment visibility

`RaySegment` validates finite origin/light endpoints and positive world-space bias,
rejecting overflow and segments shorter than twice the bias. Its GPU layout is
two vec4s. `RAY_VISIBILITY_SHADER` accepts TLAS, a read-only segment buffer and
u32 visibility output; bounded invocations test only the interval from bias to
light-distance-minus-bias. Output one means unobstructed and zero means blocked.
This handles opaque geometry; cutout/translucent materials require further work.
The shader is a lighting building block, not a finished shadow renderer.

The Metal/M4 Max probe verified a blocked segment, an off-axis unobstructed segment
and a light located before the blocker (expected [0,1,1]), including workgroup
bounds guarding. Validation scope errors are surfaced before asserting readback.
The initial shader used reserved WGSL identifier target; renaming it to destination
fixed parsing. Renderer/probe strict Clippy passed. Surface lighting composition,
point/directional lights, normal-based bias and raster fallback remain pending.

## Library-owned ray visibility jobs

`RayVisibilityJob` now creates and owns input/output buffers, binding and compute
pipeline from validated RaySegments. It revalidates segments even if constructed
through Pod/Zeroable, rejects empty jobs, and checks buffer/storage/workgroup limits
before upload. Encode after acceleration builds on the same device and submit
before consuming its u32 output storage. Bindings retain resources; this does not
provide CPU readback or a completion fence. Pipelines are currently created per job
and need caching for high-frequency use.

The probe now exercises this library API rather than manually building visibility
resources. Metal/M4 Max passed the same blocked/unobstructed/short-light readback
and all seven instance phases. Strict renderer/probe Clippy passed. Renderer light
composition, batching and native NVIDIA RTX validation remain incomplete.

## Reusable visibility pipeline

`RayVisibilityPipeline` retains its device and compiles the fixed shadow pipeline
once; `create_job` allocates independent input/output storage using the shared
pipeline. The convenience RayVisibilityJob constructor remains for one-off work.
Scene and encoder must use the pipeline's device. The Metal probe now dispatches
two jobs together with reversed segment order and verifies independent output
[0,1,1] and [1,1,0] via readback. Pipeline reuse avoids repeated shader compilation;
input/output allocations still occur per job. Metal/M4 Max and strict probe Clippy
passed. Lighting composition and NVIDIA RTX/DLSS 5 execution remain pending.

## Ray-traced point-light shadow image

The experimental probe now generates 4096 validated plane-to-point-light segments,
dispatches the library visibility job and composes its GPU output into a 64x64
RGBA8 storage image. Diffuse lighting uses the plane normal/light direction with
an ambient term; the opaque triangle produces a hard shadow. Metal/M4 Max readback
verified center shadow RGB and a lit corner against CPU lighting values, with one
UNORM level tolerance. The image is `/tmp/voxy-ray-shadow.ppm`; PNG conversion was
visually inspected and shows the projected triangular shadow. Strict probe Clippy
passed. This is an offscreen lighting prototype, not the general scene renderer's
shadow integration or a complete physically based/path-traced lighting model.
NVIDIA RTX execution, raster fallback and DLSS 5 remain pending.

## Portable CPU visibility fallback

`cpu_segment_visibility` provides double-sided opaque triangle tests for validated
world-space segments without GPU ray-query features. It honors positive origin
and light-end bias and validates geometry before testing. Intersection arithmetic
uses f64 to reduce intermediate overflow. Empty geometry is unobstructed. Work is
O(segment_count * triangle_count), so this is a correctness baseline requiring a
spatial index for large scenes; it is not a raster-shadow-map fallback.

Unit tests verify both ray directions, a light before a blocker, empty/malformed
geometry and invalid bias/length. The Metal shadow image probe compares every RGB
channel of all 4096 GPU pixels against CPU visibility + lighting with one UNORM
level tolerance; all passed. Renderer/probe strict Clippy passed. This fixture does
not prove equivalence at all degenerate/edge/tiny-triangle cases. General scene
fallback selection and lighting integration remain pending.

## Shadow segment endpoint validation

The Metal probe now compares eight visibility segments with the CPU reference,
including an origin on the triangle, a blocker within the origin bias, a blocker
beyond that bias, and blockers within/beyond the light-end bias. Expected results
are [0,1,1,1,1,0,1,0]. A second job reverses all eight segments and shares the
compiled pipeline; both independent outputs matched. Metal/M4 Max passed these
checks, the seven TLAS phases and the 4096-pixel shadow image comparison. Strict
probe Clippy passed. This tests the chosen fixtures rather than establishing
universal precision equivalence for triangle boundaries or degenerate geometry.

## Required RTX 5090 feature scope (2026-10-01)

The user additionally requires Frame Generation and other RTX 5090 capabilities.
Completion includes DLSS Frame Generation, Multi Frame Generation and Dynamic
Multi Frame Generation, DLSS Super Resolution/DLAA, Ray Reconstruction, Reflex,
and the existing DLSS 5 neural-rendering, ray-tracing and CUDA requirements.
Native integration must query supported features and generation limits rather
than infer them from the GPU name. Unsupported platforms keep normal rendering.

`FrameGenerationMode` defines off, fixed generated-frame count and dynamic target
rate policies. Validation checks live capability reports, Reflex activation, SDK
frame-count limits and finite positive explicit target rates. No SDK is loaded by
this policy module; no frames are generated. Defaults report unavailable.

The Streamline 2.14.1 DLSS-G guide requires motion vectors, depth, HUD-less color,
per-frame constants and input-resource lifetime through processing completion.
The native integration still needs proxy swap-chain/present ownership, resource
tagging, completion fences, Reflex markers, reset handling and UI composition.
Generated presentations must be measured separately from simulation/render frames.
Current temporal inputs alone do not constitute Frame Generation integration.

Sources: [DLSS-G guide](https://github.com/NVIDIA-RTX/Streamline/blob/main/docs/ProgrammingGuideDLSS_G.md)
and [DLSS-G options/state](https://github.com/NVIDIA-RTX/Streamline/blob/main/include/sl_dlss_g.h).
RTX 5090 runtime verification remains pending; the available Metal device cannot
validate NVIDIA DLL execution, generated-frame pacing or latency.

## Temporal history reset metadata

`TemporalFrame::reset_history` marks the first presented temporal input set after
enabling motion, resize/suspension, outdated-surface recovery or explicit
`SceneSurface::invalidate_temporal_history`. Invalidation hides old input access;
skipped frames do not consume a pending reset. Camera-cut callers must also reset
their object motion histories. Subsequent presented frames clear the flag.

The native scene demo checks reset metadata against prior input availability on
every presented frame. Metal/M4 Max passed 120 frames including smoke resize and
suspend/resume paths. All 36 renderer unit tests passed; renderer/app all-target
strict Clippy passed after correcting explicit default calls in FG tests. This
metadata is preparation for temporal SDK constants, not native FG execution.

## Acquisition failure and object-history coherence

Lost/validation surface acquisition now invalidates public temporal inputs before
returning. Timeout/occlusion retain the last successfully presented inputs. The
scene demo resets object motion history whenever enabled temporal input access is
invalid, covering outdated-surface recovery in addition to resize. This aligns
the next reset-marked frame with zero previous-object motion. Renderer unit tests
and renderer/app strict all-target Clippy passed. Loss/validation branches were
reviewed but not triggered through a native driver fault injection.

## Temporal consumer before present

`SceneSurface::render_scene_with_temporal` exposes aligned color, depth, motion,
reset metadata and the candidate frame ID after scene-pass encoding and before
queue submission/present. Consumers can append GPU work to that same encoder.
Callbacks run only with enabled inputs and successful acquisition. Existing
`render_scene` delegates through a no-op callback. Native Streamline integration
still needs SDK loading, native resource access/transitions, proxy presentation
and fences; this hook does not generate frames. Its borrowed resources cannot be
retained as Rust references beyond the callback. GPU writes are still pending.

The scene demo compares callback ID/reset metadata with published inputs after
present. Renderer/app all-target strict Clippy passed.

## Cross-backend temporal-hook verification

Current scene-demo binaries passed 120 presented frames with motion inputs on
Linux Mesa llvmpipe Vulkan and OpenGL, including resize/suspension and comparison
of pre-present callback IDs/reset flags with published temporal inputs. Runs used
the existing `voxy-linux-smoke` image with Xvfb and Docker `--init`. This is CPU
software rendering through real Vulkan/GL APIs, not NVIDIA RTX execution.
`cargo check -p voxy_web --target wasm32-unknown-unknown --locked --offline` passed;
that check is compilation evidence only, not a new browser runtime test. Linux
build still reports the newer-toolchain atomic fetch_update deprecation in scene
ownership code; the project's pinned host toolchain remains unchanged.

## Native Streamline FG option adapter

`native/streamline` now contains a C++ adapter compiled against official Streamline
2.14.1 headers at commit 2122257e0fce486f91b385aa63b9a09b0a34b363. It uses actual
SDK state/options types, live generated-frame limits and dynamic availability,
Reflex gating and preserved SDK error results. Function pointers must be resolved
by a future initialized native integration and called on the present thread.

CMake host tests with fake SDK callbacks passed. MinGW cross-compiled and linked
the Windows x64 test executable; Windows execution remains unverified. Git LFS
was unavailable during SDK checkout; headers were extracted from its checked-out
commit via git archive. NVIDIA binary plugins were not fetched or executed. The
adapter is not yet wired to Rust or presentation and does not implement DLSS 5.

## Streamline feature function resolution

The C++ adapter resolves both FG functions using the official
`PFun_slGetFeatureFunction` interface and `kFeatureDLSS_G`. Output remains empty
unless both SDK results succeed and both pointers are non-null, preventing use
of partial or stale tables. Tests check exact feature/function names, successful
resolution, second-function SDK failure/error retention, null results and absent
resolver. Host CTest passed; Windows x64 test executable cross-link passed.
Loading/initializing the actual NVIDIA library and device remains outstanding.

## Native Reflex/PCL adapter

Official Streamline Reflex/PCL types now back native resolution of state,
options, sleep and latency-marker functions. Options validate modes and forward
frame-limit microseconds while retaining SDK errors. Sleep/marker wrappers accept
SDK-owned FrameToken references; counters are not substituted for tokens.
Host tests passed for option forwarding and invalid/missing API rejection.
Windows x64 cross-link passed. SDK initialization, token acquisition and actual
engine marker placement remain pending; no NVIDIA latency measurement is claimed.

## Reflex availability before enabling

Native Reflex configuration now queries `ReflexState::lowLatencyAvailable` before
enabling Low Latency/Boost. Missing state functions, unavailable low latency and
SDK state errors reject requests without changing options. Off remains usable
without that capability query. Tests verify unavailable/error paths retain the
previous settings; host CTest and Windows x64 cross-link passed. The SDK reports
availability, not a current-active flag: accepted options must be tracked by the
future integration, with actual sleep/markers and FG status checked separately.
No runtime activation or latency reduction is inferred from this test.

## SDK frame-token acquisition

The native adapter acquires tokens through official `PFun_slGetNewFrameToken`,
forwarding an optional uint32 frame index. Tokens remain SDK-owned; the engine
does not reinterpret counters as token objects. Null/error results clear output
so stale tokens cannot survive failed acquisition. Fake-SDK tests cover explicit
and automatic indexing, pointer identity, error preservation and null functions/
results. Host CTest and Windows x64 cross-link passed. Actual token acquisition
and lifetime behavior require the initialized NVIDIA runtime, still pending.

## Reflex rendered-frame marker sequence

`ReflexFrame` borrows one SDK token and requires sleep, simulation start/end,
render-submit start/end and present start/end in order. Repeated/out-of-order
calls reject before invoking SDK callbacks. SDK failures retain the current stage
for explicit caller recovery; completion requires a successful present-end marker.
Methods must surround real engine work; the helper does not submit or present.
It covers the ordinary rendered-frame sequence, not out-of-band/late-warp markers.
Fake SDK tests check token identity, ordering, duplicate rejection, SDK failure
retention and completion. Host CTest and Windows x64 cross-link passed. Real
engine-loop hookup and NVIDIA timing remain pending.

## Native SDK session lifecycle

A non-copyable C++ Session now calls official init/shutdown functions, forwards
the SDK version, and registers a D3D device only after initialization. Repeated
initialization/registration and null devices reject before SDK invocation.
Shutdown clears state on success; explicit close exposes errors, while the
destructor makes a cleanup attempt. The caller must retain the library and
serialize global SDK ownership. Fake SDK lifecycle tests and host CTest passed;
Windows x64 cross-link passed. DLL loading, Vulkan registration, Rust FFI and
actual graphics interposition remain pending.

## SDK session failure-path verification

Fake-SDK tests now verify init failure does not cause shutdown, failed D3D
registration remains retryable, failed shutdown preserves initialized/device
state and the original error, successful retry clears state, and repeated close
or destruction after close does not call shutdown twice. Host CTest and Windows
x64 cross-link passed. These tests cover adapter control flow, not actual NVIDIA
shutdown/resource behavior. Loaded-module ownership is still outstanding.

## Native Vulkan SDK registration

Optional CMake Vulkan support now registers official `sl::VulkanInfo` through
`PFun_slSetVulkanInfo`. It rejects incomplete handles, pre-init calls and duplicate
D3D/Vulkan registration; failures remain retryable. Queue metadata is forwarded
unchanged and must describe real caller-created SDK queues. Official Khronos
headers and Streamline helpers compile together. Fake-device tests verify handle
validation, metadata forwarding, SDK failure retention and cross-API duplicate
rejection. Host CTest and Windows x64 Vulkan cross-link passed. Native resources,
required device extensions/queues and Vulkan present hooks remain pending.

## Native session graphics API consistency

Session now records the render API accepted by initialization and rejects D3D
registration for Vulkan sessions and Vulkan registration for D3D sessions before
calling SDK device functions. Successful shutdown clears the API selection.
Fake-device tests cover both mismatches with non-null handles. Host Vulkan-enabled
CTest and Windows x64 cross-link passed. Actual NVIDIA registration remains
unverified; this is a corrected native-adapter invariant.

## SDK support discovery for selected adapters

Session now queries official `slIsFeatureSupported` for a caller-selected feature
and adapter. D3D requires its eight-byte LUID; Vulkan requires the physical-device
handle. Calls require initialized SDK and preserve the SDK's unsupported/error
reason. Fake SDK tests cover support, unsupported adapter, pre-init and malformed
LUID rejection. Host CTest and Windows x64 cross-link passed. The caller still
must obtain identifiers from the actual rendering adapter and load the SDK.
No RTX capability is inferred from model names or these mocked tests.

## Windows interposer module loading

WindowsModule now verifies an absolute DLL path with the official NVIDIA
`sl::security::verifyEmbeddedSignature`, uses LoadLibraryExW with restricted DLL
and system dependency directories, and resolves required core exports atomically.
Failure leaves public tables empty and releases a partially loaded module. The
module owns its handle and must outlive Session/tokens/feature tables; explicit
shutdown failures must be handled before module destruction. Windows x64 with
Vulkan enabled compiled and linked under strict warnings. No signed NVIDIA DLL
was executed here, so signature/load/runtime behavior remains unverified.
Rust FFI, native resources and SDK presentation still remain pending.

## Windows runtime module/session ownership

WindowsRuntime now owns DLL loader plus SDK Session, closes SDK before module
destruction, and retains the DLL OS reference for process lifetime when cleanup
fails. Feature resolver and token-acquisition functions remain inaccessible until
device registration. Borrowed tables/tokens still require runtime lifetime and
GPU shutdown synchronization. Windows x64 Vulkan-enabled strict compilation/link
passed. Windows execution and real failed-shutdown behavior remain unverified.
Rust integration and actual FG/DLSS presentation remain pending.

## Reproducible Windows native CMake build

The checked-in MinGW x64 CMake toolchain builds the complete native library and
test EXE with Windows loader/runtime linked, Vulkan enabled and strict warnings.
Windows-only tests cover pre-load availability and invalid DLL paths, but were
only compiled here, not executed. Host CTest passed after the same source change.
This provides repeatable build evidence without implying NVIDIA runtime support.

## Streamline per-frame constants submission

The loader now requires/resolves official `slSetConstants`. Session forwards
official Constants, SDK token and viewport only after initialization/device
registration, preserving SDK error codes. Fake SDK tests verify reset metadata,
token identity, viewport and pre-registration rejection. Host CTest and full
Windows CMake strict build passed. Constants still need conversion from actual
engine camera/depth/motion conventions; this typed submission API does not prove
that real DLSS inputs or matrices have been populated correctly.

## Frame-based Streamline resource tags

Session now submits official ResourceTag arrays through slSetTagForFrame with
SDK token/viewport. It requires frame-based tagging preference plus registered
device and checks command-buffer presence for non-present-lifetime resources.
The loader resolves this export. Fake SDK tests cover forwarding, null tag arrays
and volatile-resource rejection without a command buffer. Host CTest and Windows
CMake strict build passed. Native resource conversion, barriers and GPU lifetime
fences still require implementation; no actual GPU tags were submitted to NVIDIA.

## Native feature evaluation submission

Loader resolves official slEvaluateFeature; Session forwards a selected feature,
SDK token, typed BaseStructure input array and native command buffer after device
registration. Empty/null inputs and absent command buffers reject locally; SDK
validation remains authoritative for actual resources/feature configuration.
Fake SDK tests verify viewport-input type, token/feature forwarding, malformed
input rejection and NGX error preservation. Host CTest and Windows strict CMake
build passed. Actual SR/RR evaluation still needs option configuration, native
GPU tags, matrices and output composition. FG runs through SDK presentation,
not this generic evaluate wrapper; DLSS 5 remains separately unimplemented.

## Native DLSS Super Resolution/DLAA settings

The adapter resolves official DLSS optimal-settings and set-options functions,
validates output dimensions, exposure values and HDR selection, and forwards
SDK options without reproducing its versioned layout. Optimal-settings output
clears on failure. Fake SDK tests check DLAA options/viewport, recommended size
and nonfinite exposure rejection. Host CTest and Windows strict CMake build
passed. SDK support discovery remains necessary for each selected mode; engine
render resolution/output composition and real SR/DLAA execution are still pending.
These settings are separate from DLSS 5 neural rendering.

## DLSS recommended-size validation

Optimal DLSS settings are now published only when both recommended dimensions
fall within nonzero SDK minimum/maximum ranges. Malformed success responses
return invalid_sdk_output and leave recommendations cleared. Fake SDK tests cover
zero minima, maxima below the recommendation and zero recommended dimensions.
Host CTest and Windows strict CMake build passed. Caller device/allocation limits
must also be checked when real renderer resizing is integrated.

## Opaque C ABI for Windows SDK loading

The native library exposes a C header with opaque runtime ownership, counted
UTF-16 paths and structured loader status. Loading clears output on failure,
rejects null/empty/embedded-NUL/oversized paths and catches C++ exceptions before
returning across FFI. Destroy accepts null. Windows-only ABI rejection cases
compile/link in the strict CMake build but have not run on Windows. Host CTest
passed. This is the loading boundary only: Rust bindings, SDK initialization
and renderer resource interop still require implementation.

## Rust Streamline runtime loader binding

New optional voxy_streamline crate builds the C++ adapter only for Windows with
its native feature and VOXY_STREAMLINE_SDK. Its thread-bound Rust runtime owns
the opaque C handle and destroys it once on Drop. Counted UTF-16 paths and native
loader errors cross the C ABI; SDK C++ layouts stay outside Rust. Unsupported
platform/default-feature builds return Unsupported without requiring SDK headers.
Host check/strict Clippy and Windows-native strict all-target Clippy passed.
Windows load_sdk example linked through Cargo including C++/SDK verifier code.
It has not run on Windows; it loads only the DLL, without SDK initialization or
rendering. Frame-generation/renderer hookup remains pending.

## Rust DX12 SDK initialization and shutdown

Rust StreamlineRuntime now exposes initialize_dx12 with explicit SR/DLAA, FG,
Reflex and RR feature selection plus retryable explicit close. FG automatically
requests Reflex/PCL; the native handle retains the feature list throughout SDK
lifetime. Preferences use custom engine version and frame-based resource tagging.
C ABI preserves SDK/adaptor errors and catches exceptions. Host/Windows-native
strict all-target Clippy and linked Cargo loader example passed. Actual init
requires signed DLL/plugins and Windows/NVIDIA execution, not yet performed.
Native device registration and renderer integration remain outstanding.

## Rust native DX12 device registration

Rust now exposes unsafe register_dx12_device through the opaque C ABI. Native
session checks initialization/backend/duplicate registration and preserves SDK
results. The device is borrowed: caller must retain its COM owner until successful
SDK shutdown, including retry/failed-shutdown paths. This is explicitly documented
in the unsafe contract; the wrapper does not invent a device from a wgpu handle.
Host/Windows-native strict Clippy and linked Cargo Windows example passed.
Extracting the actual renderer device and graphics interposition remain pending.

## wgpu DX12 native device bridge

Optional voxy_streamline wgpu-dx12 feature obtains the actual wgpu 30 HAL DX12
device under a borrowed guard and retains a cloned ID3D12Device COM reference
after successful SDK registration. Other backend devices reject. Explicit close
releases ownership only after successful SDK shutdown; failed cleanup conservatively
retains the COM reference for process lifetime. The unsafe caller contract still
requires SDK initialization before device creation and coordinated GPU lifetime.
Windows-native strict all-target Clippy and linked Cargo example passed; default
host strict Clippy passed. No real Windows device was registered here. Renderer
startup/presentation interposition and native resources remain pending.

## Rust feature support on a DXGI adapter

Rust runtime now queries SR/DLAA, FG, Reflex and RR support using an explicit
DXGI eight-byte LUID and typed feature selector. The C boundary copies LUID data
into local SDK AdapterInfo storage, preserving SDK unsupported/driver/OS results
without exposing mutable SDK pointers to Rust. Host/Windows-native strict Clippy
and linked Cargo Windows example passed. The selected LUID must still come from
the actual renderer adapter; no NVIDIA support query was executed on this host.

## wgpu adapter identity for SDK discovery

The wgpu-dx12 bridge now queries IDXGIAdapter GetDesc1 from the actual wgpu HAL
adapter and passes its eight-byte LUID to SDK feature-support discovery. It
rejects other backends and preserves DXGI HRESULT failures. Read-only queries
hold the HAL guard and do not guess identity from vendor/model strings.
Host/Windows-native strict Clippy and linked Windows Cargo example passed. Real
adapter discovery/SDK support execution on Windows remains unverified.

## Unified Windows SDK/DX12 startup probe

The dx12_probe Cargo example loads the signed interposer, initializes selected
plugins before graphics creation, selects a strict wgpu DX12 adapter, reports SDK
feature support for its actual LUID, registers the device and closes the SDK.
Windows-native strict all-target Clippy and linked executable passed; host strict
Clippy passed. The EXE has not been executed on Windows. Feature-support failures
are reported individually; the final pass message covers startup/registration/
shutdown only and explicitly excludes rendered DLSS/FG output.

## Rust DLSS quality configuration

Rust now exposes typed DLSS quality selection and a scalar render-size result
through C ABI. Native code resolves SDK functions after device registration,
queries/validates optimal settings and applies viewport options before publishing
recommended dimensions. SDK C++ layouts remain native; Rust receives only its
own repr(C) size pair. Host/Windows-native strict Clippy and linked DX12 probe
passed. Actual renderer resizing, evaluation, output composition and NVIDIA SDK
execution remain pending.

## Rust Reflex configuration

Rust now exposes off/low-latency/boost options with a microsecond frame limiter.
The C handle resolves official Reflex/PCL functions only after device registration,
checks low-latency availability and tracks enabled options only after SDK success.
Successful shutdown clears that tracked state; failed configuration preserves it.
Host/Windows-native strict Clippy and linked DX12 probe passed. SDK option acceptance
is not proof of sleep/marker execution or measured latency; those per-frame calls
still need connection to Rust/renderer.

## Rust Frame Generation configuration

Rust now configures off/fixed/dynamic FG through the native SDK adapter. Fixed
counts use live SDK limits; dynamic explicit targets require finite positive FPS,
while None selects display rate. Enablement uses retained SDK-accepted Reflex
configuration, rather than a caller-supplied boolean. Reflex cannot be disabled
through this API while accepted FG options remain enabled. State changes publish
only on SDK success and successful shutdown clears both.
Host/Windows-native strict Clippy and linked DX12 probe passed. No actual FG
options were submitted on NVIDIA here; per-frame inputs, synchronization, proxy
presentation and RTX runtime execution are still required.

## Rust SDK frame ownership and Reflex calls

Rust begin_frame acquires an official SDK token and returns StreamlineFrame with
an exclusive runtime lifetime borrow. While a frame exists, safe Rust cannot
close/reconfigure the runtime or acquire another token. Explicit sleep and typed
markers use the native ReflexFrame sequence; Drop frees only the adapter frame
object, not the SDK-owned token. Missing/inactive plugins return native errors.
Host/Windows-native strict Clippy and linked Windows DX12 probe passed.
Actual render-loop placement, constants/tags on this token and NVIDIA sleep/marker
execution remain pending; no generated frames or latency benefit are claimed.

## Rust frame camera constants boundary

StreamlineFrame now submits an engine-owned repr(C) camera payload on its SDK
token. Native code fills official Constants field by field, validates finite
arrays/perspective planes/FOV/aspect and carries reset metadata. This path
explicitly assumes unjittered SDK row-major matrices, normal depth and backward
2D motion including camera motion; callers must supply matching texture scales.
Rust layout tests and native static assertions agree on size/critical offsets.
Host tests/CTest, strict host/Windows Clippy and linked Windows Cargo probe passed.
Numeric rejection branches are compiled but not yet runtime-tested through this
Windows C API. Actual engine matrix conversion, tagged resources and SDK rendering
remain pending; orthographic/reversed-depth camera input needs a separate extension.

## glam camera conversion to Streamline matrices

CameraConstants::from_perspective converts unjittered glam column-vector camera
transforms into SDK row-vector/row-major storage. Current-to-previous clip mapping
uses previousVP * inverse(currentVP) before conversion, with the inverse mapping
provided separately. Cuts/missing history produce identity reprojection and reset.
Finite/invertible transforms, affine view rows, basis normalization and perspective
metadata are validated. Tests apply SDK row-vector multiplication to a moving-camera
fixture and verify projection/reprojection in both directions, position, reset
and invalid view rejection. Both Rust tests and host/Windows-native strict Clippy
passed. Actual tagged-texture scales and SDK GPU evaluation remain unverified.

## Camera-history and basis validation

Perspective conversion now rejects nonfinite supplied previous VP even during
reset, plus scaled/nonorthogonal/mirrored view bases. It accepts rigid RH camera
views within a 1e-4 basis tolerance, matching the camera axes and projection
metadata convention rather than silently normalizing arbitrary object transforms.
Tests cover NaN previous history during a cut, scaled/mirrored/singular views
and the existing moving-camera reprojection. Rust tests and host/Windows-native
strict Clippy passed. GPU SDK evaluation remains pending.

## Resident compute and ballistic physics (2026-10-01)

`ComputeJob::encode_step` records ordered steps on the same GPU storage without
host readback. A job may span submissions; `encode` consumes it, performs one last
step and copies results for the existing nonblocking readback. This supplies the
storage lifecycle for iterative physics without allocating one job per timestep.

`BALLISTIC_SHADER` integrates independent position/velocity pairs under constant
acceleration with a fixed f32 timestep. Its storage ABI is an acceleration/dt
vec4 followed by position/velocity vec4 pairs. Callers must supply finite data and
positive finite dt. This is a ballistic building block; collisions, mutual gravity,
liquids, soft bodies and authoritative runtime selection are still outstanding.
`cargo run -p voxy_render --example ballistic_smoke` passed on Apple M4 Max Metal:
257 bodies, 49 steps across four submissions, compared with f64 CPU trajectories
at absolute tolerance 0.001. Padding lanes and parameter data remained unchanged.

CUDA now reports live device identity, compute capability and total memory through
`CudaCompute::capabilities`. `upload_u32` creates reusable private device storage;
`CudaU32Buffer::affine` enqueues a kernel and `read` synchronizes explicit host
readback. The kernel/module is cached per compute owner; failed initialization
is not cached. Resident allocations retain stream/context/module ownership after
the compute owner is dropped. The probe verifies repeated/chained kernels,
repeated readback and that ownership boundary on an NVIDIA machine.

Host default tests and CUDA-feature strict Clippy pass. This host's CUDA probe
returns `DriverUnavailable`, so NVIDIA execution remains unverified. Existing
custom scene shader replacement and portable compute validation/readback smoke
checks also passed on Metal during this increment. World-generation GPU/CUDA
kernels and complete physics integration remain required by the active goal.

## Exact GPU/CUDA procedural world generation (2026-10-01)

The new `voxy_gpu` workload crate supplies native `GpuTerrainGenerator` and
`CudaTerrainGenerator` through the existing `ChunkGenerator` contract. Both keep
the CPU v1 algorithm/descriptor; no world format or generator identity changes.
`voxy_runtime::build_generated_scene` injects the chosen generator without making
the runtime depend on graphics/CUDA. Desktop `--gpu-terrain` now runs generation,
lighting, halo meshing and presentation; `--cuda-terrain` selects CUDA ordinal 0
and requires Cargo feature `cuda`. Conflicting flags reject and errors do not
silently select CPU. See `docs/procedural-terrain.md` for contracts and commands.

Verified: 502 Metal-generated chunks, 16,449,536 exact CPU block comparisons;
integrated nine-chunk mesh/light parity; actual desktop first-frame presentation.
The fixed CUDA source's host arithmetic matched 180 chunks/5,898,240 blocks.
CUDA-feature host strict Clippy and Windows/Linux cross-target checks passed.
Driver/NVRTC/device execution on NVIDIA remains required; the local runtime probe
returned `DriverUnavailable`. GPU/CUDA world generation code is now integrated,
while CUDA hardware acceptance, browser compute integration and the remaining
physical solvers are still outstanding parts of the active goal.

## Browser compute and shared terrain jobs (2026-10-01)

`TerrainProgram`/`TerrainJob`/`PendingTerrain` now provide a nonblocking portable
terrain lifecycle. Native worker generation uses it too. `voxy_gpu` builds for
wasm without its native synchronous `ChunkGenerator` adapter. The browser renderer
requests WebGPU compute limits, while preserving WebGL2 graphics limits.
`WebEngine.generate_terrain` exposes Promise-based generation with full-width
BigInt coordinates/seeds. Browser rendering of generated voxel chunks still needs
integration; this increment supplies generation and checked asynchronous readback.

Verified in the actual browser: WebGPU compute matched 98304 CPU blocks and
rendering continued; WebGL2 explicitly rejected compute and continued rendering.
Metal async smoke matched 65536 blocks and checked independent jobs, cancellation
before upload/encoding/publication and one-shot results. Native/CUDA-feature and
WASM strict Clippy passed. Remaining physical solvers and NVIDIA hardware
acceptance remain required by the active goal.

## Resident mutual gravity and velocity-Verlet (2026-10-01)

`voxy_gpu::GravityProgram` now supplies an explicitly selected f32 Newtonian
N-body solver with Plummer softening and uniform acceleration. Three ordered
WGSL passes predict from immutable input, correct using predicted positions and
commit. An atomic sticky error flag prevents any committed-state update when a
pass fails. Resident jobs span submissions and steps; host readback is explicit
and one-shot. The CPU f64 solver retains its existing API and precision.
`ComputeDispatch::copy_buffer` supplies checked range readback for this and future
engine workloads. Invalid ranges and budgets reject before command recording.

Verified on real Metal: 257 bodies/128 resident steps, maximum f64 CPU trajectory
difference 2.59126064e-5 at tolerance 2e-4; 2560 orbit steps with relative energy
drift 8.92683429e-7, trajectory tolerance 1e-3 and momentum tolerance 1e-6.
Singular predicted positions and numerical overflow left committed inputs
bit-identical, including a subsequent failed step. Native/CUDA-feature and WASM
strict Clippy passed. This does not establish performance, CUDA gravity execution,
contact/deformable/liquid acceleration or game-loop integration; those remain
parts of the active goal. See `docs/physics.md` for the API and limits.

### Explicit linear HDR presentation

`SurfaceOutput::HdrLinear` configures `SceneSurface` with `Rgba16Float` and
`SurfaceColorSpace::ExtendedSrgbLinear`. Use `new_with_instance_and_output` for
platform display instances or `new_with_adapter_and_output` for an explicitly
selected native-interop adapter. The capability check uses the format/color-space
pair in `format_capabilities`, including explicit-opt-in formats omitted from the
legacy default format list. Unsupported pairs return `UnsupportedSurface`; the
constructors do not silently choose SDR. Existing constructors retain SDR.

Create render pipelines using the surface's `color_format`; write linear BT.709
extended-range color directly, without SDR tone mapping or sRGB encoding.
`color_space` reports the configured compositor encoding, while
`display_hdr_info` queries live platform luminance/headroom and may contain
unknown fields. Query again after monitor, brightness or HDR-mode changes.

`cargo run -p voxy_render --example hdr_surface` passed on Metal: RGBA16/scRGB
configuration, three submitted/presented frames cleared to linear RGB (4,2,0.5),
and suspension/resume retaining that encoding. The current display reported
headroom 1.0, potential 16.0 and coarse HDR=false. This proves swapchain setup and
submission, not visible HDR brightness or photometric accuracy. Capability tests
also verify unsupported-pair rejection without config mutation. Strict Clippy,
Windows GNU compilation and the existing SDR temporal-surface regression passed.
HLG encoding, HDR metadata and actual Windows/Linux HDR-display execution
remain pending. The PQ encoder and format selection are described below.

### Adaptive native HDR composition

`TextureBlit::linear_exposed` applies exposure directly to linear float color for
scRGB, without an SDR curve or transfer encoding. Negative RGB clamps to zero;
positive overflow saturates to 65504 for RGBA16 or f32 maximum for RGBA32. Alpha
is preserved. `with_auto_exposure` now supports this pass and the PQ encoder in
addition to SDR tone mapping. Fixed display parameters and GPU exposure use
separate uniform buffers: attaching exposure to PQ never replaces its absolute
SDR-white luminance. Parameter snapshots retain the attached GPU exposure.

`VOXY_TEMPORAL_AUTO_EXPOSURE=1 VOXY_TEMPORAL_OUTPUT=linear|pq` enables the
corresponding adaptive window path. Both actual Metal presentations passed the
consumer-failure and resize/reset lifecycle test. The GPU pixel probe separately
checks input 0.25 at computed exposure 0.72: scRGB output is 0.18, and PQ output
matches 36.54 nits with fixed 203-nit white. SDR, scRGB and PQ pixel paths plus
existing HDR regressions passed on Linux Vulkan/OpenGL Mesa llvmpipe CPU software
rendering. Strict host Clippy and Windows GNU compilation passed. This does not
prove physical HDR brightness,
display-peak adaptation or mastering-metadata submission.

### Windowed automatic-exposure presentation lifecycle

`VOXY_TEMPORAL_AUTO_EXPOSURE=1 cargo run -p voxy_render --example temporal_surface_smoke`
now meters the window's HDR temporal color and uses the resulting uniform in
its SDR display submission. The candidate is retained as history only after
`RenderOutcome::Presented`. The injected consumer failure happens after the
exposure/display commands are encoded but before they are submitted; the
candidate is discarded while the prior exposure presentation ID stays unchanged.
The following reset frame ignores that prior history, and resize also resets
through the temporal input flag. No GPU exposure readback is used in this path.

The windowed auto-exposure test passed on Metal and Linux Vulkan/OpenGL Mesa
llvmpipe CPU software rendering. `tools/linux/auto-exposure-surface-smoke.sh`
reproduces both Linux backends. Default operation without auto exposure also
passed on Metal. The example now requests focus before its first redraw to avoid
exhausting acquisition retries while its newly created window is occluded.
Strict host Clippy passed. The same option now supports linear scRGB and PQ
outputs as described above.

### GPU automatic exposure

`AutoExposure` meters finite linear BT.709 RGBA16/RGBA32 textures in 8x8 tiles,
then reduces tile log-luminance sums/counts in a 64-lane compute pass. Negative
RGB clamps to zero; zero-luminance and nonfinite RGB pixels are excluded. Alpha
is ignored, including nonfinite alpha. Positive luminance is bounded to
1e-6..1e6 before log averaging. All-black/invalid frames retain prior exposure,
or one without history, clamped to the configured range.

`ExposureSettings` controls middle gray, exposure limits and separate exponential
adaptation rates for increasing/decreasing exposure. The lower bound must be at
least the smallest positive normal f32. `prepare` validates settings, time,
textures and capacity, producing an independent candidate. Pass the last
successfully presented `AutoExposureFrame` as history; skipped candidates do not
commit automatically. Reset by passing `None`. Encode after the HDR producer and
before the display pass. `TextureBlit::with_auto_exposure` binds the computed
16-byte GPU uniform directly for SDR tone mapping; production requires no CPU
readback. Linear scRGB and PQ display passes also accept this GPU exposure,
retaining their own fixed display parameters.

`cargo run -p voxy_render --example auto_exposure` passed on Metal and Linux
Vulkan/OpenGL Mesa llvmpipe CPU software rendering. A 17x9 fixture covers partial
tiles, colored luminance, nonfinite RGB, ignored nonfinite alpha, black exclusion,
skipped candidates, two adaptation directions, zero time and reset. f64 reference
exposure values and final GPU tone-map pixels match. The Linux HDR script also
passed existing range, PQ, SDR/sRGB, depth and guide regressions. No performance
benchmark or percentile/histogram metering is claimed. Strict host Clippy and
Windows GNU compilation passed.

### Display parameter snapshots without pipeline recompilation

`TextureBlit::with_exposure(device, value)` returns an SDR tone-map pass with
an independent exposure uniform while sharing the existing render pipeline and
bind-group layout. `with_sdr_white_nits` does the same for a PQ encoder's absolute
white level. Invalid values and incompatible pass types return `None` before
allocation. The device must match the original pipeline. Plain color and depth
passes do not accept exposure snapshots.

Each snapshot owns an immutable 16-byte uniform buffer. Multiple passes can be
encoded before a single queue submission without later parameter changes
rewriting earlier passes. `hdr_range` now verifies three exposures in one
submission; `hdr10` verifies two absolute-white levels in one submission, for
both float and RGB10A2 output. Both passed on Metal and Linux Vulkan/OpenGL
Mesa llvmpipe CPU software rendering; the existing HDR/SDR blit regressions and
strict host Clippy passed. This removes parameter-driven
pipeline creation from the snapshot API; buffers and bind groups still allocate,
and no FPS/latency improvement is claimed. GPU automatic exposure is described above.

### Linear temporal color with HDR presentation

`SceneSurface::enable_motion_vectors` now automatically captures world color in
linear RGBA16Float whenever the configured presentation color space is HDR.
A separate standard color renderer writes that texture; it never copies PQ
encoded output or clips linear values into an RGB10A2 temporal target. SDR
capture behavior is unchanged. Explicit HDR capture remains available for SDR
surfaces. Final composition selects SDR tone mapping, direct scRGB blit or PQ
encoding independently of the temporal inputs.

`VOXY_TEMPORAL_OUTPUT=pq VOXY_TEMPORAL_RG_MOTION=1 cargo run -p voxy_render --example temporal_surface_smoke` passed on Metal without explicit HDR-capture
enablement. Pixel readback preserves linear RGB (4,1,0), with RG16 backward motion
and opaque depth. Linear scRGB output, HDR10 with a transparent moving world
layer, and default SDR also passed. The transparent fixture preserves linear RGB
(2,1,0); it does not establish transparent motion or reactive-mask support.
Consumer failure and resize recovery retain the existing reset/commit behavior.
Use `VOXY_TEMPORAL_OUTPUT=sdr|linear|pq` to select the presentation encoding.
Strict host Clippy and the Windows GNU `voxy_streamline/scene-dx12` build
against Streamline 2.14.1 passed. These tests exercise temporal production and
presentation, not NVIDIA inference.

### Native Metal PQ presentation and dependency fix

`VOXY_HDR_OUTPUT=pq cargo run -p voxy_render --example hdr_surface` now uses a
linear RGBA16 offscreen buffer, clears RGB to (4,2,0.5), encodes BT.2020/PQ at
203-nit white and presents on an RGB10A2/PQ surface. Three presentations plus
suspension/resume passed on Metal. The same example defaults to linear scRGB;
`VOXY_HDR_OUTPUT=linear` selects it explicitly. Actual display headroom remains
1.0, so neither test proves physical HDR brightness.

Initial PQ configuration crashed with SIGBUS in CGColorSpaceCreateWithName.
The wgpu-hal 30.0.1 CoreGraphics loader treated an exported CFStringRef variable's
storage address as the CFString object. The vendored Cargo patch dereferences
that storage before constructing the reference and rejects a null object.
Original licenses are retained; patch scope and removal conditions are recorded
in `vendor/wgpu-hal/VOXY-PATCH.md`. A fresh PQ probe passed after this correction.
No renderer-level unsafe code was introduced. wgpu's public API still does not
submit ST 2086/MaxCLL metadata; that remains native integration work.

### HDR10 PQ presentation encoding

`SurfaceOutput::Hdr10` selects `Rgb10a2Unorm` with `Bt2100Pq`, checking the
explicit surface format/color-space pair before configuration. Unsupported
pairs return `UnsupportedSurface`. `TextureBlit::hdr10(device, format,
sdr_white_nits)` converts finite, display-referred linear BT.709 input to BT.2020
and applies the inverse ST 2084 EOTF specified by
[ITU-R BT.2100](https://www.itu.int/rec/R-REC-BT.2100).
Input RGB 1 maps to the supplied absolute white luminance, which must be in
(0,10000] nits. Negative inputs clamp to zero; encoded luminance saturates at
10000 nits. Alpha is preserved subject to output-format quantization. No artistic
tone mapping, exposure selection, HDR metadata or display-peak adaptation is
performed. Render world color into a linear HDR offscreen target, then encode
once into the PQ surface using a submission hook or `render_custom`. Direct
linear scene rendering into a PQ swapchain is not a valid final composition.

`cargo run -p voxy_render --example hdr10` checks black, neutral
100/1000/10000-nit samples, colored primaries, clipping and alpha at white levels
100 and 203 nits. The f64 reference derives the gamut transform independently
from BT.709/BT.2020 xy primaries and D65. Metal passed both float and packed
RGB10A2 readback. Native float tolerance is 0.00005; packed 10-bit tolerance is
one code step (1/1023) plus that float allowance because Vulkan llvmpipe
quantized the tested value downward rather than to nearest. OpenGL float output
uses RGBA16 with tolerance 0.0005.
`tools/linux/hdr-smoke.sh` includes this proof alongside existing HDR/SDR checks;
all passed on Vulkan/OpenGL Mesa llvmpipe CPU software rendering. Surface
capability tests and Windows GNU compilation passed.
This establishes encoding and target writes. Metal compositor submission is
verified by the window probe above; physical HDR luminance and mastering metadata
remain unverified.

### HDR tone-map numerical range

`TextureBlit::tone_mapped` uses a piecewise Reinhard evaluation: `x/(1+x)`
for exposed values at most one, and `1/(1+1/x)` above one. The first branch avoids
subtraction cancellation near black; the second keeps the final denominator
small when GPU fast division flushes reciprocals of large numbers. Exposure
multiplication overflow is saturated before evaluation. Negative RGB clamps to
zero; alpha is unchanged. Inputs must still be finite linear color.

`cargo run -p voxy_render --example hdr_range` checks four RGB/alpha samples
against independently computed f64 expectations at exposures 1, f32 maximum and
1e-30. Samples include 8e-8, negative RGB, radiance above one and f32 maximum.
Metal uses RGBA32 float readback and passed, as did the existing linear/sRGB
blit, alpha composition, guide and depth checks. OpenGL uses RGBA16 output with
absolute tolerance 0.0005 because downlevel RGBA32 rendering is unavailable.
`tools/linux/hdr-smoke.sh` runs both range and existing blit probes on Linux
Vulkan/OpenGL; both passed on Mesa llvmpipe CPU software rendering. These checks
prove HDR-to-SDR processing, not HDR monitor output,
PQ/HLG encoding or display color-space negotiation.

### HDR temporal capture and SDR composition

`SceneSurface::enable_hdr_temporal_color` captures world color in linear
RGBA16Float alongside depth/motion inputs. The capture renderer uses the standard
scene shader; `hdr_temporal_renderer_mut` supports custom shader reload. Resize
preserves the float color format and invalidates temporal history. Presentation
remains SDR; use `TextureBlit::tone_mapped` in the submitted temporal hook.
`ProcessedColorTarget::new_with_srgb_view` requires VIEW_FORMATS support; ordinary
`new` preserves OpenGL compatibility. Separate sRGB render targets work without
additional view formats.

Metal M4 Max surface smoke renders a quad using ordinary scene resource bindings
and reads the HDR center pixel as binary16 (4,1,0,1), proving values above one
survive. Five-frame recovery/resize sequence passes. HDR tone-map/sRGB pixel
checks also pass on Linux Mesa llvmpipe Vulkan/OpenGL (CPU implementations).
These checks do not prove HDR display output, DX12 hardware, DLSS or FG execution.

Temporal surface presentation now draws world content before the submitted
processing hook and draws overlay/UI afterward with color/depth Load operations.
This avoids overwriting UI with processed output and prevents double alpha
blending when the submitted hook does no work. `SceneRenderer::encode_overlays`
also exposes this composition pass for offscreen consumers. Metal surface smoke
exercises HDR world geometry and a separate overlay draw through this ordering;
offscreen Metal readback now validates all 16 final pixels for linear and sRGB
composition: untouched red background and exactly one half-alpha green overlay
blend. This proves the load/blend pass; swapchain pixel capture remains separate.

Latest Linux validation ran both `blit_smoke` and HDR-enabled
`temporal_surface_smoke` independently with strict Vulkan and OpenGL selection.
All four runs pass on Mesa 25.0.7 llvmpipe: HDR geometry (4,1,0,1) readback,
five-frame consumer failure/resize recovery, linear/sRGB tone mapping, and all
16 final overlay pixels. Windows GNU cross-check compiles both examples; no
Windows graphics execution is claimed. Those Linux runs cover opaque-world geometry; equivalent transparent/X-ray
Linux execution still needs verification.

Temporal color now includes transparent geometry and X-ray layers, excluding UI.
X-ray color uses isolated depth; the motion pass continues to capture opaque
geometry only. Metal readback checks transparent green (alpha 0.5) over HDR
(4,1,0) as (2,1,0,1), and isolated X-ray blue as (0,0,4,1). Both scenarios pass
the full five-frame consumer failure/resize sequence. This does not implement
transparent-object motion or SDK reactive masks.

The Metal X-ray temporal smoke now places blue internals at depth 0.75 behind
opaque red geometry at depth 0.0. Readback remains (0,0,4,1), proving the isolated
color-depth path reveals otherwise occluded geometry; same-depth coincidence no
longer explains the result. Metal full-subresource depth/motion readback additionally verifies opaque
depth=0 and zero backward motion while both extra layers carry a previous-frame
X offset that would produce nonzero motion if incorrectly included. Numeric zero
comparison accepts IEEE +0/-0. Transparent object motion is still not captured.


## CUDA f64 gravity and compiler acceptance (2026-10-01)

`CudaCompute::create_gravity_job` now creates a resident f64 velocity-Verlet
N-body job. Predict, correct and commit are separate ordered CUDA launches;
sticky singularity/overflow flags prevent publication of invalid body state.
`step` performs no host transfer; `read` synchronizes and copies only the header
and committed bodies. Jobs retain stream, function/module and buffer ownership
after the factory is dropped. Defaults bound jobs to 4096 bodies and 256 steps
per call, with the factory's allocation byte budget enforced before upload.

The fixed CUDA source's native arithmetic harness matched CPU f64 exactly for
257 bodies over 128 steps, and passed orbit, softened coincidence, zero-G,
singularity and overflow rollback checks. This harness does not prove device
scheduling. Host tests, strict CUDA-feature Clippy and Windows/Linux target
checks passed. The NVIDIA runtime probe returned `DriverUnavailable` locally.

`sh tools/cuda/verify_compilation.sh` downloads SHA256-verified official NVIDIA
NVRTC/NVCC 12.6.85 Linux ARM64 wheels into ignored `target/cuda-tooling`. It uses
the existing `voxy-linux-smoke:latest` Docker image (build with
`docker build -t voxy-linux-smoke -f tools/linux/Dockerfile .` when absent).
Real NVRTC compiled terrain and all three gravity entrypoints; real PTXAS
assembled terrain, gravity and affine for sm_52, sm_75 and sm_89. Outputs are
in ignored `target/cuda-verified`; this compiler check requires no GPU driver.
Actual NVIDIA execution remains required. Run the hardware acceptance probe with
`cargo run -p voxy_cuda --features cuda --example gravity_probe` on that host.
Game-loop integration of gravity and remaining physics solvers is still pending.

## Resident gravity compute-to-render acceptance (2026-10-01)

`cargo run -p voxy_gpu --example gravity_render` binds `GravityJob::buffer()`
directly as read-only vertex storage. Each frame encodes physics before drawing
in the same command encoder; no body state is mapped or uploaded between phases.
The initial frame and two subsequent 32-step frames verify green pixels at the
expected body positions and black pixels at the cleared previous positions.
Metal on Apple M4 Max passed this check. Only final pixels are mapped for proof.
This establishes compute-to-vertex visibility and persistent-buffer rendering;
it does not yet connect the interactive SceneApp or prove CUDA/graphics interop.

Transparent and X-ray HDR scenarios now pass on Linux Mesa llvmpipe Vulkan and
OpenGL, including motion readback and five-frame resize/recovery. OpenGL cannot
copy depth to a buffer and Naga GLSL rejects raw depth load/sample paths.
`TextureBlit::depth` is a diagnostic grayscale pass using 24 nearest comparison
samples to approximate normalized depth. The portable smoke reads its RGBA8
output, distinguishing opaque depth 0 from X-ray depth 0.75; this is quantized
verification, not an exact float-depth readback. Direct float depth readback was
verified previously on Metal/Vulkan. Late redraw events after test completion
are ignored to avoid a teardown race in the window smoke.

Depth diagnostic additionally passes known depth clears 0, 0.25, 0.75 and 1.0
on Metal M4 Max and Linux llvmpipe Vulkan/OpenGL, checking every pixel of a 4x4
output within one RGBA8 quantization step. The source depth texture has no
COPY_SRC usage, proving the sampling path rather than an implicit direct copy.

## Interactive GPU gravity and custom surface frames (2026-10-01)

`cargo run -p voxy_app --example gpu_gravity` runs mutual gravity in the window
on resident storage, with the reusable `voxy_gpu::GravityView` reading committed
positions directly in its vertex shader. Space pauses, R recreates the initial
orbit, Escape exits. Real-time stepping uses a 240 Hz accumulator and bounds
catch-up to 24 steps per displayed frame. No per-frame body readback is performed.

`SceneSurface::render_custom` acquires before encoding, submits only successful
callbacks, and presents on the owning queue. Suspended/skipped frames do not
invoke the callback; custom frames invalidate scene temporal history. Surface
creation now requests compute-capable limits when the adapter reports compute,
while adapters without compute retain WebGL limits. The custom API does not
publish temporal motion inputs or provide CUDA/graphics interop.

The Metal smoke (`-- --smoke`) checks suspension without encoding, an injected
callback failure, 60 presented frames, resize, ten paused frames, reset and 200
encoded resident physics steps. The offscreen pixel test still passes using the
same reusable GravityView. Renderer/GPU library tests and focused strict Clippy
passed. All-target Clippy is currently blocked by unrelated existing xray test/
example and female/hair/skinning warnings; it is not a passing repository gate.

After enabling compute limits, the existing native scene/motion-vector smoke
also passed 120 presented Metal frames. The interactive GPU gravity example's
Windows GNU cross-target check and the GPU library's WASM check passed; these
are compile checks, not Windows/browser runtime acceptance.

The interactive GPU gravity entrypoint accepts strict `--backend
 auto|metal|vulkan|dx12|gl` selection and `--fallback` adapter selection. These
options use the same GraphicsOptions as the scene app. Unknown arguments and
backend names fail before window creation. Explicit Metal passed the complete
60-frame smoke; explicit DirectX 12 on macOS failed surface creation rather
than selecting Metal. Actual Vulkan/DirectX hardware runtime acceptance remains
required on hosts exposing those APIs.

## Linux Vulkan gravity execution (2026-10-01)

`sh tools/linux/gpu-gravity-smoke.sh` uses the existing Linux Docker image,
read-only workspace/registry mounts, offline locked builds and an isolated Linux
build directory. It runs the strict Vulkan window smoke under Xvfb, then the
numerical and offscreen pixel probes. Build the image with the existing Linux
Dockerfile if absent; the host Cargo registry must already contain dependencies.
The image's installed default toolchain is selected explicitly, avoiding a
network update for the workspace's nightly channel. GPU crate recursion limit
256 fixes a Linux compiler warning for the wgpu Send/Sync auto-trait graph.

Actual Vulkan execution passed on Mesa 25.0.7 llvmpipe (CPU software adapter):
60 presented frames and 200 physics steps with resize, pause, reset, suspension
and failed custom-frame encoding; 257 bodies/128 steps with maximum CPU f64
trajectory error 0.000025912606399280946; 2560 orbit steps with relative energy
drift 0.0000008926834292013088; singular/overflow rollback and allocation bounds;
three exact offscreen position/clearing pixel checks from resident body storage.
These results verify Linux/Vulkan API execution, not physical Vulkan GPU support
or throughput. NVIDIA CUDA and physical Vulkan/DirectX hardware acceptance remain
open requirements. A pre-existing scene atomic fetch_update deprecation warning
remains in the Linux build; focused GPU strict Clippy passes on the host.

## Browser resident gravity integration (2026-10-01)

`WebEngine::start_gravity` initializes the shared WebGPU f32 orbit and GravityView.
`?backend=webgpu&gravity=1` selects it in the existing browser entrypoint. Physics
and vertex drawing use the same persistent storage buffer and command encoder;
there is no body readback in the animation loop. Rendering time drives a 240 Hz
accumulator with at most 24 catch-up steps per frame. Space freezes animation
time and therefore physics; zero-sized or unacquired surfaces encode no steps.
Calling start_gravity again replaces the orbit only after initialization succeeds.
WebGL explicitly rejects gravity compute; it does not substitute CPU physics.

The actual in-app browser rendered the two-body WebGPU orbit with over 1300
frames reported. Space froze time at 21.449899999999882 across observations;
the rendered screenshot was inspected. The explicit WebGL gravity request showed
`gravity compute unsupported on WebGL`. WASM strict Clippy and the browser build
passed. This verifies browser compute-to-render integration and pause, not browser
numerical parity or CUDA interoperability. JS/WASM cache keys were bumped together.

## Actual browser gravity numerical acceptance (2026-10-01)

`WebEngine::validate_gravity` checks 257 bodies across four batches of 32 resident
GPU steps against the existing independent CPU f64 gravity solver. It checks
returned count, exact mass preservation, finite positions/velocities and absolute
trajectory error <=0.0002. Browser scheduling yields between CPU reference
batches and while waiting for the mapped GPU result; readback has a 30-second
waiting deadline. The validation uses separate job storage and does not alter
the interactive orbit. WebGL rejects the validation explicitly.

`?backend=webgpu&gravityCheck=1` runs this acceptance check and exposes the result
in the page status and canvas data-gravity-max-error. Actual in-app WebGPU
execution passed with maximum error 0.000025912606399280946. WASM strict Clippy,
browser build and diff checks passed. Physics is a target-specific WASM dependency
for this explicit CPU oracle; the live interactive orbit still uses only GPU
physics and no body readback. Browser orbit-energy and error rollback validation
are not claimed by this particular check.

The CUDA gravity hardware probe now also exercises singularity and numerical
 overflow, repeats steps after each failure to check sticky error reporting, and
rejects zero/excess step counts and body budgets. These are assertions executed
only after an actual CUDA context is acquired, before the ordinary f64 trajectory
and owner-drop checks. CUDA-feature strict Clippy and Windows GNU target checking
passed for the expanded probe. The local runtime still returns DriverUnavailable;
these new device assertions have not run on NVIDIA and are not claimed as passing.
They do not inspect raw committed bytes after failure, so device rollback memory
proof remains a separate requirement beyond the existing source arithmetic tests.

CUDA gravity now exposes `CudaGravityJob::snapshot()` with finite committed
bodies and an optional typed sticky failure. Normal `read()` still reports the
failure rather than returning a successful simulation result. Snapshot reads
synchronize the same bounded header/body range, do not clear failure flags and
do not transfer scratch state. Unknown failure metadata is rejected. The local
unit check covers snapshot decoding after a singularity. Default/CUDA strict
Clippy and unit tests passed.

The NVIDIA acceptance probe now takes snapshots before and after singular and
overflow steps, repeats failed steps, and compares every mass/position/velocity
bit (including signed zero). This makes device rollback proof executable;
it has not been obtained locally because no NVIDIA driver is available.

## CUDA external allocation boundary (2026-10-01)

`CudaCompute::import_external_u32` imports a non-dedicated opaque exported graphics
allocation: OpaqueFd on Unix, OpaqueWin32 on Windows. It consumes a caller-owned
File handle and validates allocation budget, range overflow, nonempty element
count and u32 alignment before driver access. D3D12 resource/heap handles and
CUDA IPC handles are not accepted by this API. The unsafe caller must establish
same physical GPU identity, export/mapping compatibility, graphics completion,
exclusive CUDA ownership and backing allocation lifetime through mapping drop.

`CudaExternalU32Buffer::affine` launches the existing bounded fixed PTX directly
on mapped external memory and synchronizes CUDA completion before returning;
no host data transfer occurs. The mapping/context/module and stream usage guard
remain owned until completion/drop. cudarc's external mapping exposes only a
read pointer; the audited fixed writer uses the exclusive import contract rather
than claiming ordinary shared read access permits mutation. Graphics ownership
and resource state transitions must still be performed by the platform exporter.

Local range validation tests and default/CUDA strict Clippy passed; Windows/Linux
CUDA-feature checks and WASM GPU strict Clippy passed. No graphics export was
created and no imported allocation executed locally. Actual Vulkan export and
synchronization, D3D12-specific import types, matching-device checks and NVIDIA
roundtrip evidence remain required before claiming CUDA/graphics interoperability.

`CudaExternalU32Buffer::read` now supplies synchronized diagnostic readback of
the imported range, preserving all u32 bits and rejecting unexpected byte counts.
This host transfer enables a future export/import/write/result acceptance probe;
it is not the graphics ownership handoff and is not used by the affine write.
Seven local CUDA-crate tests, CUDA-feature strict Clippy and Windows target
checking passed. An actual exported allocation and NVIDIA driver execution remain
unverified; successful decode tests do not prove graphics interoperability.

## Vulkan external allocation export acceptance (2026-10-01)

`sh tools/linux/vulkan-external-smoke.sh` builds/runs the Linux-only
`voxy_cuda` example `vulkan_external --export-only`. It selects an adapter exposing
non-dedicated OpaqueFd buffer export, creates an external-storage buffer and
exportable device-local allocation, fills 256 words by Vulkan commands, copies to
coherent host staging with transfer/host barriers, waits for completion, checks
all words and exports an owned FD. RAII keeps allocations/device alive until FD
use ends and destroys command/buffer/memory resources after queue completion.

Actual Mesa llvmpipe execution passed with a 1024-byte allocation. Linux-target
strict Clippy and diff checks passed. Ash is only a Linux development dependency;
the portable/default CUDA library gains no Vulkan dependency. This proves the
export/fill/copy path on software Vulkan, not CUDA interoperability. The default
combined probe currently returns an explicit incomplete-handoff error: same-GPU
UUID matching and external queue-family ownership transfer must be implemented
before passing the FD to CUDA. Dedicated-only exports, D3D12 handles and actual
NVIDIA execution remain unverified requirements.

## Executable Vulkan/CUDA ownership roundtrip (2026-10-01)

CudaCapabilities now reports the selected CUDA device's 16-byte UUID. The Linux
external-memory probe compares it with Vulkan PhysicalDeviceIDProperties and
rejects zero or mismatched identities before importing the FD; matching ordinal
numbers alone do not authorize sharing.

The combined command `cargo run -p voxy_cuda --features cuda --example
vulkan_external` now records Vulkan release to QUEUE_FAMILY_EXTERNAL, waits for
queue completion, imports the exported FD, performs the fixed CUDA affine write,
synchronizes and checks CUDA readback, drops the mapping, records Vulkan acquire
and copies back to host staging with a transfer/host barrier. It checks all 256
words against the wrapping affine result. This is a deliberately synchronous
acceptance path, without an external semaphore or overlap/performance claim.
It is separate from wgpu rendering and from the gravity kernel's f64 buffers.

The exporter-only Linux llvmpipe check still passes after these changes. Strict
Linux CUDA example Clippy and Windows CUDA library target checking passed. A
concurrent physics skin edit temporarily broke one earlier dev-dependency build;
the subsequent example build passed without editing that unrelated work. Actual
CUDA/Vulkan import/write/reacquire execution still requires NVIDIA hardware;
physical Vulkan, D3D12 types and renderer integration remain open requirements.
The combined CUDA-feature probe was built and invoked in Linux; it exported the
Vulkan allocation and then returned DriverUnavailable before CUDA import. This
is a concrete environment blocker, not a successful interoperability result.

The combined Vulkan/CUDA probe now initializes the explicitly selected CUDA
ordinal first (`--cuda-device N`, default 0) and searches Vulkan adapters by that
CUDA UUID before creating a graphics device/allocation. It rejects missing/zero
CUDA identities and keeps the pre-import UUID recheck. This avoids choosing a
software/secondary Vulkan adapter just because it appears first. Vulkan 1.1 is
required for the core identity/property path. `--export-only` stays driver-free;
combining it with `--cuda-device` is rejected before initialization. Linux strict
Clippy and the llvmpipe exporter check passed after the selection change. Actual
multi-GPU CUDA selection and the ownership roundtrip remain unverified locally.

## Vulkan external-family barrier runtime check (2026-10-01)

Exporter-only acceptance now also releases the buffer to QUEUE_FAMILY_EXTERNAL,
reacquires it on the original Vulkan family, copies it to staging and verifies
all 256 words again. There is deliberately no external writer in this check.
The exact release/acquire helper used by the CUDA roundtrip ran successfully on
llvmpipe. A second run installed Khronos validation in an ephemeral Linux
container: loader logs confirmed the validation instance and device layer,
and no Validation Error/VUID message appeared during allocation, export,
release/acquire, readback or teardown. This does not prove CUDA cache visibility.

For repeatable validation, build `voxy-linux-validation` using
`docker build -t voxy-linux-validation - < tools/linux/Dockerfile.validation`,
then run `VOXY_LINUX_IMAGE=voxy-linux-validation VOXY_VULKAN_VALIDATION=1 sh
tools/linux/vulkan-external-smoke.sh`. The script requires the layer manifest
when requested, saves output under target/linux-docker/vulkan-external.log,
preserves command failure and rejects Validation Error/VUID diagnostics.
The normal existing-image exporter run and Linux strict Clippy passed.

The saved validation Dockerfile was built and the saved script executed with
VOXY_VULKAN_VALIDATION=1: it passed on llvmpipe, including the required loader
confirmation that the Khronos device layer was inserted. Negative configuration
checks passed: missing requested layer and invalid validation setting both return
failure. Build the layer image via stdin (`docker build ... - < Dockerfile`),
which sends a 2 KiB context instead of the large workspace build directory.
This is reproducible Vulkan API validation, not NVIDIA execution evidence.

Validation now includes a negative control: `--validation-negative` records a
fill with offset 1, ends the command buffer and returns failure without queue
submission. The Khronos layer reported the exact expected
VUID-vkCmdFillBuffer-dstOffset-00025. With validation enabled, the saved smoke
script requires that diagnostic and the unsubmitted-command message, separately
from the clean normal run; a missing/nonfunctional error reporter therefore
fails the check. Both paths ran successfully in the validation image. The
negative diagnostic log is target/linux-docker/vulkan-validation-negative.log.
Linux strict Clippy and diff checks passed. No CUDA hardware result is implied.

CUDA context selection now validates usize ordinals against i32 before loading
the driver, then checks the actual device count before context creation. This
prevents cudarc's internal usize-to-i32 cast from truncating a large requested
ordinal into another GPU's index. Invalid selection returns InvalidDeviceOrdinal.
Default tests, the CUDA-feature driver-free truncation regression, strict CUDA
library Clippy and Windows/WASM checks passed. Actual out-of-count device tests
still require an available CUDA driver; no NVIDIA execution is claimed.

## Renderer all-target strict gate restored (2026-10-01)

`cargo clippy -p voxy_render --all-targets --no-deps -- -D warnings` now passes,
including the X-ray examples and tests that previously blocked the gate. The
X-ray smoke separates GPU creation, scenario definitions, scene encoding and
pixel/depth readback into helpers with explicit wgpu descriptor defaults. The
unit tests retain intentional exact boundary/unchanged-material float comparisons
with a documented test-only float_cmp allowance. Rendering behavior and numerical
assertions are unchanged. All 42 renderer library tests and the actual Metal
X-ray color/depth smoke passed after refactoring. This is a renderer gate;
other workspace packages and physical NVIDIA/DirectX acceptance are not covered.

## Linux Vulkan renderer/shader acceptance with validation (2026-10-01)

`sh tools/linux/renderer-smoke.sh` uses the validation image and performs actual
Vulkan scene, shader-replacement, X-ray and compute probes. It requires the layer
manifest, confirms insertion of the Khronos device layer and the selected Vulkan
backend for every probe, preserves nonzero command status and rejects Vulkan
Validation Error/VUID diagnostics. Full logs are saved separately under
 target/linux-docker/renderer-{scene,shaders,xray,compute}.log; normal output is
condensed to adapter/layer/result evidence.

All four passed on llvmpipe: indexed/depth/textured/overlay pixels; shader reload,
unchanged-source cache and rejection of syntax/entrypoint/binding errors; X-ray
occlusion/reveal/blending/depth/UI/masking; and 1042 exact compute results with
partial groups, independent jobs, mapping, cancellation and validation failures.
This establishes software Linux/Vulkan execution, not physical Vulkan performance
or NVIDIA RTX/CUDA support. A pre-existing scene atomic deprecation warning remains
in this Linux image's newer nightly compiler build.

## Native OpenGL gravity startup acceptance (2026-10-01)

The GPU gravity window now creates its wgpu instance with the event loop's owned
display handle and passes it into `SceneSurface::new_with_instance`. Without this
handle the strict Linux GL path rejected adapter selection with
`incompatible_surface_backends: Backends(GL)` before physics could run.

`sh tools/linux/opengl-smoke.sh` records strict GL scene pixels, shader replacement
and the resident gravity window in `target/linux-docker/opengl-{scene,shaders,gravity}.log`.
All three passed on Mesa 4.5 llvmpipe under Xvfb, including 60 presented gravity
frames, resize, ten paused frames, reset and exactly 200 physics steps. The Vulkan
gravity script also passed after the display-handle change. These Linux results
use a software adapter. Windows cross-compilation of the gravity example passed.

The window smoke now fails after 30 seconds with presented-frame/step counts and
the last acquisition outcome. A current native Metal retry reported zero frames,
zero steps and `SkippedOccluded`; requesting focus did not resolve it. That run
does not establish Metal window acceptance after this change. Surface acquisition
still precedes encoding, so an occluded window cannot advance physics unnoticed.
The current Metal offscreen gravity-render probe passed initial and two evolved
frames with correct compute-to-vertex visibility and no body readback.

## Explicit CUDA terrain device selection (2026-10-01)

The native world startup accepts `--cuda-terrain --cuda-device N` instead of
always constructing device zero. Omitting the ordinal retains zero. Argument
validation happens before the event loop/window is created and is repeated at
the terrain construction boundary: missing/non-numeric/negative/out-of-driver-range
ordinals, duplicate device options, conflicting GPU/CUDA terrain flags and an
ordinal without CUDA terrain all fail explicitly. Other existing app flags remain
available. Actual device existence is still checked by `CudaCompute::new`.

Focused parser tests passed, and launching the native app with ordinal -1 returned
the argument error before window creation. The CUDA-enabled Windows target builds;
these checks do not establish execution on a physical NVIDIA device.

## CUDA compilation matched to the selected GPU (2026-10-01)

Terrain and gravity compilation now query NVRTC's supported virtual architectures
and the selected CUDA context's compute capability. They select the highest
compiler-supported target that does not exceed the device capability, instead of
relying on the compiler's default architecture. A newer device can use the highest
compatible target from an older compiler; a device below the compiler's minimum
fails explicitly with `UnsupportedCompilerArchitecture`. Query errors retain their
NVRTC error. The gravity solver retains its precise division/square-root options
and disabled FMA contraction/fast math.

Selector tests cover unsorted compiler lists, exact capabilities, intermediate
capabilities, newer GPUs, malformed capabilities and no compatible target. Default
and CUDA-enabled tests passed, as did CUDA all-target strict Clippy and Windows
cross-compilation. The pinned NVIDIA NVRTC 12.6 compiler actually compiled terrain
and all three gravity entrypoints separately for compute_52, compute_75 and
compute_89; PTXAS assembled each matching target and the affine kernel. These are
real compiler checks without GPU execution; physical driver/JIT acceptance remains
unverified.

## Physical Linux CUDA acceptance entrypoint (2026-10-01)

`sh tools/cuda/hardware-acceptance.sh N` builds the CUDA-enabled probes and runs,
in order, private buffer transfers/resident affine kernels, exact procedural
terrain parity, resident f64 gravity parity/failure rollback, and matched-device
Vulkan external-memory ownership roundtrip. Every probe receives the same explicit
CUDA ordinal. The terrain probe now accepts `terrain_smoke cuda N`; extra arguments
are rejected. The script requires native Linux builds, preserves failed exit
statuses, bounds each probe to 300 seconds, and saves separate logs under
`target/cuda-hardware`. Cargo resolves configured artifact paths; no Python is
required by this entrypoint.

An actual Linux Docker run compiled all four probes with CUDA enabled, then
correctly stopped at the first buffer probe with exit 1 / `DriverUnavailable`.
This verifies unavailable-driver failure handling and log preservation, not any
NVIDIA device execution. Physical execution of the four probes remains required.

## CUDA gravity writes an external graphics render view (2026-10-01)

`CudaGravityJob::write_render_view` converts the committed resident f64 bodies
directly into an exclusively owned `CudaExternalU32Buffer`. The render-only ABI
matches the gravity vertex shader: eight header words, then position xyz, mass,
velocity xyz and padding per body. It rejects a different CUDA context and a short
destination before launch. Two ordered kernels validate every f32 conversion and
conditionally publish the view; solver failure, overflowing f32 values or mass
underflow leave the previous graphics contents untouched. The solver remains f64.
Only a four-byte status is read by this API; it synchronizes before the caller's
graphics ownership handoff. It does not export wgpu allocations or connect the
main renderer to CUDA yet.

The Vulkan/CUDA acceptance probe now additionally checks an evolved two-body
render view, untouched allocation tail, conversion-error rollback and singular
solver rollback before the Vulkan acquire/readback. This combined path remains
unexecuted on NVIDIA. A direct host harness of the identical CUDA source passed
257-body conversion, signed zero, overflow/underflow and solver-error rollback.
NVRTC/PTXAS compiled all five gravity entrypoints for 52/75/89. CUDA library strict
Clippy and Windows compilation passed. Standard tests and Linux Cargo example
checks were blocked by concurrent astrophysics radiation signature errors.
An isolated Linux rustc build of the CUDA-enabled Vulkan example succeeded with
warnings denied; its export-only llvmpipe ownership roundtrip passed with Khronos
validation enabled. That run did not execute the CUDA render-view writer.

## Render views independent of the physics backend (2026-10-01)

`GravityView::from_buffer` accepts a device-owned graphics storage buffer and an
explicit body count using the render-only CUDA/WGSL body ABI. Empty counts, u32
address overflow, missing STORAGE usage and short allocations are rejected before
pipeline construction. Binding validation declares the required minimum size.
`GravityView::new` still binds the resident WGSL job through this shared entrypoint.
This is the renderer attachment boundary; native CUDA allocation export/import
into a wgpu buffer remains necessary for a complete CUDA-rendering integration.

The gravity pixel probe now draws its initial frame directly from the resident
job and two evolved frames from a separate minimal 64-byte render buffer, populated
by GPU copy. It verifies expected green pixels, clearing of old positions and
invalid-buffer rejection without reading body data on CPU. Actual Metal M4 Max
and Linux llvmpipe Vulkan runs passed. The full Linux gravity window/numerical
script also passed after this change; the latter remains software Vulkan proof.
GPU library/example strict Clippy and WASM compilation passed. Standard CUDA tests,
all-target strict Clippy and the CUDA-source host parity test now pass after the
concurrent astrophysics compilation errors were resolved.

The NVIDIA ownership probe additionally rejects a 256-word destination for a
32-body render view and checks unchanged contents, plus f64-mass underflow into
f32 with rollback. These device assertions are implemented but not executed here.
The Linux image lacks cargo-clippy, so its Linux-only probe lint gate remains
unverified; the host all-target Clippy gate excludes that cfg-gated Linux body.

## Exportable wgpu Vulkan storage (2026-10-01)

The new `voxy_vulkan` crate confines native unsafe Vulkan operations to a separate
ownership boundary; `voxy_gpu` retains the workspace unsafe-code prohibition.
`VulkanExportBuffer::new` creates non-dedicated opaque-FD storage on the actual
wgpu Vulkan device, initializes it with a completed GPU fill, then transfers
buffer/allocation destruction to wgpu-hal's managed buffer. It requires explicit
VULKAN_EXTERNAL_MEMORY_FD, checks export properties, dedicated requirements and
device-local memory, and rejects invalid sizes. The buffer supports STORAGE and
GPU copies and can be bound by `GravityView::from_buffer`.

Unsafe release/acquire operations document queue host synchronization, retained
binding exclusion, allocation/import lifetimes and completion of external writes.
They perform CPU-synchronous queue-family transitions with queue-idle completion;
no semaphore overlap is claimed. Access and duplicate transitions are rejected
while the allocation is in the wrong ownership phase. Importers receive the exact
Vulkan allocation size through `allocation_bytes`.

`sh tools/linux/wgpu-external-smoke.sh` now creates this actual wgpu storage,
copies resident gravity bodies to it, exports/drops an FD, reacquires it and renders
before any further write. Initial and two evolved pixel positions and clearing of
old positions are checked without body readback. Actual Mesa llvmpipe execution
passed; the log requires confirmed Khronos device-layer insertion and rejects
Validation Error/VUID diagnostics. Logs are in
`target/linux-docker/wgpu-external.log`. Native-boundary library/example Clippy
passed for the Linux GNU cross-target. This proves export/acquire and rendering
on software Vulkan; a CUDA writer and NVIDIA UUID-matched rendering remain to be
connected and physically verified.

## CUDA writes the wgpu Vulkan render allocation (2026-10-01)

`cargo run -p voxy_vulkan --features cuda --example cuda_gravity_render -- N`
now connects resident f64 CUDA gravity to the actual exportable wgpu buffer.
It queries the selected CUDA device UUID, enumerates only Vulkan adapters and
matches the physical UUID before graphics allocation. Missing/zero UUID or no
matching adapter fails explicitly. The native boundary exposes a read-only
`adapter_uuid` query; the software export probe checks this query too.

For each frame the probe releases Vulkan ownership, imports the exact allocation
into the same CUDA context, writes a render view, drops the CUDA mapping and
reacquires Vulkan ownership. `CudaCompute::synchronize` explicitly drains the
context even after a writer error; failed completion prevents acquire/draw.
wgpu then draws directly from that allocation. Body data is uploaded once for
initialization; subsequent frames read only status and final pixels on CPU.
The initial position and two evolved positions after 64 CUDA steps have exact
pixel assertions and clearing checks. The hardware acceptance script now adds
this fifth probe, preserving the selected ordinal for every probe.

CUDA-enabled Linux compilation and Linux cross-target all-target strict Clippy
passed. An actual Linux invocation failed at CUDA context construction with
`DriverUnavailable`, before graphics allocation, so physical CUDA-to-wgpu pixels
remain unverified. The software Vulkan export/acquire/shader-pixel probe still
passes with confirmed Khronos device-layer insertion and no validation errors.
This is a connected offscreen acceptance path; main-game-loop adoption, Windows
interop and NVIDIA runtime evidence remain completion requirements.

## Reusable CUDA graphics owner (2026-10-01)

With the CUDA feature, `voxy_vulkan::CudaGravityGraphics` owns the CUDA factory,
resident f64 solver and exported render allocation as one component. Constructor
validation compares CUDA UUID against the actual wgpu logical device's physical
UUID, checks render addressing and allocation budget, then creates the resources.
`buffer` and `body_count` feed the existing gravity view. Unsafe `publish(steps)`
performs ordered physics, export, direct render-view writing, context completion
and graphics reacquisition under a documented exclusive queue/binding contract.
Zero steps publishes initialization/current state; failed ownership is rejected
before advancing physics. Conversion errors preserve old render contents.

The CUDA pixel probe now uses this component rather than a private copy of the
handoff sequence. Linux execution passed the render-capacity/overflow unit test
and compiled the component/example; the example still stops with
`DriverUnavailable` before device construction on this host. Linux cross-target
all-target strict Clippy passed. The software Vulkan probe checks that adapter
and logical-device UUID queries agree. This is a library integration point,
not proof of NVIDIA execution or adoption by the main game loop.

## Windows Vulkan opaque-memory export (2026-10-01)

The native boundary and CUDA gravity component now also compile on Windows.
Linux uses OPAQUE_FD/VULKAN_EXTERNAL_MEMORY_FD; Windows uses
OPAQUE_WIN32/VULKAN_EXTERNAL_MEMORY_WIN32 and an owned NT handle returned by
vkGetMemoryWin32HandleKHR. `EXTERNAL_MEMORY_FEATURE` exposes the required wgpu
feature for the host platform. Both constructors, UUID queries and frame probes
retain strict Vulkan selection and the same allocation/ownership checks.

The Windows export is wrapped as an owned File handle; the existing CUDA importer
retains it for the full external-memory lifetime and closes it on teardown.
No D3D12 resource or CUDA IPC handle is passed through this opaque Vulkan route.
Windows GNU and Linux GNU all-target Clippy with CUDA enabled passed, including
the native gravity component and both examples. Windows runtime/NT-handle/CUDA
execution remains unverified; successful cross-compilation is not that evidence.
DirectX/CUDA resource interoperability remains a separate completion requirement.

## Acquired-window CUDA physics loop (2026-10-01)

`SceneSurface::new_with_adapter` accepts a caller-selected adapter from the supplied
instance and explicit device features, checks surface compatibility and uses the
shared device/surface initialization. This lets a CUDA UUID-matched adapter enable
the platform external-memory feature without silently selecting another device.
Existing constructors retain their adapter selection and device-limit behavior.

`cargo run -p voxy_vulkan --features cuda --example cuda_gravity_window -- --cuda-device N`
now runs the native orbit through `CudaGravityGraphics` and `GravityView`. Space
pauses, R transactionally replaces the physics/view resources, Escape exits. Physics
publication occurs inside `render_custom`, after surface acquisition; suspension,
occlusion and skipped frames cannot advance CUDA. A bounded `--smoke` checks 60
presented frames, actual resize, ten paused frames, reset, injected callback failure
and 200 resident steps. The hardware acceptance script adds this sixth gate;
native Linux X11/Wayland display access is required for that gate.

Explicit `--export-only --smoke` exercises the same window and ownership shell
with WGSL physics and no CUDA writer. Actual Linux llvmpipe/Xvfb execution passed
60 frames and all lifecycle assertions. `tools/linux/external-window-smoke.sh`
requires Khronos device-layer insertion and rejects validation errors, preserving
logs separately from a normal-mode CUDA-unavailable check. That software mode is
not NVIDIA execution and cannot be combined with CUDA ordinal selection.
Linux/Windows native-example Clippy passed; 49 renderer tests, WASM compilation
and the existing Linux gravity numerical/pixel/window suite passed after the
constructor refactor. Main voxel-game physics adoption and physical CUDA window
execution remain requirements; this increment connects the native gravity loop.


### Typed D3D12 committed-resource import (2026-10-01)

`voxy_cuda` now exposes Windows-only `CudaCompute::import_d3d12_resource_u32`
for shared committed buffer NT handles. It uses CUDA's D3D12_RESOURCE handle type
and required DEDICATED flag, rather than treating the resource as opaque Win32
memory. CUDA confirms this contract in its [driver interoperability reference](https://docs.nvidia.com/cuda/cuda-driver-api/cuda_driver_api/group__CUDA__EXTRES__INTEROP.html).
The retained NT handle closes after freeing the CUDA mapped pointer and imported
memory. Context completion is established before release; on completion/free
failure the CUDA reference is retained and the context records the error.
Mapping setup failures unwind the imported resource. Existing affine and gravity
publication methods accept either mapping through the same buffer abstraction.

This is an import boundary, not a functioning D3D12 renderer bridge. Callers must
export a shared committed buffer, match physical devices, validate actual resource
allocation/range, complete graphics queue work and perform state transitions.
D3D12 heap handles are excluded. D3D12/wgpu resource export and actual NVIDIA
execution on Windows remain outstanding.

Verification: strict library Clippy passed for Windows GNU (CUDA enabled and
 disabled) and Linux GNU with CUDA enabled; Windows test targets compile, including
 the descriptor type/flag assertion (not executed on Windows). Nine local tests
passed and one CUDA-source host test remained ignored. Existing Linux llvmpipe
Vulkan export/acquire and shader pixels passed with Khronos validation and no
validation errors. All-target Clippy is not green: the current shared `physics`
dev dependency reports unrelated warnings promoted to errors.


### Shared D3D12 storage export (2026-10-01)

Windows-only `voxy_vulkan::D3d12ExportBuffer` creates a shared committed DEFAULT
heap buffer with UAV/storage capability and adopts it through wgpu HAL's DX12
raw-buffer constructor. It rejects non-DX12 devices, linked-node devices and
invalid sizes. Allocation size and owned resource NT handles are exposed for
`voxy_cuda`'s typed dedicated-resource import; no opaque Win32 substitution occurs.

Memory is initialized before the unsafe wgpu import: a temporary GPU-zeroed wgpu
source is copied using a native D3D12 command list. A subsequent ordered wgpu fence
is awaited with the allocator/list/source still alive. Completed buffer command
lists decay to COMMON, as specified by [Microsoft's resource-state documentation](https://learn.microsoft.com/en-us/windows/win32/direct3d12/using-resource-barriers-to-synchronize-resource-states-in-direct3d-12).
No CPU data upload is needed. NT handle access uses GENERIC_ALL as required by
[CreateSharedHandle](https://learn.microsoft.com/en-us/windows/win32/api/d3d12/nf-d3d12-id3d12device-createsharedhandle).

`cargo run -p voxy_vulkan --example d3d12_export` is a strict Windows/DX12
acceptance probe for creation, owned-handle closure, allocation size and 96-byte
zero readback through wgpu. It has no CUDA writer and is not an interop acceptance
result. The owner exposes an unsafe export contract; synchronized CUDA publication,
physical device matching and renderer handoff remain to be connected.

Verification: Windows GNU library strict Clippy passed with CUDA; Windows test
sources compiled (not executed); the diagnostic example passed strict Clippy
with no dependency linting. Linux GNU library strict Clippy passed with CUDA.
Actual Windows/DX12 execution is unverified on this macOS host.


### D3D12/CUDA physical-device matching (2026-10-01)

`CudaCompute::windows_adapter_identity` queries CUDA's Windows LUID and node mask.
`D3d12ExportBuffer::adapter_luid` queries the retained D3D12 device;
`validate_cuda_device` rejects mismatching/zero identities and masks other than
single-node 1. `import_cuda` connects the exporter to the dedicated-resource CUDA
import using exact native allocation size, and checks the requested u32 range
against the logical buffer size (not merely the potentially larger allocation).
Its unsafe contract still requires graphics completion, exclusive external use,
owner lifetime and state restoration. It does not silently change adapters.

Strict Windows GNU library Clippy passed with CUDA enabled. Windows test sources
compile, including identity rejection cases; Windows execution remains unverified.
Full graphics/CUDA queue handoff and resident gravity publication on DX12 are
still outstanding.


### Ordered CUDA gravity publication into DX12 storage (2026-10-01)

Windows `CudaGravityD3d12Graphics` owns CUDA's resident f64 solver and a shared
D3D12 buffer. Constructor validates LUID/node identity and both logical and native
allocation budgets. `publish` drains the graphics queue through an ordered wgpu
fence before import, publishes the committed render ABI without CPU body transfer,
drops mapping and drains CUDA before exposing graphics storage again. Failed
completion leaves `buffer()` inaccessible and future publication rejected.
Retained bind groups and concurrent submissions remain excluded by the unsafe
contract. This CPU-synchronized path relies on completed D3D12 buffer lists
returning to COMMON; CUDA writes do not alter the D3D12 resource state.

The strict DX12 diagnostic now supports `cargo run -p voxy_vulkan --features cuda
--example d3d12_export -- --cuda-device N`. CUDA affine writes transform the
initialized shared buffer to 24 exact words of value 7, then wgpu copies the same
resource to diagnostic readback. Default mode still checks zero initialization
without a CUDA writer. Explicit CUDA mode rejects a missing feature, malformed
ordinal or mismatching adapter and never falls back to the zero-only check.

Windows GNU strict library Clippy and test compilation passed. The diagnostic
passes strict Clippy with CUDA enabled. No physical Windows/NVIDIA execution is
available here: publication and the probe are compiled implementation, not a
verified runtime result. Shader pixels/window integration of the DX12 gravity
factory remains outstanding.


### DX12 CUDA gravity shader-pixel acceptance (2026-10-01)

The existing native `cuda_gravity_render` probe now accepts an explicit Windows
DX12 mode: `cargo run -p voxy_vulkan --features cuda --example
cuda_gravity_render -- N --dx12`. It queries the selected CUDA LUID/node mask and
selects the matching DX12 adapter before device creation. Missing/mismatching
identity or unsupported platform fails; the default remains UUID-matched Vulkan.
A read-only `voxy_vulkan::adapter_luid` query supports adapter selection without
creating temporary D3D12 resources.

Both modes use the same GravityView, texture and exact pixel verifier. The probe
publishes initial state, then two batches of 32 CUDA steps; green pixels must
move from x=16 to 32 to 48 and old positions must be black. Body snapshots never
reach the host; only rendered diagnostic pixels are read back. DX12 invokes the
ordered shared-storage gravity publisher added previously. This provides the
Windows/NVIDIA shader acceptance command, but physical execution remains pending.

Windows GNU strict Clippy passed for the modified diagnostic. Full objective
still requires actual DX12/NVIDIA pixel execution, window integration and main
voxel-game GPU physics adoption, alongside other platform/hardware gates.


### Native DX12 initialization failure lifetime (2026-10-01)

Auditing the shared-buffer constructor found that an unsuccessful completion
wait after a native copy could unwind and release its command allocator/list and
memory while GPU completion remained unproven. The constructor now retains all
native copy dependencies on this failure path, including the temporary wgpu
buffer owner (a raw COM resource alone does not preserve allocator ownership).
It returns the original wait error and exposes no graphics buffer. The failed
allocation is intentionally retained for process lifetime instead of attempting
unsafe reuse/free after an unconfirmed native completion. Normal successful
initialization releases temporary resources as before. Windows GNU strict
library Clippy passed; runtime fence-failure injection remains unverified.


### DX12 CUDA acquired-window loop (2026-10-01)

`cuda_gravity_window --dx12 --cuda-device N` selects the Windows DX12 adapter by
CUDA LUID and invokes `CudaGravityD3d12Graphics` for the same two-body orbit.
Publication remains inside SceneSurface's acquired-frame callback: skipped,
suspended or occluded frames cannot execute physics. Resize, Space pause and
transactional R reset use the existing loop. DX12 requests no Vulkan external
memory feature; reset preserves the selected backend. `--smoke` retains the
60-frame/200-step lifecycle assertions. Linux rejects DX12 before event-loop
creation, and DX12 cannot be combined with the Vulkan-only `--export-only` mode.
The default Vulkan/CUDA mode and explicit WGSL/Vulkan export test remain available.

Windows and Linux diagnostic Clippy checks are run for both cfg branches. Physical
Windows/NVIDIA window execution remains unverified; this is implemented window
integration, not proof of complete hardware support or main VoxyWorld adoption.


### Native Windows hardware acceptance runner (2026-10-01)

`powershell -File tools/cuda/hardware-acceptance.ps1 -DeviceOrdinal N` builds
native locked CUDA examples and runs six required gates: buffers, exact terrain
parity, f64 gravity/errors, CUDA writes into committed DX12 storage, exact shader
pixels, and the acquired-window lifecycle. Each gate preserves a separate log
under `target/cuda-hardware-windows`, checks exit status and required success
markers, and stops on failure. Numeric ordinals are validated; non-Windows and
cross-target Cargo configurations are rejected. CUDA modes and explicit DX12
flags prevent silent zero-only/Vulkan/software-compute substitution. Runtime
probes require NVIDIA driver/NVRTC and an accessible Windows desktop.

The runner bounds each probe at 300 seconds and compilation at 900 seconds;
timeout termination targets only its own spawned process tree, with an additional
bounded confirmation wait. PowerShell 7.6 parsed the script locally. Fixture
execution verified success, nonzero child exit, missing-success-marker rejection
and retained logs; invalid ordinals and the non-Windows platform guard rejected
as expected. These runner checks are not physical NVIDIA acceptance. Windows
process-tree timeout handling and all actual hardware gates remain unverified.


### Selected CUDA device for the DX12 memory gate (2026-10-01)

The standalone `d3d12_export --cuda-device N` diagnostic previously requested the
platform's default DX12 adapter before checking CUDA identity. On hybrid or
multiple-NVIDIA systems that could reject a valid requested CUDA device because
the default adapter was different. CUDA mode now queries that device's LUID/node
mask first and enumerates DX12 adapters to select the exact match before device
creation. Missing/non-single-node identity fails without a different-adapter
fallback. Zero-only mode retains normal DX12 adapter selection. The Windows
hardware runner additionally requires the selected ordinal/LUID marker for its
memory gate.

Windows GNU diagnostic Clippy passed with and without CUDA; PowerShell parser
and runner fixture checks passed after the required-marker update. Physical
multi-GPU Windows execution remains unverified.


### Explicit graphics policy in the main voxel application (2026-10-01)

The main `voxy_app` now accepts `--backend auto|metal|dx12|vulkan|gl` and explicit
`--fallback`. Parameter validation runs before window creation; duplicate,
missing and invalid backend values are rejected. The same policy reaches both
voxel rendering and `--gpu-terrain` compute generation. CUDA terrain keeps its
separate explicit ordinal selection. `Renderer::new_with_instance` allows the
application to pass the event loop display handle needed by native GLES; existing
new/new_with_options entrypoints retain their defaults. Surface initialization
and world-backend failures now propagate to a nonzero main process exit.

Verified: two graphics-option tests passed, renderer strict library Clippy passed,
49 renderer library tests passed, and WASM renderer checking passed. Actual
`--backend bogus` rejected before window initialization with exit 1. Existing
CPU character/projectile/vehicle/contact/water behavior was not replaced by the
free-body GPU gravity demos: main voxel-game GPU physics remains outstanding.

Actual main-app `--backend dx12` on macOS subsequently returned the surface
initialization error with exit 1 and did not select Metal. An intermediate build
was interrupted by a concurrently edited physics source; after that source was
corrected, the graphics tests and the executable check completed. Main voxel
presentation with each requested API still needs its platform acceptance run.


### Main voxel Metal presentation and honest autopilot completion (2026-10-01)

The main application's presented-frame counter now advances only for Presented,
not Reconfigured. Its graphical autopilot requires at least 60 actual presented
frames, reports that count, and rejects a requested run that exits without a
completed result. Two graphics-parameter tests passed after this change.

Actual main-app Metal + GPU terrain execution showed Apple M4 Max/Metal for both
generation and rendering, nine chunks and 1074 greedy quads, and 687 presented
frames. The full autopilot failed with water=0 and peak_vehicle=0, despite camera
and character movement, two explosions and 599 animation frames. A controlled
Metal run with CPU terrain also failed with the same water/vehicle results and
402 presented frames. Thus real Metal voxel presentation is verified, but full
gameplay acceptance is not; the failure is reproduced without GPU generation.
Logs are `target/main-metal-autopilot.log` and
`target/main-metal-cpu-terrain-autopilot.log`. No complete-support claim follows
from these rendering results; gameplay and hardware/platform gates remain open.


### Terrain-derived gameplay initialization (2026-10-01)

Main gameplay initialization previously used fixed actor y=34 and a y=18 water
patch inherited from the old simple terrain showcase. Actor spawn now scans the
four voxel columns touched by its AABB footprint, chooses clearance above the
highest collidable cell, and rejects missing support/headroom. Active water cells
are discovered from actual registered water states in resident chunk snapshots.
Water work above max_active is retained for later ticks, including when the
current batch is settled, and next-active cells are deduplicated without dropping
deferred work. Coordinates reject chunk-origin overflow.

A Metal/GPU-terrain run after the spawn change reached peak vehicle speed 4.16
(previously 0) and 1192 presented frames, though water acceptance still failed.
After adding water discovery, another run failed movement/vehicle/water checks;
full gameplay acceptance therefore remains unproven. Two focused tests were added
for loaded noncolliding procedural spawn clearance and discovery of actual water
cells in the simple terrain's known 81-cell patch. Their execution is currently
blocked by a concurrently edited physics/cohesive.rs reference to undefined
`progress`; no passing-test claim is made for these additions. Saved runtime logs:
`target/main-metal-terrain-spawn.log` and `target/main-metal-live-water.log`.


### Focus-independent autopilot movement and restored startup tests (2026-10-01)

The concurrent physics compilation error was resolved in the shared tree. All
13 main application tests passed, including terrain spawn clearance, actual
water-cell discovery and a new focus-clear regression. Autopilot now reapplies
its scheduled W/A movement each fixed simulation tick; ordinary focus-loss input
clearing remains intact. This prevents desktop focus changes from erasing the
script's held keys. Commands stop at the original scripted boundaries.

Actual Apple M4 Max/Metal + GPU terrain run found 8217 active water cells,
presented 596 frames, moved camera/character, committed two explosions and reached
vehicle speed 3.93. The run still failed the unchanged requirement for at least
one water transaction (water=0). Thus water discovery is verified but full main
app acceptance remains incomplete. Log: `target/main-metal-scripted-input.log`.


### Wake settled water after authoritative destruction commits (2026-10-01)

Main-app explosion commits now reactivate registered water cells at or adjacent
to the actual changed voxel positions from the commit receipt. Existing pending
work is retained and deduplicated; unloaded cells and overflowing coordinate
neighbors are excluded. This repairs the missing connection between settled
water (whose active queue was cleared) and later terrain destruction. It does
not invent water or mutate unrelated cells to satisfy the graphical test.

The regression removes support under a known water source through World::commit,
reactivates neighboring water, verifies duplicate wakeup/extreme unloaded edits
do not add work, runs the real fluid planner and commits flow into the removed
support. This test passed; all 14 main binary tests passed after the change.
The full procedural-world graphical autopilot's water-transaction requirement
still needs a successful runtime reproduction; main fluid/vehicle algorithms
remain CPU-backed and no complete GPU physics claim follows.


### Main voxel OpenGL execution and exact terrain parity (2026-10-01)

Actual Linux/Mesa OpenGL 4.5 main-app execution with `--backend gl --gpu-terrain
--autopilot` used strict Gl for both compute generation and voxel rendering,
presented 988 frames, moved camera/character, committed two explosions and reached
vehicle speed 3.93. Full gameplay acceptance still failed water=0; its original
transaction requirement is retained. Log: `target/linux-docker/main-opengl-autopilot.log`.

`terrain_smoke gl` now selects OpenGL explicitly. Actual execution passed all
502 chunks / 16,449,536 exact CPU block comparisons, descriptor/seed bits, i64
extremes and cancellation checks. Diagnostic Clippy passed. The Linux OpenGL
suite includes this gate with a 90-second bounded timeout and checks both Gl
adapter identity and the complete 502-chunk marker. Existing graphics, shader
reload and resident-physics-window gates remain included.

These are actual software llvmpipe API executions, not physical GPU benchmarks
or proof for every vendor/driver. Native NVIDIA, Windows DX12, mobile and VR
hardware acceptance remain requirements, as does main-game GPU physics adoption.

The updated complete Linux/OpenGL suite subsequently passed: scene/shader pixel
checks, acquired-window 60 frames and 200 resident gravity steps with lifecycle
assertions, and all 502 terrain chunks. Combined log: `target/opengl-suite.log`.


### Typed voxel shader/resource initialization errors (2026-10-01)

The voxel renderer now scopes validation and out-of-memory errors while its
new device is still private. Shader/pipeline/bind-group/resource initialization
failures return `RendererError::Initialization` before publishing the renderer,
rather than reaching wgpu's default uncaptured-error panic path. Both scopes are
popped before either error is returned. Adapter/device creation errors retain
their existing typed paths. Runtime device failures outside initialization are
not claimed to be solved by this boundary.

Strict renderer library Clippy, all 52 current renderer tests and WASM checking
passed. Actual main-app Metal initialization and frame presentation succeeded;
its full autopilot still exited 1 on water=0. Log:
`target/main-scoped-renderer-init.log`. Physical out-of-memory failure injection
and unsupported vendor-specific shader execution remain unverified.


### CUDA projectile integration in the main voxel application (2026-10-01)

The main application now accepts `--cuda-projectiles`, independently of terrain
selection, with the shared `--cuda-device` ordinal. `CudaCompute::projectile_motion`
runs a bounded f64 semi-implicit Euler batch, using precise NVRTC options and
separate multiply/add instructions. The aggregate input/output allocation budget
is checked before allocation. Device output is read back for the existing CPU
voxel sweep: this is motion integration on CUDA, not GPU-resident voxel collision.
Integer world anchors never enter the device ABI. The unchanged world raycast
still rejects impact/unavailable/budget outcomes without advancing projectile
state. Character, vehicle and water physics remain on CPU.

`physics_voxel::step_projectile_with_motion` is the common collision/application
boundary for CPU and externally integrated motion. A regression covers continuous
hits and successful movement at x=9007199254740993, preserving the exact i64 anchor,
and rejects nonfinite live motion without state mutation. CUDA failure terminates
the requested application run; there is no implicit fallback. On this Apple host,
the actual main binary with the new flag returned `DriverUnavailable` with exit 1
before creating a window. NVIDIA execution remains unverified.

The new `projectile_probe` requires a real device and checks 257 projectiles over
128 batches with exact f64 Euler motion, overflow rejection and recovery. Both
Linux and Windows hardware acceptance runners include it. NVRTC 12.6 and PTXAS
actually compiled and assembled the new source for sm_52, sm_75 and sm_89 via
`tools/cuda/verify_compilation.sh`; this is compiler proof, not GPU execution.
CUDA library/probe strict Clippy passed, Windows GNU probe check passed, and the
main binary checked with and without CUDA. Projectile and CLI selection tests
passed. Concurrent contact-code type inference failed an intermediate build;
the concurrent fix was verified by the quadratic friction/surface tests.


### CUDA character motion boundary (2026-10-01)

The main voxel application also accepts `--cuda-character-motion`, independently
of `--cuda-projectiles` and terrain generation. These motion flags share the
selected CUDA context/ordinal. The precise f64 Euler kernel calculates uniform
acceleration and movement; the CPU still selects planar input/jump velocity,
clamps terminal falling speed and performs the complete authoritative voxel
sweep-and-slide/step-up controller. This is an incremental hybrid implementation,
not GPU-resident character collision or a performance claim.

`physics::step_character_with_motion` and the corresponding voxel adapter preserve
atomic publication and integral anchors. The ordinary CPU controller uses the same
collision boundary. A 240-tick regression checks exact reports and state through
floor, wall and jump behavior, plus nonfinite-motion rollback. All 31 voxel physics
tests and all 16 application binary tests passed; physics_voxel/CUDA library strict
Clippy passed. The headless `cuda_character_probe` is included in both physical
hardware runners and checks 240 real-CUDA integration ticks against CPU state and
voxel contacts. Its separate CPU fixture test confirmed that ground and jumping
are actually exercised. Native and Windows GNU probe checks passed; the runner
PowerShell parser and shell syntax checks passed.

The actual main application invoked with `--cuda-character-motion --cuda-device 0`
on Apple M4 Max returned `DriverUnavailable` with exit 1 before window creation.
NVIDIA execution of the new character path remains unverified. Vehicle/water
physics, GPU-resident voxel collision and the other outstanding full engine and
platform requirements remain open.


### Compute resource ownership across devices and instances (2026-10-01)

`ComputeProgram` now retains the device that created its shader and layout.
`create_job` returns `ComputeError::DeviceMismatch` for unequal device handles
before allocating resources, and always allocates storage/readback/bind groups
on the retained owner. This latter rule matters because wgpu 30 device equality
compares resource IDs that can collide across separate instances. An initial
regression reproduced a cross-instance `BindGroupLayoutId` lookup panic despite
equal device handles; allocating on the retained owner fixes that creation path.
Callers still must submit and poll through the program owner, as documented;
this does not validate arbitrary foreign command encoders or queues.

Two regressions cover unequal devices in one instance, and equal device IDs from
separate instances with an owner-encoded job. All 54 renderer library tests and
strict renderer Clippy passed. Actual Metal compute smoke passed 1042 exact
results, partial workgroups, independent jobs, readback/cancellation and validation
checks. Actual Metal terrain parity passed 502 chunks / 16449536 exact CPU block
comparisons. The portable GPU crate also checked for wasm32. These proofs cover
the compute ownership increment, not the still-open full hardware/platform goal.


### Resident gravity program device ownership (2026-10-01)

The same retained-device allocation rule now applies to `GravityProgram`:
unequal handles return the shared `ComputeError::DeviceMismatch` before upload,
and buffers/bindings are always created on the program owner, including when
separate wgpu instances compare equal due to reused internal device IDs. Both
same-instance unequal handles and cross-instance equal-ID cases have regressions.
Submission and polling still belong to the original owner; foreign encoders are
outside this validation boundary.

GPU crate tests passed (3 executed, 1 existing CUDA host-source test ignored),
strict GPU library Clippy and wasm32 checks passed. Actual Apple M4 Max Metal
execution passed 257 bodies over 128 resident Verlet steps: measured maximum
f64 CPU comparison error 0.000025912606399280946 (allowed 0.0002). The 2560-step
orbit fixture passed with relative energy drift 0.0000008926834292013088;
singular/overflow atomicity and budgets also passed. This verifies the ownership
fix and the existing f32 resident solver, not full voxel-game GPU physics or
physical NVIDIA/Windows/mobile/VR acceptance.


### Explicit API selection in gravity acceptance (2026-10-01)

`gravity_smoke` and `gravity_render` now accept exactly one optional native API
argument: auto, metal, vulkan, dx12 or gl. Unsupported/extra arguments reject
before adapter initialization. The two examples share the parser; their parser
tests and strict example Clippy passed. On Apple M4 Max explicit Metal passed
the orbit/trajectory/error fixtures and three-frame compute-to-vertex pixel
checks. Explicit DX12 returned NotFound with requested_backends DX12, exit 1,
without substituting Metal.

The Linux gravity runner now invokes both numerical and pixel probes with
explicit Vulkan, applies execution timeouts and checks each actual backend log.
The complete runner passed on Mesa llvmpipe Vulkan, including 60 presented window
frames and 200 resident steps, 257 bodies x128 steps, 2560 orbit steps, and exact
physics rendering pixels. This is software Vulkan API execution.

The OpenGL suite now also includes explicit GL numerical/orbit and physics-pixel
probes. Its full actual run passed all six gates: scene, shader replacement,
resident physics window, numerical/orbit fixtures, physics pixels and terrain.
All logs reported Gl on Mesa llvmpipe OpenGL 4.5. Terrain produced 502 chunks with
16449536 exact CPU block comparisons. Numerical max CPU error remained
0.000025912606399280946 and orbit relative energy drift 0.0000008926834292013088.
Logs live under target/linux-docker/opengl-*.log and gravity-vulkan-*.log.
These results prove API selection and software Mesa compatibility, not physical
NVIDIA/DX12/mobile/VR completion or GPU-resident voxel collision.


### Vehicle overflow contract before accelerator migration (2026-10-01)

The current CPU bicycle controller could hide nonfinite intermediate arithmetic:
finite positive mass/forces could overflow division, then speed clamping or brake
sign reversal converted infinity into a finite speed. `step_vehicle` now rejects
nonfinite longitudinal/lateral projection, force integration, heading integration
and final planar velocity with `VehicleError::NumericalOverflow` before querying
voxel collision or publishing state. This establishes an explicit failure contract
for subsequent CUDA control/motion work rather than silently accepting an invalid
physics step.

A regression uses finite extreme engine/brake forces with tiny positive mass,
and tiny positive wheelbase at high speed. A query-forbidden voxel view proves
that these cases reject before any world read; state is exactly preserved.
All 32 voxel physics tests, all 16 app binary tests and strict voxel library
Clippy passed. This is an arithmetic correctness increment; vehicle computation
is still on CPU and has not been moved to CUDA by this change.


### CUDA vehicle chassis motion in the main application (2026-10-01)

`--cuda-vehicle-motion` now opts the main vehicle chassis into the precise f64
CUDA Euler integration used by the character. It shares the selected ordinal
and context with the other motion flags, independently of terrain selection.
Bicycle engine/brake/drag/grip/steering calculations and voxel collision remain
CPU work. This is hybrid chassis integration, not an entirely GPU vehicle solver
or GPU-resident collision. No speedup is claimed.

The new application `cuda_motion::step_character` helper is shared by player,
vehicle and their acceptance probes; the character probe no longer duplicates
production integration glue. `step_vehicle_with_chassis_step` executes a selected
controller against a private candidate vehicle and only publishes after success.
A regression deliberately changes that candidate then returns an error, proving
that heading, speed and chassis remain exactly unchanged. Runtime vehicle errors
now propagate to the application's final failure result as well as exiting the
window loop. No failed requested CUDA path silently falls back to CPU.

`cuda_vehicle_probe` compares 240 ticks of the actual shared CUDA chassis path
against CPU vehicle state and voxel contacts. It is added to both Linux and
Windows hardware acceptance scripts. Its separate CPU fixture test exercised
forward movement, braking to zero and reverse movement. All 33 voxel tests,
all 17 application binary tests, and the character/vehicle fixture tests passed.
Windows GNU checked the binary and both CUDA probes with CUDA enabled; the
application library checked for wasm32. Strict voxel Clippy passed. Strict app
library/probe Clippy currently reports 193 errors in other application modules;
JSON diagnostic filtering found no primary errors in the new motion/probe files.
This does not make the overall application Clippy gate green.

The actual main application with `--cuda-vehicle-motion --cuda-device 0` returned
DriverUnavailable/exit 1 on this Apple host before creating a window. The physical
NVIDIA vehicle execution gate remains unverified. CPU water, GPU-resident voxel
collision and the outstanding platform/RTX/VR requirements remain open.


### Integer GPU liquid transfer pass (2026-10-01)

`WaterTransferProgram` now runs the existing downward-then-horizontal water rule
as an integer WGSL graph pass. It deliberately executes one ordered invocation,
so adjacent active cells observe earlier transfers in canonical node order.
Amounts remain eighths of a voxel (0..8), impermeable registered cells remain
separate, and missing neighbors fail only when actually required. Unique sample
budgets precede missing-neighbor errors, matching the CPU read ordering. The
private output is rejected on sample/write limits, malformed data, changed solid
classification or nonconserved volume; no world transaction is published by this
API. Limits are 131072 graph nodes and 16384 sorted unique active indices.

`water_smoke` maps one actual 32-cubed bootstrap chunk into the adjacency graph
and compares every returned GPU amount to the existing CPU `step_water` plus
committed world transaction after each of 16 ticks. Actual Metal execution passed
524288 exact cell comparisons, preserved volume and canonical neighbor transfers.
Additional actual GPU probes passed lazy empty-cell reads, missing neighbors,
sample/write budgets, error precedence and successful independent recovery.
Strict water library/example Clippy and wasm32 library checks passed. The step
API currently uses native synchronous readback; browser nonblocking execution
and main-application world-transaction integration remain open. CUDA water is
also not implemented by this WGSL increment. No parallel performance claim is made.

`tools/linux/water-smoke.sh` passed the same error contracts and 16-tick parity
on explicit Vulkan and OpenGL 4.5 Mesa llvmpipe. Those are software API runs.
Its first attempt exposed a concurrent workspace Rush dependency on the sibling
tokenizer checkout. Seven existing Cargo-based Linux runners now mount that
checkout read-only at /Sources/tokenizer, with VOXY_TOKENIZER_ROOT override and
an explicit missing-checkout failure. All nine Linux runner shell syntax checks
passed; the new water runner executed with the mount and passed both APIs.
Other runners were not rerun solely for this mount change. The full requested
engine/hardware/platform goal remains incomplete.


### GPU water exact read provenance (2026-10-01)

`WaterTransferProgram::step_detailed` now returns both amounts and a node-order
mask of successful lazy reads. The existing `step` amount-only API is preserved.
Readback validates boolean mask encoding, exact unique sample count and that every
changed node was read. This is the metadata required to derive expected chunk
revisions from captured world snapshots, including unchanged and impermeable cells.
It does not itself capture world snapshots or publish transactions.

The actual `water_smoke` CPU oracle now instruments VoxelView sampling and compares
the exact set of GPU-read voxel positions to CPU reads on every one of the 16
ticks. For transactions it additionally compares sampled chunk coverage to the
CPU expected-revision list. A lazy-empty fixture proves that an unneeded second
node is not reported as read. Actual Metal and explicit Mesa Vulkan/OpenGL runs
passed these read-set checks together with 524288 exact amount comparisons and
the previous error contracts. Strict GPU library/example Clippy passed. World
snapshot/revision capture and main application water transaction integration remain
open; the full engine/hardware goal is not complete.

### GPU water transactions from frozen chunk snapshots (2026-10-01)

`voxy_gpu::WaterWorldSnapshot` now captures immutable chunk data together with its original revisions, maps canonical active positions to the graph, and converts detailed device output into a `WaterPlan`. Only chunks actually sampled by the shader contribute expected revisions. The adapter rejects changed unread cells, malformed output, changed solid classification, volume loss, write/sample budget violations and next-active coordinate overflow before any world write.

The water smoke example now compares the entire GPU-derived transaction and next-active set with the CPU oracle, then commits the GPU-derived transaction. Actual Metal on Apple M4 Max and Mesa software Vulkan/OpenGL each passed 16 ticks / 524288 exact cell comparisons, exact read provenance and lazy-read/error checks. A focused unit test proved that an intervening edit in a captured chunk makes the old GPU plan conflict without partially transferring water; recapturing permits the transfer. Crate-only strict Clippy passed; dependency-inclusive Clippy currently fails on 177 diagnostics in other physics modules.

This is a loaded-graph transaction adapter and native compute proof. Main gameplay still uses the CPU water planner. Sparse/unloaded graph handling, browser asynchronous execution and physical NVIDIA/Windows/mobile/VR acceptance remain unfinished; this evidence does not establish complete hardware support or parallel water speedup.

The loaded snapshot adapter also supports `capture_active`: canonical deduplicated active cells plus their five potential transfer destinations, with active budget and checked-coordinate validation. Every nonempty tick in the native water smoke now runs both sparse and full graphs and requires identical complete transactions, expectations and next-active sets before committing. This passed on actual Metal and Mesa software Vulkan/OpenGL. The focused snapshot regression also verifies deduplication and active-budget rejection; crate-only strict Clippy passes. Eager rejection of unavailable or unknown data in the loaded capture stage still needs lazy semantics before main-game integration.

### Lazy unavailable water graph nodes (2026-10-01)

`WaterTransferProgram::step_detailed_available` adds an explicit availability mask while preserving the existing loaded-graph API. Unavailable nodes must have `None` amounts; their packed sentinel differs from registered solid blocks. The ordered shader checks the sample budget before reporting a required unavailable read and ignores unavailable cells it never reads. Native smoke checks verified both cases on Metal and Mesa software Vulkan/OpenGL, while retaining full/sparse transaction equality. Readback validation rejects changed unavailable nodes, visited unavailable nodes and corrupted immutable graph structure; a focused corruption test passes.

The world snapshot layer still rejects unavailable capture data eagerly. Passing the mask through world capture, preserving typed unknown-state/coordinate errors and enabling the path in main gameplay are pending. Browser compilation alone is not browser execution proof.

### Availability retained by frozen world capture (2026-10-01)

`WaterWorldSnapshot::capture_available` and `capture_active_available` now retain missing chunks as unavailable graph nodes with an explicit mask. Each missing chunk is queried once per capture; loaded data and revisions still come from the same immutable chunk snapshot. A mismatched chunk identity remains an error. Planning rejects any successful result claiming a read of an unavailable node, including forged otherwise-settled output.

Actual Metal smoke verifies loaded empty active cells can settle despite an unrelated missing chunk, both in full and sparse capture; activating the missing cell fails on the device. The full capture case also passed Mesa software Vulkan/OpenGL. Existing 16-tick full/sparse/CPU transaction equality continues to pass. Unknown loaded block states and coordinate overflow still fail eagerly at capture; typed lazy errors and main gameplay integration remain pending. Full platform and physical CUDA acceptance remain open.

### Lazy coordinate-overflow water edges (2026-10-01)

A water graph edge encoded as `Some(u32::MAX - 1)` now represents coordinate overflow separately from missing graph data. The shader returns typed `WaterComputeError::CoordinateOverflow` only when traversing this edge, before incrementing the sample count, matching CPU offset-before-sample ordering. Empty or solid source cells never traverse it. `capture_active_available` omits unrepresentable neighbor positions while retaining these error edges; ordinary strict loaded capture remains strict. Post-transfer next-active overflow remains an atomic planning failure.

Actual Metal and Mesa software Vulkan/OpenGL smoke checks pass both ignored-overflow and required-overflow cases, including precedence over an exhausted sample allowance. All existing water equality checks remain green. Crate-only strict Clippy and WASM compilation pass. Unknown-state lazy errors and main gameplay integration are still pending, as is physical CUDA and full platform acceptance.

### Lazy unknown water node classification (2026-10-01)

`WaterNodeStatus::{Loaded, Unavailable, Unknown}` and `step_detailed_status` distinguish registered solids from missing chunks and unknown loaded block states. An unknown read fails after sample-budget checking, with typed `WaterComputeError::UnknownNode(index)` identifying the actual first failing graph node. Unread unknown nodes remain unchanged and do not fail the dispatch. Error output identifies the failing index; success readback rejects changes/reads of either fault classification.

Availability-aware world capture now retains unknown IDs, exposes full `node_status`, and provides `unknown_state(index)` for translating device errors back to the original block ID. The boolean compatibility mask treats unknown nodes as unavailable; callers needing typed errors must use the status API. Strict capture still rejects unknown IDs eagerly.

Actual Metal and Mesa software Vulkan/OpenGL smoke tests passed unknown-source/unknown-destination errors, budget precedence, unread unknown nodes and successful recovery after failure, together with the prior 16-tick equivalence checks. Crate-only strict Clippy passed. A malformed-world snapshot fixture for this classification, a unified world-step adapter and main-game wiring remain pending. Physical CUDA/full platform acceptance remains open.

### Native GPU world-water adapter and gameplay opt-in (2026-10-01)

`WaterTransferProgram::plan_world` validates states/budgets, captures the sparse immutable world graph, dispatches ordered integer transfers, translates unknown-node/coordinate/sample errors, and returns a revision-checked `WaterPlan` without invoking the CPU solver. Empty batches settle without device work. The water smoke requires the adapter's complete plan to equal both the full GPU graph and CPU oracle on every nonempty tick. Actual Metal and Mesa software Vulkan/OpenGL pass these checks; crate-only strict Clippy passes.

Main gameplay now accepts `--gpu-water`, creates the shader on the renderer's device, and uses its queue for each scheduled water batch. Failed explicit GPU initialization and simulation/commit errors propagate to the process exit status. The CPU default remains available. Native readback is synchronous, and the ordered shader is not parallel throughput proof.

Actual Apple M4 Max/Metal graphical launch completed a GPU water tick for 8217 natural active cells and presented 126 frames in the final run, with movement, camera and two explosions. Autopilot exited 1 because water commits stayed zero; its existing water-motion gate was preserved. Thus startup and a real gameplay GPU dispatch are proved, while full gameplay acceptance remains failed. The earlier run presented 1164 frames but also had zero water commits. WASM compilation passed after a transient concurrent physics compilation error was resolved in the current shared tree. Physical NVIDIA/DX12/mobile/VR proof and full-engine completion remain open.

### Natural-water gameplay acceptance (2026-10-01)

The graphical autopilot now explicitly destroys one existing destructible block beneath naturally generated water at tick 240, using `plan_explosion` with radius zero, authoritative commit and the ordinary water wakeup path. It selects a loaded margin around that real terrain block during startup; it neither inserts water nor changes the world generator. Missing/no-longer-destructible candidates fail the scenario. The existing water-commit acceptance gate remains required.

Actual Metal/Apple M4 Max with `--gpu-water --autopilot` passed: 7 GPU water ticks, 5 water commits, 3 destruction commits, 599 animation frames and 119 presented frames. The CPU/default scenario also passed (8 water commits, 184 frames). All 18 main-binary tests pass; the new procedural-world regression proves the release leaves the original water source unchanged, removes only its existing support, wakes it and produces a water write into that bed. These counts prove the scenarios, not identical CPU/GPU timing; exact plan equivalence is covered separately by water smoke.

`sh tools/linux/gameplay-water-smoke.sh` is a new bounded acceptance runner requiring explicit graphics API, real GPU-water dispatch, destruction of the natural bed, successful process exit and nonzero water commits. Logs are retained under `target/linux-docker/gameplay-water-{vulkan,gl}.log`. Current Mesa software Vulkan passed with 7 GPU ticks, 5 water commits and 735 presented frames; OpenGL is being checked separately. Physical NVIDIA/Windows/mobile/VR and full hardware support remain open.

The same runner subsequently passed Mesa software OpenGL: 7 GPU ticks, 5 water commits, 3 destruction commits and 737 presented frames. Both API runs completed with exit zero and the runner's required markers. This verifies the full graphical scenario through software Vulkan/OpenGL, not physical Linux GPU coverage.

### Combined native NVIDIA gameplay acceptance gates (2026-10-01)

Both physical NVIDIA acceptance runners now build the water smoke example and main application. Windows adds explicit DX12 water-transaction equivalence and a combined gameplay run with `--gpu-water --cuda-terrain --cuda-projectiles --cuda-character-motion --cuda-vehicle-motion --cuda-device <ordinal> --autopilot`; Linux uses explicit Vulkan. The gates require CUDA startup/terrain evidence, an actual GPU-water tick, destruction of existing natural terrain under water, successful autopilot water commits and process exit zero. These gates supplement the existing dedicated CUDA math, interop, shader pixel and window probes rather than replacing them.

PowerShell parser and POSIX shell syntax checks passed. Both runners rejected this macOS host with exit 2, as intended; no physical CUDA acceptance was executed. `cargo check -p voxy_app --bin voxy_app --features cuda --target x86_64-pc-windows-gnu` passed on the current shared tree after a transient concurrently missing physics module appeared. This is a cross-compilation check, not linking/running proof or validation of DX12 WGSL pipelines. Physical Windows/NVIDIA and Linux/NVIDIA runs remain required.

### Nonblocking GPU water graph submission (2026-10-01)

`WaterTransferProgram::begin_step_detailed_status` validates and submits work, starts mapping and returns `PendingWaterTransfer` without polling or waiting on the device. `try_result` returns pending/completed/error through the existing callback-driven readback and applies the same structural/provenance/volume validation. It is single-consumption, including solver errors. Native synchronous `step` APIs now wrap this submission path and wait explicitly, preserving their existing behavior. Browser callers can use the pending graph API while yielding to their event loop.

Actual Metal and Mesa software Vulkan/OpenGL smoke passed independent successful/failed pending submissions, consumed-result rejection and recovery after dropping a readback, plus all prior water equivalence/error checks. Crate-only strict Clippy and WASM compilation passed. Linux and physical NVIDIA runner gates now require the pending-readback marker; shell and PowerShell syntax checks passed.

This enables a nonblocking graph API, not finished browser world-water integration: a pending world-plan adapter retaining captured revisions and an actual browser event-loop execution probe remain pending. Native gameplay still uses synchronous world planning. Physical DX12/NVIDIA/mobile/VR and full-engine completion remain open.

### Pending world-water transactions (2026-10-01)

`WaterTransferProgram::begin_plan_world` now returns `PendingWaterPlan`, retaining the immutable sparse chunk graph, original revisions, edit source and budgets until device mapping finishes. `try_plan` never polls or waits: it returns pending, a revision-checked transaction or a typed error, and consumes both success and failure. Empty batches settle immediately without submission, with the same single-consumption contract. Synchronous and pending world paths share compute-error translation.

Actual Metal smoke submitted a world tick, changed an unrelated voxel in its captured chunk before completion, verified the completed plan exactly matched the pre-edit CPU oracle, and confirmed commit rejected stale revisions while preserving the water source. Fresh capture then matched the new CPU plan and committed successfully. Empty pending plans reject repeated consumption. Crate-only strict Clippy and WASM compilation passed. The Linux runner now requires this pending-world proof marker; its first attempt was dependency-lock blocked by concurrent tokenizer work, and it is being rerun after lock resolution.

Real browser event-loop execution and integration of pending plans into the browser shell remain pending. Physical CUDA/DX12/mobile/VR and full-engine completion remain open.

The stable-file Linux rerun completed with exit zero: both Mesa software Vulkan and OpenGL passed pending-world revision conflict/fresh recovery and all prior water checks. An intervening run had completed both probes but the runner itself exited 2 because its source was edited while shell execution was still active; that run was not treated as runner acceptance.

### Actual browser GPU water execution (2026-10-01)

The browser shell exports `WebEngine::validate_water` and the demo runs it with `?backend=webgpu&waterCheck=1`. The validation builds a real fixture world, submits a pending GPU plan, changes a captured chunk before completion and requires revision-conflict rejection. It then performs 16 event-loop-driven world ticks, compares every complete GPU plan with the CPU oracle, commits the GPU transaction, and checks single consumption. Readback yields through browser timers with a bounded deadline; no native blocking device poll runs.

Actual Codex in-app browser execution showed `BrowserWebGpu · water verified: 16 commits`; subsequent 2D/3D rendering continued (2877 frames observed at the initial screenshot). The proof screenshot is `docs/browser-water-proof.png`. This establishes browser WebGPU dispatch/readback and world transaction correctness for the fixture, not physical adapter identity or voxel gameplay rendering. Explicit WebGL compute requests show `water compute unsupported on WebGL`, preserving the rendering-only WebGL capability boundary.

WASM build and crate-only strict Clippy passed. A matching wasm-bindgen-cli 0.2.127 was installed under ignored `target/tooling/wasm-bindgen-0.2.127` and used for bindings, leaving the existing CLI untouched. The demo is served locally on 127.0.0.1:8793; the occupied 8774 service was preserved. Full browser gameplay and physical CUDA/DX12/mobile/VR acceptance remain open.

### Shared stereo GPU exposure (2026-10-01)

`AutoExposure::prepare_stereo` meters the union of two equally sized linear eye textures. Both eyes contribute finite positive luminance to the same logarithmic mean and adaptation output; black or invalid samples are excluded. One `AutoExposureFrame` can feed both display passes through `TextureBlit::with_auto_exposure`, preserving each eye image with the same gain and no CPU exposure readback. The caller retains only the last successfully presented stereo candidate and resets history on tracking/session loss. Mismatched dimensions and aggregate sample-count overflow are rejected before allocation.

The GPU verifier uses eye luminances 1 and 4: joint geometric luminance 2 produces exposure 0.09, then direct GPU linear display yields 0.09 and 0.36. It also verifies single-valid-eye metering and all-invalid stereo history retention. Native Apple M4 Max Metal execution, focused strict Clippy and Windows library cross-compilation passed. This is an offscreen stereo fixture; headset/OpenXR presentation and physical RTX 5090 inference remain unverified.

The Linux HDR runner also passed the shared-stereo fixture and existing HDR/blit regressions on both Mesa llvmpipe Vulkan and OpenGL (software CPU execution).

Stereo temporal exposure acceptance additionally covers an encoded-but-unpresented candidate, next-frame adaptation from the last presented pair rather than the skipped candidate, explicit history reset, zero-time retention, and mismatched-eye rejection. These GPU checks passed on Metal and Mesa software Vulkan/OpenGL; focused strict Clippy passed. The fixture exercises caller-selected presentation history, not an automatic OpenXR session commit.

### Nonblocking GPU water in main gameplay (2026-10-01)

Main `--gpu-water` now submits `begin_plan_world` and retains its source batch in `PendingGameWater`. Subsequent fixed updates drive nonblocking native device polling and read the pending plan; the game no longer waits synchronously for water mapping. New edits append activations while work is in flight. Successful publication merges next-active cells with the queued work; a revision conflict restores the original source batch, deduplicates/sorts the queue and schedules fresh capture. A 30-second pending deadline fails explicitly instead of waiting forever. Other simulation/commit errors remain fatal. CPU/default water uses its original direct planner.

All 19 main-binary tests passed, including a new publication regression proving stale revisions do not lose concurrent activations, fresh capture commits the transfer and settled publication retains deferred work. Actual Metal gameplay passed with 7 GPU ticks, 5 water commits and 1180 presented frames. The bounded graphical Linux runner passed Mesa software Vulkan (832 frames) and OpenGL (823 frames), each with 7 GPU ticks and 5 water commits. Strict crate-only GPU Clippy, Windows GNU/CUDA main cross-check and tracked diff whitespace checks passed. These are correctness and scheduling proofs, not measured frame-latency improvements.

Physical NVIDIA/DX12/mobile/VR acceptance, full browser gameplay and the broader engine objective remain unfinished.

### OpenXR projection submission validation (2026-10-01)

Stereo layer submission now rejects singular horizontal/vertical fields of view and angles at or beyond ±pi/2 before calling the runtime. Previously, only finite angles and valid poses were checked at this boundary, unlike the renderer projection check. Legal flipped frusta remain accepted. Since validation precedes `end_frame`, rejection preserves the pending frame for correction or empty-layer submission. Nine XR library tests and strict crate Clippy passed, as did Windows and Android library cross-compilation. No headset runtime was exercised; automatic exposure/motion commit coupled to native stereo presentation remains pending.

### OpenXR temporal-history submission boundary (2026-10-01)

`XrSession::end_frame_with_history` now submits nonempty composition layers before replacing an application-owned `Option<T>` history. Bundle the candidate `XrMotionHistory` and shared `AutoExposureFrame` in a single tuple/struct: prepare motion/exposure from the last submitted tuple, encode and complete both eye GPU work, release swapchain images, build the projection layer, then call this method with the candidate tuple. A runtime or validation error preserves the old tuple and discards the candidate. A successful call replaces both histories together. Candidates must match the submitted layers; the API cannot inspect opaque application data or prove GPU completion.

Empty-layer frames continue through ordinary `end_frame` and do not advance temporal history. Tracking loss, reference-space changes, session changes and swapchain reconfiguration still require explicit history reset by the application. The runtime-result unit test verifies failure retention, success replacement and failure with absent history. Ten XR library tests, strict Clippy and Windows/Android cross-compilation passed. This adds a native submission path with success-gated history ownership, but no physical headset or complete GPU/OpenXR application loop was executed.

### Stereo projection plus temporal history API (2026-10-01)

`StereoSubmission` bundles runtime blend mode, reference space, the exact left/right views, released swapchain subimages and composition flags while retaining their borrows. `XrSession::end_stereo_frame_with_history` builds/submits the existing validated stereo projection layer and replaces the combined application history only after success. It reuses the original stereo submission implementation, including foreign-space, eye validation and pending-frame checks, rather than a separate projection builder.

A compile-tested API example shows direct stereo submission with history ownership. Ten library tests, the doctest, strict Clippy and Windows/Android library checks passed. Rendering, GPU completion, image release and tracking/reference-space reset remain application responsibilities. No physical headset loop was run, and the candidate's opaque data is not automatically checked against its eye images.

### Current mobile compile gates and explicit Android device selection (2026-10-01)

The current `voxy_gpu` crate passed cross-checks for `aarch64-apple-ios` and `aarch64-linux-android`; the full `voxy_mobile` shell, including shared application/render code, also passed both targets. This is compile evidence, not shader execution, linking, signed-app installation or device lifecycle proof.

Android packaging now supports `--install --device SERIAL`. It checks online adb devices before building/installing, selects automatically only when exactly one device is online, and rejects ambiguous, missing, offline or unauthorized selection. Installation and waited activity launch both use the selected serial explicitly. All nine Android packaging/selection tests passed, including existing ELF architecture and 16 KiB segment checks.

This host lacks configured Android SDK/NDK (`build.py --check` rejects missing prerequisites) and `xcrun simctl` is unavailable. No APK build or iOS/Android execution was claimed. Physical mobile graphics/compute, lifecycle, input and presentation acceptance remain required, together with the other uncompleted full-engine requirements.

### Runtime-timed XR history invalidation (2026-10-01)

`XrHistoryReset` owns the selected reference-space type and pending origin-change timestamps. Pass routed `XrSessionEvent` values to `handle_event` with the application's combined history; before preparing either eye, call `begin_frame` with predicted display time and both eyes' tracking validity. Matching reference-space changes invalidate history at the first frame at/after their effective time, with multiple future changes retained and duplicate timestamps coalesced. Changes for unrelated reference-space types do not invalidate this history. Lifecycle restarts/stops/loss, instance loss and lost events clear history immediately; invalid tracking clears before rendering. Interaction-profile changes preserve camera/exposure history.

Tests cover out-of-order future events, duplicate events, exact effective time, skipped-over time, unrelated space, tracking loss, lifecycle loss and harmless profile changes. Twelve library tests, strict Clippy and Windows/Android cross-compilation passed. Application event/frame-loop wiring and physical headset acceptance remain pending.

### Session-owned temporal invalidation entrypoints (2026-10-01)

`handle_event_with_history` routes native events through session lifecycle handling and applies the resulting invalidation to the combined application history. Lifecycle failures conservatively clear that history. `locate_stereo_views_with_history` obtains poses at the pending frame's predicted time and applies tracking/effective-origin invalidation before returning them for rendering. Runtime-skipped frames preserve history unless an origin change becomes due; renderable frames without valid tracking and runtime location failures clear it. Calls outside a running frame clear history and return the existing lifecycle error. These methods compose with `end_stereo_frame_with_history` for success-only commit.

Thirteen library tests, strict Clippy and Windows/Android library checks passed. A specific regression test distinguishes runtime render skips from tracking loss and location errors. The application must select these entrypoints, supply the reset owner's matching reference-space type and bundle its histories; no full renderer/headset loop was run.

### Validate located eyes before rendering (2026-10-01)

`locate_stereo_views` now validates both runtime eyes after tracking-valid flags, rejecting nonfinite positions, nonunit quaternions and invalid frusta before publishing either eye. Previously these values were checked only at layer submission, after rendering could already consume them. The history-aware location entrypoint consequently resets combined history on malformed located data. Invalid-tracking poses remain unpublished without validating their unspecified numeric contents. Fourteen library tests, strict Clippy and Windows/Android checks passed; tests corrupt each eye independently and confirm invalid-tracking numeric values remain ignored. No headset runtime was exercised.

### Rough specular BRDF evaluation (2026-10-01)

`ReconstructionMaterial::ggx_reflection` evaluates isotropic Trowbridge–Reitz/GGX reflection with correlated Smith masking and Schlick Fresnel. Perceptual roughness maps to alpha=roughness². Evaluation uses f64 intermediates and a stable normal-distribution denominator near normal incidence, returning finite representable RGB BRDF values per steradian. It rejects delta roughness and invalid directions, returns zero below the surface, and does not confuse BRDF values with RR reflectance guides or BSDF*cosine/PDF sampling weights. Model equations are grounded in https://www.pbr-book.org/4ed/Reflection_Models/Roughness_Using_Microfacet_Theory .

The analytical normal-incidence peak, reciprocity, normal rescaling, back-facing light, invalid input and delta rejection test passed. Strict library Clippy and Windows library cross-compilation passed. Rough reflection ray sampling, PDF handling, GPU evaluation and multi-bounce integration remain pending; the existing GPU surface-reflection path still handles ideal mirrors only.

### Parallel integer voxel-region broadphase foundation (2026-10-01)

`VoxelRegionProgram` classifies inclusive regions in a dense canonical x/y/z grid, with one GPU invocation per query. It returns solid counts, the first solid candidate and the first unknown/unavailable cell index. Empty/solid/unavailable/unknown classifications remain distinct input data. Coordinates are local unsigned integers, permitting callers to retain far-world i64 anchors externally without float conversion. Immutable grid/header/query fields are checked on readback. The API bounds dimensions, grid/query lengths and aggregate work (131072 cells, 16384 queries, 4194304 cell visits), rejecting invalid inputs before submission.

Actual Metal and Mesa software Vulkan/OpenGL passed 257 queries on a non-cubic 7x9x11 grid against an independent CPU enumeration, plus invalid shape/region/work-budget rejection and successful recovery. `sh tools/linux/voxel-regions-smoke.sh` retains explicit backend logs. WASM compilation and crate-only strict Clippy passed. This initial query API uses native synchronous readback and is not yet a frozen-world adapter, asynchronous browser path, collision contact solver or CUDA collision kernel. Precise continuous sweep/contact resolution remains CPU work; full GPU collision and physical platform acceptance remain unfinished.

### GGX furnace acceptance (2026-10-01)

A deterministic 128×128 solid-angle midpoint quadrature integrates the implemented BRDF times incident cosine over the hemisphere without reusing GGX sampling/PDF. It checks dielectric/metallic white materials at roughness 0.4/0.7/1 and view cosines 0.05/0.25/0.7/1 against the unit-energy bound with 1% quadrature allowance. The normal-view alpha=1, F0=1 case also matches the independent analytic integral 1-ln(2) within 1e-5. Both GGX tests passed. This is sampled CPU model validation, not proof for all roughness/directions or a GPU renderer.

Strict library Clippy passed. The broader test-target Clippy attempt exposed existing unreadable literals, float equality checks and items-after-test-module errors in other rendering tests; that broader gate is not passing. Rough ray sampling and GPU integration remain outstanding.

### GGX NDF reflection direction sampling (2026-10-01)

`sample_ggx_reflection` samples an isotropic GGX microfacet normal, reflects the surface-to-camera direction and returns the world-space direction, solid-angle PDF and BRDF*cosine/PDF throughput. Uniform inputs must be in [0,1). Below-surface/back-facing results return None and retain their zero-contribution probability mass; callers must not retry or renormalize accepted samples. This is full NDF sampling, not VNDF, and weights may exceed one at grazing angles. Delta roughness remains a separate mirror path.

A deterministic sampled furnace at alpha=1 and normal view matches independent analytic energy 1-ln(2), including exactly half null samples, within 1e-4. Three GGX tests, strict library Clippy and Windows compilation passed. GPU rough reflection integration and weights above one's HDR resource path remain pending. PDF/weights use f64 intermediates while returned direction and BRDF evaluation use f32-facing APIs; extremely narrow lobes need further numerical acceptance before production integration.

### Stable GGX sampling throughput (2026-10-01)

GGX sampled throughput now cancels the common NDF factor analytically before computing Fresnel × correlated-Smith masking × view-half-cosine/(view-normal-cosine × normal-half-cosine). It no longer divides a rounded f32 BRDF by a potentially huge PDF. The sampling PDF denominator uses the directly computed sine squared instead of subtracting cosine squared from one, retaining narrow-lobe precision.

Four GGX tests passed, including finite PDFs and mirror-limit weights at roughness 0.001, 1e-10 and f32::MIN_POSITIVE. Strict library Clippy passed. Directions are still returned as f32, so sub-f32 angular spread cannot be preserved; the tested narrow-lobe weights/PDF remain finite. The GPU weighted reflection API still limits weights to one and needs a wider HDR resource path before full rough-reflection integration.

### Wide HDR weighted ray output (2026-10-01)

`SpecularDistancePipeline::new_hdr` uses RGBA32Float storage output for weighted reflected radiance. Its weighted-job validation permits finite nonnegative throughput above one, conservatively capped at f32::MAX/65536 so multiplication by the existing emission-table maximum 65504 remains representable. The original constructor keeps RGBA16Float and its weight cap of one. Both paths share ray validation, binding construction and shader logic; only the radiance storage format differs.

Actual Apple M4 Max Metal ray execution verified weight 2 × emission 65504 = finite 131008 in RGBA32Float, exceeding binary16's maximum. Existing closest-hit/miss/range/bias, shadow, guide/material composition and TLAS regressions passed in the same probe. Strict render-library Clippy and Windows library cross-compilation passed. This enables wider weighted ray jobs; automatic GGX sample-to-ray integration and rough GPU primary-surface sampling remain pending. Emission table validation still caps individual incoming values at 65504.

### Nonblocking voxel region readback

`VoxelRegionProgram::begin_classify` now submits without GPU waiting and returns an owned `PendingVoxelRegions`. `try_result` yields pending or a single validated result; native `poll_native` advances callbacks without waiting. The synchronous API delegates to the same submission/decode path. Readback validates query volume, candidate membership and class, immutable inputs and count/candidate consistency.

Verified on Apple M4 Max Metal and Mesa llvmpipe Vulkan/OpenGL: 257 queries match the independent CPU enumeration, two simultaneous readbacks complete, consumed results reject reuse, and dropping an in-flight readback preserves remaining work. Focused corruption test, strict GPU library/example Clippy and wasm32 compilation pass. WASM compilation is not browser runtime proof for this new API. Frozen-world revisions and collision integration remain to be implemented; continuous sweep/contact resolution remains external.

### Frozen world inputs for voxel broadphase

`VoxelRegionSnapshot` captures a bounded dense collision grid from each chunk exactly once, retaining chunk data/revisions and integer world anchors. Unknown registry IDs and absent chunks produce fault classifications. Candidate indices map back to checked i64 world coordinates. `is_current` conservatively rejects revision changes, data identity replacements, unloads and missing-to-loaded transitions; callers must prevent edits between validation and publication. `begin_classify` submits the immutable grid through the existing nonblocking compute API.

The focused snapshot test verifies an anchor above 2^53, canonical index mapping, one chunk capture, stale revisions, same-revision data replacement, unloading/loading and extent-overflow rejection before reads. This is a world adapter, not yet a collision solver integration.

### Coherent GGX sample-to-GPU-ray path (2026-10-01)

`GgxSurfaceSample` derives a world-space reflected ray and Monte Carlo weight from the same validated surface/material/random pair. `create_ggx_job` requires RGBA32Float output and preserves row-major optional samples, encoding null samples with an internal validity marker. The compute shader writes zero radiance/distance for these entries before ray-query initialization. Existing weighted public jobs normalize unused input alpha to a valid marker, retaining their previous RGB-weight behavior.

Actual Apple M4 Max Metal execution verified a rough-material sample's coherent ray hit and BRDF weight, plus sampled/explicit null entries writing black and zero distance. Existing wide-HDR, ray/shadow/guide/TLAS checks passed. Strict render-library Clippy and Windows library compilation passed. Direction sampling remains CPU-side; GPU primary-surface rough sampling, temporal sample accumulation, general scene shading and multibounce paths remain incomplete. The checked rough sample used the normal-facet random endpoint; this proves plumbing/null semantics, not a full rough-lobe image.

### Nonzero GGX ray-deflection acceptance (2026-10-01)

The GPU GGX fixture now uses roughness 0.5 and uniform sample (0.25,0), producing a nonzero reflected angular deviation. Its independent analytic oracle derives tan²(theta)=1/48, reflected z=47/49 and intersection distance 98/47 to the emitter plane; correlated Smith masking and Fresnel give separate expected RGB values. Apple M4 Max Metal execution matched these values, retained null-sample black/zero outputs and passed the existing ray/shadow/material/TLAS regressions. This replaces the earlier normal-facet endpoint-only fixture. Sampling still runs on CPU and no accumulated rough-lobe image or physical RTX runtime has been verified.

### World snapshot GPU execution proof

The voxel-region smoke probe now captures the real bootstrap world, independently enumerates 32768 CPU collision cells, submits the frozen grid, commits a solid-to-air edit while the request is pending, and verifies the GPU result still matches the original snapshot. The snapshot rejects publication against the changed world; a fresh capture sees one fewer solid and a new first candidate. A missing-world query anchored at 9007199254740993 verifies exact first-fault index conversion back to i64 coordinates.

Both probes passed on physical Apple M4 Max Metal and software Mesa llvmpipe Vulkan/OpenGL. The Linux runner requires their PASS markers; strict example Clippy passed. This validates the capture/dispatch/readback/revision boundary, not continuous GPU sweep or contact integration.

### GPU-assisted continuous voxel sweep

`begin_sweep` uses the shared validated `sweep_candidate_bounds` from physics_voxel, captures integer collision inputs and submits broadphase asynchronously. `PendingVoxelSweep::try_sweep` rejects changed world snapshots, returns clear movement for GPU-empty regions, and sends solid/fault regions to the existing exact f64 continuous CPU solver. It retains the immutable registry used at submission. World edits must be serialized during publication. Dense GPU dimension limits remain explicit errors.

The smoke probe verifies exact SweepResult parity for clear movement, downward terrain collision, loaded/unloaded boundaries and far i64 anchors, plus consumed result and stale-world rejection. Physical M4 Max Metal passed; original four CPU collision tests and strict GPU/example Clippy passed. This is a hybrid API; it does not claim GPU narrowphase, application-loop integration or measured acceleration.

### GPU primary-surface GGX sampling (2026-10-01)

`SurfaceReflectionJob::with_ggx_material_map` now selects GGX NDF directions directly in compute from GPU primary normal/roughness and matching F0 texture. A caller seed produces deterministic per-pixel uniforms; reflection ray queries and Fresnel/Smith sampling weights execute without CPU surface readback. Null samples output black/zero distance. The path writes RGBA32Float and uses the delta limit for roughness below 0.001. The options' uniform material is unused in this overload; F0 comes exclusively from the map. Existing mirror overloads retain their behavior.

Actual Apple M4 Max Metal execution of roughness 0.5 on a 4×4 primary fixture produced finite positive HDR values; mirror/shadow/material/TLAS regressions passed. Strict library Clippy and Windows library checks passed. This is a compute-execution smoke, not an independent per-pixel GGX oracle or accumulated rough image. Detailed GPU/CPU agreement, extreme grazing/narrow-lobe numerical acceptance, temporal accumulation and multibounce/general scene shading remain pending.

### NVIDIA acceptance collision coverage

Windows/NVIDIA acceptance now builds `voxel_regions_smoke` and requires explicit DX12 execution with world snapshot correctness, exact hybrid sweep parity, far integer coordinates, stale-world rejection, concurrent/dropped readback recovery and 257-query CPU parity markers. Linux/NVIDIA acceptance adds the same requirements on explicit Vulkan. Each probe must exit successfully under the existing timeout.

Shell syntax and PowerShell parser checks passed; the complete GPU collision probe cross-check passed for x86_64-pc-windows-gnu with the CUDA feature. Both runners reject this macOS host with exit 2 before building or claiming physical CUDA success. Physical NVIDIA/DX12/Vulkan acceptance remains unexecuted.

### Per-pixel GPU GGX acceptance against CPU model (2026-10-01)

The GPU primary-GGX verifier now checks each RGB value and hit distance against CPU f64 GGX sampling and analytic intersection with the emitter triangle. Shared deterministic hashing selects the same uniforms, while direction/weight evaluation is independent CPU code and ray intersection uses explicit plane/triangle bounds. Expected F0 accounts for the guide texture's binary16 quantization. Radiance tolerance is 0.003 absolute and distance tolerance 0.001.

Actual Metal passed all 16 pixels for roughness 0.2/0.5/1.0 and seeds 0/17/31 in both existing probe cases, plus ray/material/TLAS regressions. This replaces the prior finite-positive-only smoke. It validates these sampled fixtures; temporal accumulation, general scenes, multibounce and extreme numerical cases remain open.

### Browser WebGPU collision execution

`WebEngine::validate_collisions` and `?backend=webgpu&collisionCheck=1` run four asynchronous GPU-assisted sweeps in the browser and compare complete results with the CPU solver: empty movement, downward terrain collision, an unloaded boundary and a far i64 anchor. The probe also requires consumed-result rejection and rejection after a real world edit. Browser callbacks progress through event-loop yields with bounded readback deadlines.

Actual in-app browser execution reported `BrowserWebGpu · collisions verified: 4 sweeps`; evidence is `docs/browser-collision-proof.png`. Explicit WebGL reported `collision compute unsupported on WebGL`. This proves compute API execution, not full browser voxel gameplay, a GPU narrowphase or physical adapter identity.

### GPU static radiance accumulation (2026-10-01)

`RadianceAccumulator` prepares immutable candidate `RadianceAccumulationFrame` objects, averaging linear RGB into RGBA32Float entirely on GPU. Keep only the last presented candidate as history; reset with None after any camera/scene/lighting/material/resolution discontinuity. The frame exposes its sample count and output texture, with a count cap of 2^24. Null rays are valid black samples; negative/nonfinite input channels become zero contributions, alpha is ignored and output alpha is one. History device and dimensions are validated. This averages fixed pixels; moving-scene reprojection is not implemented.

The Metal GGX probe now averages a GPU material-map first sample with a GPU traced rough-radiance second sample and verifies every mean channel against the independent expected values, along with counts 1/2. All nine roughness/seed cases in both probe variants passed, plus ray regressions. Strict library Clippy and Windows library checks passed. Multi-frame convergence, skip/reset edge-case acceptance, moving-scene temporal reuse and an accumulated rendered image remain pending.

### Mobile collision compile gates and early failure proof

The current GPU library, including immutable snapshots and asynchronous GPU-assisted sweeps, passes cargo check for aarch64-apple-ios and aarch64-linux-android. These are compile checks, not simulator/device execution.

The voxel region probe now uses a view that panics on any sample/chunk read to verify that NaN displacement, zero candidate budget, excessive candidate volume, i64 world overflow and an unsupported dense extent fail before accessing the world or dispatching compute. Actual Metal execution and strict example Clippy passed. Linux and NVIDIA acceptance runners now require this early-failure marker; shell and PowerShell syntax checks passed.

### Shared character controller with GPU collision adapter

`GpuVoxelCollisionWorld` implements the shared physics CollisionWorld on native targets. Each query submits integer GPU broadphase through begin_sweep, retains stale-world rejection, then resolves occupied/fault regions with exact CPU collision. A bounded 30-second native polling loop waits for each query; this synchronous adapter is not a nonblocking game-loop integration or a performance claim.

On actual M4 Max Metal, 240 character ticks with falling, walking, reversal and a jump matched complete CPU controller state and contact reports exactly. Strict GPU library/example Clippy passed. The physics dependency is now production-owned because the adapter implements its public trait. Vulkan/OpenGL controller execution and native application integration remain outstanding.

### Portable accumulation history acceptance (2026-10-01)

`radiance_accumulation` verifies an encoded skipped bright candidate does not contaminate the next candidate built from last presented history, reset starts at sample one, null/invalid samples contribute zero without acceptance renormalization, alpha stays one, and 65 sequential samples match an independent arithmetic mean. Readback occurs only after GPU submission for verification. The fixture is 17×9 and checks representative output pixels; it does not inspect every pixel.

Native Metal and Mesa llvmpipe software Vulkan/OpenGL all passed. Strict library/example Clippy passed. The Linux HDR runner now includes this verifier and all prior HDR/blit/exposure regressions passed in the same run. Real convergence of an accumulated rough-reflection image, moving-scene reprojection and hardware NVIDIA acceptance remain outstanding.

### Eight-sample GPU rough-reflection accumulation acceptance (2026-10-01)

The primary-GGX fixture now accumulates eight independently seeded, GPU-generated/traced rough-reflection samples per pixel. It verifies each accumulated RGB against the CPU GGX/analytic-emitter-intersection mean for the identical eight seeds, including null/missed samples as zero. Counts are checked at eight. The earlier material-map-plus-reflection accumulation fixture has been replaced by this traced-reflection sequence.

Actual Metal execution passed all sixteen pixels across roughness 0.2/0.5/1, three initial seeds and both existing probe variants; ray/shadow/material/TLAS regressions passed. This proves finite-sequence accumulation for these fixtures, not statistical convergence, a full rendered scene or moving-scene temporal reuse. Pipelines are currently created per seeded job; reuse and runtime integration remain pending.

### Nonblocking shared character step

`PendingGpuCharacter` evaluates the pure shared controller from its original state, caching submitted/completed collision queries and replaying arithmetic to resume after pending readbacks. `try_step` never waits on device completion, returns None while queries are pending, and publishes a new state/report only after the entire controller succeeds. Every replay checks all captured world snapshots; changed data invalidates the whole step. Inputs and registry are retained, query identity is checked, and completed/error tasks are single-use. Callers still serialize world edits while polling/publication.

On actual Metal M4 Max, 240 nonblocking character tasks matched exact CPU states and contact reports, including falling/walking/jump. Strict library/example Clippy and wasm32 compilation passed. Full game-loop adoption and browser execution of this task remain pending; GPU broadphase still uses CPU exact narrowphase.

### Reusable compiled GGX pipeline (2026-10-01)

`GgxReflectionPipeline` compiles the GPU rough-reflection shader once and creates independently owned seeded jobs using that pipeline. Each candidate retains separate immutable camera/seed uniforms, bindings and outputs, allowing multiple seeds in the same command submission without later parameter updates changing earlier jobs. The convenience `SurfaceReflectionJob` constructors remain available.

The eight-sample accumulation verifier now uses one compiled pipeline per fixture sequence and all per-pixel CPU/analytic checks passed on Metal, along with ray regressions. Strict render-library Clippy and Windows library checks passed. This removes per-sample shader/pipeline creation on the reusable path; no performance measurements or FPS claims were made. Job resource allocation and full render-loop integration remain pending.

### Nonblocking controller stale-world and backend proof

The native collision probe now requires a character task to remain pending, edits its captured chunk, verifies StaleWorld then Consumed errors, drops that task and checks a fresh task against exact CPU state/contact output. This verifies rejection after submission; cached multi-query interleavings need further dedicated coverage.

Actual Metal M4 Max and software Mesa Vulkan/OpenGL passed this scenario and all 240 nonblocking controller ticks. Strict example Clippy passed. Linux and NVIDIA acceptance runners require the controller and stale-recovery markers. Native game-loop integration remains unfinished.

### Cached-contact interleaving proof

The stale character probe now installs an explicit collision floor and performs a diagonal fall. It requires exactly one completed cached sweep while the next query remains pending before editing the captured chunk. The entire task then rejects StaleWorld/Consumed and a fresh task matches the full CPU state and contacts. `completed_sweeps` exposes the retained query count for diagnostics.

This closes the previously missing cached multi-query interleaving case: physical M4 Max Metal and software Mesa Vulkan/OpenGL passed, as did strict library/example Clippy. Native application-loop adoption remains pending.

### Rendered accumulated rough-reflection image (2026-10-01)

`cargo run -p voxy_ray_probe -- --experimental --rough-image` renders a 128×128 primary plane with varying metallic roughness, ray-traces an emissive triangle, accumulates 64 GGX samples per pixel on GPU and tone-maps into an sRGB output before PNG export. Native Metal execution completed and the exported image was visually inspected. `docs/rough-reflections-proof.png` preserves the result. The first image exposed horizontal correlation from overlapping index+seed sequences; pixel index now mixes with a separately hashed seed. The rerender was inspected, and all per-pixel CPU GGX/analytic intersection and accumulated-mean fixtures passed after that change.

The image is a static, single-bounce probe scene, not a production renderer or statistical convergence proof. Noise remains at 64 samples. Motion reprojection, multibounce/general scene shading and physical RTX/headset acceptance remain outstanding.

### Native gameplay nonblocking character collision integration

`voxy_app --gpu-collisions` creates a VoxelRegionProgram on the renderer device and runs character movement through PendingGpuCharacter. An incomplete fixed tick retains its original movement/jump/sprint intent and camera-derived movement; subsequent frames poll without blocking and publish the character state only on completion. Stale-world failures restart that tick. Driving mode cancels pending character work and continues the existing vehicle controller. `--gpu-collisions` combined with `--cuda-character-motion` currently returns an explicit startup error; external CUDA motion is not silently ignored.

Actual command `cargo run -p voxy_app --bin voxy_app -- --backend metal --gpu-collisions --gpu-water --autopilot` passed on M4 Max: 179 completed GPU character ticks, 7 GPU water ticks, 5 water commits, 3 destruction commits, 599 animation frames and 1216 presented frames. All 19 binary tests passed. This proves native character-loop execution and continued presentation, not measured latency improvement, GPU exact narrowphase, GPU vehicle collision or physical NVIDIA execution. Readback timeout enforcement in the application path remains to be added.

### Extreme HDR accumulation acceptance (2026-10-01)

The accumulation verifier now checks 37 successive f32::MAX inputs remain finite and near the maximum, maximum-plus-black yields half-maximum, and maximum mixed with unit history yields half-maximum within relative tolerance 1e-5. Native Metal and Mesa software Vulkan/OpenGL passed, as did strict library/example Clippy and the existing HDR regression runner. This adds extreme-value evidence without changing the accumulator shader; it is not exhaustive floating-point proof for every count/input pair.

### Gameplay collision deadlines and Linux execution

Native pending character ticks now retain one 30-second deadline across stale-world retries; success and driving cancellation clear it. Expiry exits explicitly rather than leaving movement waiting forever. No forced device-hang fault injection was performed.

The graphical Linux water runner now also enables --gpu-collisions and requires its explicit backend plus nonzero completed character ticks. Actual Mesa llvmpipe Vulkan/OpenGL runs passed: each completed 179 GPU character ticks, 7 water ticks, 5 water commits, 3 destruction commits and 599 animation frames; Vulkan presented 999 frames, GL 972. All 19 native binary tests passed. NVIDIA Windows/Linux scripts add a separate collision gameplay gate with CUDA terrain/projectiles/vehicle motion and preserve the existing CUDA-character gate; physical NVIDIA execution remains pending.

### Wide HDR lighting composition (2026-10-01)

`RadianceComposition::new_hdr` accepts matching RGBA16Float/RGBA32Float lighting contributions and writes their RGB sum to RGBA32Float, saturating overflow at f32::MAX and fixing alpha to one. The original constructor retains its binary16 input/output contract and 65504 saturation. This enables direct-light contributions to compose with accumulated wide GGX radiance without prematurely clipping to binary16. Inputs must still be finite nonnegative linear radiance and describe the same pixel/surface.

GPU fixtures verify unit+three, 131008+100 and f32::MAX+f32::MAX saturation. Metal and Mesa software Vulkan/OpenGL passed, as did strict library/example Clippy and Windows library compilation. Physical NVIDIA SDK/HDR display integration and moving-scene reconstruction remain pending.

### Retained CUDA motion with asynchronous collision

`PendingGpuCharacter::with_motion` retains externally integrated f64 velocity/displacement across controller replays and uses the shared step_character_with_motion path. Native gameplay now computes CUDA character motion only when creating a pending task; repeated polls reuse it. Stale-world retry recreates motion from unchanged character state. The prior startup rejection for --gpu-collisions plus --cuda-character-motion has been removed. Exact narrowphase remains CPU; CUDA integration itself uses its existing synchronous API.

On physical M4 Max Metal, the 240-task probe alternates internal integration and externally supplied exact motion, matching full CPU states/contact reports. Strict GPU library/example Clippy passed. Native CUDA-feature and x86_64-pc-windows-gnu CUDA-feature application checks passed. NVIDIA acceptance collision gameplay now enables CUDA character motion as well; physical NVIDIA execution of this combined path remains unverified.

### CUDA integer voxel-region broadphase

CudaCompute::voxel_regions now owns a cached CUDA kernel using the same dense canonical integer grid, inclusive regions and count/first-solid/first-fault output as the WGSL classifier. Input dimensions, class values, query bounds, aggregate 4194304 visits and allocation budget are checked before driver work. Readback checks immutable metadata, query volume, candidate membership/class and zero-count consistency. No CPU fallback is provided. The standalone voxel_regions_probe checks 257 queries against independent CPU enumeration and invalid-input recovery on a physical NVIDIA host.

Input/corruption unit test and strict CUDA-feature Clippy passed. Actual NVRTC 12.6 and PTXAS compilation passed for compute/sm 52, 75 and 89. These compiler checks do not prove NVIDIA execution or collision-world/application integration of the new CUDA classifier; those remain pending.

### Direct GGX light with accumulated reflections

`SurfaceLightingJob::with_ggx` adds single-scattering GGX point-light specular
radiance to the supplied linear diffuse reflectance, using primary world-space
normals/roughness, a matching F0 texture and world-space camera. It shares the
opaque shadow query with the existing Lambertian path and outputs RGBA32Float.
The BRDF uses Schlick Fresnel and correlated Smith masking. Roughness below
0.001 has no finite direct GGX lobe; this path does not approximate a delta
highlight. F0 values outside [0,1] contribute no specular light. This remains one
point light and one opaque shadow ray, without multiple scattering compensation.

The Metal ray probe compared direct lighting against the CPU GGX evaluator for
all 16 fixture pixels at roughness 0.2, 0.5 and 1.0 alongside the existing
nine seeded reflection/accumulation fixtures. Reflection distance, eight-sample
means and Lambertian shadow regressions passed. The direct fixture uses an
unoccluded light and the same F0 texture as diffuse input to exercise both BRDF
terms; it is a numeric fixture rather than a physically authored material.

`voxy_ray_probe --experimental --rough-image` now renders direct GGX lighting
plus 64 accumulated reflection samples through RGBA32Float HDR composition
and SDR tone mapping. The generated 128x128 result was visually inspected and
saved to `docs/rough-reflections-proof.png`. Grain remains visible with 64
samples. Strict render-library Clippy and the Windows GNU library cross-check
passed. These checks used Apple M4 Max Metal and do not establish RTX 5090,
DLSS/Frame Generation execution or performance.

### CUDA classification of retained world snapshots

VoxelRegionSnapshot::classify_cuda converts its retained collision classes and inclusive regions into the validated CUDA integer API and returns the shared VoxelRegionResult. Integer anchor/position mapping and chunk revision/data identity remain in the snapshot. The API performs synchronous CUDA readback; publication still checks is_current. Query count is rejected before temporary host packing.

The cuda_voxel_world physical acceptance example independently enumerates 32768 world cells, checks exact CUDA counts/candidates, edits the world and verifies the old snapshot still yields original GPU results while is_current rejects publication, then checks fresh results and a far missing-world fault candidate above 2^53. Both NVIDIA acceptance runners build and require this example plus the 257-query low-level CUDA probe. Strict CUDA-feature GPU library/example Clippy and shell/PowerShell syntax passed. Actual NVIDIA execution and CUDA-classifier collision/controller integration remain pending.

### Frame Generation policy preflight

`FrameGeneration::validate(&FrameGenerationState)` exposes SDK-reported FG/MFG
capabilities to application policy selection without calling the native SDK or
consuming another presentation counter. Fixed counts must be nonzero and at most
`maximum_generated_frames`; dynamic mode requires an explicit true capability
flag and a finite positive optional FPS target. `None` preserves the SDK target
policy. Turning FG off remains valid even when no generation mode is supported.
No silent reduction of a requested multiplier occurs.

The snapshot may become stale after device/runtime changes; the native bridge
still queries current SDK state and validates Reflex before configuration. This
preflight does not generate or present frames. Unit coverage exercises supported
fixed multipliers, invalid counts/targets, unsupported/unknown dynamic flags and
adapter-capability downgrade. Physical RTX 5090 presentation remains unverified.

### CUDA collision controller adapter

CudaVoxelCollisionWorld implements the shared CollisionWorld interface. It uses the same capture_sweep helper as the WGSL path, runs CUDA integer region classification, rejects changed snapshots and skips exact sweep for clear regions. Occupied/fault regions use the authoritative CPU continuous solver. CUDA compilation/device failures are propagated without fallback. This adapter uses synchronous CUDA readback.

The physical CUDA world probe now includes 240 character ticks comparing full states and contacts with CPU; NVIDIA runners require that marker. Strict CUDA-feature library/example Clippy and Windows GNU CUDA-feature example check passed. The refactored WGSL path passed the actual Metal smoke including all 240 nonblocking character tasks and stale cached-contact rejection. CUDA controller execution and CUDA broadphase gameplay selection remain unverified/pending.

### Combined CUDA integration/classification acceptance

The physical cuda_voxel_world probe now alternates native integration and actual CudaCompute Euler integration during its 240-character-tick comparison. Even ticks retain CUDA velocity/displacement and run the complete controller through CudaVoxelCollisionWorld; all states/contact reports must match the CPU oracle. Four additional full SweepResult comparisons cover clear movement, downward collision, loaded/unloaded boundaries and anchors above 2^53.

Both NVIDIA runners require the combined integration/classification and boundary markers. Strict CUDA-feature example Clippy and shell/PowerShell syntax passed. This is prepared hardware acceptance coverage; actual NVIDIA execution remains unverified.

### Renderer FG policy to Streamline configuration

The optional `voxy_streamline/render-policy` feature connects
`voxy_render::FrameGenerationMode` to native Streamline configuration without
requiring DX12 resources on the policy path. `scene-dx12` enables it automatically.
`render_policy::capabilities` translates a queried state only when the caller
supplies SDK-confirmed support for the selected adapter; adapter names and raw
runtime error flags are not treated as support evidence. `resolve` preserves the
exact fixed multiplier or dynamic FPS target and the renderer's specific errors.
`StreamlineRuntime::configure_renderer_frame_generation` performs this preflight
and then calls the existing native bridge, preserving native errors separately.
The bridge still checks current capabilities and SDK-accepted Reflex state, so
caller-supplied snapshot/Reflex hints cannot bypass native validation.

Preflight does not query the SDK or consume presentation counters. Snapshots
must be refreshed after device/runtime changes. Native configuration can consume
counters and does not install hooks, acquire FG resources or present generated
frames. Tests cover exact translation, missing Reflex, excessive counts, missing
SDK feature support, unknown dynamic flags, invalid targets, policy-before-native
error ordering and unsupported-platform propagation. Windows scene-DX12
cross-check and render-policy library Clippy passed; physical FG remains pending.

### CUDA f64 continuous box contact kernel

CudaCompute::box_sweeps now submits independent local f64 box contacts on CUDA using the existing CPU slab-sweep rules: stationary axis exclusion, strict axis entry tie order, initial-overlap zero fraction/normal, and [0,1] entry interval. Inputs retain integer world anchors externally, validate finite local bounds, and enforce aggregate 160-byte-per-query device allocation limits. NVRTC disables fmad/fast math and enables precise division without flushing denormals. Output validates discrete hit/normal protocol and finite fractions.

The corruption unit test and strict CUDA-feature library/example Clippy passed. box_sweep_probe prepares 267 exact fraction-bit/normal comparisons against physics::sweep_box, including stationary, overlap, tiny displacement and invalid inputs. Actual NVIDIA execution and voxel/controller narrowphase integration remain pending; compilation is not equivalence proof.

### Windows FG/MFG configuration probe

`dxgi_proxy_probe` now accepts an optional policy argument after the signed
interposer path: `off`, `fixed:N`, `dynamic` or `dynamic:FPS`. Fixed counts exclude
the rendered frame (`fixed:3` requests three generated frames). The parser rejects
zero counts, integer overflow and nonfinite/nonpositive dynamic targets. Existing
invocations without a policy retain the factory/device/swapchain-only probe.

On Windows, the probe selects an SDK-supported FG/Reflex adapter, queries FG
state, validates the requested policy, enables SDK Low Latency Reflex for active
FG, configures the exact policy and disables FG/Reflex before resource teardown.
With `scene-dx12` it exercises the renderer-policy bridge; `wgpu-dx12` retains a
standalone native path. Example command from a Windows checkout:

```powershell
cargo run -p voxy_streamline --example dxgi_proxy_probe --features scene-dx12 -- "C:\Streamline\bin\x64\sl.interposer.dll" fixed:3
```

`VOXY_STREAMLINE_SDK` must point to the official SDK source/header directory at
build time. This is a configuration/lifecycle probe: its composition swapchain
is never presented and it does not provide frame inputs or prove generated-frame
counts, latency or visual quality. Parser tests and local Clippy passed; Windows
GNU cross-check covers the native branches. Execution with physical RTX hardware
and the signed SDK remains pending.

### CUDA voxel narrowphase integration

CudaVoxelCollisionWorld now runs nonempty voxel contacts through CudaCompute::box_sweeps rather than calling the CPU narrowphase. CPU world sampling preserves UnknownBlock/unloaded/unavailable semantics and builds canonical x/y/z local obstacle boxes; CUDA computes f64 fractions/normals; host reduction keeps the earliest contact and original voxel order for ties. The retained snapshot is checked again after device readback. Absolute i64 anchors never enter CUDA float buffers. This is synchronous CUDA execution with host candidate packing/reduction, not fully resident physics.

Strict CUDA-feature library/world-example Clippy and Windows GNU CUDA-feature check passed. Contact corruption tests passed. NVIDIA runners now require the 267 exact f64 box-sweep oracle probe before world/controller checks. Actual NVIDIA fraction equivalence and full controller runtime remain unverified; source integration and compile success do not establish numerical parity. The existing WGSL gameplay path retains CPU narrowphase.

### Direct GGX opaque-shadow regression

The rough-surface GPU verifier now shades the same reconstructed primary samples
with a second point light at z=4, behind the emitter/occluder plane at z=3.
Every RGB channel must be zero (absolute tolerance 1e-6) and alpha must remain
one for all 16 pixels at roughness 0.2, 0.5 and 1.0. The independent light at z=2
retains the CPU diffuse-plus-GGX BRDF comparison. Both variants run alongside
all seeded reflection and accumulation checks, so direct lighting is checked
without changing the reflection scene or relying on a black material input.

`cargo run -p voxy_ray_probe --locked --offline -- --experimental` passed on
Apple M4 Max Metal, including all direct shadow fixtures, reflection/distance
checks, primary motion/background and TLAS update regressions. This proves the
opaque shadow gate for the fixture; it does not establish alpha-tested geometry,
transparent shadows, multiple scattering or physical RTX/DLSS execution.

### Reusable direct GGX pipeline

`GgxLightingPipeline::new` compiles the RGBA32Float direct-light shader once for a
device. `create_job` validates camera/light/maps and creates independent uniforms,
bindings and output textures while cloning the existing compute pipeline handle.
The existing `SurfaceLightingJob::new` and `with_ggx` convenience constructors
retain their behavior and share the extracted shader/pipeline construction code.
This removes repeated shader/pipeline creation when an application retains the
pipeline owner; it does not eliminate per-job resource allocation or establish
an FPS improvement.

The Metal ray verifier now creates both unobstructed and occluded direct GGX
jobs from one pipeline and submits their commands together. All 16-pixel CPU
BRDF/opaque-shadow fixtures, seeded reflection means, motion/background and TLAS
regressions passed. Separate job uniforms preserved both light positions through
submission. Strict render-library Clippy passed. Ray-query-capable hardware is
still required; no raster fallback or physical NVIDIA acceptance is implied.

### Explicit standalone WebGL build feature

`voxy_render/webgl` explicitly enables `wgpu/webgl`; `voxy_web` requests this
feature on its renderer dependency. Previously the browser application enabled
WebGL through its separate wgpu dependency, while a standalone renderer build
only received wgpu's default WebGPU browser backend. Both default WASM and
explicit WebGL renderer library cross-checks passed offline. This feature exposes
the existing raster backend; WebGL does not supply compute shaders or hardware
ray queries. Build evidence alone does not establish browser runtime rendering.

### Browser WebGL execution proof

Installed wasm-bindgen-cli 0.2.127 under `/tmp/voxy-bindgen-0.2.127` because the
user's default executable was 0.2.125. The default installation was preserved.
Built the current browser source with:

```sh
VOXY_WEB_PROFILE=dev VOXY_WASM_BINDGEN=/tmp/voxy-bindgen-0.2.127/bin/wasm-bindgen sh web/build.sh
```

Served `web/` on localhost port 8789, opened `?backend=webgl` in the Codex in-app
browser and reloaded after the build completed. The visible status reported
`Gl`, the canvas frame counter reached 1241, and visual inspection showed the
colored rotating 3D geometry plus the blue 2D rectangle. This establishes the
current browser raster example's WebGL execution, not compute/ray-query support,
HDR browser output, all mobile browsers or a performance result.

### Browser WebGPU compute execution proof

Using the freshly built browser WASM, the in-app browser selected
`BrowserWebGpu` and completed `waterCheck=1`: 16 GPU-planned world-water commits
matched the CPU planner, while the validator also checked stale-revision rejection
and single-consumption of readback results. The combined
`gravityCheck=1&collisionCheck=1` page completed four GPU-assisted voxel sweeps
and the 257-body, 128-step gravity comparison against the independent f64 CPU
solver. Its reported maximum absolute gravity error was
0.000025912606399280946. The canvas continued rendering (1699 frames observed).

These values came from browser DOM result attributes after successful validation,
with the displayed backend/status confirming WebGPU. This is actual browser
compute execution on the current host, not a cross-build, CUDA result or benchmark.
The collision pipeline is GPU-assisted; its continuous sweep logic is not claimed
to execute entirely on the GPU. Mobile/browser coverage remains incomplete.

### Current Android/iOS portability gates

Offline ARM64 `cargo check --lib` passed for both `voxy_render` and
`voxy_mobile` on `aarch64-linux-android` and `aarch64-apple-ios`. The mobile
checks include the common app, runtime and GPU crates, rather than only render
API declarations. The app emits existing unused-item warnings; these checks are
not strict warning-free builds. Android packaging helper tests passed (nine
cases), including artifact alignment/architecture and device-selection gates.
The iOS build script also passed shell syntax validation.

Native mobile build/run acceptance remains incomplete: `adb` was absent from
PATH and `xcrun --sdk iphoneos --show-sdk-path` reported no iPhoneOS SDK in the
selected developer environment. These are concrete limitations for APK/device
and Xcode linking verification; successful Rust checks do not establish either
package production or actual Metal/Vulkan/GLES execution on mobile hardware.

### Bounded instance-level OpenXR event routing

`XrRuntime::poll_events` polls the shared instance queue with an explicit maximum
number of events per call, lending each event to an application handler before
reusing the event buffer. The caller routes each event to every interested
session, including `handle_event_with_history` for combined motion/exposure
history. Remaining events stay queued when the budget is exhausted; zero performs
no polling. Runtime and handler errors propagate, and a handler error does not
replay its already consumed event. The application must reset/recover temporal
state if processing cannot continue safely.

All 14 XR library tests, strict library Clippy and Windows/Android ARM64
cross-checks passed. The tests cover lifecycle/history/pose/swapchain contracts;
they do not emulate the native event queue. Actual event-pump execution and stereo
presentation still require an OpenXR runtime and headset.

### Multiple GGX point lights in wide HDR

`GgxLightingPipeline::create_lights` prepares a nonempty slice of independent
point lights. `GgxLightingBatch` retains every lighting job and an ordered chain
of RGBA32Float HDR additions; `encode` runs light/shadow dispatches before the
sum passes. Each light retains separate uniforms and output, with the compiled
lighting pipeline shared. Empty light lists are rejected. This is direct
per-light dispatch with intermediate textures, not clustered/tiled lighting or
one multi-light shader dispatch. Cost grows with light count.

The Metal ray verifier composed two independently parameterized lights and the
eight-sample reflection mean and compared every RGB channel of all 16 pixels
with the CPU BRDF plus independently computed reflection expectation, across
roughness 0.2/0.5/1.0 and three seeds. All numeric and existing ray regressions
passed, as did strict render-library Clippy and Windows library cross-check.
The two lights share a position and differ in spectral intensity in this fixture;
separate blocked/unblocked jobs continue checking opaque shadow suppression.
Physical NVIDIA, mobile ray-query and runtime DLSS acceptance remain pending.

### Spatially distinct GGX lights

The two-light GPU fixture now places its second source at (0.75,-0.25,2),
instead of sharing the first source's position. A separate CPU point-light
reference evaluates each direction, distance-squared attenuation, cosine and GGX
BRDF before adding the independent reflection mean. All 16-pixel comparisons at
three roughness values and three seeds passed on Metal, alongside the full ray
regression suite. This strengthens independent-light-position coverage; the
blocked-light checks remain separate fixtures.


### Mobile portability after asynchronous character and CUDA collision batching

Current `cargo check -p voxy_gpu --target aarch64-apple-ios` and `--target aarch64-linux-android` both passed. The complete `voxy_mobile` shell also passed both targets, including current shared application/render/physics dependencies. Existing unrelated application dead-code warnings remain.

These checks do not execute mobile shaders, link/install signed apps, or establish GPU collision gameplay on phones. `crates/voxy_mobile/src/lib.rs` currently launches `voxy_app::SceneApp`; native voxel gameplay in the desktop binary is not yet the mobile entrypoint. Full mobile gameplay integration and physical lifecycle/graphics/compute acceptance remain unfinished.

### Current CUDA feature gates

With the `cuda` feature enabled, 12 host-contract tests passed and one native
CUDA-arithmetic parity test remained ignored in the normal run. Strict CUDA
library Clippy and the Windows GNU library cross-check passed. Running the
actual `cuda_probe` on the current Mac returned `DriverUnavailable`, so no
kernel execution, external-memory interoperability or RTX 5090 performance is
established by these checks.

The explicitly requested ignored `cuda_gravity_host_parity` run was blocked
before execution by the current shared physics source referencing a missing
`crates/physics/src/liquid/thixotropy.rs`. It must be rerun once that dependency
is consistent. This C++ arithmetic test would still not constitute NVIDIA GPU
execution even if it passed.


### Dense collision execution across Metal and Mesa Vulkan/OpenGL

The current `voxel_regions_smoke` passed on Apple M4 Max Metal and Mesa 25.0.7 llvmpipe Vulkan/OpenGL, including 4913 obstacles with the nearest contact late in canonical enumeration and the first voxel retained for overlapping ties. Each backend also passed 240 asynchronous character tasks, stale/fresh recovery, 257 integer region queries and concurrent/dropped readback checks. Crate-only strict Clippy passed.

The Linux offline runner now mounts the existing Cargo Git cache read-only (`VOXY_CARGO_GIT_CACHE` overrides its location), because Rush is now pinned to a Git dependency. The runner remains network-isolated and uses the locked dependency resolution. Mesa results are software API execution, not physical NVIDIA or DX12 proof; CUDA dense batching still requires physical execution.

### Reusable HDR additions and CUDA arithmetic recheck

`HdrCompositionPipeline` compiles wide-HDR addition once per device and creates
independent `RadianceComposition` jobs with existing texture/dispatch validation.
`GgxLightingPipeline` retains this owner for multi-light sums, so subsequent
batches do not recompile an addition shader for every light. Intermediate output
textures and per-job bindings remain allocated; no throughput claim is made.
The existing HDR/binary16 convenience constructors remain available.

Render-library Clippy and Windows cross-check passed. After the shared missing
physics module appeared, the full Metal ray probe passed with reusable additions,
including spatially distinct lights/reflection composition. The previously
blocked ignored `cuda_gravity_host_parity` test was then rerun and passed. It
validates CUDA-source arithmetic through native C++ and the CPU reference, not
NVIDIA execution. The earlier `DriverUnavailable` hardware limitation remains.


### Current offline Linux gameplay and water gates after Git dependency migration

`tools/linux/gameplay-water-smoke.sh` and `water-smoke.sh` now expose the existing Cargo Git cache read-only, matching the collision runner. Both finished without network access using locked dependencies. Current Mesa Vulkan/OpenGL gameplay each completed 179 GPU character ticks, 7 GPU water ticks, 5 water transactions and 3 explosion commits; presented frames were 923 and 767 respectively. Each backend also passed the standalone 16-tick water comparison (524288 exact CPU cell comparisons), volume conservation, stale revision and dropped-readback recovery gates.

These runs exercise Mesa llvmpipe software rendering and compute. They do not establish physical NVIDIA, DX12 or mobile performance/acceptance.

### Reusable HDR extreme-value portability

The radiance-accumulation GPU example now creates all wide HDR addition fixtures
through one `HdrCompositionPipeline`. Its independent outputs check 1+3=4,
131008+100=131108 and f32::MAX+f32::MAX saturation, alongside accumulation
history/reset/extreme-value checks. Metal execution and focused example Clippy
passed. The full Linux HDR smoke suite also passed on Vulkan and OpenGL using
Mesa llvmpipe CPU software rendering.

Two build blockers were repaired before rerunning: the missing `SourcePath`
import in the asset worker and the HDR Docker script's missing read-only Cargo
Git cache mount. `VOXY_CARGO_GIT` can select this cache independently of the
registry cache. The container remains network-disabled; no Git dependency fetch
or physical Linux/NVIDIA execution is implied by the software-renderer result.


### CUDA contact inputs reuse retained chunks

CUDA narrowphase now obtains loaded block IDs from the retained `VoxelRegionSnapshot` used by broadphase, avoiding a second live voxel sample for each loaded candidate. Missing chunks still use live sampling to preserve unloaded/unavailable fault details; the retained snapshot is checked again before publishing contacts. Host packing/reduction and synchronous CUDA readbacks remain.

The snapshot unit test confirms retained block IDs survive replacement/unload and that reading the retained ID does not call the world again. Physical NVIDIA execution and performance measurements remain unverified.


### WGSL contact resolution reuses broadphase snapshots

`PendingVoxelSweep` now feeds exact CPU contact resolution from retained loaded chunks instead of resampling each loaded voxel. Missing/unavailable chunks still use the world sample to preserve fault detail, and existing revision/identity checks remain mandatory. The dense 4913-obstacle GPU fixture forbids live sampling, demonstrating that this path resolves all loaded contacts from captured blocks.

Current actual Metal and Mesa Vulkan/OpenGL runs passed that fixture plus the existing 240 asynchronous character tasks, stale recovery and boundary/region/readback checks. Strict crate-only Clippy and wasm32 compilation passed. This avoids redundant world reads; it does not move WGSL narrowphase arithmetic onto the GPU or establish a measured speedup.

### Atomic compute shader reload

`ComputeProgram::reload_shader` compiles and validates a complete replacement
before publishing it. Identical source is a cache hit; failed replacements retain
the working source, pipeline and `shader_revision`. Existing `ComputeJob` owners
retain their previous pipeline, while new jobs use the replacement. Validation
scopes require serialization on the owning device task as in program creation.

The Metal compute probe retained a job with multiplier 3, reloaded multiplier 5,
rejected syntax/entrypoint/binding failures and submitted old/new jobs together.
Both GPU readbacks matched their independent expected results. The full 1042-value
compute/cancellation regression and scene shader-reload test with MSAA also passed.
Strict library/example Clippy and WebGL-enabled WASM library cross-check passed.
The WASM check proves API portability; compute execution still requires WebGPU
rather than WebGL, and browser compute hot-reload was not exercised here.


### Browser acceptance after captured-contact integration

The current rebuilt WASM executed 245 collision checks in the in-app browser using explicit WebGPU: four sweeps, 240 asynchronous character ticks compared against CPU state/contact reports, and stale-character rejection/fresh recovery after a world edit. This run uses the new retained-chunk contact view, with cache version `captured-contacts-v1`; the current screenshot is `docs/browser-character-proof.png`. Crate-only strict WASM Clippy passed. Full browser voxel gameplay, physical NVIDIA/DX12/mobile/VR acceptance and complete engine support remain unfinished.

### Linux compute reload execution

`tools/linux/compute-smoke.sh` builds offline with read-only registry/Git caches
and explicitly executes the compute probe on Vulkan and OpenGL under Xvfb.
`VOXY_COMPUTE_BACKEND` now selects the probe backend strictly, rejecting unknown
values rather than relying on automatic adapter selection. Both Mesa llvmpipe
CPU software backends passed old/new shader-job coexistence, failed reload
retention, 1042 exact values, partial workgroups, independent readback and
cancellation checks. Focused native library/example Clippy also passed. This does
not imply WebGL compute support or physical NVIDIA execution.


### Native vehicle CUDA contact integration

`--cuda-collisions` now routes both character and vehicle chassis contacts through CUDA integer broadphase and f64 box kernels. `--cuda-vehicle-motion` independently selects CUDA motion integration; bicycle forces/steering and canonical contact reduction remain on the host. Vehicle state is still published only after the caller-selected chassis controller succeeds.

The 240-tick physical vehicle probe now alternates CPU-contact/CUDA-motion and CUDA-contact/CUDA-motion ticks, comparing complete state and contact reports to CPU. Linux/Windows hardware gates require its new combined-contact marker. Current native application/probe compilation and the CPU forward/reverse/braking fixture passed; shell/PowerShell parsing and diff checks passed. Physical NVIDIA vehicle execution remains unverified. Projectile collision uses its existing DDA contract and remains on CPU despite CUDA ballistic integration.

### Explicit compute entry points

`ComputeProgram::with_entry_point` accepts a named WGSL compute entry point while
preserving the existing group-0 storage-buffer ABI. `new` retains `cs_main` as
the default. Reload retains the selected name, including error diagnostics and
old-job pipeline ownership. The probe now uses `selected_kernel`, verifies that
the default constructor rejects that source, replaces its multiplier and checks
old/new GPU results after rejected replacements.

Metal and Linux Vulkan/OpenGL (llvmpipe CPU software) execution passed, alongside
strict focused Clippy and the Windows compute-example cross-check. All Streamline
scene-DX12 examples also cross-checked against the configured SDK headers. These
Windows checks are compile evidence, not DX12/DLSS inference or FG presentation.

### Multiple compute entry points in one module

The shader-reload probe now includes `selected_kernel` and `alternate_kernel`
in the same WGSL module, sharing the declared storage ABI. One submission runs
the retained selected kernel (multiplier 3), its replacement (multiplier 5) and
the alternate entry point (multiplier 101), comparing all outputs independently.
Rejected missing-entry replacements leave the alternate function present, so
validation must honor the selected name rather than accept any compute entry.
Metal and Linux Vulkan/OpenGL llvmpipe execution passed; focused Clippy passed.
This verifies entry-point selection and coexistence, not arbitrary resource ABIs.


### Nonblocking vehicle collision task

`voxy_gpu::PendingGpuVehicle` retains original vehicle state and uses `PendingGpuCharacter` for asynchronous chassis contacts. Each poll re-evaluates deterministic bicycle arithmetic from the original state; only successful completion returns the whole candidate. Validation/collision failures consume the task, and repeated consumption rejects. No blocking GPU poll occurs inside this API.

Actual Apple M4 Max Metal acceptance compared 240 vehicle ticks against full CPU states/contact reports, exercising forward/reverse/brake and consumed rejection. Strict library/example Clippy passed. Native gameplay integration, explicit stale-vehicle recovery and other backend execution are still pending; this is not yet full GPU vehicle gameplay.

### Compute-job shader provenance

`ComputeJob::entry_point` and `shader_revision` expose the immutable selection
captured when the job was created. Reload leaves existing jobs' metadata intact,
matching their retained pipelines. Entry-point text is shared with Arc rather
than copied into every job. Revision is local to a program owner, not a globally
unique shader identifier; applications must pair it with their owner/frame IDs
and retain it before consuming a job for dispatch/readback.

The Metal probe checked retained revision 0, new revision 1 and both selected
entry-point names while verifying the independent GPU outputs. Focused library/
example Clippy and the WebGL-enabled WASM library cross-check passed.


### Vehicle asynchronous recovery and API acceptance

Current Metal and Mesa Vulkan/OpenGL execution passed 240 nonblocking vehicle ticks with full CPU state/contact parity, forward/reverse/brake coverage and consumed rejection. A pending vehicle query was invalidated by an actual captured-chunk edit: the task rejected stale contacts, rejected further consumption, and a fresh task matched the complete CPU vehicle state/report. The Linux and NVIDIA Vulkan/DX12 runners now require both vehicle markers. Strict example/library Clippy and shell/PowerShell parsing passed.

Native gameplay integration remains pending. These tests prove the vehicle task API on Metal and Mesa software backends, not physical NVIDIA/DX12 or mobile execution.

### Wide HDR to half-float input resolve

The HDR range probe now exercises `TextureBlit::linear_exposed` with exposure 1
from RGBA32Float to a sampleable RGBA16Float render target, matching the color
format currently required by the HDR Streamline import path. Readback checks
131008 and f32::MAX saturation to 65504, unchanged ordinary linear values,
negative RGB clamp and preserved alpha. No tone mapping or transfer encoding
occurs in this pass. Applications should retain wide HDR until this explicit
range-limited boundary; clipping loses values beyond half-float range.

Metal and Linux Vulkan/OpenGL llvmpipe execution passed, as did focused example
Clippy. The full Linux HDR regression suite also passed. These checks establish
conversion behavior only, without claiming SDK resource import, DLSS inference,
Frame Generation or NVIDIA GPU execution.


### Native asynchronous vehicle gameplay

`--gpu-collisions` now runs `PendingGpuVehicle` in native gameplay when CUDA vehicle integration is not selected. Inputs are retained while the step is pending; fixed simulation time and vehicle/race state advance only on successful completion. Stale world queries retry from original state; a 30-second deadline persists across retries. Toggling driving cancels pending vehicle work. `--cuda-vehicle-motion` currently retains the prior synchronous path; combining it with asynchronous vehicle queries remains pending.

Current Mesa Vulkan/OpenGL graphical autopilot runs each passed with 420 GPU vehicle ticks, 179 GPU character ticks, 7 GPU water ticks, 5 water transactions, 3 explosion commits and 2461 presented frames. The Linux runner now requires nonzero GPU vehicle ticks. Native application compilation passed before subsequent parallel physics edits. The first Metal gameplay process ended without autopilot completion; the retry failed compilation in the concurrently edited `liquid/pressure_work.rs` (E0506). Thus this change has verified Mesa gameplay, not Metal gameplay acceptance. Physical NVIDIA/DX12/mobile/VR and full engine support remain open.

### Owned reusable HDR half resolve

`HdrHalfResolvePipeline` retains a linear unexposed conversion pipeline.
`prepare` validates a matching-device, sampleable single-layer/single-sample
RGBA16/32Float source and returns an independent `HdrHalfResolveJob`. Its output
is sampleable RGBA16Float with copy-readback usage; `encode` runs after source
production. Finite linear RGB clips to [0,65504], with alpha in [0,1] preserved
at half-float precision. No SDK import or temporal metadata is synthesized.

The existing HDR range proof now exercises this public owner/job API. Metal,
Linux Vulkan/OpenGL llvmpipe, focused Clippy and Windows library cross-check
passed. A shared physics borrow failure was repaired by capturing both pressure
values before mutating velocities, instead of retaining a closure borrowing the
particle slice. All eight pressure-work energy/rollback regression tests passed.
Physical SDK inference and mobile/NVIDIA execution remain unverified.


### CUDA motion retained by asynchronous vehicle queries

`PendingGpuVehicle::try_step_with_integrator` invokes a caller-supplied chassis integrator only when creating its retained character task. Native `--gpu-collisions --cuda-vehicle-motion` now uses CUDA f64 motion once and keeps the result across asynchronous WGSL collision polls. Integration failures reject the vehicle step; original state remains unpublished. NVIDIA Vulkan/DX12 gameplay gates require nonzero completed GPU vehicle ticks for this combination.

Actual Metal's 240-tick vehicle comparison alternates built-in integration and supplied exact CPU Euler motion, asserting one integration callback per tick and full CPU state/contact parity. Native application compilation with CUDA passed. This proves callback retention and source wiring, not physical CUDA/WGSL combined execution; that acceptance remains pending.

### Ray lighting to half-float HDR GPU chain

The rough-primary ray probe now feeds spatially distinct GGX point lights plus
an eight-sample reflection mean into wide HDR composition, then into
`HdrHalfResolvePipeline`. Only final verification performs readback. Every
half-float RGB channel is compared with the independent CPU lighting/reflection
expectation including range clipping and quantization tolerance; alpha must have
exact half-float bits for one. All 16-pixel fixtures at three roughness values and
three seeds passed on Metal together with the full ray regression suite.

This is a GPU-produced sampleable color resource with the current SDK input
format. It does not yet prove DX12 resource tagging, matching temporal metadata,
Ray Reconstruction inference or generated-frame presentation.


### Metal native vehicle gameplay acceptance

The current Apple M4 Max Metal graphical autopilot finally completed successfully: 420 nonblocking GPU vehicle ticks, 179 GPU character ticks, 7 GPU water ticks, 5 water transactions, 3 explosion commits and 114 presented frames. Full gameplay API acceptance on Metal is now verified for `--gpu-water --gpu-collisions`; actual CUDA combination still needs NVIDIA hardware.

Vehicle recovery acceptance now also injects an integrator error before any chassis task is created, checks rejection and consumed semantics, and verifies a fresh task against CPU. Metal collision/vehicle probe and strict crate-only Clippy passed. Native window-close diagnostics record incomplete autopilot tick and completed GPU counts when a close event arrives; no close interruption occurred in the final successful run. Earlier incomplete runs have no verified cause.


### Browser WebGPU asynchronous vehicle execution

The current in-app browser WebGPU run passed 485 checks: four continuous sweeps, 240 character ticks, 240 vehicle ticks and character stale recovery. Vehicle state and complete contact reports match CPU on every tick; forward/reverse/brake are required, and repeated task consumption rejects. WASM compilation and crate-only strict Clippy passed. Screenshot: `docs/browser-vehicle-proof.png`; cache version: `vehicle-validated-v1`.

This verifies vehicle compute/controller API execution in the browser. The visual shell still renders its triangle/quad demo; full voxel browser gameplay and mobile/VR/NVIDIA/DX12 acceptance remain unfinished.

### HDR resolve validation boundary (2026-10-01)

`HdrHalfResolvePipeline::new` rejects insufficient fragment sampling, uniform-buffer and binding limits before pipeline creation. `hdr_range` now verifies that RGBA8 input, an unsampleable RGBA32 texture and a layered RGBA32 texture return `InvalidGeometry` without emitting GPU validation errors. Valid linear half-float conversion still checks clipping, negative RGB and alpha by GPU readback.

Validation: native Metal on Apple M4 Max passed; strict Clippy for the render library and `hdr_range` passed; `tools/linux/hdr-smoke.sh` passed all five HDR fixtures on Mesa llvmpipe Vulkan and OpenGL. Linux execution uses a software adapter. This validates the HDR conversion boundary, not native NVIDIA DLSS/Frame Generation execution on RTX 5090.

### Explicit base mip for SDK HDR resolve (2026-10-01)

HDR half resolve now creates a source view restricted to mip zero and documents this contract. The GPU fixture uses a two-level RGBA32 texture with deliberately different data in mip one; readback proves base-level RGB/alpha and full base resolution. Metal on Apple M4 Max, strict render library/example Clippy and the complete Linux software Vulkan/OpenGL HDR smoke suite passed. Physical NVIDIA inference and frame generation remain unverified.

### Owned wide-HDR scene Ray Reconstruction import (2026-10-01)

`SceneRayReconstruction::import_wide_radiance` connects the renderer's reusable `HdrHalfResolvePipeline` to RR resource import. It retains the resolve job in the temporal candidate; `prepare` records base-mip linear half conversion before DX12 resource transitions. The existing RR importer retains the converted output in its native lease set. Presentation identity, reset state, depth, motion and material guides still come from the exact caller-supplied candidate; this API does not invent missing temporal data. Radiance producers must be encoded first and all resources/pipeline must share the registered device under the documented unsafe queue/lifetime contract.

Validation: Windows GNU cross-check for every Streamline example with `scene-dx12` and SDK 2.14.1 passed; strict Windows-target library Clippy passed. The conversion itself was previously verified by GPU readback on Metal and Linux software Vulkan/OpenGL. Native RR evaluation with this composed path on physical NVIDIA hardware remains unverified.


### Actual voxel geometry in the browser scene

`?backend=webgpu&voxel=1` and `?backend=webgl&voxel=1` now replace the demo geometry with the origin chunk from `build_procedural_scene(42, 0)`, converting its real greedy quad mesh into shared `SceneMesh` vertex/index buffers. The world is retained in `WebEngine` for subsequent gameplay integration. Colors use the bootstrap material IDs and captured ambient occlusion; the demo overlay is hidden. Generation/meshing are on CPU, rendering is through the requested browser graphics API.

Current in-app WebGPU and WebGL execution visibly displayed 190 voxel quads. WASM build and crate-only strict Clippy passed; screenshot: `docs/browser-voxel-proof.png`. This is an actual world geometry entrypoint, not complete browser gameplay: character/vehicle controls, world edits, physics/render coordination and full hardware acceptance remain pending.

### Atomic compute kernel selection during reload (2026-10-01)

`ComputeProgram::reload_with_entry_point` validates and publishes WGSL source and kernel selection together. Identical source with a different entry point compiles a new pipeline and advances the program-local revision; only both matching values are a cache hit. Failed replacement leaves the old kernel and revision intact. Existing `reload_shader` preserves the selected entry point and delegates to this atomic path.

GPU proof now submits old source, changed source, independently selected alternate kernel and a dynamically switched kernel together. Exact integer readback checks all four results, and missing-kernel rejection preserves revision two. Metal on Apple M4 Max and Linux Mesa llvmpipe Vulkan/OpenGL passed, as did strict render library/compute example Clippy. Software Linux execution does not prove physical NVIDIA compute performance.


### Browser world edits and mesh rebuild

The voxel scene's `Destroy center` button now finds the highest solid block at local (16,16), plans a radius-three explosion through the shared destruction API, commits its actual world transaction, and rebuilds the origin chunk's halo-aware mesh/light via `rebuild_bootstrap_chunks`. The derivation epoch increments with checked overflow and the updated `SceneMesh` is uploaded to the renderer.

Actual WebGPU and WebGL browser interactions each destroyed 75 blocks and visibly displayed the resulting crater. Current WASM build and strict crate-only Clippy passed. Screenshot: `docs/browser-voxel-edit-proof.png`; cache version: `voxel-edits-v1`. Destruction/meshing are CPU operations and rendering uses the selected API. Browser character/vehicle controls, world-water simulation and complete hardware acceptance remain unfinished.

### Compute reload cross-platform compilation (2026-10-01)

After atomic kernel-selection reload, `voxy_render` library checks passed offline/locked for Windows GNU x86_64, Android ARM64, iOS ARM64 and wasm32 with explicit `webgl`. This supplements actual Metal and Linux software Vulkan/OpenGL compute readback. Compilation alone does not prove mobile deployment, browser compute execution or NVIDIA runtime behavior; WebGL compute capability remains device/backend-dependent and must be rejected where compute limits are absent.

### Scene shader reload device ownership (2026-10-01)

`SceneRenderer` now retains its device. Shader reload rejects a different device before source-cache lookup or GPU allocation, and compiles replacements on the retained owner even where instance-local device identifiers collide. The shader smoke fixture adds rejection on a second device with no validation error and unchanged revision, followed by its existing reload/pixel checks.

Strict render library Clippy passed. The native fixture did not run: its build failed in concurrent physics changes because `liquid/gas.rs` accesses private `ImagePair` fields. The new fixture is pending GPU verification; no passing runtime result is claimed.

### Scene reload GPU verification completed (2026-10-01)

The physics field-visibility blocker was corrected in the shared worktree. Native Metal `scene_smoke --shader-test` and `--shader-test --msaa` now pass: second-device rejection emits no validation error, revision remains unchanged, shader syntax/ABI failures retain the working replacement, and final pixels verify depth/textures/overlay. Strict render library and scene-example Clippy passed before a subsequent argument-filter fix. That fix prevents `--msaa` from becoming the output filename; the corrected MSAA command also passed and writes the intended temporary PPM. Windows/wasm library compilation was verified in the preceding turn. Other GPU backends have not yet run this new second-device scene fixture.

### Linux scene shader reload and GL multi-device limitation (2026-10-01)

New `tools/linux/scene-shader-smoke.sh` builds the actual scene fixture offline and runs Vulkan/OpenGL, each in single-sample and MSAA4 modes. Mesa llvmpipe passed all four final pixel checks. Vulkan additionally passed the second-device rejection check. On GL, creating a second device on the adapter caused the original scene's depth/texture pixel check to fail (black instead of expected blue); the root cause remains unresolved. Default GL tests now explicitly print that the second-device probe is skipped and verify normal single-device reload. `VOXY_SCENE_PROBE_GL_MULTIDEVICE=1` re-enables the failing probe for investigation. This is a recorded multi-device limitation, not a claim of full GL multi-device support. The Linux adapter is software rendering.

### GLES second-device VAO regression fixed (2026-10-01)

The vendored GLES adapter binds a per-device VAO when opening a device, while queues share the adapter context. A second device can replace that binding and delete its VAO at teardown; the original queue previously assumed its VAO stayed bound. Queues now retain their device VAO handle and explicitly bind it before executing submissions. The previously failing GL second-device shader/pixel fixture now passes on Mesa llvmpipe in both single-sample and MSAA4 modes; Vulkan also passes. The default fixture again tests second-device rejection on GL, without a skip or environment opt-in. This fixes the observed regression, not every possible multi-device state interaction.

### GLES fix platform gates (2026-10-01)

With queue VAO rebinding and the default second-device fixture restored, the complete Linux scene shader script passed on software Vulkan and OpenGL in normal/MSAA4 modes without skipped ownership checks. Post-fix render library checks also passed for Android ARM64 and wasm32 with WebGL enabled, compiling the shared GLES changes on both target configurations. These checks do not prove Android device deployment or browser rendering. A transient editor `SceneEdit` import build failure was already fixed in the shared source before the successful Linux retry.

### Post-fix browser WebGL runtime (2026-10-01)

Rebuilt the development WASM bundle with wasm-bindgen 0.2.127 after GLES VAO rebinding. Navigated the existing local browser demo to `?backend=webgl`; visible status reports `Gl`, canvas frame count reached 1464, and screenshot shows the colored 3D primitive plus blue 2D overlay. Captured `/tmp/voxy-webgl-gles-proof.png`. This proves normal browser WebGL rendering after the shared GLES change, not WebGL multi-device behavior or browser compute support.

### Browser voxel character gameplay

The retained browser voxel world now drives a visible character marker with WASD movement, J jump and Space pause. WebGPU uses the nonblocking PendingGpuCharacter broadphase; exact f64 contact resolution remains on the CPU. WebGL explicitly uses the CPU collision controller with GPU rendering. Fixed simulation steps accumulate actual frame time, and GPU stale-world retries retain the original input and a 30-second deadline. Diagonal input is normalized.

Verified locally in the in-app browser: both backends settled at local center y=0.05625, keyboard J increased height (WebGPU y=0.121875; WebGL y=0.12614584), destroying 75 center blocks left simulation running and both settled in the crater at y=-0.13125. WebGPU pause reported data-paused=true. The character-position and character-ticks attributes come from completed controller ticks. Screenshot: browser-character-gameplay-proof.png. WASM development build and scoped strict Clippy passed. Sustained WASD movement, browser vehicle/water gameplay, physical NVIDIA/Windows/mobile/VR acceptance and fully GPU-resident physics remain unverified or unfinished.

### XR predicted-time discontinuity (2026-10-01)

`XrHistoryReset` now tracks the previous predicted display time and clears combined application history on a backward timestamp. Equal timestamps permit retries; pending reference-space events retain their effective timestamps. Explicit lifecycle reset clears the previous-time baseline. A regression verifies backward time, equal-time retry, future origin application and a fresh timeline after reset. All 15 XR library tests and strict library Clippy passed. This is deterministic lifecycle logic, not physical-headset runtime proof.

### XR begin-frame history error handling (2026-10-01)

Session integration already routes lifecycle events and stereo location through `XrHistoryReset`. New `XrSession::begin_frame_with_history` also clears application history on lifecycle validation, runtime wait or frame-begin errors. Successful begin preserves history until `locate_stereo_views_with_history` validates tracking and predicted time; it does not commit a frame or change pending-frame retry semantics. Existing 15 XR tests, strict library Clippy and Android ARM64/Windows GNU library checks passed. No native-runtime fault injection or headset execution was performed for the new wrapper.

### CUDA current-source verification (2026-10-01)

Current CUDA-feature library tests passed (12 normal tests), and the explicitly enabled native clang++ gravity arithmetic parity test also passed. Strict CUDA-feature library Clippy and Windows GNU library cross-check passed. Executing `cuda_probe` on this Apple host returned `DriverUnavailable`; no CUDA GPU kernel or RTX5090 runtime/performance result is claimed. The parity test compiles CUDA source arithmetic for host comparison and cannot substitute for device execution.

### Browser vehicle in the retained voxel world

Added Enter/Exit vehicle with a shared voxel chassis controller. WebGPU retains PendingGpuVehicle and input across asynchronous polls and stale-world retries, with a 30-second deadline; WebGL executes the CPU reference controller. Screen throttle (coast/forward/reverse), steering and brake supplement WASD/B for accessible control. Enter/exit preserves the current local foot position and planar center, clears pending work and never publishes an incomplete GPU candidate. Vehicle marker scale and heading follow driving state.

Live in-app browser proof: WebGPU forward moved local z from 0.000125 to 1.075, reverse moved it back to 0.25278017, steering moved local x to 0.5375, brake held the same position on subsequent observations, and exit retained x/z=0.5375/0.2625 (center y correctly changes with body height). WebGL forward reached z=1.075 and reverse moved to z=-2.0125 with completed vehicle ticks. These are gameplay observations, not performance measurements or fully GPU-resident physics claims. Exact contacts/vehicle forces still execute on CPU for WGSL broadphase, and physical CUDA/RTX acceptance remains outstanding.

### Dynamic FG zero-capacity preflight (2026-10-01)

`FrameGeneration::validate` now rejects dynamic FG when the reported maximum generated-frame count is zero, even if the dynamic-support flag is one. This aligns Rust preflight with the renderer's availability requirement. A regression covers the contradictory snapshot and preserves disabling via `Off`. All 8 Streamline/render-policy tests, strict library Clippy and Windows SDK scene-DX12 library cross-check passed. The native C++ configuration branch still requires an equivalent zero-capacity audit; no physical FG execution was performed.

Final vehicle build: scoped strict WASM Clippy and development wasm-bindgen build passed. Reloaded final build completed 384 WebGPU vehicle ticks; screenshot browser-vehicle-gameplay-proof.png shows the existing mesh used as a vehicle marker, with Forward and Brake selected. A dedicated vehicle mesh is not yet provided.

### Native Dynamic FG capacity parity (2026-10-01)

The C++ Streamline configuration path now also rejects dynamic FG with zero `numFramesToGenerateMax`, even when dynamic support is true. The fake SDK regression sets precisely that contradictory state and verifies both `dynamic_unavailable` and unchanged options-call count. Native CMake build and CTest against official SDK 2.14.1 headers passed, as did the Windows GNU `scene-dx12` library cross-check rebuilding the C++ bridge. The native test substitutes SDK callbacks; it does not execute FG/MFG on NVIDIA hardware.

### FG SDK options failure propagation (2026-10-01)

Native fake-SDK tests now separate successful state query from a failing options call. After accepted Dynamic FG at 144 FPS, a 240 FPS request returning `eErrorDriverOutOfDate` verifies exactly one options attempt and preserved `sdk_failure`/SDK result. The fake keeps its last accepted options unchanged; this is a test oracle, not a guarantee that a real SDK failure never changes internal state. Clearing the injected failure allows the next request to succeed. CMake build and CTest passed against SDK 2.14.1 headers.

### Current integrated ray-query verification (2026-10-01)

The current `voxy_ray_probe --experimental` passed on Apple M4 Max Metal after the shader ownership/GLES changes. It checks TLAS transforms/shared BLAS/masks/removal, retained scene after rejected updates, finite shadow segments, primary depth/world surfaces and RG16 motion/background reset. GGX fixtures cover three roughness values and three seeds in both occluded/unoccluded outer cases; GPU readback compares single samples, eight-sample reflection means, direct lighting/opaque shadows, multi-light wide-HDR composition and half-float resolve against CPU/analytic expectations. This verifies the renderer producer chain; native NVIDIA RR inference and frame generation are separate unverified steps.

### Browser gameplay world uses WGSL terrain generation

WebGPU voxel scene loading now asynchronously generates all 27 center/halo chunks before invoking the existing renderer-neutral bootstrap builder with PreparedTerrain. The adapter remaps the compute palette into the actual scene registry, rejects unknown IDs, duplicate chunks, missing requested positions, mismatched seed and cancellation; selected GPU errors propagate without CPU fallback. WebGL keeps the explicitly CPU-generated scene. Meshing and lighting remain CPU operations. Generated chunks are read back to the authoritative editable World; this is not a fully resident GPU voxel engine.

Optional `?backend=webgpu&voxel=1&worldCheck=1` compares all 884736 retained center/halo cells against an independent CPU terrain generator using the actual scene registry. The same gate can check the explicit WebGL CPU scene. It expects pristine terrain and deliberately rejects later edits.

Live acceptance: final in-app WebGPU scene reported GPU generation of 27 chunks and matched all 884736 retained blocks to the CPU reference; WebGL separately matched 884736 with its explicit CPU generation. Both produced 190 visible quads. WebGPU gameplay continued through 1414 completed character ticks after verification. Screenshot: browser-gpu-world-generation-proof.png. Scoped strict WASM Clippy, development build and JavaScript syntax check passed. This does not establish native NVIDIA/CUDA or mobile/VR hardware acceptance.

### Scene resource device preflight (2026-10-01)

Mesh upload, dynamic geometry reservation, texture upload and transform creation now reject foreign devices with `SceneError::DeviceMismatch` before allocating resources or writing queues. Successful allocation uses the renderer's retained device, including instance-local ID collisions. Texture queues must still belong to that device under the documented caller contract. The GPU scene fixture exercises all four rejection paths under a validation scope and then verifies normal rendering. Metal, Linux software Vulkan/OpenGL (single-sample and MSAA4) and strict render library/scene-example Clippy passed.

### Application-level platform gates after scene ownership checks (2026-10-01)

Current `voxy_web` wasm library and `voxy_app` Windows GNU library checks passed, covering callers of the new `SceneError::DeviceMismatch` API. Android/iOS `voxy_mobile` checks failed before application compilation in BLAKE3 1.8.7's ARM NEON C build: Android lacks `aarch64-linux-android-clang`, and iOS lacks the `iphoneos` SDK. Earlier mobile checks predate this native dependency gate and cannot establish current mobile compilation. Full Android NDK/iPhoneOS toolchains must be configured and these checks rerun; no mobile success is claimed from the passing renderer-only checks.

### iOS architecture slices and current mobile SDK gates

The Xcode Rust build script now handles the standard simulator ARCHS list `arm64 x86_64`, builds each Rust target, and creates the simulator universal archive with xcrun lipo. Device arm64 and single simulator slices use a direct copy. Every architecture is validated before starting any build; paths are passed as separate quoted arguments. Four mocked orchestration tests passed (device debug, simulator universal release with spaces, invalid second architecture, wrong platform), plus shell syntax. These tests do not prove native linking or device execution.

Current full voxy_mobile checks for Android ARM64 and iOS ARM64 failed before application validation: the new voxy_assets BLAKE3 C build requires respectively aarch64-linux-android-clang and the iphoneos SDK, both absent on this host. Earlier mobile cross-checks therefore do not establish current whole-application compilation. Hardware/SDK blockers must remain explicit; SIMD support has not been disabled to hide the missing tools.

### Mobile packaging toolchain audit (2026-10-01)

No Android SDK/NDK or full Xcode app was found in the inspected standard local installation directories. Android packaging already configures the target linker, C compiler and archiver from the explicitly supplied NDK; iOS packaging preflights the selected SDK with xcrun. Do not treat direct cargo checks without these toolchains as packaged-app proof. Current Android artifact/device-selection tests passed (9), iOS architecture/profile packaging tests passed (4), and iOS shell syntax passed. Native libraries, APK signing/installation and physical-device launch remain unverified in this environment.

Separate current voxy_gpu library cross-checks passed offline/locked for both aarch64-linux-android and aarch64-apple-ios, including the shared render and asynchronous character/vehicle controllers. This validates those libraries only; the whole mobile application is still SDK-blocked as described above.

### Current CUDA kernel compilation and Linux build bounds

Re-ran pinned NVIDIA NVRTC 12.6 and PTXAS on the current gravity, terrain, projectile, voxel-regions, exact f64 box-sweep and affine kernels: all compiled/assembled for sm_52, sm_75 and sm_89. Unsupported architecture 999 was rejected before compilation. This is compiler validation without GPU execution. Linux physical-NVIDIA acceptance now bounds each of its five Cargo build stages to 900 seconds, matching the existing Windows gate policy; physical probes retain their 300-second limits. Native CUDA/RTX execution remains outstanding.

### Current Linux collision/water acceptance and locked Rush cache

Removed the obsolete sibling-tokenizer checkout requirement from the voxel-region, water and gameplay-water Docker runners. The current voxy_rush dependency is a locked Git release dependency, resolved through the existing read-only Cargo Git cache mount. All three scripts passed shell syntax; region and water runners were actually executed with VOXY_TOKENIZER_ROOT pointing to a nonexistent path.

Current Mesa llvmpipe Vulkan and OpenGL each passed exact 240-tick nonblocking character and vehicle state/contact parity, integration-error/NaN rejection and stale-world recovery, 4913-obstacle late-contact/tie fixtures, exact far i64 fault coordinates and 257 classification queries. Water separately passed 16 ticks/524288 CPU cell comparisons per backend, conserved volume, canonical transfers, stale transactions and independent readback/error recovery. This is software Vulkan/OpenGL execution, not physical GPU performance or NVIDIA acceptance.

The updated gameplay-water runner also passed with the deliberately missing sibling Rush path: current Mesa Vulkan and OpenGL each completed 420 GPU vehicle ticks, 179 GPU character ticks, 7 GPU water ticks, 5 water commits, 3 explosions and 2461 presented frames. The native application compiled in the Linux container with its current asset dependencies. Logs are target/linux-docker/gameplay-water-vulkan.log and gameplay-water-gl.log. This does not remove the Android NDK/iPhoneOS SDK or physical NVIDIA/headset acceptance requirements.

### HDR resolve straight-alpha regression (2026-10-01)

Expanded the GPU half-resolve fixture to four pixels, including alpha zero with nonzero HDR RGB and alpha 0.25 with independent RGB. Readback verifies conversion preserves straight alpha and does not premultiply or clear hidden color. Existing wide-range clipping, negative RGB and base-mip checks remain. Metal, complete software Linux Vulkan/OpenGL HDR smoke suite and strict render library/HDR example Clippy passed.

### Linux shader-runner dependency repair and current compilation failure

HDR, temporal-motion and windowed-auto-exposure Docker runners no longer require a sibling tokenizer path. Temporal-motion and windowed-auto-exposure now mount the read-only Cargo Git cache, matching the current locked Rush Git dependency; HDR retains its existing cache mount. All three scripts passed shell syntax and were invoked with a deliberately nonexistent VOXY_TOKENIZER_ROOT.

All three actual runs stopped at the current physics compilation: liquid/reaction.rs:60 invokes Liquid::effective_properties, which is absent from the current Liquid implementation. No shader execution occurred and these runs are not passing acceptance. Concurrent physics work was preserved. This compiler failure is distinct from missing mobile SDKs and physical NVIDIA/VR hardware requirements.

### Shader acceptance resumed after physics compilation repair

Revalidated after the current physics reaction API changed: the former Liquid::effective_properties compiler failure is no longer present. The repaired HDR runner completed on Mesa Vulkan and OpenGL with a nonexistent VOXY_TOKENIZER_ROOT: range/half resolve, tone mapping/sRGB/alpha, HDR10, RR material-guide readbacks, automatic exposure and radiance accumulation passed. The windowed auto-exposure/temporal runner also passed both APIs, including geometry readback, opaque depth/stationary motion, prepare-submit-publish and consumer-failure/resize resets. These are software llvmpipe results; HDR-display output, native NVIDIA RR inference, CUDA and headset execution are not established by them.

The temporal-motion runner subsequently passed on both Vulkan and OpenGL: primary/background identity and resets, relative motion at 100000/200000 world coordinates, composition, skinned presentation with skipped pose/resize/suspension, and asymmetric stereo motion with skipped frames/tracking recovery. XR fixtures here use synthetic stereo inputs, not a physical headset runtime.

### Current scene shader reload acceptance

The current scene-shader Docker runner passed all four combinations of Vulkan/OpenGL and single-sample/MSAA. It exercised device ownership rejection, live shader replacement, unchanged-source caching, syntax/entrypoint/binding rejection preserving the previous shader revision, and pixel readback for indexed geometry, depth occlusion, RGBA textures and overlay alpha. The same --shader-test --msaa probe ran successfully on physical Apple M4 Max Metal. These results establish the tested general-scene shader path, not full NVIDIA/DX12/CUDA or mobile/VR execution.

### Voxel diagonal metadata reaches both rendering paths

Browser mesh conversion now emits indices for QuadDiagonal::Uv or Vu instead of always using the Uv diagonal. The native voxel WGSL vertex stage now reads the packed high bit of ao_diagonal.w and chooses the matching corner sequence; AO values continue to mask that bit before shading. Both diagonals retain the same winding. Current mesher output uses uniform AO and Uv, so a dedicated nonuniform-AO/Vu pixel fixture is still required to prove the alternate interpolation visually.

Scoped strict WASM Clippy passed. Actual Apple M4 Max Metal native gameplay with the changed voxel shader completed 420 GPU vehicle ticks, 179 GPU character ticks, 7 GPU water ticks, 5 water commits, 3 explosions and 163 presented frames. This proves shader compilation and the current gameplay path, not the pending dedicated Vu interpolation fixture or NVIDIA/DX12/mobile/VR execution.

### Shared presentation-device subsystem factories (2026-10-01)

`SceneSurface` now creates explicitly selected `ComputeProgram` kernels and reusable `HdrHalfResolvePipeline` instances on its retained presentation device, alongside its existing scene renderer factory. Application code can initialize raster/shader, compute and wide-HDR conversion from one owner and schedule jobs in the same frame encoder through existing hooks. HDR resolve does not negotiate the display format or enable HDR by itself. Strict render library Clippy passed. A combined application-level runtime fixture for these new factories remains to be added; full engine integration is not claimed from factory compilation.

### One-owner scene/compute/HDR frame integration (2026-10-01)

`VOXY_TEMPORAL_INTEGRATED=1 cargo run -p voxy_render --example temporal_surface_smoke` now initializes raster, an explicitly selected compute kernel and half-HDR resolve through `SceneSurface` factories. Each submitted candidate encodes compute and HDR conversion in the same encoder as final display processing. The resolve output is actually used as the display source; compute readback checks frame-identity-derived integer results exactly. The injected consumer-failure candidate is discarded before submission; retries, resize reset and presentation-only history publication remain exercised. This verification path blocks for GPU readback; production frame scheduling does not require that wait.

Both integrated mode alone and with `VOXY_TEMPORAL_AUTO_EXPOSURE=1` passed on Apple M4 Max Metal. Strict library/example Clippy passed. This connects scene, compute, HDR and GPU exposure under one owner; asset-catalog orchestration and other platform runtime fixtures remain separate unfinished work.

### Integrated frame Linux runtime (2026-10-01)

New `bash tools/linux/integrated-frame-smoke.sh` runs the one-owner native-window scene/compute/HDR fixture in a network-disabled container, both with and without GPU auto exposure on Vulkan and OpenGL. All four runs passed on Mesa llvmpipe, including exact compute readback, HDR scene/layer readback, motion/depth, injected consumer error and resize/history recovery. The display source uses the resolved HDR texture. This is software Linux runtime proof; physical Linux GPU, Android and iOS runtime coverage remains outstanding.

### Integrated compute shader reload (2026-10-01)

The integrated native frame fixture now rejects invalid WGSL without advancing revision, then replaces the compute kernel after the injected failed frame. Successful subsequent submissions must produce the new multiplier's exact GPU result while HDR resolve, exposure, resize and presentation history continue. Reload is guarded by revision so a skipped acquisition can retry without reapplying it. Integrated auto-exposure mode passed Metal; Linux Vulkan/OpenGL passed with and without exposure. Strict render library/example Clippy passed. This is compute-kernel hot reload inside the shared frame fixture; asset publication and raster material reload still have their separate integration paths.

### Transactional scene mesh replacement (2026-10-01)

`SceneRenderer::replace_mesh` validates and uploads a complete replacement on the retained device before swapping the published GPU geometry. Upload errors leave the old geometry intact; successful replacement preserves the target's depth mode. The real-file threaded OBJ reload fixture now uses this API and additionally attempts a NaN-color mesh replacement before each pixel verification. It retains transparent depth mode across successful/failed replacement. Metal pixel readback passed: failed import retains pixels, corrected OBJ moves 16 pixels, and an old asset snapshot still renders its old geometry. Strict library/example Clippy passed. GPU out-of-memory/device-loss handling is not established by validation-error tests; asset orchestration with the integrated frame fixture remains unfinished.

### Surface-owned mesh replacement and temporal reset (2026-10-01)

`SceneSurface::replace_scene_mesh` uploads and commits replacement geometry on
its presentation device, then invalidates temporal history. Invalid meshes and
foreign renderer devices return before changing the geometry or temporal state.
The integrated temporal fixture rejects a NaN mesh while a presented history is
available, checks that its presentation ID survives, and replaces valid geometry
before frame 3, asserting that the next presented frame requests a history reset.
The fixture also covers compute shader reload, half-float HDR conversion,
consumer failure recovery, resize, and optional GPU auto exposure.

Verified on Apple M4 Max Metal with integrated auto exposure, and in the Linux
container on both Vulkan and OpenGL, each with and without auto exposure (four
successful runs). Linux uses Mesa llvmpipe CPU rendering, not hardware GPU proof.

### Material replacement participates in frame history (2026-10-01)

`SceneSurface::replace_scene_texture` validates/uploads an sRGB RGBA8 texture
with explicit sampling on the surface's device and queue before replacing the
active material. Only success invalidates temporal history. The integrated
window fixture checks malformed texel input preserves the previously presented
history and uses a sixth frame to check material replacement resets history
independently of the earlier mesh replacement and resize. Existing five-frame
non-integrated modes retain their original lifecycle.

Passed on M4 Max Metal with integrated HDR/compute/auto exposure and on Linux
Mesa llvmpipe Vulkan/OpenGL with and without auto exposure. Package-only strict
Clippy (`--no-deps`) and diff whitespace validation passed. No hardware Linux
GPU or NVIDIA DLSS inference claim is implied by these checks.

### Decoded images and threaded PNG material reload (2026-10-01)

`SceneRenderer::upload_image` accepts the engine's decoded PNG/JPEG `ImageAsset`
and explicit texture sampling, retaining existing device and GPU capacity checks.
The scene smoke example now uses this path instead of manually unpacking pixels.
`asset_hot_reload_pixels` additionally imports an actual PNG through
`AssetImportWorker`, detects source changes through `SourcePollWorker`, and
renders four catalog snapshots through GPU texture uploads and readback.
Corrupt PNG input preserves the previous pixels; repaired red PNG changes the
material while retaining the geometric center; a held old asset snapshot restores
the original pixels. The resulting strip is `/tmp/voxy-image-hot-reload.png`.
This scenario passed on Apple M4 Max Metal; it does not yet prove device-side
texture reload on all supported platforms.

### Cross-API material source reload proof (2026-10-01)

`tools/linux/asset-reload-smoke.sh` builds offline and runs the threaded OBJ/PNG
reload fixture on explicitly selected Vulkan and OpenGL backends. The fixture
accepts `VOXY_ASSET_BACKEND=auto|metal|vulkan|gl|dx12` and rejects unknown values.
The PNG repair now changes texture dimensions from 1x1 to 2x1; assertions verify
decoding runs off the owner thread, GPU pixels change without moving geometry,
corruption retains the last material, and held snapshots retain old pixels.
All checks passed on M4 Max Metal and on both Linux Mesa llvmpipe APIs. Strict
package-only Clippy, shell syntax and whitespace checks passed. Linux evidence
is software rendering, and the DX12 selector has not been run on Windows here.

### Authored scene material mip chains (2026-10-01)

`SceneRenderer::upload_image_mips` uploads a complete authored `ImageAsset` chain
from the base size down to 1x1. Empty/incomplete chains, unexpected level sizes,
GPU dimension limits and foreign device ownership are rejected before GPU
allocation or writes. Existing single-level upload APIs use the same validated
upload path. Material samplers use the requested minification filter for mip
selection/interpolation as well. This API uploads supplied levels; it does not
currently generate mipmaps or prove alpha-aware mip generation.

The material reload fixture rejects an incomplete 2x1 chain and uploads/renders
a complete 2x1 + 1x1 chain with GPU validation and pixel readback. Linux Vulkan
and OpenGL llvmpipe passed. This checks upload/binding and base rendering, not
LOD selection under minification. Strict package-only Clippy passed.
A blocking editor compilation typo (`Vec<u8>.bytes`) was corrected to pass its
actual byte slice to UTF-8 decoding; no editor behavior redesign was performed.

### Linear-light alpha-aware image mip generation (2026-10-01)

`ImageAsset::mip_chain` now produces a complete CPU mip chain including the base
image. Each step partitions the entire previous level into box regions, including
odd final rows/columns. RGB is decoded from sRGB, weighted by linear alpha,
averaged, unpremultiplied, and encoded back to sRGB; zero-alpha texels produce zero
RGB. This avoids treating sRGB bytes as linear light and prevents hidden color in
fully transparent texels from bleeding into opaque neighbors. The asset reload
fixture now uploads generated levels instead of constructing its final level by
hand. Mips can be prepared on the asset import worker before GPU publication.

Five focused image tests passed, including linear black/white averaging (188),
transparent red beside opaque blue, fully transparent output, and an odd-width
final-column case. Generated mip upload/rendering and failed PNG reload retention
passed on Linux Vulkan/OpenGL Mesa llvmpipe. This does not yet measure temporal
aliasing or prove implicit LOD choice at distance.

### Voxel diagonal AO rasterization proof

Added the voxel_diagonal real-device example and included it in the Linux scene-shader runner. The fixture takes vertex structures and corner/AO evaluation directly from the production voxel.wgsl source, substitutes a local diagnostic camera/light context, and rasterizes two 8x8 AO outputs with packed Uv/Vu metadata and nonuniform corner values [0,3,0,3]. All 128 RGB/alpha pixels are compared to independent analytic triangle interpolation within one RGBA8 code value. It tests actual rasterization and diagonal/AO decoding; the fragment emits AO directly instead of the production material/LCD color pass.

Physical Apple M4 Max Metal and Mesa llvmpipe Vulkan/OpenGL each passed the fixture. The updated Linux runner also passed its existing four scene-shader single-sample/MSAA combinations. This closes the dedicated native vertex-logic AO interpolation check; browser conversion pixel equivalence, native NVIDIA/DX12 and mobile/headset hardware remain separate requirements.

### Scene implicit mip LOD verified by GPU pixels (2026-10-01)

`material_mips` renders a 256x256 one-texel black/white checkerboard on a quad
covering 32x32 pixels through the ordinary SceneRenderer. It compares base-only
upload with the generated complete mip chain. All 24x24 interior sampled pixels
of the mipmapped result must be linear gray (128 +/- 2 in RGBA8Unorm) with opaque
alpha, and the two rendered images must differ. This exercises implicit scene
shader derivatives and sampler LOD selection, rather than explicitly reading a
chosen mip or merely proving upload success. The fixture passed on M4 Max Metal
and on Linux Vulkan/OpenGL Mesa llvmpipe. The existing Linux asset-reload script
now also runs this proof. Strict package-only Clippy, shell syntax and diff
whitespace checks passed. This is static minification evidence, not a measured
moving-camera temporal aliasing or anisotropic-filtering benchmark.

### Voxel diagonal fixture in physical NVIDIA gates

The diagnostic example now accepts explicit dx12 as well as Metal/Vulkan/OpenGL. Linux CUDA hardware acceptance builds and runs its Vulkan fixture; Windows CUDA hardware acceptance builds and runs its DX12 fixture. Both require the 128-pixel AO parity marker and --require-nvidia, which rejects non-NVIDIA adapters and CPU/unknown/virtual device classes before rendering. This independently checks the selected graphics adapter; it does not assert identity with a particular CUDA ordinal on multi-GPU hosts.

Current Windows GNU example cross-check passed with --locked --offline after initial shared lockfile inconsistency resolved. Shell and PowerShell parser checks passed. Actual --require-nvidia invocation on Apple M4 Max Metal rejected the adapter with exit 1 as expected. Physical NVIDIA Vulkan/DX12 execution remains outstanding; compiler and rejection tests are not evidence of that execution.

### Requested anisotropic material filtering (2026-10-01)

`TextureSampling::anisotropy` requests sampler anisotropy from 1 through 16, with
1 as the compatible default. Requests above 1 require linear minification and
magnification; mip filtering follows minification. Invalid ranges and incompatible
filters are rejected in the shared upload path before allocating/writing textures.
As in wgpu, downlevel devices without anisotropic filtering may lower the request
to 1; this field is a request and does not prove hardware feature availability.
Existing explicit complete TextureSampling literals need to include the new field
or use `..TextureSampling::default()`.

The material mip GPU fixture now rejects requests 0, 17 and nearest-filtered 4,
and renders a valid linear-filtered 4 request. Linux Vulkan and OpenGL llvmpipe
passed the existing pixel gray/LOD checks and shader validation. Package-only
strict Clippy passed. This validates sampler creation and mip selection; it does
not yet quantify image-quality improvement under oblique viewing angles.

AO diagnostic maintenance: separated shader-source adaptation, render/readback and independent analytic pixel assertions; removed nested formatting and made WGPU descriptor defaults explicit. Scoped strict example Clippy passed, and the refactored fixture again matched all 128 pixels on physical Apple M4 Max Metal. The initial unrelated concurrent editor syntax failure was gone on the successful verification.

### Mipmapped material replacement on the presentation owner (2026-10-01)

`SceneSurface::replace_scene_image_mips` uploads every supplied image mip on the
surface's device/queue before swapping the material and invalidating temporal
history. Incomplete chains, invalid sampling, oversized levels and foreign
renderers return before material/history changes. Generate CPU chains on an
asset worker for large textures, then publish them through this method.

The integrated temporal fixture rejects an empty chain while keeping the last
published frame, then replaces a complete generated 2x2/1x1 PNG material with
linear 4x anisotropy request before its sixth presentation. Its callback checks
require the new frame's history reset. Linux Vulkan/OpenGL llvmpipe passed with
and without GPU auto exposure (four runs), alongside scene/compute/HDR shader
reload, failure recovery and resize. Strict package-only Clippy passed.

### Current CUDA host-side failure contracts

Current locked/offline voxy_cuda library tests passed: 12 with the cuda feature enabled and 13 with default features; one native-clang arithmetic fixture was ignored in each run. Coverage includes pre-driver budget/ordinal/terrain validation, compiler target bounds, corrupt f64 box contacts and voxel candidates, aggregate projectile allocation limits, external export/readback bounds and metadata. Actual cuda_probe with feature cuda on this Mac returned DriverUnavailable and exit 1, confirming explicit absence of CUDA rather than successful fallback. These are host-side contracts, not NVIDIA kernel execution. The live processes initially delayed output, were polled without replacement, and all reached terminal states.

### Browser mipmapped scene material (2026-10-01)

The browser 2D/3D example now decodes a bundled procedural 64x64 checker PNG,
generates its seven-level alpha-aware linear-light mip chain, and uploads it
through SceneRenderer with linear min/mag/mip filters and requested anisotropy 4.
The rotating primitive now has distinct vertex UVs, making the mapped material
visible instead of sampling a constant texel. The 2D layer shares the material.
The PNG is bundled in voxy_web/assets rather than referencing render examples.

Fresh wasm-bindgen dev build passed. The in-app browser visibly rendered the
checker on both WebGL (`Gl`) and WebGPU (`BrowserWebGpu`), with no captured
warning/error logs. Screenshots: /tmp/voxy-webgl-mips.png and
/tmp/voxy-webgpu-mips.png. This proves browser upload, validation and rendering;
implicit mip minification has separate native GPU fixtures and was not pixel-
measured in this browser turn. Requested anisotropy may be downgraded by backends.

### Explicit native-compiler CUDA gravity arithmetic fixture

Ran the previously ignored cuda_gravity_host_parity test explicitly with CUDA feature enabled and native clang++. It passed 257 bodies across 128 steps with maximum CPU-reference error 0, 2560 orbit steps, singular/overflow rollback, zero-G and softening cases. The harness executes arithmetic from the CUDA source as host C++; this completes that previously skipped fixture but does not establish NVIDIA execution, device-memory ownership or CUDA performance.

Public README corrected against current source: documented the interactive browser voxel world, 27 GPU-generated chunks and 884736-block reference gate, actor/vehicle controls, 485 separate collision fixture checks, native GPU/CUDA chassis collision choices and GPU-water selection. Removed obsolete CPU-only/demonstration claims while preserving explicit CPU meshing/contact/force boundaries, unfinished browser-world water and unverified physical NVIDIA/mobile/VR execution. Documentation-only changes passed diff whitespace checks.

### Stereo discontinuities checked in actual motion pixels (2026-10-01)

The headless `xr_motion` GPU fixture now changes the near and far clipping planes
on separately committed stereo histories and explicitly resets after a teleport.
Both eyes must report invalid history and all 4x4 readback motion pixels must be
zero after each discontinuity, even though eye positions have moved. Existing
asymmetric frusta, opposite per-eye motion, skipped frames and tracking loss
checks remain. `VOXY_XR_BACKEND` selects the graphics API explicitly, and
`tools/linux/xr-motion-smoke.sh` reproduces Vulkan/OpenGL checks offline.
Linux Vulkan/OpenGL Mesa llvmpipe passed both extended scenarios. This checks
actual depth-to-motion GPU execution and does not emulate OpenXR runtime,
headset presentation, latency, reprojection or lens distortion.

### Recoverable discarded OpenXR swapchain image (2026-10-01)

`XrSwapchain::discard_image(timeout)` finishes an outstanding acquisition without
publishing a projection layer. It waits only if still Acquired, returns false on
wait timeout while retaining the acquisition, then releases once Ready. Runtime
wait/release errors retain the owned state for retry or session teardown. Idle
state and negative timeouts are rejected. The caller must finish all GPU use
before calling; this method performs no GPU fence synchronization.

Runtime status-to-state transitions are shared with the actual wait/release path
and tested with timeout, runtime failure, invalid release-before-wait, successful
retry and final release. All 16 XR unit tests passed. This is state-machine proof,
not an injected native OpenXR runtime or headset test. Android and Windows GNU
XR library compilation passed; native package-only strict Clippy passed.

### Browser gameplay-water API foundation

Added BrowserWater plus start_voxel_water/poll_voxel_water on WebEngine. It captures registered water in the retained center chunk, submits immutable GPU plans without blocking the frame, commits only with expected chunk revisions, retries revision conflicts while retaining the 30-second deadline, and rebuilds/uploads the displayed center mesh after successful water commits. WebGL is explicitly rejected by this GPU-only API. Scene replacement drops its previous simulation. Mesh upload failures after a world commit are reported; this is not transactional renderer rollback.

Scoped strict WASM Clippy passed. Interface controls, activation after edits/pouring, frame cadence and live browser-world flow proof remain unfinished; the API foundation is not yet advertised as complete browser water gameplay.

Browser water activation after destruction: the retained simulation now wakes registered water in the displayed center chunk after a successful explosion commit, merges it with existing active positions, and cancels the pre-edit pending plan so its completion cannot overwrite the new activation set. The original deadline remains bounded across these cancellations. Scoped strict WASM Clippy passed. Browser UI/cadence/pouring, live flow proof and general multi-chunk dormant-water activation remain unfinished; this is still the center-scene API foundation.

### Combined stereo-frame cancellation (2026-10-01)

`XrSession::discard_stereo_frame` checks pending-frame state, timeout, blend mode,
and both swapchains' instance/session ownership before performing image calls.
It discards outstanding eyes, skips already-idle eyes on retry, and ends the
pending frame with an empty composition-layer list only after both eyes are idle.
A second-eye timeout returns false and retains the pending frame; runtime errors
preserve recoverable state. No temporal candidate is committed. The renderer
must establish GPU completion first; the method does not perform fence waits.

Sixteen XR unit tests and both compile-only documentation examples passed;
strict package-only Clippy passed after the owner validation addition. The
partial native timeout and runtime end-frame path remains hardware/runtime
unverified. Stored ownership uses both instance and session handles, preventing
cross-instance equal session handles from being accepted by the cancellation API.

### Multiview-compatible XR cancellation (2026-10-01)

`XrSession::discard_frame_images` accepts a mutable slice of distinct swapchains,
including one array-layer swapchain shared by both eyes. This removes the prior
two-distinct-chain restriction of the stereo cancellation helper.
`discard_stereo_frame` delegates with a stack array and no heap allocation.
All ownership, frame, blend and timeout checks precede image release; retries
skip idle chains and only end the frame once all supplied chains are idle.
The application must include every outstanding chain and establish GPU completion.

Sixteen unit tests and strict package-only Clippy passed. Current XR library
checks passed for Android ARM64 and Windows GNU. Compilation of the multiview
usage example is checked by rustdoc; real compositor and partial native timeout
behavior remains unverified without runtime/headset access.

### Resident compute results without extra dispatch or eager staging (2026-10-01)

`ComputeJob::encode_readback` copies the current resident storage without running
another compute step. `encode_step` can now perform exactly the application’s
requested steps, followed by readback when needed. Existing `encode` retains its
one-dispatch-plus-readback contract by delegating after the dispatch. Jobs no
longer allocate a MAP_READ staging buffer at creation; that allocation occurs
only upon final readback, so GPU-resident jobs do not reserve unused host staging.
Nonblocking begin_read/try_read and cancellation semantics remain unchanged.

The native compute fixture now executes its 1025-element job through encode_step
then encode_readback and checks exact one-step results (an accidental extra step
would fail). All 1042 primary results, shader-reload version retention, invalid
shader replacement and readback cancellation passed on M4 Max Metal and Linux
Vulkan/OpenGL llvmpipe. Strict package-only Clippy and diff checks passed. No
throughput improvement is asserted without benchmark measurements.

### Multi-step resident compute in the integrated frame (2026-10-01)

The integrated scene/HDR/exposure temporal fixture now records two compute steps
on one resident job, then uses `encode_readback` to copy its current value without
a third dispatch. Exact readback validation applies the expected shader transform
twice, including after the transactional compute shader reload. Both steps and
HDR resolve/exposure/presentation share the submitted command encoder. The
injected consumer failure discards the encoder before submission; previous
presented temporal state remains governed by the existing recovery checks.
Linux Vulkan and OpenGL llvmpipe passed with and without auto exposure (four
runs). Readback waits remain explicit in this verification fixture; this does
not claim that production frames perform no CPU waits or benchmark improvements.

### Browser WebGPU water gameplay (2026-10-01)

The interactive voxel world now starts the GPU water transaction program before
animation begins. `Pour water` inserts one full-level water block at (16,18,16),
wakes the simulation and rebuilds the displayed center mesh. The frame loop polls
pending readback every frame and advances completed water steps at 0.1 seconds;
settled water sleeps until pouring or destruction. Poll results distinguish
pending (-1), settled (0), and committed (1). Destruction invalidates pending
water plans through wake-up. WebGL does not expose this compute control.

Live in-app BrowserWebGpu proof: the real 27-chunk world passed 884736 CPU-reference
block comparisons, the first pour completed 11 water commits, destruction removed
82 blocks including water, and a second pour raised the commit count to 23.
Character updates continued to 4207 ticks and rendering to 8484 frames. Screenshot:
`docs/browser-water-gameplay-proof.png` shows the first pour on the surface.
WASM dev build, strict target-only Clippy and JavaScript syntax checks passed.
This is CPU-owned editable world storage with GPU transfers and CPU mesh rebuilds;
activation scans the center chunk, so dormant water outside it still needs broader
wake-up support. Physical NVIDIA, mobile and headset acceptance remain pending.

### Compute snapshots while a simulation remains GPU-resident (2026-10-01)

`ComputeJob::encode_snapshot` copies storage at the current encoder position
without consuming the job. Each dispatch owns an independent staging buffer, so
subsequent compute submissions can keep changing resident storage while the CPU
maps older snapshots. `encode_readback` delegates to this operation before
consuming the job. Applications must bound outstanding snapshots to their memory
budget; there is no implicit unlimited snapshot queue.

The compute fixture runs four separate submissions, begins snapshot mapping
after each of the first three, then takes the final consuming readback after the
fourth. Every snapshot must match its own exact iteration and remain unchanged
by later GPU steps. The fixture passed on Linux Vulkan/OpenGL llvmpipe, including
existing shader reload, invalid replacement and cancellation checks. Mapping may
complete before later steps on fast devices; the test asserts independent data
and continued job use, not measured compute/map concurrency or throughput.

### Renderer integration regression sweep (2026-10-01)

Following resident compute snapshots, authored/generated material mips and XR
recovery additions, all 65 current `voxy_render` library tests passed locally.
Coverage includes camera/frustum math, shader resource ownership, mip color/alpha,
materials/GGX CPU analytical cases, skinned and stereo temporal histories,
HDR surface selection and frame-generation policy gates. Current renderer library
checks also passed for Windows GNU and wasm32 with WebGL enabled. These compile
and unit gates complement the preceding native GPU/browser fixtures; they do not
prove DirectX execution, NVIDIA inference, headset presentation or mobile apps.

### Browser water boundary audit and failure containment (2026-10-01)

A live attempt to activate water in all 27 retained chunks failed with
`GPU water snapshot error: Compute(MissingNeighbor)`: outer terrain water needs
neighbors beyond the loaded halo. The expansion was removed; center-chunk wake-up
remains the shipped scope. This is direct evidence that simply scanning more
chunks does not complete streaming-water support.

The JavaScript water poll failure path now records `canvas.dataset.waterError`,
stops water polling and disables pouring while leaving character/vehicle updates
and rendering available. It previously freed the entire engine and terminated
animation. This does not conceal or retry failed water transfers. WASM rebuild,
JavaScript syntax and diff checks passed after reverting the unsafe expansion.

### Transactional ray instance batches (2026-10-01)

`RayScene::set_instances(&[RayInstanceUpdate])` validates capacity, 24-bit custom
indices and every affine transform before modifying any TLAS instance. Updates
share the existing BLAS; duplicate slots apply in input order. The application
still submits earlier queries before mutation and rebuilds TLAS before querying;
this method does not fence GPU work or implicitly rebuild acceleration structures.

The Metal ray probe installs two instances through the batch API, then attempts
a batch whose first update would move the active instance and whose second
transform is singular. Subsequent actual GPU hits retain the valid previous
scene, proving that the first update did not partially commit. The full current
M4 Max experimental ray probe passed (shadows, GGX samples, wide HDR reflections,
primary guides/motion and composed ray lighting). Strict renderer-only Clippy and
whitespace checks passed. An unrelated blocking float literal `.1` in the active
app source was corrected to `0.1` to restore probe compilation.

### Indexed scene meshes enter the ray geometry path (2026-10-01)

`RayScene::from_scene_mesh` constructs a BLAS from the ordinary scene's immutable
validated vertex/index arrays. GPU BLAS input retains shared vertices and uploads
an explicit Uint32 index buffer rather than expanding all triangle corners.
The existing nonindexed constructor shares creation and capacity validation.
Primitive counts use index counts when present; builds bind the index buffer at
first index zero. This geometry path treats triangles as opaque and does not
implement texture alpha intersection filtering or skinned BLAS updates.

The ray probe now initializes its instanced geometry through this bridge, with
four vertices and three indices (one triangle plus an unused vertex), checking
that primitive counting and ray hits follow indexed topology. Strict renderer-
only Clippy and diff validation passed. Runtime results are recorded separately
once the current Metal probe finishes.

Indexed bridge runtime confirmation: the current full experimental Metal probe
completed successfully on M4 Max. Indexed one-triangle hit/miss distances,
instance transforms, masks/removal and failed batch retention passed alongside
all existing shadow, GGX, motion-guide and HDR-composition checks.

### GPU water missing-cell provenance (2026-10-01)

The production water shader now returns status 6 with the first unavailable node
index when a captured unavailable cell is actually read. Uncaptured graph edges
still return MissingNeighbor. Host decoding validates the index and original node
classification before returning UnavailableNode; world plans translate this to
MissingSample with the exact captured VoxelPos. Errors still consume the pending
plan and never publish partial transfers. No unloaded cell becomes air or solid.
This supplies the coordinate needed by a future loader/retry path; automatic
loading and wider browser water activation remain unfinished.

Metal water smoke verified lazy reads, exact missing-cell coordinates, failed-plan
single consumption, stale-revision rejection and 16 ticks / 524288 exact CPU cell
comparisons with conserved volume. Library checks passed 7 tests (1 host-compiler
fixture ignored); strict Clippy for the library and water example passed.

### Vulkan/OpenGL missing-cell shader acceptance (2026-10-01)

`tools/linux/water-smoke.sh` now requires the exact missing-sample provenance and
failed-plan consumption marker on each backend, in addition to existing volume,
CPU parity and revision checks. The updated script passed on both explicit Vulkan
and OpenGL using Mesa 25.0.7 llvmpipe (CPU software execution). Each run completed
16 water ticks / 524288 exact CPU comparisons, lazy missing/unknown-node errors,
sample/write budgets, dropped readback recovery and stale-world rejection.
Logs: `target/linux-docker/water-vulkan.log` and `water-gl.log`.
This confirms the changed WGSL error protocol across those APIs, not physical
NVIDIA/AMD hardware performance or CUDA execution. MissingSample still requires a
loader/retry integration before browser halo water activation can be enabled.

### Ray geometry replacement retains instance placement (2026-10-01)

`RayScene::replace_scene_mesh` prepares replacement indexed geometry on its
retained device, restores all active instance transforms/custom indices/masks and
all removed slots, then swaps the scene. Validation/capacity failure returns
before replacement. RayScene retains a CPU instance description alongside TLAS
entries, updated by both individual and transactional batch operations.
After replacement callers must run full `build` and recreate consumer bindings:
TLAS-only rebuild does not initialize the new BLAS, and old bind groups retain
old TLAS objects. The API does not fence prior GPU work or reset temporal history.

The Metal probe replaces geometry with two active instances and again after
removing an instance/masking another, then checks actual hit/miss results. Initial
fixture failures revealed its TLAS-only rebuild and stale bind group; both usage
errors were corrected and the final full probe passed on M4 Max, including
GGX/shadows/primary guides/HDR. Renderer-only strict Clippy and diff checks passed.
The replacement fixture currently reuploads identical mesh geometry; differing
mesh topology/material import and hardware allocation failures are not covered.

### Loaded-neighbor GPU water retry proof (2026-10-01)

The water smoke now seeds water at x=63 with a solid floor and an air cell at
x=62. The missing x=64 neighbor is read after a provisional leftward transfer.
The failed plan reports that exact position; the authoritative source, left cell
and source-chunk revision remain unchanged. Loading an air chunk at x=2 and
capturing a fresh GPU plan produces exactly the CPU transaction, then committing
it places one water unit at x=64. This verifies that loading plus fresh capture
can recover from a failed partially executed GPU transfer without publishing it.

The new recovery case passed on physical Apple M4 Max Metal and software Mesa
Vulkan/OpenGL; strict example Clippy passed. The Linux acceptance script now
requires its marker on both backends. Browser automatic loading still needs to
integrate this protocol and preserve its pending gameplay intent and timeout.

### Different indexed topology replaces ray geometry (2026-10-01)

The Metal ray probe now replaces its one-triangle indexed mesh with a two-triangle
mesh: the hit surface shifts by 0.25 while a second disconnected triangle is
added outside the tested rays. GPU readback must change hit distance from 1 to
1.25, report two BLAS primitives, and preserve the other instance's disabled mask.
Restoring the original topology must report one primitive and distance 1 again.
Both replacements use a full build and new consumer binding, retaining the same
instance placements. The complete M4 Max experimental ray probe passed; diff
whitespace validation passed. This extends replacement proof beyond identical
mesh reupload, but not to alpha filtering, skinned refits or NVIDIA execution.

### Ray geometry version for consumer binding caches (2026-10-01)

`RayScene::geometry_revision` starts at zero and advances only after successful
indexed mesh replacement, with checked overflow before any resource changes.
Transforms, masks, removal and TLAS-only rebuilds preserve the revision because
the binding object remains the same. Different RayScene owners may have equal
revisions: applications must retain owner identity together with the version.
This is a binding-cache identity, not a full content/temporal-history revision.

The ray probe retains one consumer binding between instance updates and refreshes
it only when this geometry revision changes, asserting expected generations
through four replacements and checking changed-topology ray distances. Prior
code recreated the binding every phase; this now exercises a real versioned cache.
Runtime verification is pending the currently running Metal probe.

### Browser nonblocking GPU water neighbor loader (2026-10-01)

BrowserWater now handles MissingSample by requiring a currently Unloaded cell,
submitting a seed-42 terrain job on the existing WebGPU terrain program, and
polling its PendingTerrain readback from later animation frames. Completed data
is remapped through the same PreparedTerrain palette adapter as initial world
generation and inserted before a fresh water snapshot is captured. Original
active cells and the original 30-second deadline survive generation and retries;
no CPU terrain generation or CPU liquid solver fallback is used. Unavailable data
is rejected instead of replaced. A 64 additional-chunk lifetime limit bounds
retained streaming storage; there is no eviction yet.

WASM dev build passed. Center-only wake-up and center-only displayed mesh rebuild
remain current limitations. The loader branch still needs a live browser boundary
fixture; native Metal/Mesa loading-retry proof validates the world protocol but
is not a browser loader execution proof.

Versioned binding-cache verification completed: the full current experimental
Metal ray probe passed on M4 Max, including revision assertions, binding reuse
for instance updates and rebinding for changed topology. Strict renderer-only
Clippy passed after documentation formatting correction.

### Live browser water neighbor-loader proof (2026-10-01)

`?backend=webgpu&voxel=1&streamingCheck=1` now runs a boundary fixture on a clone
of the GPU-generated browser world. Water at x=63 encounters missing x=64. The
actual BrowserWater poll path submits GPU terrain generation, polls its mapping,
remaps registry IDs, inserts exactly one chunk and commits a fresh water step.
An independent CPU generator and CPU solver build the expected state; all 65536
cells of the source and neighbor chunks match. The in-app BrowserWebGpu run passed
and continued to 829 character ticks / 1661 frames. This verifies the browser
loader, not visual publication of streamed chunks: the fixture is cloned and the
rendered gameplay world still displays the center mesh.

WASM build, strict target-only Clippy, JavaScript syntax and diff checks passed.
Screenshot: `docs/browser-water-streaming-proof.png`. Dormant halo-water wake-up,
streamed-chunk meshes and eviction remain unfinished; no physical CUDA proof is
implied by this browser result.

### Browser streamed-chunk mesh publication (2026-10-01)

The browser mesh adapter now supports multiple chunks using checked anchor-relative
chunk offsets. Conversion rejects offsets outside i16 chunk range instead of
silently rounding arbitrary i64 coordinates. BrowserWater retains visible chunk
positions, adding newly loaded chunks and successfully committed write chunks.
The water commit path rebuilds and uploads this complete visible set. Pouring and
center destruction preserve the existing displayed set instead of resetting to
only origin.

The live BrowserWebGpu streaming check again matched 65536 CPU cells and now also
rebuilds and converts the fixture meshes, requiring translated neighbor geometry
beyond scene x=3. This proves geometry construction for the streamed neighbor;
the fixture remains cloned, so the screenshot itself still shows the ordinary
center gameplay world. A live streamed-neighbor render remains to be demonstrated.
WASM build and diff checks passed. Wider dormant-water activation and eviction
remain unfinished.

### Explicit experimental ray features on a scene surface

`SceneSurface::new_with_adapter_and_output_experimental` accepts the caller's
wgpu experimental acknowledgment token. Existing constructors retain the default
(non-acknowledged) token. Requesting ray queries uses the selected adapter's ray
limits; the engine library retains its unsafe-code prohibition.

Verified on macOS Metal with:
`cargo run -p voxy_ray_probe --example surface_ray --locked --offline -- --experimental`.
The isolated probe creates a window-compatible device and indexed quad BLAS,
builds its TLAS, dispatches visibility queries and reads exact blocked/visible
results `[0, 1]` on that same presentation device. It does not yet composite ray
lighting into a presented frame or establish Windows/NVIDIA runtime behavior.
Clippy passed for the renderer library, temporal surface smoke, and this probe
with `--no-deps -- -D warnings` (unrelated app dependency warnings remain).

### Streamed geometry publication without liquid writes (2026-10-01)

BrowserWater now tracks mesh dirtiness independently of the water step result.
Loading a terrain chunk marks geometry dirty immediately; committing a water edit
also marks it. poll_voxel_water rebuilds/uploads dirty geometry even when the
water result is pending or settled, and only clears the flag after successful
GPU upload. Pending/settled/committed cadence results stay unchanged. This fixes
a loaded chunk remaining invisible when the retried solver produces no writes.

The streaming validator now checks that geometry publication is requested and
that acknowledgement clears its flag. Strict WASM Clippy, dev WASM build and diff
checks passed after a transient shared physics type-inference error was fixed
concurrently. Visible streamed-world rendering still needs its own demonstration.

The Metal `surface_ray` probe now also encodes BLAS/TLAS building, visibility
compute, GPU-buffer fragment visualization, verification copy and window drawing
in a single submission through `SceneSurface::render_custom`. It requires an
actual `Presented` outcome and then checks exact visibility `[0, 1]`. Initial
occluded acquisitions retry via redraw events, bounded to 120 attempts. This ran
successfully on macOS Metal. The output is a visibility diagnostic, not yet
material lighting/reflection composition; no screenshot/pixel readback of the
surface is claimed.

The window probe now feeds `DirectLightingJob`'s RGBA16Float texture into a
fragment shader with Reinhard tone mapping. Two Lambertian material samples
share normal -Z, reflectance `[0.8, 0.4, 0.2]`, point-light intensity 100 and
light distance 2. The quad blocks one sample; the other remains unobstructed.
The same submitted frame copies the HDR texture for half-float verification:
shadow RGB zero; unoccluded RGB `[20/pi, 10/pi, 5/pi]` within 0.005, alpha 1.
The Metal run passed these numeric checks and required successful presentation.
This establishes point-light material/shadow HDR composition in the window path;
full scene G-buffer lighting, reflections and surface pixel verification remain.

### NVIDIA acceptance graphics-adapter enforcement (2026-10-01)

The physical CUDA acceptance scripts previously ran water and voxel-region
shader probes on the graphics API default adapter without proving NVIDIA
selection. On multi-adapter hosts those stages could pass on Intel or software
while later CUDA stages used NVIDIA. Both Linux and Windows scripts now require
an actual water/voxel-region adapter log line with NVIDIA vendor 0x10de (4318) and
DiscreteGpu or IntegratedGpu device type before accepting these stages. A vendor
string elsewhere in the log cannot satisfy this guard. The water stages also
require missing-cell provenance and loaded-neighbor recovery markers.

Shell syntax and PowerShell parser checks passed. Executing the actual extracted
guard functions rejected Intel, CPU-with-NVIDIA-vendor, unrelated NVIDIA text,
and the real Mesa Vulkan/OpenGL logs. Synthetic NVIDIA adapter fixtures passed;
these are guard contract checks, not physical NVIDIA execution. A physical CUDA
acceptance run remains unavailable on this Mac. Vendor validation does not prove
that two NVIDIA adapters are the same device as the selected CUDA ordinal;
interop UUID/LUID matching remains the stronger proof for shared-memory stages.

The Metal window probe now also traces ideal-mirror samples using
`MirrorSurfaceSample` and `SpecularDistancePipeline::create_mirror_job`.
A metallic mirror with linear F0 `[0.8, 0.4, 0.2]` reflects opaque triangle
emission `[4, 2, 1]`. GPU half-float readback verifies hit RGB `[3.2, 0.8, 0.2]`
and miss RGB zero within 0.005, with alpha 1. Direct lighting and reflection
occupy separate diagnostic panels in the same presented frame, sharing the
BLAS/TLAS and command submission. The Metal run passed. This is a single-bounce
emissive mirror probe, not yet full-scene G-buffer reflection or multiple bounces.

### Ordered primary-surface ray lighting

`RayLightingFrame` now owns primary reconstruction, opaque point-light shadows,
per-pixel F0 mirror reflection and RGBA16Float HDR composition. `RayLightingInputs`
connects matching raster depth, world normal/roughness, diffuse and mirror F0 maps
with the exact view-projection/clear depth. One `encode` orders all four passes;
callers rasterize guides and build the ray scene first, then consume its HDR
output, radiance components and reflection distance. It performs no submission,
readback or temporal-history commit. Recreate the object when input bindings,
camera/light configuration or TLAS geometry bindings change. Mirror mode only
supports zero roughness; the existing GGX pipelines remain separate.

The existing primary-guide GPU fixture now uses this public owner instead of
manually connecting lighting/reflection/composition. Full Metal ray probe passed
on Apple M4 Max: all 16 raster-derived pixels matched analytical distances,
material-weighted reflection and HDR composition for occluded/unoccluded lights;
existing GGX, motion, background and TLAS mutation checks also passed. This proves
the ordered owner on the headless raster fixture, not yet SceneSurface G-buffer
presentation or NVIDIA execution.
The combined strict Clippy invocation for renderer and the full ray-probe binary
is not green: the probe reports 21 lint errors, including overlong fixture
functions. The GPU run above is successful independently of this lint gate.

### Complete CUDA source compiler coverage guard (2026-10-01)

The NVRTC/PTXAS verifier now recursively inventories all .cu and .ptx sources
under voxy_cuda/src and rejects uncovered or missing files before invoking the
compiler matrix. A new kernel cannot silently escape this explicit matrix.
A negative real-NVRTC probe also requests a nonexistent terrain entrypoint,
requires exit 1 and verifies no rejected PTX is published.

The current five CUDA sources plus affine.ptx passed NVRTC 12.6 / PTXAS for
sm_52, sm_75 and sm_89. Unsupported architecture 999 and the nonexistent entry
were rejected with their expected codes. Isolated inventory fixtures confirmed
rejection of a nested extra source and a missing gravity source. Shell syntax
and diff checks passed. Tooling wheel hashes were verified from the existing
pinned cache. These are compiler/assembler proofs without NVIDIA execution;
physical CUDA runtime and other requested platform acceptance remain open.

### Raster-derived ray lighting in a presentation frame

`SceneSurface::create_ray_guides` allocates reconstruction resources using the
window's retained adapter and device. The `surface_ray` example now rasterizes a
primary triangle into 64x64 depth/normal/diffuse/F0 guides, encodes
`RayLightingFrame`, tone maps its HDR texture and requires a real `Presented`
outcome on that same device. BLAS/TLAS build, guide rasterization, ray lighting
and display encoding share one submitted command encoder. The Metal/M4 Max run
passed and is followed by the existing numerical shadow/reflection diagnostic.
Strict Clippy passed for the renderer library and this window example. Pixel
readback of the raster-derived presented output is still pending; this fixture
is not the full application's scene renderer integration or NVIDIA proof.

The raster-derived window frame now copies HDR pixel (32,32) from its 64x64
lighting output in the same submitted encoder before tone mapping. After a
successful presentation, half-float readback is compared against independently
calculated pixel-center position, Lambertian inverse-square light and metallic
F0-weighted emissive mirror contribution. All RGB/alpha values must be finite
and within 0.01. The Metal/M4 Max run passed. This checks a primary interior HDR
pixel, not the complete image or post-tone-mapping swapchain pixels.

The window raster HDR verification now covers every pixel of the 64x64 output.
The same submitted encoder copies the full RGBA16Float image. CPU pixel-center
unprojection determines triangle coverage, inverse-square Lambertian lighting
and angle-dependent Schlick Fresnel mirror weight. All four channels must be
finite and match within 0.01; uncovered pixels require RGB zero/alpha one.
Metal/M4 Max passed all 4096 pixels: 2872 shaded, 1224 background, followed by
successful presentation and the independent ray diagnostic. This supersedes the
single-center HDR check. Post-tone-mapping swapchain pixels remain unverified.

### Scene mesh bridge to ray lighting guides

`ReconstructionGuideMesh::from_scene_mesh` accepts immutable indexed `SceneMesh`,
an affine model transform and uniform metallic/roughness. It expands indexed
triangles using the same welded smooth-normal calculation as `SceneRenderer`,
bakes world-space positions and inverse-transpose normals, and uses linear
vertex RGB as material base color. Invalid/singular/projective transforms,
degenerate normals, reflectance errors and oversized buffers are rejected.
Texture/UV material sampling and alpha masking remain outside this bridge.
The window raster fixture now uses this constructor with identity transform;
Metal/M4 Max still passed all 4096 analytical HDR pixels and presentation.
Nonuniform transformed GPU geometry remains a separate verification requirement.

### Explicit NVIDIA selection for shader acceptance (2026-10-01)

The shared native shader-probe selection now accepts an optional
`--require-nvidia` after the backend. It enumerates adapters of the configured
instance and selects a physical NVIDIA GPU rather than accepting the default
adapter and rejecting an otherwise usable multi-GPU host afterward. Absence of
such an adapter returns an error before device creation or shader execution.
Water, voxel-region and both gravity examples use this shared selection; ordinary
invocations retain their default-adapter behavior. The Linux/Windows CUDA hardware
acceptance water and collision commands now request this selection explicitly.

Argument/vendor/type tests passed; strict Clippy passed for all four affected
examples. An actual Metal invocation rejected Apple M4 Max with exit 1 and the
expected missing-NVIDIA message. An actual Vulkan invocation in Mesa rejected
llvmpipe CPU before water execution. Ordinary Mesa Vulkan/OpenGL water runs still
passed 16 ticks / 524288 exact CPU comparisons plus loading recovery. Shell and
PowerShell syntax checks passed. Physical NVIDIA selection and Windows execution
remain unverified here. Multiple NVIDIA cards are not matched to a CUDA ordinal
by this vendor selector; shared-memory UUID/LUID checks remain separate.

The indexed scene-to-guide window fixture now exercises a nonuniform affine
model: scale `(2, 0.5, 1.5)`, X rotation 0.7 radians and translation
`(0.3, -0.2, 0.8)`. Local vertex positions describe the previously verified world
triangle under that model, so all 4096 analytical pixel references stay fixed.
Metal/M4 Max passed full HDR coverage/light/Fresnel verification and presentation,
exercising world-position baking and inverse-transpose smooth normals. Singular
zero-Y scale and projective view-projection matrices were rejected by the bridge.
This supersedes the earlier identity-only transform verification limitation.

### UV transport for reconstruction materials

Reconstruction guide vertices now carry finite UV coordinates via `with_uv`;
legacy constructors default to zero UV. The indexed `SceneMesh` bridge preserves
source UVs. All guide/F0/object raster pipelines use the updated 52-byte vertex
layout, and the vertex shader interpolates UV to fragment stages. Nonfinite UVs
are rejected before upload. Metal window rendering retained all 4096 verified HDR
pixels; strict renderer-library Clippy passed. Texture sampling/bindings still
need implementation: transporting UV alone does not enable textured materials.

### Textured base color in reconstruction guides

`ReconstructionGuidePass::textured_inputs` now binds a single-sample opaque
RGBA8Unorm/RGBA8UnormSrgb base-color texture and nonfiltering sampler. Interpolated
UV sampling multiplies linear vertex color before diffuse, specular-albedo and
F0 guide generation. Texture alpha is ignored; alpha masking, filtering samplers
and per-draw texture binding are still pending. Legacy inputs bypass material
sampling and retain their earlier vertex-color behavior.

The transformed indexed window fixture uses a two-color 2x1 texture with varying
UVs. CPU references select the matching texel, calculate diffuse and metallic F0,
then point-light and reflected emission. Metal/M4 Max passed all 4096 HDR pixels
(2872 shaded/1224 background), actual presentation and independent ray checks.
Renderer library and surface-ray example strict Clippy passed. This verifies
linear UNORM texture sampling; sRGB decode still needs a dedicated GPU case.

Guide material bindings now accept filtering/noncomparison samplers for the
filterable RGBA8 base-color formats. The window fixture adds
`VOXY_RAY_TEXTURE_SRGB=1` and `VOXY_RAY_TEXTURE_LINEAR=1`. Independent CPU references
decode each sRGB texel to linear before bilinear blending and material/light/F0
calculation. Metal/M4 Max passed all 4096 HDR pixels and presentation both with
sRGB nearest sampling and sRGB linear sampling. This supersedes the earlier
nonfiltering-only and sRGB-unverified limitations. Mip minification/anisotropy
and per-draw material bindings are still separate verification/integration work.

### Independent guide materials per draw

`encode_draws` and `encode_material_f0_draws` accept mesh/input pairs, changing
material bindings within one primary/F0 attachment pass. All input dimensions
are validated before attachment clears; camera/world/device consistency remains
a caller contract. Existing single-input methods delegate to this path.
The window fixture splits the primary triangle into adjacent left/right meshes,
using colored texture on the left and a separate white texture on the right.
Metal/M4 Max passed all 4096 analytical HDR pixels and actual presentation,
including diffuse/F0/reflection differences between the two objects. This is
opaque material drawing; alpha-masked ray geometry remains unsupported.

`VOXY_RAY_TEXTURE_MIPS=1` adds an authored 256x256 nine-level diagnostic texture
to the per-draw window fixture. Base mip is black/white checkerboard; every lower
mip is RGB `(64,128,192)`, deliberately distinguishable from any filtered base
color. Sampler requests linear min/mag/mip filtering and anisotropy clamp 4.
Metal/M4 Max passed all 4096 HDR pixels and presentation against colored-mip
material/light/reflection references. This demonstrates implicit lower-mip
sampling through the guide shader and acceptance of the anisotropic sampler;
it does not measure oblique anisotropic quality or generated-mip color accuracy.

### Shared scene textures for ray material guides

`SceneTexture` now retains the uploaded texture and sampler alongside its scene
bind group, with read-only accessors for other GPU passes. `scene_material_inputs`
reuses these resources in reconstruction guide rendering, preserving sRGB,
mip-chain and sampler configuration without copying/reuploading image data.
The window fixture's second material now comes from the surface's SceneRenderer
upload API and passes through this bridge. Metal/M4 Max passed all 4096 HDR
pixels and presentation. This establishes shared material-resource integration;
it does not yet replace the application's full scene frame path with ray lighting.

### NVIDIA adapter enumeration for production voxel shader probe (2026-10-01)

The voxel diagonal/AO probe now enumerates adapters of its explicitly selected
backend when --require-nvidia is requested, choosing a physical NVIDIA GPU rather
than checking only the default adapter. This aligns the hardware acceptance
shader-pixel stage with the water/voxel-region selection and avoids rejecting
multi-GPU hosts merely because Intel was selected by default. Unknown or extra
arguments are rejected instead of silently ignored.

Strict example Clippy passed. Physical Apple M4 Max Metal still produced all 128
expected Uv/Vu AO pixels. The same binary with --require-nvidia rejected the
non-NVIDIA host before rendering, and an unknown option was rejected. A physical
NVIDIA positive case remains unavailable; this vendor selection still does not
match a specific CUDA ordinal on a host with several NVIDIA cards.

The window material bridge fixture now verifies three independent submitted
versions: original white material, corrected sRGB colored texture after
`SceneSurface::replace_scene_texture`, and old retained guide bindings after
replacement. Invalid three-byte texture replacements are rejected. For the
corrected version, CPU references explicitly decode the new sRGB color before
light/F0/reflection calculation; retained bindings must preserve the old white
result. Metal/M4 Max passed 4096 HDR pixels in each of the three frames, plus
presentation and independent ray diagnostics. This proves retained input resource
lifetime, not automatic invalidation/rebinding of cached guide inputs.

### Rendered browser GPU water boundary world (2026-10-01)

The waterBoundary=1 diagnostic now seeds a water source/floor/air channel at x=63
in the actual gameplay world, activates its source chunk and uses the existing
nonblocking water/terrain loader. It does not clone the scene. The multi-chunk
camera computes checked anchor-relative visible bounds and frames the whole set.
Counters expose streamed and published chunk counts.

Live BrowserWebGpu proof: one neighbor streamed, three chunks rendered, 13 water
commits settled. Destroy center removed 75 blocks while all three chunks remained
visible; the character fell to the new floor and continued to 2836 ticks, with
5677 rendered frames. Screenshot: docs/browser-water-boundary-world-proof.png.
The visible lake is seed-generated terrain water in the neighboring chunks;
this is not a claim that the single poured voxel created that lake.

WASM build, strict target Clippy, JavaScript syntax and diff checks passed.
This closes the previously missing live streamed-mesh rendering proof. Streaming
still has a 64-chunk retained limit without eviction; dormant water wake-up scans
only center; physical NVIDIA/mobile/headset acceptance remains incomplete.

Material-guide input creation now retains the guide pass and SceneTexture device
and rejects device mismatches before native bind-group creation. The comparison
is scoped to one wgpu Instance: an attempted independent-Instance fixture exposed
colliding wgpu resource identities and aborted inside wgpu-core TextureViewId
lookup. Cross-Instance resource identity protection remains unresolved. The
fixture now requests its second device from the surface's retained adapter via
`SceneSurface::adapter`, keeping one Instance. Runtime verification of this final
case was blocked by concurrent voxy_editor compilation errors (missing
SceneSimulation methods); earlier cross-Instance runs are failures, not passes.

### Browser multi-chunk camera range and aspect fitting (2026-10-01)

The multi-chunk browser camera now fits the bounds' sphere to the smaller of the
horizontal and vertical perspective half-angles, with a 5 percent distance
margin. Its far plane follows camera distance plus scene radius instead of
remaining at 100 while streaming expands the scene. Non-finite range/aspect is
rejected. Ordinary one-chunk framing remains unchanged.

The browser streaming validator includes a synthetic distant-chunk camera case
(x=100), requires far >100 and projects all eight bounds corners at aspect 1/3
and 3, checking x/y framing and depth range. This passed in the live combined
waterBoundary=1&streamingCheck=1 run, alongside 65536 CPU block comparisons and
actual three-chunk rendering / one GPU-loaded neighbor / 13 water commits.
Screenshot: docs/browser-streaming-camera-proof.png. The camera range case is a
projection calculation fixture, not physical rendering of chunk x=100.
Strict WASM Clippy, dev build and diff checks passed. Full hardware/platform
acceptance and dormant halo-water activation remain incomplete.

The cross-Instance device-identity issue is now fixed by a local wgpu 30.0.1
patch (`vendor/wgpu/VOXY_PATCH.md`): CoreDevice equality/order/hash include its
owning Global address as well as DeviceId. Context lifetime is retained by Arc;
clones preserve identity. Other wgpu resource comparison types are unchanged.
The window fixture now actually requests its foreign device from a second Metal
Instance. Both foreign-material checks returned errors before bind-group creation;
the process completed all three 4096-pixel HDR/presentation checks without the
previous TextureViewId abort. This supersedes the unresolved device-identity and
editor-build-blocked notes above. Windows/wasm builds of this new patch still
need verification; no broad cross-Instance resource-safety claim is made.

The local native Device-identity patch passed Windows GNU cross-check of
`voxy_streamline --features scene-dx12 --examples` with SDK 2.14.1 and wasm32
renderer check with `--features webgl`. Strict Clippy passed for the renderer
library and window ray example. Metal runtime additionally verifies clone
identity and HashSet behavior (original plus clone plus independent-Instance
device form exactly two keys), foreign material rejection, all three 4096-pixel
HDR versions and presentation. These are cross-compilation and Metal results;
Windows/NVIDIA execution and browser runtime with this patch remain unverified.

RayScene ownership is now validated before visibility jobs, specular-distance
jobs, primary surface lighting/reflection and RayLightingFrame allocations/binds.
A foreign scene returns explicit `RaySceneError::DeviceMismatch`. The window
fixture requests a second independent ray-enabled Metal device, builds an indexed
scene there and verifies the first device's visibility pipeline rejects it
before segment validation/native binding. The run also passed all three 4096-
pixel HDR versions and presentation. Raw externally supplied guide textures and
primary buffers still require the documented same-device caller contract.

### Browser GPU deadline pause accounting (2026-10-01)

The animation loop applies gameplay pause/resume to WebEngine before polling
simulation. Explicit pause time is excluded from existing character, vehicle and
water task deadlines by shifting their original start timestamps on resume;
pending tasks and retained intent are preserved. Repeated pause calls do not
restart the pause interval, and resume without a pause adds no time. Rendering
continues while gameplay steps remain paused. This does not change active
30-second GPU timeout behavior or automatically suspend hidden tabs.

Live BrowserWebGpu proof paused with exactly one pending GPU deadline. A real
35-second wait exceeded the normal 30-second timeout; character ticks stayed at
325 and simulation time at 6.8404 while paused. After resume the same game reached
1712 character ticks / 9440 rendered frames without a timeout, retaining three
visible chunks and the completed 13 water commits. The observed pending deadline
was the character path; vehicle/water pause timing still needs direct live cases.
Screenshot: docs/browser-gpu-pause-resume-proof.png. Strict WASM Clippy, dev build,
JavaScript syntax and diff checks passed.

PrimarySurfaceJob now retains its device. PrimaryMotionPass and primary surface
lighting/reflection constructors validate this owner before native GPU bindings,
returning DeviceMismatch for a foreign primary buffer. The two-Instance Metal
fixture builds a primary reconstruction job on the second device and verifies
motion consumption on the first returns DeviceMismatch. All three 4096-pixel
HDR/material versions and presentation still pass. External texture ownership
remains a caller precondition; this guard covers the owned primary buffer.

### Owned raster-to-ray frame API

`RasterRayFrame` owns G-buffer guides, conventional depth, guide pass, distance
placeholder and ordered RayLightingFrame. `RasterRayOptions` supplies camera,
light, reflection configuration and dimensions. Its material factory reuses
SceneTexture; `encode` rasterizes per-draw primary/F0 guides then reconstructs,
lights, reflects and composes HDR in the caller's encoder. The caller builds the
matching ray scene first and controls submission, tone mapping and temporal state.
Only conventional clear depth 1 and opaque mirror reflection are supported here;
recreate after resize/camera/light/TLAS-binding changes.

The window fixture now consumes this public owner's encode/output instead of
manually ordering the guide and lighting passes. Metal/M4 Max passed all three
4096-pixel material/HDR versions, independent-device rejection and presentation.
This is a reusable scene frame API, not yet the full application's selected
rendering mode or a GGX/temporal implementation.

### Browser WebGPU vehicle: pending-task pause proof

The diagnostic URL `?backend=webgpu&voxel=1&vehiclePauseCheck=1`
pauses once immediately after a vehicle step returns pending. It uses the same
explicit pause accounting as the normal Space control and retains the submitted
GPU task. Ordinary URLs do not enable this diagnostic.

On the Apple M4 Max browser WebGPU path, one pending task was held for at least
35 seconds. Simulation time remained 6.6668, character ticks remained 397 and
position remained `(0, 0.05625, 0)` while rendering continued. After selecting
Forward and resuming with Space, vehicle ticks reached 181 and position became
`(0, 0.0375, 0.68248475)` with simulation time 11.1831; no timeout occurred.
Throttle was then returned to Coast and Brake enabled. The rendered result is
recorded in `browser-gpu-vehicle-pause-proof.png`; numeric evidence came from the
canvas diagnostic dataset, not the screenshot.

JavaScript syntax and scoped diff whitespace checks passed. This directly proves
vehicle pending-task resumption; direct water-task pause and automatic background
visibility lifecycle checks remain outstanding. It does not establish NVIDIA
CUDA, Windows, mobile or headset hardware acceptance.

`RasterRayFrame::camera_motion` now prepares backward normalized RG16Float UV
motion using this owner's reconstructed primary buffer and current camera.
Encode it after RasterRayFrame; it supports static world geometry, leaving
moving-object/correspondence motion and presentation history to separate APIs.
The Metal window fixture encodes motion and copies all 4096 pixels in the same
submitted frame: previous camera shifted by -0.1 world X yields the analytical
UV displacement on the primary plane, background stays zero, and reset history
in version 2 zeros every pixel. All three 4096-pixel HDR and motion cases passed
alongside presentation and independent ray/device checks. This does not prove
temporal accumulation or DLSS evaluation.

### Browser WebGPU water: pending-task pause proof

The opt-in `?backend=webgpu&voxel=1&waterBoundary=1&waterPauseCheck=1`
diagnostic pauses once when the actual water poll returns pending. The actor
step now also checks the current pause flag, preventing a same-frame actor
submission after the water diagnostic pauses. Ordinary URLs retain normal
behavior.

Live Apple M4 Max BrowserWebGpu held one pending GPU deadline for a measured
35 seconds: simulation time stayed zero, streamed chunks stayed zero and visible
chunks stayed two, while frames increased from 1467 to 6818. After Space resume,
the same retained water job completed, GPU terrain loaded one neighboring chunk,
visible chunks reached three and water settled after 13 commits. Character ticks
also reached 315; no water error or timeout occurred. Numeric evidence was read
from the canvas dataset. Screenshot: `browser-gpu-water-pause-proof.png`.

The lake is seed-generated terrain water in the streamed neighbor. This proof
covers explicit water pause/resume across the active-job timeout threshold;
automatic hidden-tab lifecycle handling and physical NVIDIA/Windows/mobile/VR
acceptance remain outstanding. JavaScript syntax and scoped whitespace checks
passed; this diagnostic change did not modify the compiled WASM.

`RasterRayFrame::encode_object_ids` writes per-mesh IDs at its opaque depth;
`object_motion` creates RG16Float motion from matching [current, previous] affine
model pairs and previous camera. Encode the ID pass after primary rendering and
before motion. This covers affine object motion, not deforming-mesh correspondence.
The Metal window fixture now assigns separate IDs to left/right materials and
opposite previous X translations (-0.1/+0.1). All 4096 motion pixels match the
analytical opposite UV displacements, with zero background and complete reset
in version 2; three full HDR/presentation checks also passed. Camera-only motion
verification from the earlier owner run remains applicable independently.

### Browser visibility lifecycle pauses GPU deadlines

After asynchronous WASM startup finishes, the page installs a visibilitychange
handler. It applies the effective pause (`manual pause || document.hidden`)
immediately rather than waiting for RAF, which may already be suspended. Hidden
frames do not submit gameplay or render work. Returning to the foreground resets
the frame clock and excludes the hidden interval from retained GPU deadlines.
Visibility changes clear keyboard and jump input; manual pause survives a hide
and return. The listener is aborted before fatal frame errors free WebEngine.

`node web/tests/pause-lifecycle.mjs` exercises the actual handler extracted from
the page with mocked DOM/engine boundaries: no RAF required, repeated events do
not duplicate engine transitions, hidden input resets, manual pause is preserved
and unpaused foreground return resumes. JavaScript syntax and scoped whitespace
checks passed. This is a handler test, not physical browser suspension evidence;
actual hidden-tab/device lifecycle acceptance remains unverified.

### Strict CUDA probe device arguments

The buffer, gravity, projectile, voxel-region and exact box-sweep CUDA probes
now share a strict parser: default ordinal zero, one decimal usize ordinal, and
no ignored trailing arguments. Invalid, negative, signed, whitespace-bearing or
overflowing input rejects before constructing CudaCompute. This prevents an
incorrect diagnostic invocation from silently targeting a different selection.

The standalone parser test passed; all five probes passed CUDA-feature offline
Cargo checking. This validates argument handling and compilation, not NVIDIA
execution. CUDA/Vulkan interop already matches the physical device by UUID;
ordinary NVIDIA graphics smoke selection remains vendor-based rather than a
match to a particular CUDA ordinal on multi-GPU hosts.

CUDA device-argument follow-up: strict Clippy passed all five probes with the
CUDA feature. Cargo example tests passed the parser test in each of the five
actual example targets. Linux and Windows hardware acceptance scripts now run
this selection gate before physical CUDA probes; shell and PowerShell parser
checks passed. The platform acceptance scripts were not run on NVIDIA hardware.

CUDA selection gate completeness: Linux/Windows acceptance now requires exactly
five passing parser-test lines, rejecting a successful Cargo invocation that
filters out missing tests. Five existing actual example test binaries passed;
both shell and PowerShell guards accepted their output, and synthetic counts
0/1/4/6 rejected. Script syntax and whitespace checks passed. A fresh locked
Cargo rerun was blocked by current manifest/lockfile drift in the shared tree;
this turn did not change Cargo.lock. No physical NVIDIA execution is claimed.

### Deformed mesh motion in the raster/ray frame

`RasterRayFrame::deformation_motion` creates an RG16Float backward-UV raster
motion pass using the frame-owned primary depth and camera. Callers supply
matching current/previous world-space triangle-list vertices and an unjittered
previous camera, then encode after primary coverage. It supports pose changes,
zero background and explicit history reset; it does not animate meshes or
advance presentation history.

`VOXY_RAY_DEFORMATION=1` selects a continuous previous-pose shear in the
`surface_ray --experimental` fixture. Current vertices must use exactly the
same topology and model-transform roundtrip as the primary meshes: idealized
world vertices can fail the Equal-depth test even with the same apparent plane.
The fixture checks full-frame motion against the analytic shear while retaining
its HDR/material/presentation checks.

On M4 Max Metal, the deformation fixture passed all 4096 motion pixels in each
of three presented frames, including zero background and reset, together with
2872 shaded / 1224 background HDR pixels per frame and the ray-lighting probe.
This proves raster correspondence for the supplied shear, not automatic
skinning, temporal accumulation or NVIDIA Frame Generation execution.

The default rigid-object fixture also passed after this change. The final
combined renderer/example Clippy run was blocked by a concurrent `voxy_app`
change importing `voxy_rush` without a linked dependency; the GPU executable
above was built and verified before that unrelated change appeared.

Fresh CUDA check after shared lockfile resolution: all five CUDA-feature probes
passed offline cargo check against current manifests. Compared with the saved
pre-check lockfile, only voxy_rush dependency references to physics and serde_json
were added; no dependency version changed. Concurrent Cargo processes were
active, so this note does not attribute that lockfile edit to this check.
The subsequent locked offline five-example parser test invocation accepted the
lockfile and passed the first box-sweep example; remaining targets were still
running at this observation, not yet a completed fresh test gate.

### Fresh locked CUDA library verification

The previously pending locked/offline five-example parser invocation completed
with all five tests passing. Current CUDA-feature library tests also completed:
12 passed, zero failed, one explicitly ignored host-arithmetic test. Coverage
includes launch/buffer limits, ordinal truncation rejection, terrain shape,
external-memory ranges and bit-preserving decode, corrupt voxel/contact output,
projectile budgets, compiler/device architecture limits and gravity errors.
The ignored `cuda_gravity_host_parity` was then explicitly run with native clang
and passed separately. This validates CUDA source arithmetic through its host
fixture; it is not NVIDIA kernel execution or driver/interop hardware acceptance.

### Current Metal and Mesa water shader verification

A fresh locked offline `water_smoke metal` build ran on Apple M4 Max Metal and
passed all pending-task, provenance, stale-revision, unavailable-chunk and
load/retry checks plus 16 ticks / 524288 exact CPU cell comparisons with volume
conservation. Linux `tools/linux/water-smoke.sh` then rebuilt current sources and
passed the same contracts on both Vulkan and OpenGL Mesa llvmpipe CPU adapters.
These Linux runs verify backend shader execution, not physical Linux GPU speed.

Unavailable captured chunks must remain lazy and cannot enter successful world
transactions; this existing fixture is now mandatory in Linux shader smoke and
Linux/Windows NVIDIA water acceptance logs. Shell/PowerShell syntax and scoped
whitespace checks passed. Physical NVIDIA/DX12 remains unverified.

### Mobile prerequisite recheck

Current locked Android voxy_assets cross-check fails in BLAKE3's AArch64 NEON
C build because aarch64-linux-android-clang is unavailable. Android packager
--check also rejects missing ANDROID_HOME/ANDROID_NDK_HOME. iPhoneOS xcrun SDK
lookup fails. Hardware SIMD remains enabled; no portable-hash substitution was
made to disguise missing platform tooling. Nine Android packaging tests passed.

Added an iOS mock-tool regression asserting an unavailable SDK stops before
Cargo and publishes no library. Python syntax and whitespace checks passed;
full iOS mock suites were still running at this observation. These are packaging
contracts, not actual iOS/Android linking or device runtime evidence.

Both pending iOS mock runs then completed: original four tests passed and
updated five tests passed, including missing-SDK early rejection.

### Current Windows CUDA/DX12 compile verification

Locked offline cross-checks passed for x86_64-pc-windows-gnu: CUDA library, CUDA
library test targets (including Windows D3D12 resource descriptors), and all
three voxy_vulkan CUDA examples d3d12_export/cuda_gravity_render/cuda_gravity_window.
The D3D12 export selector matches the CUDA LUID and rejects zero LUID or linked
node masks. This is code/compilation evidence, not Windows device execution.

Windows hardware acceptance now explicitly executes the committed-resource
descriptor unit test and requires its passing marker before importing real CUDA
memory. The test checks D3D12_RESOURCE handle kind, dedicated flag, resource size
and zeroed reserved fields. PowerShell syntax and whitespace checks passed;
actual Windows descriptor-test execution and physical NVIDIA/DX12 remain open.

### Current XR contract and stereo-shader verification

All 16 current voxy_xr library tests passed with locked offline Cargo: tracking
validity and malformed eye rejection, origin negotiation, predicted-time history
reset, successful-submission history publication, swapchain ownership retry,
format/viewport/layer negotiation and haptic bounds. No headset runtime was used.

Current xr_motion shader fixture passed on both Vulkan and OpenGL Mesa llvmpipe
CPU adapters: asymmetric eyes, opposite motion, skipped frame, tracking recovery,
clipping changes and teleport. The Linux runner now retains separate backend
logs and requires the final proof marker instead of relying only on exit zero;
the updated runner passed. Logs: target/linux-docker/xr-motion-vulkan.log and
xr-motion-gl.log. Shell syntax and whitespace checks passed. Physical Meta Quest
and PC headset/session/swapchain runtime acceptance remains unverified.

### Compute dispatch device-limit regression

Current compute_smoke now checks max_compute_workgroups_per_dimension + 1
on each of X/Y/Z, requiring InvalidDispatch before encoding GPU work. The existing
validation error scope also ensures rejected requests do not poison subsequent
valid submissions. Fresh locked offline Metal run on Apple M4 Max passed these
limits plus 1042 exact results, partial workgroups, independent/resident jobs,
mapping snapshots/cancellation and shader reload retaining old jobs. Formatting
and whitespace checks passed. This covers the shared compute dispatch contract;
physical Windows/NVIDIA/mobile execution remains unverified.

### Browser terminal GPU device-loss handling

WebEngine now retains the wgpu device-lost callback diagnostic and exports it
to the page. Each animation frame checks this before gameplay submission or
rendering; terminal loss reports the driver reason/message, disables gameplay
controls, aborts visibility handling and frees the engine. The render method
independently rejects a known lost device. This reports terminal loss, not
automatic recreation of GPU resources or restoration of gameplay state.

Strict WASM Clippy, JavaScript syntax, lifecycle tests and whitespace checks
passed. Browser WASM packaging was still running at this observation; actual
WebGPU device-loss injection and WebGL context loss remain unverified.

Browser device-loss diagnostic follow-up: explicit `?deviceLossCheck=1` invokes
real device.destroy after 30 rendered frames. It requires page reload for new
resources; ordinary URLs never trigger it. Source-level page handler test passed
for diagnostic publication, controls disabled, listener abort, single engine
free and no subsequent submission. Pause lifecycle test and JavaScript syntax
passed. First WASM package completed before the destroy diagnostic addition;
the fresh package containing that export was still building. Real browser
destroy/callback proof remains pending.

### Live WebGPU device destruction and page recovery

The fresh WASM package completed and exported destroy_device_for_validation.
Actual BrowserWebGpu deviceLossCheck invoked device.destroy after 30 frames.
The driver callback reported `GPU device lost (Destroyed): Device was destroyed.`
Canvas frames stayed 30 and character ticks 14; controls were disabled and the
Reload GPU link appeared. Screenshot: browser-device-loss-proof.png.

The recovery link preserves current query settings and removes deviceLossCheck
to avoid repeating intentional destruction. Clicking it recreated the WebGPU
engine/world; ordinary rendering reached 973 frames and 485 character ticks,
with controls enabled and no device error. This is full page recreation: edits
and gameplay progress reset. The callback and recovery-link source tests pass.
It proves explicit WebGPU destruction, not spontaneous driver reset, WebGL
context loss, hidden-tab device failure or state-preserving automatic recovery.

### Live WebGL2 context-loss handling

The page now listens to real webglcontextlost after startup and routes it through
a single terminal stop owner shared with WebGPU loss. Repeated terminal events
or already queued RAF callbacks cannot free or use the engine twice. The explicit
deviceLossCheck diagnostic uses WEBGL_lose_context on WebGL2; ordinary URLs do
not lose their context. Handler/pause tests, JS syntax and whitespace checks pass.

Actual browser Gl loss held frames at 30 and character ticks at 19, disabled
controls and displayed the recovery link. Screenshot: browser-webgl-loss-proof.png.
Reload GPU removed only the diagnostic flag and recreated the Gl renderer: 754
frames / 382 character ticks, no deviceError, controls enabled. WebGL terrain and
physics remain CPU with GPU rendering; no compute fallback is claimed. Recovery
restarts the page/world and does not preserve gameplay edits.

### Browser terminal frame errors share the GPU cleanup owner

Character/vehicle step failures and render failures now route through stopEngine
instead of freeing WebEngine directly. Both disable gameplay controls, abort
visibility/context listeners and expose page recovery; repeated queued events
cannot free again and retain the first terminal diagnostic. Source-handler tests
exercise a second failure followed by another frame and confirm one free, one
abort, no further submission and first-message preservation. JavaScript syntax,
pause tests and whitespace checks passed. This is handler evidence; direct
physics/render failure injection and startup-error cleanup are separate open
cases. No compiled WASM change was required.

### Browser startup failure owns cleanup

A startup owner is retained after WebEngine creation until RAF registration.
Outer startup failures disable gameplay controls, publish startupError and free
the retained engine once. Explicit validation catches no longer free separately;
scene/water startup failures now also clean up instead of leaking a created
engine. Failed creation has no owner to free. Handler tests cover before/after
creation and repeated failure cleanup; terminal-device and pause tests pass.

Actual Gl voxel startup with waterCheck rejected unsupported compute, disabled
vehicle/destroy controls and produced no frame dataset. Screenshot:
browser-startup-failure-proof.png. This retains explicit unsupported behavior,
not a compute fallback. JS syntax and whitespace checks passed.

### Vulkan/OpenGL compute limit and reload gate

Fresh Linux compute smoke passed on explicit Vulkan and Gl Mesa llvmpipe CPU
adapters: zero dispatch and over-limit X/Y/Z reject before encoding, subsequent
valid submissions leave the validation scope clean, 1042 results match exactly,
resident jobs continue through independent snapshots, and shader reload retains
old jobs/rejects invalid replacement. This is backend execution on software
adapters, not physical Linux/NVIDIA throughput or Windows/mobile acceptance.

compute_smoke now emits a dedicated dispatch-limit marker. Linux runner retains
compute-vulkan.log / compute-gl.log under target/linux-docker, verifies actual
backend and requires all limit/residency/reload/result markers plus exit zero.
The updated runner, shell syntax, formatting and whitespace checks passed.

### CUDA water kernel foundation

Added water.cu with the ordered integer graph ABI of the WGSL water solver,
including lazy reads, sampled-node provenance, downward/horizontal budgets,
write budget and missing/unavailable/unknown/overflow status codes. A single CUDA
thread preserves canonical ordering; publication remains the host transaction
owner's responsibility. Runtime graph validation/decoder/API integration and
NVIDIA execution are not implemented by this kernel-only addition.

The complete CUDA compilation inventory now includes water. Fresh pinned NVRTC
12.6 and PTXAS passed all sources for sm_52/sm_75/sm_89 including water_transfer;
unsupported-architecture and missing-entry rejection also passed. The standalone
water_host.cpp fixture compiles the actual kernel as host arithmetic to exercise
ordered transfers, lazy reads/provenance, error budgets and the one-thread guard;
this does not constitute GPU parity or world transaction acceptance.

### CUDA packed water graph runtime API

CudaCompute::water_graph now validates the packed graph before driver work,
caches the NVRTC water_transfer function and launches exactly one CUDA thread.
Validation bounds node/active counts, exact allocation shape, neighbor indices
and overflow/missing sentinels, initial/provenance fields, canonical active order,
transfer/sample/write budgets and combined input/output byte budget. The method
returns raw solver status/provenance; no world publication occurs. Shared world
decoding/revision checks, gameplay integration and physical NVIDIA parity remain
unfinished. Strict CUDA-feature Clippy passed. Graph validation tests compiled
but were still running at this observation; the default-feature compile check
was waiting for the shared build directory.

### Shared CUDA/WGSL water graph adapter

CudaWaterTransferProgram now uses the same extracted graph packing/validation
and result decoding as WaterTransferProgram. It executes CudaCompute::water_graph
synchronously, maps driver errors, and rejects failed partial solver output
instead of returning a successful transfer result. It is a graph API, not yet
a CUDA world-plan/gameplay integration.

The previously pending CUDA graph validation test and default-feature build
passed. CUDA-feature voxy_gpu compilation passed. After extraction, fresh Metal
water smoke passed all error/provenance/stale/drop/load-recovery checks plus
524288 exact CPU cell comparisons and conserved volume. Strict GPU Clippy found
one documentation-format issue, corrected; recheck was pending at this note.
Physical NVIDIA graph execution and transaction/gameplay acceptance remain open.

### CUDA water world-plan and physical acceptance entrypoint

CudaWaterTransferProgram::plan_world now captures the existing availability-aware
world graph, executes CUDA, translates missing/unknown/budget errors and uses
WaterWorldSnapshot::plan for revision-checked writes and next-active publication.
It performs no world mutation; the caller must commit the plan. Strict CUDA GPU
library Clippy passed.

Added cuda_water_world physical probe: constructs actual CudaCompute, compares
16 complete CUDA water plans with independent step_water CPU plans, including
revisions/writes/next-active, and commits the matching transactions. Linux and
Windows NVIDIA acceptance now build/run it and require its proof marker. Shell
and PowerShell syntax checks passed. Example compilation/Clippy were still
waiting at this observation. Physical NVIDIA parity, dedicated failure/stale
fixtures and --cuda-water interactive application routing remain unfinished.

### Native --cuda-water gameplay route

The application now accepts --cuda-water, rejects its combination with
--gpu-water and shares an Arc-owned selected CUDA compute context with other
CUDA motion/collision operations. Ordered synchronous CUDA water plans use the
existing commit, activation/deferred batching and geometry refresh path at the
CPU-path tick cadence. Successful CUDA plan count is separately reported.

Current CUDA water world probe passed compilation and strict Clippy; native
CUDA-feature application check passed with existing unrelated library warnings.
Seven option tests passed including independent CUDA water ordinal selection.
Linux/Windows hardware acceptance now includes full CUDA-water/motion/collision
gameplay and requires positive water ticks and commits before final success.
Physical NVIDIA execution, stale/failure-specific CUDA fixtures and performance
remain unverified; synchronous CUDA readback is not a resident world solver.

### CUDA water stale-plan acceptance fixture

The physical cuda_water_world probe now seeds a controlled downward transfer,
calculates a CUDA plan, changes the sampled chunk revision, and requires stale
commit rejection. It compares every targeted cell with its pre-rejection sample
to reject partial publication. A fresh CUDA plan must equal independent CPU
step_water, then publish source AIR and a full-water destination. Linux and
Windows acceptance both require the dedicated stale/recovery marker.

Current CUDA-feature example strict Clippy passed; shell and current PowerShell
syntax and whitespace checks passed. This fixture has not executed on NVIDIA;
compilation and required markers do not prove physical stale/recovery parity.
The previously pending PowerShell check also completed successfully.

### CUDA water solver-failure acceptance fixtures

The physical cuda_water_world probe now requires sample/write budget failures,
lazy unavailable/unknown node errors with exact node provenance, missing edges
and coordinate overflow rejection. Dormant hidden nodes must remain unsampled;
a subsequent valid request must return the complete downward transfer. Both
hardware acceptance scripts require the dedicated fault/recovery marker.

Current CUDA-feature example strict Clippy and shell syntax/whitespace checks
passed. These fixtures have not executed on NVIDIA; they add mandatory physical
acceptance checks without claiming driver execution or performance evidence.

### Current shared water ABI: Metal execution

After the CUDA graph validation additions, the current water_smoke completed on
Apple M4 Max using Metal. All eight markers passed, including lazy fault/sample
precedence, pending task isolation/drop recovery, retained world revisions and
stale rejection, unavailable chunk exclusion, missing chunk load/retry, and 16
ticks with 524288 exact CPU cell comparisons and conserved volume. Linux Vulkan
and Windows DX12 hardware acceptance now also require the pending world-plan
revision/stale/fresh marker. Their script syntax checks passed; this run proves
Metal execution only, not NVIDIA, DX12, or physical Vulkan execution.

### Physical acceptance requires general compute lifecycle

The Linux Vulkan and Windows DX12 NVIDIA runners now require compute_smoke's
resident-job, dispatch-limit, shader-reload and exact 1042-result markers. The
probe accepts --require-nvidia and selects only a physical NVIDIA adapter within
the selected backend. Windows restores its previous compute-backend environment
after the gate. Strict example Clippy, both script parsers and whitespace checks
passed. On this Mac the NVIDIA requirement failed explicitly before dispatch;
the normal Metal run then passed all four markers on Apple M4 Max. Physical
NVIDIA execution remains unverified.

### Windows compile coverage for current water and compute probes

Locked offline checks for x86_64-pc-windows-gnu passed for voxy_gpu's
CUDA-feature cuda_water_world example and voxy_render's compute_smoke example.
This covers the current CUDA water graph/world adapter/fault fixtures and the
physical NVIDIA compute selection path at Rust compilation level. It does not
link or execute Windows binaries and does not prove DX12 or NVIDIA driver use.

### Exhaustive CUDA water downward capacity cases

The host arithmetic harness executes the actual water.cu source for all 648
combinations of source/destination levels 0..8 and downward limits 1..8. Exact
transfer amounts, conserved volume, unchanged solid neighbors and lazy sampled
provenance passed under clang++ with warnings as errors. The physical
cuda_water_world probe now requires the same cases through real CUDA calls;
both NVIDIA acceptance scripts require its dedicated marker. Strict CUDA-feature
example Clippy, shell/PowerShell syntax and whitespace checks passed. The first
locked check saw a transient concurrent manifest/lock mismatch; a subsequent
locked offline check passed without this task updating the lock. These new
physical cases remain unexecuted on NVIDIA.

### Ordered CUDA water cascades

The actual-source host harness passed three column cases: one active source
stops at its immediate destination; activating that destination cascades the
received volume again in the same tick; a downward limit of four leaves four
units at the source and moves four to the final destination. Lazy read flags
are checked, with a write budget of two accepting the cascade because the
intermediate node returns to its initial amount. The physical CUDA probe now
contains the same fixtures, and both hardware runners require their marker.
Strict example Clippy, host C++ warnings-as-errors, script syntax and whitespace
checks passed. The physical fixtures have not executed on NVIDIA.

### Water backend failure diagnostics

CUDA and graphics-compute water errors now format the underlying backend error
instead of flattening it into Debug text. World snapshot calculation errors use
that display text, preserving the CUDA label in application failure messages.
Both water error wrappers expose their underlying errors through Error::source,
including CPU water validation errors in the snapshot wrapper. Strict CUDA-
feature library Clippy and scoped whitespace checks passed. This changes error
reporting only and adds no physical driver execution evidence.

### Shared CUDA uploaded-buffer budget

CudaU32Buffer allocations now reserve from one atomic budget owned by their
CudaCompute. Live uploaded buffers cannot individually consume the full limit
multiple times; failures release reservations, and dropping a buffer frees its
reservation after its device allocation. Reservations retain the budget even
if the compute owner is dropped. Two driver-free tests passed for exact-limit
rejection/reuse and concurrent full-budget contention; strict CUDA library/test
Clippy passed. This first integration covers uploaded reusable u32 buffers.
Transient kernels, gravity jobs and imported resources still need integration
before claiming a global CUDA memory bound. Physical NVIDIA allocation/release
behavior remains unverified.

### Shared CUDA transient-buffer budget

Roundtrip, terrain, water, voxel-region, projectile and box-sweep operations now
reserve their actual simultaneous device buffer sizes from the same budget as
uploaded u32 buffers. Two-buffer operations reserve input/output together;
scope guards release after the buffers, including on driver errors. The physical
cuda_probe adds resident/transient contention, resident-data preservation and
fresh projectile/full-buffer recovery, required by both hardware runners.
Current reservation tests passed, strict CUDA library/test and example Clippy
passed, and both script parsers/whitespace checks passed. Physical NVIDIA
execution remains unverified. Gravity and imported-resource allocations are not
yet included; host staging and driver/module overhead are outside this budget.

### Shared CUDA gravity budget

Gravity jobs now retain reservations for their complete packed device storage;
render-view validation also reserves its temporary four-byte device status.
The physical gravity probe fills a 256-byte budget with one body, requires
competing uploaded/gravity allocations to fail without changing resident state,
then drops the job and requires a full-budget upload/readback to succeed. Both
hardware runners require this marker. Two current reservation tests, strict
CUDA library/test and gravity-example Clippy, both script parsers and whitespace
checks passed. The physical fixture remains unexecuted on NVIDIA. External
imports, host staging and driver/module overhead still lack this accounting.

### Shared CUDA external-import budget

Opaque Vulkan-compatible memory and dedicated DX12 resource imports now reserve
their complete declared allocation size before CUDA import, not just the mapped
subrange. Their reservation drops after the mapping. Current CUDA library/test
strict Clippy and Windows GNU CUDA library cross-check passed. Three driver-free
tests of the actual budget module passed in a standalone rustc test wrapper,
including retained-reservation accounting after owner drop. Source inventory
shows reservations at every current private buffer allocation and external
import in voxy_cuda; host staging and driver/module overhead remain outside the
limit. Actual Vulkan/DX12 import contention and release remain unverified on
physical NVIDIA hardware.

### CUDA reservation telemetry

CudaCompute::reserved_device_bytes exposes the current atomic buffer/import
reservation count, excluding host staging and module/driver overhead. The
physical buffer probe now checks 64 live bytes, zero after transient completion,
128 at full capacity and zero after final drop. Three actual-source standalone
budget tests passed after this change; strict CUDA library/test/example Clippy
and scoped whitespace checks passed. The physical counter transitions remain
unverified on NVIDIA.

### Current CUDA-budget portability checks

After shared-budget/telemetry integration, locked offline Windows GNU checks
passed for the CUDA library, cuda_probe and gravity_probe. Strict default-feature
library/test Clippy also passed. The full default-feature library suite passed
18 tests with no failures; one explicit CUDA-source gravity host test remained
ignored. This confirms validation and reservation behavior without driver use,
not physical CUDA, Windows linking or DX12 execution.

### Physical interop reservation probes

Vulkan opaque-memory and DX12 dedicated-resource probes now require the
reservation counter to equal the full allocation size while imported and zero
after import/job release. Both NVIDIA acceptance scripts require their markers;
script syntax and scoped whitespace checks passed. The Linux GNU CUDA Vulkan
example cross-check passed. The first Windows DX12 check found a u64/usize
comparison; checked conversion was added, and the locked offline Windows GNU
CUDA d3d12_export cross-check passed. Physical import counter transitions remain
unverified on NVIDIA.

### Gravity reservation counter fixture

The physical gravity shared-budget fixture now checks exact counter transitions:
256 bytes while its job lives, zero after job drop, 256 for the replacement
uploaded buffer and zero after its drop. Current strict CUDA gravity-example
Clippy passed. These physical transitions remain unexecuted on NVIDIA.

### CUDA budget rejection precedes kernel loading

Shared reservations now precede kernel-cache lookup, NVRTC compilation and module
loading for uploaded buffers, terrain, water, voxel regions, projectile, box
sweeps and gravity. An exhausted budget therefore returns BufferLimit before
compiler/driver work; compilation/load errors release the reservation by scope
cleanup. Strict CUDA library/test Clippy and scoped whitespace checks passed.
Physical failure precedence remains unverified on NVIDIA.

### CUDA world-probe budget integration

Current CUDA water/voxel world probes passed strict example Clippy and locked
offline Windows GNU cross-checks after admission moved before kernel loading.
The water probe now retains the compute owner and requires zero reserved device
bytes after its complete success/failure/cascade/stale/fresh fixture sequence.
Both physical runners require that marker; script syntax and whitespace checks
passed. The updated water example passed a fresh strict Clippy check. Physical
world-plan reservation release remains unverified on NVIDIA.

### DX12 failed-cleanup reservation retention

ResourceMapping intentionally keeps CUDA resource references when synchronization
or destruction cannot safely complete. Its budget reservation now lives inside
the mapping and is retained on failed context drain, mapped-pointer free or
external-memory destruction, instead of the outer buffer releasing the charge.
Successful cleanup releases it normally; failures conservatively keep the charge
for the retained resource lifetime. Strict Windows GNU CUDA-library Clippy and
native CUDA-library Clippy passed, as did scoped whitespace checks. Physical
DX12 cleanup-failure injection remains unverified.

### Checked CUDA import release before graphics handoff

CudaExternalU32Buffer::release consumes the import, drains the context, destroys
the mapping and inspects cudarc's recorded Drop errors before releasing the
reservation. Failed completion retains mapping/module/reservation; destruction
errors retain the reservation conservatively and return an error. Vulkan and
DX12 gravity publication and physical export probes now use this checked path.
Graphics ownership is not restored on release failure; a numeric render-view
failure can restore graphics only after successful cleanup. Strict Windows GNU
CUDA Vulkan-library/DX12-example Clippy, native Vulkan-library Clippy, final CUDA
library Clippy, Linux GNU external-example cross-check and whitespace checks
passed. Physical cleanup-failure behavior remains unverified on NVIDIA.

### CUDA synchronization reports recorded cleanup errors

CudaCompute::synchronize now checks cudarc's recorded destructor errors after
context completion, so a successful drain cannot hide a preceding mapping/free
failure. Current explicit Vulkan/DX12 gravity handoffs already call this method
and use checked import release. Strict CUDA library Clippy and scoped whitespace
checks passed. Physical recorded-error injection remains unverified on NVIDIA.

### Optional CUDA build after checked release

The default-feature CUDA crate passed strict library/test Clippy and its full
library suite: 18 passed, no failures, one explicitly ignored gravity host
arithmetic test. Import safety docs now require successful checked release
before graphics reacquisition instead of treating ordinary Drop as checked
cleanup. These checks confirm the non-CUDA build stays usable without toolkit
or driver linkage; they do not prove physical CUDA release behavior.

### Checked CUDA u32-buffer release

CudaU32Buffer::release now drains work, destroys storage, waits for queued frees
and inspects destructor errors before returning its reservation. Failed
completion retains storage/module/reservation; failed cleanup retains the charge
conservatively. Import release also waits after destruction. The physical buffer
probe uses checked release for owner-drop and reservation-reuse fixtures, and
affine_u32 checks release on success and operation failure. Final strict CUDA
library/example Clippy and scoped whitespace checks passed. Windows GNU CUDA
library/buffer-example cross-check passed for the new release API before the
subsequent affine wrapper change. Physical cleanup execution remains unverified
on NVIDIA.

### Checked CUDA gravity-job release

CudaGravityJob::release drains work, destroys packed device storage, waits for
queued frees and checks recorded cleanup errors before returning its budget.
Failure retains the charge conservatively; failed completion also retains
storage/modules. The physical gravity probe now uses checked release after
owner-drop trajectory verification and its exact reservation/reuse fixture.
Strict CUDA library/gravity-example Clippy and Windows GNU CUDA library/example
cross-check passed. Physical gravity cleanup remains unverified on NVIDIA.

### Checked transient CUDA storage cleanup

Terrain, water, voxel regions, projectile, box sweep and roundtrip now destroy
their temporary device allocations and inspect completion/cleanup errors before
returning success or decoding solver output. Launch/readback errors after
allocation also pass through checked cleanup; projectile/box sweep release the
first allocation if the second allocation fails. A shared reservation helper
implements the same completion/free/error ordering for these operations, owned
u32 buffers and gravity jobs. Failed completion retains resources and their
charge; failed cleanup retains the charge conservatively. Final strict CUDA
library/test Clippy, Windows GNU CUDA-library check and whitespace checks passed.
Physical allocation/launch/readback/free fault injection remains unverified.

### CUDA checked-cleanup validation and redundant wait removal

The CUDA-feature library suite passed 17 driver-free tests with no failures;
one explicit gravity CUDA-source host arithmetic test remained ignored. Terrain,
water, voxel-region, projectile and box-sweep closures no longer synchronize the
stream immediately before the shared helper's context completion wait. The helper
still completes readback before storage destruction and waits for frees before
returning the reservation. Final strict CUDA library/test Clippy and scoped
whitespace checks passed. These tests do not execute CUDA cleanup on NVIDIA,
and no latency improvement is measured or claimed.

### Exact CUDA terrain hardware gate

Linux acceptance now requires Terrain CUDA identification and the complete
502-chunk / 16449536 exact-block / descriptor-seed-i64-cancellation parity marker,
matching the current terrain_smoke fixture inventory (5 seeds x20 XZ cases x5 Y
levels plus two extreme-Y cases). Windows now requires the same complete marker
instead of separate partial phrases. Shell/PowerShell syntax and whitespace
checks passed. These gate changes do not provide physical CUDA terrain execution
evidence; block IDs are already checked against the terrain palette on decode.

### Complete CUDA acceptance result markers

Linux now requires the existing final CPU-parity markers for projectile,
character, vehicle and gravity probes, including sticky gravity errors and
owner-drop/repeated-readback coverage. Its shader-pixel gate requires the full
Vulkan-export/CUDA-gravity/evolved-pixel marker. Windows likewise requires the
complete DX12 shader-pixel marker. These markers were verified against current
probe source; shell/PowerShell parsers and scoped whitespace checks passed.
Physical execution of these acceptance runners remains unverified on NVIDIA.

### Renderer hardware-error source chains

RendererError now exposes original surface, adapter, device, skinned-upload,
ray-scene and scene-resource errors through Error::source. Strict native render
library Clippy and scoped whitespace checks passed. Compute polling/readback
paths inspected in this pass already propagate errors; native renderer device
loss still needs a dedicated terminal diagnostic instead of only surface status.

### Native renderer terminal device loss

Renderer now retains the first wgpu device-loss callback diagnostic in a shared
OnceLock. Both render entrypoints return RendererError::DeviceLost before using
the surface; resize returns before reconfiguration or resource writes after a
registered loss. The native app checks the diagnostic before animation/rebuild
work and records terminal device/surface render errors as startup_failure so
main returns an error after the event loop exits.

The new device_loss_surface example executed successfully on Apple M4 Max / Metal
with real device.destroy(), a real native window and the driver's loss callback.
It checked a nonempty retained diagnostic, both render guards, and positive/zero
resize after loss. Native app cargo check passed with 13 existing library
warnings; strict renderer library and device_loss_surface example Clippy passed.
This is explicit device destruction evidence, not spontaneous driver-reset or
state-preserving recovery evidence. Physical NVIDIA, DX12 and mobile/headset
acceptance remains outstanding.

### Backend-specific native device-loss acceptance

The native device_loss_surface probe accepts strict auto/metal/vulkan/dx12/gl
selection and optional --require-nvidia. The latter rejects a renderer whose
selected adapter is not physical NVIDIA before destruction. Linux CUDA acceptance
now requires the probe on Vulkan, and Windows CUDA acceptance requires it on
DX12, with both backend and complete terminal-guard result markers. The shared
physical-adapter log classifiers recognize Device loss GPU output and retain
vendor/type checks; synthetic NVIDIA/other-vendor/CPU/unknown-label checks passed.
Shell and PowerShell syntax, strict example Clippy and scoped whitespace checks
passed. Explicit `device_loss_surface metal` again passed on Apple M4 Max.
The new Vulkan/DX12 gates have not been executed on physical NVIDIA hardware.

### Device-loss guards for renderer uploads

Chunk/lit-chunk upload entrypoints and their internal allocation path, camera
updates, skinned mesh uploads, joint updates and model updates now check the
retained terminal device diagnostic before writes or allocation. Existing void
material replacement follows resize behavior and returns before GPU work after
registered device loss. Raw device/queue access and scene renderer construction
remain low-level APIs; these changes do not promise universal protection for
callers that use those handles directly.

The real native Metal device-destruction example now also replaces a valid
material pack after loss and checks DeviceLost for empty geometry/lit uploads,
camera, joint and model updates. It passed on Apple M4 Max with both terminal
render and upload markers; strict library/example Clippy passed. Both NVIDIA
Vulkan/DX12 acceptance runners require the new upload marker as well. Script
syntax and scoped whitespace checks passed, but physical NVIDIA execution is
still unverified. The skinned mesh upload guard is compiled; this particular
probe does not build/upload a skinned mesh after destruction.

### Compute readback after native device destruction

The native device-loss probe now creates a WGSL compute program, submits a
storage-to-readback dispatch, destroys the device and only then begins mapping.
It requires a nonempty ComputeError::Mapping within five seconds of polling and
ComputeError::Consumed on the next try_read. Existing readback code already
propagates this terminal error; no production compute change was needed.
The real Metal / Apple M4 Max run passed this new marker plus the existing upload
and render guards. Strict example Clippy passed after extracting the readback
check into a helper. Linux Vulkan and Windows DX12 hardware acceptance require
the full new marker; script syntax checks passed. This fixture covers mapping
started after explicit destruction, not every race between an already pending
map and spontaneous driver loss. Physical NVIDIA execution remains outstanding.

### Native renderer recreation after explicit device loss

The native surface fixture releases the old compute program, scene and renderer,
then creates a fresh instance/renderer for the same retained Window. A fresh
WGSL dispatch must map the exact native u32 value 42 and carry no previous
terminal diagnostic. This sequence executed successfully on Apple M4 Max / Metal
with all earlier terminal loss markers. Vulkan/DX12 acceptance now requires the
same-window recreation marker. Strict example Clippy and script parsers passed
following extraction of the recreation helper. This proves explicit renderer
recreation and fresh compute, not automatic app/world restoration, surface
presentation after recovery, or spontaneous driver-reset recovery. Physical
NVIDIA Vulkan/DX12 execution remains unverified.

### Native surface presentation after renderer recreation

The device-loss surface fixture retains the recreated renderer/window, requests
redraw through the native event loop and requires RenderOutcome::Presented.
Transient non-presented outcomes retry until a five-second deadline; errors or
expiry fail acceptance. The real Apple M4 Max / Metal run passed all loss,
readback, recreation and new surface-presentation markers. Strict example
Clippy passed after extracting recovery scheduling. Linux Vulkan and Windows
DX12 hardware acceptance require the full surface marker; script syntax and
scoped whitespace checks passed. Presentation is wgpu's successful submit/present
outcome, not a pixel readback or visual comparison. Automatic application/world
restoration and physical NVIDIA runs remain outstanding.

### Bounded renderer hardware acceptance lifecycle

Linux/Windows hardware runners now prebuild voxel_diagonal, compute_smoke and
native device_loss_surface together with a separate 900-second build budget;
their runtime gates retain the existing 300-second limit and required markers.
The recreation fixture also wakes its event loop at the presentation deadline,
so absence of redraw events cannot bypass its own timeout. Closing its window
before completion records failure. The normal real Metal / M4 Max path again
passed all five markers, strict example Clippy and both script parsers passed,
and scoped whitespace checks passed. Timeout and user-close branches were
compiled but not separately triggered in this run. Physical NVIDIA execution
remains outstanding.

### Checked gravity render-status cleanup

CudaGravityJob::write_render_view now collects launch/readback results before
releasing its four-byte status buffer through Reservation::release. Both the
successful and failed launch/readback paths wait for completion and checked frees
before returning budget capacity. Synchronization/cleanup failure conservatively
retains storage or its reservation according to the shared cleanup helper,
instead of dropping the status reservation through ordinary scope cleanup.
Strict CUDA library Clippy and Windows GNU CUDA library cross-check passed;
CUDA-feature unit tests passed 17 with one explicit clang host-arithmetic test
ignored. These tests do not execute the cleanup path or fault injection on
NVIDIA. Scoped whitespace checks passed; physical CUDA/graphics interop probes
remain required.

### Opaque CUDA import failure accounting

Opaque external-memory imports now load the affine module/function before
import/mapping, eliminating shader-setup errors after a successful mapping.
Mapping errors conservatively retain the full reservation: current cudarc
map_range consumes ExternalMemory and can fail creating its event after obtaining
a mapped device pointer, without returning a mapping owner whose cleanup can be
checked. A failed mapping therefore cannot prove capacity is free. API error
docs state retained budget and that failure does not authorize graphics reuse.
Strict CUDA library Clippy, Windows GNU CUDA library cross-check and CUDA-feature
unit tests passed (17 tests; one clang host-arithmetic test ignored). Scoped
whitespace checks passed. No physical NVIDIA import-failure injection was run;
normal successful import/release accounting is unchanged.

### DX12 shader setup before external resource import

CUDA/D3D12 resource import now loads its affine module/function before constructing
ResourceMapping. Shader preparation failures therefore occur before external
memory is imported; successful imports retain the existing dedicated-resource
mapping and checked cleanup/budget behavior. Formatting and scoped whitespace
checks passed. The Windows GNU CUDA library strict Clippy check was launched and
remains live waiting for the shared Cargo build lock at this observation; no
passing compilation result is claimed yet. Physical DX12/NVIDIA execution is
still unverified.

### Completed DX12 preparation check and external module cleanup

The previously pending strict Windows GNU CUDA library Clippy check completed
successfully for DX12 shader preparation before import. External-buffer release
now drops both its mapping and affine function before final synchronize/check_err,
so final module-unload errors recorded by cudarc are included when the function
owns the final module reference. Previously its function could drop after the
last error check. Native and Windows GNU strict CUDA library Clippy and scoped
whitespace checks passed for this change. These are compilation checks; no
physical CUDA module-unload error injection was performed.

### Resident CUDA completion includes recorded cleanup errors

CudaU32Buffer::read, CudaGravityJob::snapshot and external mapped-buffer read now
check context.check_err after successful stream synchronization before decoding
or returning host data. External affine's synchronous completion does the same.
This surfaces driver errors recorded by cudarc resource cleanup even when stream
completion itself succeeds, matching CudaCompute::synchronize's existing policy.
Native and Windows GNU strict CUDA library Clippy passed; CUDA-feature unit tests
passed 17 with one explicit clang source-arithmetic test ignored. Scoped
whitespace checks passed. These are source/compilation/driver-free validation;
physical NVIDIA cleanup-error injection and normal GPU execution remain required.

### Interop caller checks after CUDA cleanup changes

The Linux GNU CUDA vulkan_external example cross-check passed with the current
import/release/readback changes. Strict Windows GNU Clippy passed for both
voxy_vulkan d3d12_export and cuda_gravity_render examples. Current Vulkan gravity
publication performs checked imported.release and compute synchronization before
storage.acquire; DX12 likewise does not restore graphics_ready until checked
release/completion succeed. These caller checks confirm target compilation and
source ordering, not actual device execution. Physical NVIDIA host availability
was requested separately; no hardware acceptance result is claimed. Scoped
whitespace checks passed.

### Browser water wakes published neighbor chunks

BrowserWater::wake now rescans water in every published visible chunk, preserving
its existing active cells and canonical deduplication instead of rescanning only
chunk zero. Coordinate reconstruction uses checked join_voxel. The existing
browser validate_loading fixture now seeds a specific water cell in a previously
loaded neighbor in a cloned world, clears activation and requires that exact cell
to be reactivated. Strict wasm32 library Clippy and scoped whitespace checks
passed. The new fixture has not yet executed in a rebuilt browser module; no live
browser proof is claimed. The 64 streamed-chunk ceiling and active/graph budgets
remain; this change does not implement eviction or fully resident water storage.

### Live browser neighbor-water activation proof

Rebuilt development WASM with project-local wasm-bindgen 0.2.127 and bumped both
web shell JS/WASM cache keys to water-neighbor-wake-v1. The in-app browser loaded
WebGPU voxel+streamingCheck and displayed 'GPU water streaming verified: 65536
blocks'. That validation now includes the exact seeded neighbor-cell wake check
before returning. DOM also reported GPU 27-chunk world generation, GPU water,
520 rendered frames and 258 completed character ticks at observation. Screenshot
browser-water-neighbor-wake-proof.png records the rendered result. All three web
lifecycle tests passed. Scoped whitespace checks passed. This validates the new
wake fixture in real browser WebGPU, not unbounded streaming, eviction, NVIDIA,
DX12 or mobile/VR execution. The 64-chunk streaming limit remains.

### Bounded browser water activation batches

Browser water now submits at most WaterBudget::default().max_active (16384)
queued cells per GPU plan. Successful commits remove that prefix and append
next-active cells after the retained tail with deduplication, so recurring head
cells cannot starve deferred work. Settled prefixes retain unprocessed cells and
report still-pending work rather than settling the whole simulation. Missing
chunk loads and revision conflicts do not consume queued cells. This is ordered
bounded batches, not one atomic world-wide water tick.

The browser streaming fixture additionally exercises a 16386-cell activation
queue: two tail cells remain ahead of recurring work, duplicates are omitted,
and a subsequently settled batch drains the queue. Strict wasm32 Clippy and
scoped whitespace checks passed. This new fixture and batching solver path have
not yet executed in a rebuilt browser module; the earlier browser proof covers
neighbor wake before batching. The 64 streamed-chunk limit still remains.

### Shared bounded scene frame scheduling

`voxy_runtime::FrameLoop` wraps the existing simulation clock with an immediate
pause policy and explicit platform-boundary reset. Default fixed updates are
60 Hz with at most six steps per frame; frame-update delta is capped at 100 ms.
Discarded real time and simulation backlog are reported by `FrameWork`, while
interpolation alpha remains available to presentation consumers. The caller
owns the wall-clock timestamp and GPU history invalidation.

`SceneApp` now uses this scheduler for scene behaviors and Rush fixed/frame
updates rather than a separate accumulator. Pause changes reset the timestamp;
suspension, resume and occlusion reset partial ticks. Specialized physics demos
retain their own solvers and clocks. This is scheduling integration, not yet a
unified scene/resource/compute/HDR renderer.
The lifecycle window fixture now verifies fixed callbacks as well as behavior
start, activity and destruction.

### Browser execution of water batch queue fixture

Rebuilt dev WASM using the local pinned bindgen and bumped both shell asset keys
to water-batches-v1. WebGPU voxel+streamingCheck completed in the in-app browser:
DOM reported waterBatchQueueVerified=true, 65536 compared blocks, GPU 27-chunk
generation and continued frames/character ticks. The streaming fixture executes
the 16386-cell queue retention/deduplication/fair-tail checks before returning.
Screenshot browser-water-batch-queue-proof.png records the rendered streaming
result. Three web lifecycle tests and scoped whitespace checks passed.
This is real browser execution of queue bookkeeping plus the small streaming
GPU/CPU solver fixture; it is not yet a multi-full-batch GPU solver acceptance
with more than 16384 active world cells. The 64-chunk streaming cap remains.

### Real WebGPU full-prefix and tail water solver acceptance

The streaming browser fixture now seeds a cloned center chunk with 16384 solid
active cells and a water source at the tail (16385 total). The real BrowserWater
poll path must settle the full first GPU batch without settling the queue, then
commit the tail-water batch. Every one of the 32768 center cells is compared with
an independent two-step CPU reference. Strict wasm32 Clippy and dev WASM build
passed. In-app browser WebGPU execution reported waterGpuBatchesVerified=true,
waterBatchQueueVerified=true and 65536 separate streaming block comparisons,
with ongoing rendering/character ticks. Screenshot browser-water-gpu-batches-proof.png
records the page. Asset keys are water-batches-v2; lifecycle tests and scoped
whitespace checks passed. This validates a full solid prefix plus a flowing tail,
not a worst-case full batch of flowing water or unbounded streaming. The
64-chunk cap and physical NVIDIA/mobile/VR acceptance remain outstanding.

### Scene device-loss reporting and redraw recovery

`SceneSurface` retains the first device-lost callback message, exposes
`device_failure` and nonblocking `poll_device`, and rejects reported loss before
resize, custom-frame submission or scene-frame submission. Reported failure
invalidates temporal access. Callers must recreate device-dependent resources.

`SceneApp` detects reported loss before scene uploads. At redraw and resize boundaries it
recreates GPU resources for device/surface loss while retaining the CPU scene
and behaviors, clearing motion history, scheduler remainder and wall timestamp.
A second failure before successful presentation exits rather than retrying
indefinitely. Other rendering errors follow the existing fatal path.

`VOXY_SCENE_DEVICE_LOSS_SMOKE=1` destroys the actual device at frame 30 in smoke
mode and requires its callback. On M4 Max Metal, the lifecycle example recovered
and completed 120 presentations, observed resize/activity changes, 47 fixed
updates, and exactly one behavior start/destruction. Four scheduler tests and
normal lifecycle smoke passed. Destruction is controlled loss injection, not a
physical GPU reset or proof on Windows, Android or browsers.

The Metal `scene_demo --smoke --motion` loss-injection run also recovered and
completed 120 frames. Its per-presentation checks verified aligned motion/depth
frame IDs, sizes and reset flags through recreation. Renderer/runtime strict
Clippy passed; the application build still emits pre-existing dead-code warnings.

### Dense water full-batch acceptance fixture

The browser streaming validator now retains the solid-prefix/tail scenario and
also seeds 16384 active cells with water level eight. The CPU reference must
produce a transaction for that full dense prefix, then the real GPU first commit
is compared against all 32768 center cells. The second scenario stops at the
first prefix commit; it does not claim second-batch dense-fluid cadence parity.
Strict wasm32 Clippy and scoped whitespace checks passed. The expanded fixture
has not yet run in a rebuilt browser module. Shell keys are water-batches-v3 and
waterDenseBatchVerified is set only after the complete validator returns.
World currently exposes no unload/eviction API found in this pass; removing the
64-chunk streaming cap would cause uncontrolled residency growth, so it remains
pending a world-retention/eviction implementation.

### Skeletal pose bridge for raster and ray geometry

`SkinnedMesh::posed_scene_mesh` explicitly bakes indexed world-space geometry
using the same four UNORM16 influences and model/palette ordering as temporal
correspondence. UVs and index topology are retained; callers choose linear
vertex color. Use identity model when feeding the resulting mesh into primary
raster guides and `RayScene::replace_scene_mesh`, then rebuild the scene and
refresh bindings on the changed geometry revision before queries. Scene guide
normals are recomputed from the deformed positions; authored skinned normals
are not preserved by this bridge. CPU baking/rebuilding is not a GPU refit path.

The ray probe adds a two-joint weighted pose replacement and restoration to its
hit-distance, barycentric and revision checks. The CPU correspondence test
checks exact shared indexed positions, UVs, colors and invalid input rejection.

### Live full dense-water GPU batch proof

Dev WASM rebuilt successfully with pinned project bindgen. In-app browser WebGPU
voxel+streamingCheck completed both batch scenarios and reported
waterDenseBatchVerified=true, waterGpuBatchesVerified=true,
waterBatchQueueVerified=true and 65536 separate streaming comparisons. The dense
fixture requires a CPU transaction from 16384 active level-eight water cells and
checks all 32768 center cells after the first GPU commit. Continued rendered
frames and completed character ticks were observed after validation. Screenshot
browser-water-dense-batch-proof.png records the rendered result. Scoped whitespace
checks passed. This verifies the first dense prefix, not repeated dense-batch
cadence, adjacent-chunk output parity for this dense case or performance. Physical
CUDA/DX12/mobile/VR tests and bounded world eviction remain outstanding.

On M4 Max Metal the expanded 11-phase ray probe passed, including the weighted
pose's approximately 1.25 world-unit hit distance, unchanged barycentrics,
geometry revisions and original-mesh restoration. Existing shadows, wide HDR,
GGX roughness/seed cases and primary motion checks also passed in that run.
The focused CPU posed-geometry/correspondence test passed. This is pose-to-BLAS
proof; an interactive skeletal ray-lighting example and GPU refit remain work.

### World unload/restore foundation for bounded streaming

World now exposes unload_chunk returning exact retained ChunkSnapshot data, and
restore_chunk borrowing that snapshot so failures do not consume caller data.
Retired revision metadata prevents insert_generated from silently replacing an
unloaded edited chunk. Restoration validates data and advances the revision;
older captured transactions and snapshots from earlier unload cycles reject.
The caller still must retain/persist returned data; no durable store or browser
residency eviction policy is implemented here, and the 64-chunk cap remains.

A focused test exercises edited-data preservation, generation rejection, fresh
revision, stale transaction rejection and old-snapshot rejection after another
unload. Strict world library Clippy and scoped whitespace checks passed. The
focused test binary is compiled and its live process has not returned a result
at this observation; the full world library test invocation is also live waiting
on the shared Cargo build lock. No passing unit-test result is claimed yet.

The focused unload/restore test subsequently completed successfully (one passed,
12 filtered out). Full world library tests remain live/pending at this observation.

Final strict Clippy remained blocked by concurrent changes: exact float UV-tag
comparisons in `scene.rs`, and unresolved `serde_json` plus literal-format lints
in the diagnostic binary. The pose test and Metal GPU run above passed before
those subsequent edits; they do not establish a clean current workspace build.

### Unload/restore atomic failure coverage

The initial full world library suite completed successfully (13 tests). Two new
cases then verified that unknown block IDs leave retained snapshot data and
unloaded revision metadata intact, and revision overflow leaves resident data
loaded without a tombstone. All six focused world tests passed (nine filtered).
Test-enabled strict Clippy identified an expect in the public unload path; it
was replaced with a typed ChunkNotLoaded error. Final strict library/test Clippy
and the expanded full 15-test suite are live waiting on the shared Cargo lock at
this observation; their results are not yet claimed. Scoped whitespace checks
passed. Browser eviction integration and durable retained-data storage remain
unfinished.

### Completed world gates and browser retained-chunk reload foundation

Final strict world library/test Clippy and all 15 world library tests passed.
BrowserWater now retains unloaded ChunkSnapshots in a host map. Its explicit
retirement helper rejects active cells and in-flight water/loading tasks, removes
visibility and requests mesh refresh. On MissingSample, the loader restores a
retained snapshot before terrain generation, republishes visibility and retries
from a fresh plan. Restore failure preserves the retained entry; successful
restore advances world revision. The streaming validation fixture exercises
retirement/restoration against a cloned world and requires a fresh revision.
Strict wasm32 Clippy and scoped whitespace checks passed.

This fixture and the new loader branch have not yet executed in a rebuilt
browser module. No automatic eviction policy or actor pinning is implemented;
the helper is currently invoked by validation. Retained snapshots remain in RAM,
so total host memory is not bounded by this change. Durable storage and the
64-chunk streaming cap remain outstanding.

### Live retained-chunk reload through WebGPU water loader

Retirement validation now seeds edited neighbor water, computes an independent
CPU transaction, unloads the chunk, and polls the real BrowserWater GPU path.
MissingSample must restore the retained snapshot and retry. The fixture requires
zero new terrain generations, removal from retained storage, visibility restored,
a newer revision, and exact CPU parity for all 32768 neighbor cells after commit.
Strict wasm32 Clippy and dev WASM build passed. In-app browser WebGPU execution
reported waterRetainedReloadVerified=true plus all prior dense/batch markers and
65536 separate streaming comparisons, followed by continued frames/character
ticks. Screenshot browser-water-retained-reload-proof.png records the page.
Lifecycle tests and scoped whitespace checks passed. Asset keys are
water-retained-reload-v1. This proves retained RAM reload through the loader,
not automatic eviction, actor pinning, durable storage or total host-memory
bounds. The 64-chunk streaming cap remains.

### Presented skeletal frame snapshots and animated ray window

`SkinnedMotionHistory::prepare_frame` captures immutable world-space scene
geometry, correspondence and the exact pose. `presented_frame` commits that
captured pose only after the caller confirms successful presentation. Candidate
snapshots from another history or from before reset/another commit return
`StaleHistory`, leaving history unchanged. A cloned history starts a separate
branch from the same generation. Existing direct pose APIs remain available;
constructing history now allocates a generation token and is no longer const.

Run `cargo run -p voxy_ray_probe --example animated_ray -- --experimental`.
This Metal example drives a two-joint surface, replaces/rebuilds indexed BLAS,
rasterizes textured primary guides, computes shadows/reflections into linear
HDR, rasterizes deformation motion and tone maps to a window. It commits the
snapshot only for `Presented`. A 2D overlay shows exposure and pause state;
Space pauses, R resets pose history, +/- changes exposure, Escape exits. Resize
recreates frame attachments and resets history. Scene textures/overlay are
retained; the ray/frame pipelines and geometry are recreated per candidate.
This is a correctness example, not a scalable GPU-skinning/refit implementation.

`--smoke` passed on M4 Max Metal: 120 presentations, an observed resize after
frame 30, reset after frame 60, finite center HDR greater than one, finite
nonzero animated motion, and zero center motion on history-reset frames. It
samples the center per frame rather than validating all animated pixels or
measuring GPU performance. Three skeletal history tests passed, including
captured-pose commit and stale/foreign candidate rejection. No temporal
reprojection/denoising or NVIDIA inference is implied by these checks.

The updated renderer also passed wasm32-unknown-unknown checking with `webgl`.
That is library portability, not browser ray queries or the Metal window demo
running in a browser. Final combined example Clippy was blocked by concurrent
physics errors (`collected`/`nearest` in quadratic automatic impact). The native
interactive executable was also launched successfully and stopped after the
check. Native UI automation could not resolve its unbundled executable as an
application, so no screenshot/visual-layout acceptance is claimed here.

A separate final renderer-library strict Clippy run passed. Combined example
Clippy remains unverified because of the unrelated physics compilation failure.


### Collision request chunk footprints (2026-10-02)

`VoxelRegionSnapshot::captured_chunks` exposes the exact integer chunk keys read by
a collision snapshot, including missing chunks. `PendingVoxelSweep`,
`PendingGpuCharacter` and `PendingGpuVehicle` forward these keys without consulting
the mutable world. Character replay retains completed query footprints as well as
pending ones; duplicate positions are intentional and can be collected into a pin set.

Validation: the two `voxel_snapshot::tests` passed, including a loaded negative
chunk next to an unloaded positive chunk and a check that enumeration performs no
additional world reads. Strict `voxy_gpu` library Clippy and scoped diff checks passed.

This exposes the query footprint only. Browser actor-body pinning, automatic eviction
and durable archived chunk storage are still pending. No new physical NVIDIA or
mobile/headset execution evidence was obtained in this step.

### Depth-rejected HDR temporal reprojection

`TemporalResolve` encodes nearest-pixel backward normalized UV lookup and an
explicit radiance blend into RGBA32Float. It rejects offscreen/NaN motion before
integer coordinate conversion and rejects background or depth disagreement.
Inputs include previous presented radiance/depth and expected previous-camera
NDC depth at current coverage. That expected depth must follow the previous
pose and camera; current depth cannot replace it for animation/camera motion.
All raw inputs/encoders must belong to the resolver's device. Texture formats,
dimensions/usages and settings/dispatch limits are checked before binding.

The caller retains history only after presentation and supplies reset for cuts,
resize and device loss. Current invalid RGB becomes black; invalid/negative
history is rejected. Output alpha is one. This pass is a reprojection primitive,
not a variance-clipped denoiser, adaptive sample accumulator, automatic history
manager or DLSS replacement. Animated-ray expected-depth generation and wiring are implemented in the
following section.

`cargo run -p voxy_render --example temporal_resolve` checks analytic four-pixel
cases on the GPU: valid translated UV history, disocclusion, offscreen motion,
NaN motion/current radiance and reset. Initial Metal execution passed all color
and alpha channels; final rerun/Clippy results are recorded below.

The final Metal rerun and renderer-library strict Clippy both passed.


### Browser actor chunk protection (2026-10-02)

`body_chunks` uses the collision solver's validated integer-relative bounds and
exclusive maxima to enumerate occupied chunks without converting world anchors to
floating point. Browser character and vehicle protection unions current body
footprints with every retained GPU sweep footprint. `poll_voxel_water` refreshes
this set before polling; retirement refuses protected chunks before mutating the
world or retained archive.

Validation: the body footprint test passed (negative boundary, exclusive upper
edge, coordinates beyond f64 integer precision, overflow rejection). Strict native
`voxy_gpu` and wasm `voxy_web` library Clippy passed. Three web lifecycle JS tests
and scoped diff checks passed. The rebuilt dev WASM ran in the existing WebGPU
browser at port 8793: `waterActorPinsVerified=true`, retained reload/dense batching
markers true, 65536 streaming cells compared, running character/frame counters.
The retirement fixture verified pin rejection preserves the chunk revision and
archive, then unpinned and exercised the existing actual GPU reload path. Screenshot:
`docs/browser-water-actor-pins-proof.png`. Asset cache key: `water-actor-pins-v1`.

Automatic eviction scheduling and durable archive storage remain pending. The
retained archive still consumes host RAM and the cumulative new-terrain cap remains
64. This step adds no physical NVIDIA/CUDA/RTX or mobile/headset proof.

### Previous-pose depth generation and animated temporal wiring

`PreviousDepthPass` rasterizes expected previous-camera NDC depth at exact
current opaque coverage, using the existing paired-vertex rasterizer and Equal
depth testing. Output R32Float clears to zero; previous positions behind the
camera or outside the depth interval are invalid zero. The frame owner exposes
`RasterRayFrame::previous_depth` using its own depth and current camera.

The animated ray example now moves a joint along both X and Z. It generates
expected previous depth from correspondence and current depth from current/
current pairs, then runs `TemporalResolve` before tone mapping. Radiance and
current-depth candidates become history only on successful presentation,
together with the matching pose/camera. Resize clears GPU and pose history;
manual pose reset triggers temporal reset. This is nearest-pixel reprojection
with fixed blend/depth tolerance, not a full temporal denoiser.

The smoke reference uses screen barycentrics with perspective correction to
compute previous and current center depths from the paired world vertices. It
compares both R32 GPU results with that reference and checks finite HDR/motion
and explicit reset while requiring resize and nonzero motion over 120 frames.
Verification results are recorded below.


### Automatic browser water chunk retirement (2026-10-02)

Before capturing the next GPU water plan, BrowserWater now trims its visible
resident working set toward 32 chunks when no water readback or terrain load is
in flight. The bootstrap chunk, actor/query pins, all queued active water chunks
and a conservative one-chunk halo are retained. Protection takes priority over
the target: if all candidates are protected the set can exceed 32. Selection is
deterministic by chunk ordering. Retired snapshots are preserved in the existing
RAM archive and trigger mesh refresh.

The rebuilt WASM executed the fixture in real WebGPU at port 8793:
`waterAutoRetirementVerified=true`, 35 to 32 visible chunks, three archived chunks
restored with identical data, protected actor/water/neighbor chunks preserved.
The all-protected fixture preserved all 35 chunks. Existing retained reload, dense
water batching and 65536-cell streaming parity checks also passed. Screenshot:
`docs/browser-water-auto-retirement-proof.png`; cache key `water-auto-retire-v1`.
Three JS lifecycle tests and scoped diff checks passed.

Strict WASM Clippy is not passing for the final shared tree: it encountered an
ambiguous array type in physics quadratic render_surface; the points array was
annotated as [Vec3; 6]. The retry then encountered an independent ongoing physics
edit in liquid/film_impact_events.rs:158, where impact_events_impl lacks its ninth
bool argument. That call was left unchanged rather than guessing its semantics.

The target covers the water-visible working set, not all initial World chunks,
all CPU memory or GPU residency. Archived chunks still occupy RAM; durable storage
and resident resource accounting remain pending. The cumulative cap of 64 newly
generated water chunks is unchanged. Physical NVIDIA/CUDA/RTX verification remains
unavailable.

Both initial and stricter Metal smoke runs passed 120 presentations. The final
run used a 1e-6 depth tolerance and required a nonzero previous/current depth
difference, so substituting current depth for the previous pose is detected.
Renderer-library strict Clippy and scoped whitespace checks passed. These are
center-sample integration checks, not full-image temporal quality acceptance.


### Complete installed-NVRTC architecture compilation matrix (2026-10-02)

The CUDA verifier now obtains supported architectures directly from the installed
NVRTC instead of selecting only sm_52, sm_75 and sm_89. Its strict C probe supports
`--list-architectures`; all six CUDA sources and the embedded affine PTX are compiled
and assembled for every returned architecture. The existing unknown architecture
and missing entrypoint rejection gates remain enabled.

This exposed a real coverage defect: affine.ptx declared sm_52, preventing assembly
for compiler-supported sm_50. Its integer-only kernel now declares sm_50. The
rerun passed all seven sources on 14 architectures: 50, 52, 53, 60, 61, 62, 70, 72,
75, 80, 86, 87, 89, 90. Evidence: 84 NVRTC CUDA-source compilations and 98 PTXAS
source/architecture assemblies; both rejection gates passed. Summary retained in
`docs/cuda-all-architectures-proof.txt`; detailed ignored log is
`target/cuda-verified/all-architectures.log`. No physical GPU execution was performed.
This matrix covers pinned NVRTC 12.6.85, not architectures absent from that compiler.

The previous shared physics missing argument was already fixed when re-read; no
additional edit to that call was made in this step. Three onset regression tests
passed, and strict wasm voxy_web Clippy passed. CUDA-feature library unit test
process remains live waiting for the shared Cargo build lock (session 6580); it
must be resumed before reporting that additional gate as passing. Shell syntax
and scoped diff checks passed. The full hardware goal remains incomplete.

### Neighborhood bounds for temporal HDR history

`TemporalResolve::prepare_clipped` adds opt-in 3x3 linear RGB min/max bounds
before blending accepted history. Out-of-image neighbors are skipped; nonfinite
neighbors are ignored and finite negative RGB is clamped to zero. This prevents
stale accepted history from exceeding current local color bounds. The original
`prepare` remains unclipped for callers needing the previous estimator behavior.
Clipping can bias noisy ray radiance and is not a variance/moments denoiser or a
proof that every ghosting artifact is eliminated.

The animated ray demo uses clipped reprojection with its existing pose/depth
and successful-presentation history rules. The analytic GPU fixture compares
unclipped history, reset, bright-history clipping, dark-history clipping at the
image edge, and a nonconstant HDR neighborhood. Verification follows below.

CUDA-feature unit-test session 6580 subsequently completed: 17 passed, 1 ignored
(native clang host arithmetic test). This is host-side unit validation, not physical
NVIDIA execution. No test process from this step remains pending.


### Unified runtime CUDA precision policy (2026-10-02)

All six runtime NVRTC compilation call sites now use CudaCompute::compiler_options
without workload-specific overrides. The shared options explicitly disable fmad,
fast math and flush-to-zero, and enable precise division and square root, matching
the flags used by the installed-NVRTC architecture matrix. Gravity/projectile/box
sweep retain their previous precision policy; integer terrain/water/voxel-region
workloads now inherit the same explicit settings instead of compiler defaults.
Their arithmetic algorithms and packed ABI are unchanged. The separately embedded
affine PTX is still assembled directly.

Scoped formatting and diff checks passed. Native and Windows GNU CUDA-feature
library Clippy sessions are pending on the shared Cargo build lock (70085, 33419).
No physical NVIDIA execution is claimed; complete hardware acceptance remains
unverified.

Both CUDA precision-policy Clippy sessions completed successfully: native 70085
and Windows GNU 33419, each with CUDA enabled and warnings denied. No validation
process from this step remains pending. This verifies compilation/lints, not driver
execution or physical numerical parity.

On M4 Max Metal, all five analytic cases passed across every captured RGB/alpha
channel; renderer-library and diagnostic-example strict Clippy passed. The
animated clipped path also completed 120 presentations with depth/motion checks
and resize/reset after an initial size-compatibility failure. The demo now
reconciles actual window dimensions before preparing each candidate instead of
relying only on resize event delivery, clearing incompatible GPU/pose history.
Its smoke mode additionally defers resize handling to exercise draw-time
reconciliation; the final result is recorded below.


### Repository-wide WGSL source validation (2026-10-02)

Added voxy_render example `wgsl_inventory`, which recursively discovers every
WGSL file under crates and parses/validates it with the renderer's own re-exported
Naga version. Every source must contain at least one entrypoint; failures are
reported together and produce a failing exit code. Sources containing rgba16float
also receive an rgba32float variant check. The scanner does not follow symlink
directories. All validation flags and capabilities are enabled: this verifies
source semantics, not a particular adapter's feature support or pipeline layouts.

The current repository passed: 32 files, 37 variants, 59 validated entrypoints,
zero failures. This includes ray-query shaders, raster/temporal/HDR shaders, both
material shaders, and GPU compute sources. Command:
`cargo run --locked --offline -p voxy_render --example wgsl_inventory`. Evidence:
`docs/wgsl-inventory-proof.txt`. Strict example Clippy passed. Linux scene-shader
smoke now builds and runs this inventory before its Vulkan/OpenGL device checks.
Shell syntax and scoped diff checks passed; the updated complete Docker smoke
was not executed in this step. No additional physical GPU execution is claimed.

The final animated Metal smoke passed 120 presentations with resize notifications
intentionally deferred between frames 30 and 60. Draw-time size reconciliation
reset incompatible history, while previous/current depth reference checks and
motion/reset checks continued to pass. This validates the integration and
specific clipping cases; full-image ghosting/temporal quality and performance
acceptance remain unverified.


### Updated Linux shader smoke execution (2026-10-02)

The full updated `sh tools/linux/scene-shader-smoke.sh` completed with exit 0.
Its container-built inventory passed 32 WGSL files / 37 variants / 59 entrypoints.
Both Vulkan and OpenGL then passed scene shader replacement, unchanged-source
cache, rejection of syntax/entrypoint/binding errors, and image checks for clear,
indexed mesh, depth occlusion, textured RGBA and overlay alpha, with and without
MSAA. Both voxel-diagonal runs passed 128 AO pixel comparisons against analytic
interpolation. Summary: `docs/linux-shader-smoke-proof.txt`; ignored detailed log:
`target/linux-shader-smoke-current.log`.

Both backend adapters were Mesa llvmpipe CPU devices. This is actual driver/pipeline
and software rendering verification, not physical NVIDIA/Vulkan/OpenGL hardware
acceptance. Ray-query source validation does not prove ray-query device execution.
The new inventory gate is now exercised end to end in the Linux smoke workflow.


### WGSL inventory in physical NVIDIA acceptance gates (2026-10-02)

Linux Vulkan and Windows DX12 CUDA hardware acceptance scripts now prebuild and
run the `wgsl_inventory` example. Each requires an anchored summary with positive
file, variant and validated entrypoint counts and zero failures. These remain
source gates; subsequent existing probes still require physical NVIDIA graphics
adapters and actual CUDA execution.

Validation: POSIX shell syntax and PowerShell AST parsing passed. Both summary
classifiers accepted the current positive result and rejected zero file/variant/
entrypoint counts, nonzero failures and truncated summaries. The inventory example
and its dependencies passed Windows GNU cross-check. Scoped diff checks passed.
The physical acceptance scripts were not run on NVIDIA hardware; CUDA/RTX/DX12
execution proof remains unavailable on this Apple host.


### Per-channel temporal HDR sanitation (2026-10-02)

Current RGB sanitation now uses a per-channel finite mask. A NaN or infinity
in one channel no longer discards valid radiance in the other two. Negative
finite values still clamp to zero; output alpha remains one. History acceptance
and neighborhood sample rejection retain their whole-RGB policy.

The temporal_resolve Metal diagnostic passed on Apple M4 Max with its five
existing analytic cases and a sixth colored case covering partial NaN, positive
and negative infinity, negative radiance, and ignored input alpha. Command:
`cargo run --locked --offline -p voxy_render --example temporal_resolve`.
This verifies numeric sanitation, not full-image denoising quality.

Strict example Clippy was blocked by existing physics dependency lint errors,
including unreadable literals in liquid/film_impact_events.rs and naming lints
in droplet_coalescence.rs. No passing Clippy result is claimed for this step.


### Current physical Metal ray-query verification (2026-10-02)

The existing isolated experimental ray probe was rebuilt and run against the
current worktree with `cargo run --locked --offline -p voxy_ray_probe --bin
voxy_ray_probe -- --experimental`; exit 0. The reported adapter was Apple M4 Max,
IntegratedGpu, Metal. This is GPU execution evidence, not the Mesa CPU path.

The probe passed BLAS/TLAS geometry, transforms, two shared-BLAS instances, masks
and removal, invalid-update retention, blocked/unobstructed finite shadow segments
and the 64x64 shadow image. It also passed primary GGX comparisons over roughness
0.2/0.5/1 and seeds 0/17/31 for both occlusion configurations, 16-pixel/eight-sample
CPU reference means, depth-to-position/normal, static-camera UV motion with camera/
object translation/reset, material-weighted HDR reflection composition, and reserved
background IDs/zero motion. See `docs/metal-ray-hardware-proof.txt` and
`docs/metal-ray-shadow-proof.ppm`; ignored full build log:
`target/ray-hardware-current.log`. This verification uses wgpu's explicitly enabled
experimental ray-query API in the isolated probe. It does not enable ray tracing
automatically in games, prove production-wide state recovery, or provide NVIDIA
CUDA/RTX/DX12 execution evidence. Full hardware support remains incomplete.


### GGX reflections in the common animated raster-ray frame (2026-10-02)

RayLightingFrame::with_ggx and RasterRayFrame::with_ggx now prepare the existing
seeded GGX reflection shader in the same ordered guide/reconstruction/shadow/HDR
path used by mirror frames. Existing new constructors retain mirror behavior.
GGX composition uses new_hdr to accept RGBA32Float reflection contributions
without truncating them to binary16. Direct point lighting remains Lambertian;
reflections see opaque emissive triangles, not recursive indirect lighting.

The animated_ray example now supplies guide roughness 0.35 and a seed derived
from the presentation count. It integrates those samples into existing clipped
temporal history. The smoke reads the unfiltered reflection contribution, checks
finite nonnegative RGB each frame and requires a positive sample before success.
RasterRayFrame exposes reflected_radiance for this inspection.

The initial run rejected the mixed RGBA16/RGBA32 composition; this exposed and
led to the wide-HDR integration above. The final command passed on Apple M4 Max
Metal: `cargo run --locked --offline -p voxy_ray_probe --example animated_ray --
--experimental --smoke`. It presented 120 frames with skeletal BLAS replacement,
positive finite GGX reflections, resize, analytic previous/current depth checks
and reset motion. Strict renderer library Clippy passed with --no-deps; scoped
diff whitespace checks passed. No full-image visual quality or performance claim
is made. Pipeline/resource reuse, direct GGX integration, richer reflected
lighting and browser fallback remain work toward the full engine.


### Strict ray-probe backend and physical NVIDIA selection (2026-10-02)

The existing ray-query probe now parses all CLI arguments before device creation.
`--backend auto|metal|vulkan|dx12|gl` restricts the instance backend. Unknown, repeated
and conflicting arguments are errors, and --experimental remains mandatory.
Existing --face/--rough-image modes remain available as mutually exclusive modes.
`--require-nvidia` enumerates only the selected backend and chooses a physical
DiscreteGpu/IntegratedGpu NVIDIA adapter advertising EXPERIMENTAL_RAY_QUERY;
CPU/virtual/non-NVIDIA adapters cannot satisfy that request. No implicit switch to
another API is possible when a strict backend is requested.

Validation: focused options unit test passed. The complete physical M4 Max probe
passed with `--experimental --backend metal`; evidence:
`docs/metal-ray-strict-backend-proof.txt`. The NVIDIA-requirement rejection probe
on this Metal host produced the required error and exit 1; wrapper gate exit 0.
Scoped diff checks passed. Strict probe Clippy failed on 25 existing diagnostics
in face/specular probe code (including numerical casts and function lengths);
no broad changes to those independently edited probe files were made. This gate
is not reported as passing. No process from this step remains pending and no
physical NVIDIA execution is claimed.


### Direct GGX in coherent animated lighting (2026-10-02)

The with_ggx constructors now select SurfaceLightingJob::with_ggx for direct
point-light shading as well as seeded GGX emissive reflections. Both consume
the same reconstructed normal/roughness, material F0 map and camera. The new
constructors keep Lambertian diffuse plus GGX specular in wide HDR; existing
mirror constructors retain their previous direct-light behavior. This supersedes
the previous section's Lambertian-only limitation for with_ggx.

RasterRayFrame::direct_radiance exposes the unfiltered direct contribution.
The animated smoke separately reads its RGBA32Float center pixel and requires
finite nonnegative RGB with positive direct light on every presented frame.
The final animated_ray --experimental --smoke passed 120 presentations on
Apple M4 Max Metal, including direct GGX and reflection samples, resize,
previous/current depth references and motion reset. Renderer library Clippy
with --no-deps and -D warnings passed. This integration check does not prove
full-image quality, all material extremes or performance acceptance.

The full headless regression also passed: `cargo run --locked --offline -p
voxy_ray_probe -- --experimental --backend metal`. It verified direct GGX and
opaque shadows against CPU references across roughness 0.2/0.5/1 and seeds
0/17/31 on both occluded/unoccluded fixtures (16 pixels each), alongside
reflection means/distances, wide HDR, primary reconstruction, camera/object
motion, background validity and TLAS instance regressions. This exercises the
underlying shaders in addition to the animated common-frame integration.


### Preserve face-probe arguments under strict backend selection (2026-10-02)

Inspection found a regression introduced by the new strict ray-probe parser:
existing face rendering controls were being rejected. Options now accepts all
22 existing face argument names, including the valued --face-preset option and
its JSON path. It requires --face for those controls, rejects repeated controls,
missing/empty preset paths and a subsequent flag being consumed as a path.
Face rendering still consumes the original process arguments, so the path and
diagnostic flags reach the existing renderer without changing its algorithms.

Both focused options tests passed, covering strict backend/unknown/conflicting
arguments and compatible face preset/skin/eye/lid controls. Source-name inventory
confirmed every current face flag is represented in the parser. Scoped diff checks
passed. Strict probe Clippy still reports 25 face/specular diagnostics (numeric
literal style, casts, duplicated branches, closures, function lengths); no new
options-module diagnostic was present. A new physical face render was not performed
in this step. The previous physical strict Metal ray-query proof remains separate
from this parser compatibility validation. Full hardware goal remains incomplete.


### Retained GGX ray-lighting pipelines (2026-10-02)

GgxRayLightingPipeline retains the existing compiled direct GGX, reflection
GGX and wide HDR composition pipelines on one device. Its prepare method
creates independent immutable frame bindings/uniforms/outputs using those
pipelines, validates scene ownership, and reconstructs primary surfaces.
RasterRayFrame::with_ggx_pipeline integrates this cache without changing the
uncached mirror/GGX constructors. Recreate the cache after device replacement.

animated_ray creates one cache during device initialization and retains it
through skeletal geometry replacement, changing seeds, temporal reset and
resize. The Metal smoke passed 120 presentations on Apple M4 Max with positive
finite direct GGX/reflected samples, previous/current depth references and reset
motion. Command: `cargo run --locked --offline -p voxy_ray_probe --example
animated_ray -- --experimental --smoke`. Strict renderer library Clippy
(--no-deps, -D warnings) and scoped diff whitespace checks passed.

This removes recompilation of these three shading stages from frame preparation
by construction; no measured frame-time improvement is claimed. Raster, primary
reconstruction, motion/depth pipelines, frame attachments and acceleration
resources still require further retention/reuse work. Immutable prepared frame
resources may coexist, but concurrent submission equivalence has not been tested
here. Browser/mobile/physical NVIDIA execution remains separate acceptance work.

## Typed requirements and derived gameplay data

`SceneGraph::active_components_with<A, B>` borrows effectively active owners with
both component types. `ComponentTable::rebuild_active_with` stages all derived
rows before publishing them; factory failure preserves the previous table.
For resource identity preservation and selective construction,
`AssociatedData<A, B, T>` reuses rows with unchanged component revisions and
provides revision-validated `get`/`query`. Mutable component access conservatively
changes its revision. These APIs do not enforce parallel system access.

Run `cargo run -p voxy_scene --example associated_gameplay` for queued component
changes, factory reuse, failed-build suppression, repair and activity retirement.
Run `cargo run -p voxy_scene --example asset_derived_data` for immutable AssetCatalog
publication: failed reload preserves last-good identity, successful replacement
invalidates derived rows, and removal hides unavailable data. Both examples and
focused strict Clippy passed. Asset dependencies are connected explicitly by the
consumer; automatic dependency discovery is not implemented.

`invalidate` hides published rows until a successful synchronization and preserves
old data for retry construction. `clear` immediately releases rows while keeping
scene components; subsequent synchronization constructs fresh data without a
previous row. Use explicit retirement for processor removal or an unavailable
dependency that will not be rebuilt. Cleanup regression verification is pending.
Factory external side effects, shared interior mutation and user Drop panics remain
outside rollback. Cheap-row caching is an explicit strategy: see
`engine-research/measurements/associated-data-2026-10-02.md` for local measurements
that do not support a universal speedup.


### Retained primary-surface reconstruction (2026-10-02)

PrimarySurfacePipeline compiles the depth-to-world compute stage once per device
and prepares independent camera uniforms, texture bindings and output buffers.
PrimarySurfaceJob::new retains its uncached behavior and shared validation. The
GGX ray-lighting cache now also retains this reconstruction pipeline, so common
animated frame preparation no longer recompiles it. Raw input textures must
belong to the cached device; scene ownership is validated by the common path.

The headless primary-position verifier now uses PrimarySurfacePipeline::prepare.
Full Metal ray-probe passed on Apple M4 Max, including all 16 reconstructed world
positions/normals in both occluded/unoccluded fixtures, GGX CPU references, motion,
background and TLAS regressions. The animated common-frame smoke also passed
120 presentations through geometry updates, resize and temporal reset. Commands:
`cargo run --locked --offline -p voxy_ray_probe -- --experimental --backend metal`
and `cargo run --locked --offline -p voxy_ray_probe --example animated_ray --
--experimental --smoke`. Renderer library Clippy with --no-deps/-D warnings and
scoped diff checks passed.

This supersedes the previous reconstruction-pipeline retention limitation.
Surface buffers, raster attachments, raster/motion/depth pipelines and frame
bindings remain allocation/reuse work. No timing improvement or new platform
hardware acceptance is claimed.


### Face ray-probe numeric bounds and position keys (2026-10-02)

Studio disk sampling now validates a positive u16-range sample count before
converting counts/indices losslessly into f32. Zero and larger counts return typed
errors. The active 4/16 sample formulas retain the same numerical inputs. Visibility
mean denominators also use checked, exact conversions instead of usize casts.

Skin-normal hash keys preserve the previous rounded-f32 quantization but store
its bit representation, canonicalizing signed zero. This fixes distinct large
positions collapsing at the saturated i32 boundaries. Nonfinite or overflowed
quantized positions return errors before publishing the shaded mesh. Three focused
face tests passed: finite/deterministic disk coverage, count limits including the
maximum accepted count, and far-position/signed-zero/overflow/NaN key behavior.

The actual strict Metal face command (--experimental --backend metal --face
--lid-surface --eye-side) completed with exit 0 on physical Apple M4 Max. Three
poses each evaluated 53856 ray segments against 3968 triangles, with 1864/2112/2607
occluded segments. The rendered open/partial/closed lid strip was viewed and saved
as `docs/metal-face-ray-numeric-proof.png`; selected runtime evidence is in
`docs/metal-face-ray-numeric-proof.txt`. This also exercised the previously restored
face CLI arguments. It is not a visual equivalence comparison to a previous image.

Strict probe Clippy still fails on 18 remaining diagnostics; seven numeric cast
diagnostics removed by this change. Scoped diff checks passed. No validation
process from this step remains pending. Physical NVIDIA/CUDA/RTX and other target
platform verification remains separate and incomplete.


### Explicit primary-surface storage transfer (2026-10-02)

PrimarySurfacePipeline::prepare_reusing consumes a previous reconstruction job
and transfers its surface buffer when dimensions match. Changed dimensions
allocate replacement storage. Device ownership is checked before transfer;
normal camera/texture/capacity validation is preserved. Uniforms and bindings
remain per-job so a new camera is not silently replaced by the previous one.

Consumers must order previous buffer reads before the new reconstruction encode
on the same queue. Consuming the job does not imply exclusive GPU resource
ownership: cloned raw Buffer handles must obey the same ordering. This is an
explicit transfer API, not an automatic pool or GPU completion tracker.

The primary verifier encodes an identity-camera job, transfers its buffer to a
different camera job, asserts identical Buffer handles, then verifies all 16
world positions/normals against CPU references. Both occluded/unoccluded fixtures
passed on Apple M4 Max Metal in the full ray-probe. A separate preparation check
resizes 4x4 to 1x1 and asserts different storage, dimensions [1,1] and 32 bytes;
it does not claim rendered resized-guide validation. Command: `cargo run --locked
--offline -p voxy_ray_probe -- --experimental --backend metal`. The existing
GGX, shadow, motion, background and TLAS regressions passed. Renderer library
Clippy (--no-deps/-D warnings) and scoped diff checks passed.

The animated common frame still prepares fresh surface storage; transferring
completed frame resources into its next frame is the next integration step.
No measured allocation/frame-time improvement or additional platform proof is
claimed for the animated path in this step.


### Animated common-frame reconstruction storage reuse (2026-10-02)

GgxRayLightingState carries a seed and optional previous PrimarySurfaceJob.
GgxRayLightingPipeline::prepare_with_state transfers its storage through the
existing checked prepare_reusing API. GgxRasterRayResources binds this state
to a cached pipeline for RasterRayFrame::with_ggx_resources. Existing frame
constructors remain available. RayLightingFrame/RasterRayFrame::into_primary
consume the frame to transfer its reconstruction resource for a later encode.
All previous consumers must precede new writes on the same queue; raw cloned
buffer handles obey the same explicit ordering contract.

animated_ray retains the reconstruction job after frame handling and passes it
to the next frame. Only presented outcomes commit pose/image/camera history;
storage reuse is independent of temporal validity. Device initialization clears
the retained job. Resize passes the old job through dimension checks and allocates
replacement storage. The smoke asserts identical Buffer handles at equal sizes,
different handles across size changes, and requires both reuse and replacement
before success.

Metal on Apple M4 Max passed 120 presentations with these assertions, skeletal
BLAS replacement, finite positive direct GGX/reflections, previous/current depth
references and reset motion. Command: `cargo run --locked --offline -p
voxy_ray_probe --example animated_ray -- --experimental --smoke`. This closes
the previous animated primary-storage integration gap. No timing/allocation
profile is claimed; other frame textures, bindings and raster/motion/depth
resources remain reuse work.


### Physical NVIDIA ray-query acceptance mode and Windows check (2026-10-02)

Existing CUDA hardware acceptance runners now offer an explicit ray-query gate:
Linux command `sh tools/cuda/hardware-acceptance.sh 0 --ray-query`; Windows command
`tools/cuda/hardware-acceptance.ps1 -DeviceOrdinal 0 -RequireRayQuery`. The extra gate
prebuilds the ray probe, runs the strict Vulkan or DX12 backend with --experimental
and --require-nvidia, and requires physical NVIDIA adapter metadata plus ray-scene,
background/motion and primary HDR reflection result markers. Missing backend ray
support is a failing gate, not a fallback. Existing CUDA-only acceptance behavior
remains available for devices without ray-query capability; a CUDA-only pass is
not a ray-query proof. Ray selection is independent of the CUDA ordinal and its
actual adapter is logged, so this gate does not claim CUDA/ray device identity.

POSIX shell syntax and PowerShell parser checks passed. Both physical-ray adapter
classifiers passed seven synthetic cases: discrete/integrated NVIDIA on the expected
API accepted; wrong vendor, CPU, virtual GPU, wrong API and absent metadata rejected.
These synthetic cases are classifier verification, not physical GPU evidence.

The full ray-probe binary passed Windows GNU cross-check. The initial attempt hit
an independently edited physics API missing closest_surface_interpolated; the
method was present when re-read, and the subsequent check passed without editing
that API. The app dependency emitted 15 existing dead-code warnings. Cross-check
proves compilation only: no Windows/NVIDIA ray execution was performed. Scoped
diff checks passed and no process from this step remains pending. Full hardware
acceptance remains incomplete.

Strict renderer library Clippy (--no-deps/-D warnings) passed after correcting
a missing documentation backtick around SceneDraw in scene/lod_geometry.rs.
Scoped diff whitespace checks also passed.


### NVRTC compiler diagnostics and same-program recovery (2026-10-02)

The pinned compiler probe now has a --recovery-check gate, invoked by the normal
CUDA architecture verifier. It creates one NVRTC program, requires INVALID_OPTION
and a diagnostic containing the deliberately invalid option, then requires
COMPILATION and a diagnostic containing a conditional source error. Finally it
compiles that same program with valid options and requires the recovery entrypoint
in PTX. This distinguishes compiler recovery from merely starting another process.

The full updated verifier completed with exit 0: both failure diagnostics and
same-program recovery passed; all six CUDA sources and affine PTX again passed
the 14 compiler-supported architectures (84 source compilations, 98 assemblies).
Unknown architecture and missing-entrypoint rejection gates also passed. C probe
compilation used -Wall -Wextra -Werror; shell syntax and scoped diff checks passed.
Evidence: `docs/cuda-compiler-recovery-proof.txt`; ignored detailed log:
`target/cuda-verified/recovery-matrix.log`.

Inspection of production compile call sites found kernel caches assigned only
after successful compile/module/function loading. That observation is not driver
fault-injection evidence: no runtime CUDA cache recovery or physical NVIDIA
execution is claimed by the compiler-only gate. Full hardware support remains
incomplete and no process from this step remains pending.


### Retained raster guides and depth in animated ray frames (2026-10-02)

RasterRayAttachments owns the frame's guide textures, Depth32Float attachment,
placeholder texture and compiled ReconstructionGuidePass. into_resources moves
these and primary reconstruction storage out of a consumed RasterRayFrame.
GgxRasterRayResources::previous_attachments transfers them into the next frame:
matching dimensions retain attachments/pass, resize replaces the set, and a
foreign device returns DeviceMismatch before bindings are created. Previous
texture/buffer consumers must precede new writes on the same queue; cloned raw
handles obey this order too. This is explicit ordered reuse, not a completion
tracker or automatic pool.

animated_ray now retains this set between frames, clears it on device
initialization, and keeps temporal color/previous-depth history separate. Smoke
assertions compare depth, normal/roughness, diffuse and F0 Texture handles on
each frame: identical at equal size, different after resize. The existing
primary-storage assertions require observed reuse and replacement.

Apple M4 Max Metal passed 120 presentations with retained raster/primary
resources, skeletal BLAS replacement, positive finite direct GGX/reflections,
previous/current depth references, deferred resize handling and motion reset.
Command: `cargo run --locked --offline -p voxy_ray_probe --example animated_ray
-- --experimental --smoke`. Strict renderer library Clippy (--no-deps/-D
warnings) and scoped whitespace checks passed. No measured speedup or full-image
quality claim is made. Motion/depth pass outputs, lighting outputs/bindings and
acceleration-resource reuse remain further work.


### Readable and named runtime NVRTC diagnostics (2026-10-02)

CudaError Display now renders the structured NVRTC compile-failure variant as
its result code, compilation options and actual multiline log, instead of an
escaped CString Debug representation. The underlying CompileError remains
available through Error::source. Other error variants retain their existing
format. All six runtime compilation call sites now pass their static .cu filename
into compiler_options; cudarc uses CompileOptions.name to name the NVRTC program,
so compiler source diagnostics identify the kernel file. Kernel arithmetic, entry
names and precision/architecture settings are unchanged.

The focused CUDA-feature diagnostic test passed: error code, options, source
location/newlines and underlying error type were preserved. Strict CUDA-feature
Clippy for library and tests passed after the final filename changes. Scoped
format/diff checks passed. This validates error formatting and compilation; it
does not provide runtime NVIDIA failure-injection evidence. Physical CUDA execution
and full hardware support remain incomplete. No process from this step remains
pending.


### Retained deformation motion and expected-depth resources (2026-10-02)

RasterMotionPass::next_frame_reusing and PreviousDepthPass::next_frame_reusing
consume a previous pass, retain its pipeline and transfer its output texture
when dimensions match; resize allocates a replacement. They retain fresh camera
uniforms/geometry bindings and reject a foreign device before GPU binding use.
RasterMotionPass::next_frame now also checks device ownership. The new
PreviousDepthPass::next_frame shares compilation while allocating independent
output, for depth that must remain immutable as temporal history.

animated_ray retains motion and expected-previous-depth passes between frames.
Its current-depth pass shares the expected-depth pipeline but has a distinct
texture; it becomes previous-depth history only after presentation. That history
is never transferred into the current expected-depth producer. Old consumers
must precede transferred-texture writes on the same queue; this is explicit
ordered reuse, not automatic completion tracking. Geometry/camera uniform
buffers and current-depth history textures still allocate per prepared frame.

The final Metal smoke passed 120 presentations on Apple M4 Max, including
texture identity assertions for motion/expected depth at stable sizes and
replacement after resize, all retained raster/primary resources, skeletal BLAS,
positive finite GGX light/reflection samples, independent previous/current depth
references and reset motion. Command: `cargo run --locked --offline -p
voxy_ray_probe --example animated_ray -- --experimental --smoke`. An initial
build failed on a transient DropletLifecycle field mismatch in concurrently
changing physics/app sources; the field was present on inspection and rerun
passed without changing those modules. Renderer library Clippy
(--no-deps/-D warnings) and scoped whitespace checks passed. No speedup or
additional hardware/platform coverage is claimed.


### Retained wide HDR composition output (2026-10-02)

HdrCompositionPipeline::create_job_reusing transfers a previous composition's
output texture. Equal size/format retain storage; changed dimensions/format or
output aliasing either input allocate replacement storage. Composition jobs
retain device identity and foreign jobs return DeviceMismatch before binding
creation. Inputs still require the cached device and matching pixel coordinates.
Old consumers must precede new writes on the same queue; raw cloned handles
follow that order. Per-job bindings remain independent.

GgxRayLightingState::previous_combined and RasterRayFrame::into_reusable_resources
carry this unfiltered output between animated frames. The older into_resources
pair remains available. Temporal filtered history is stored separately and is
not reused as this unfiltered current-frame target.

animated_ray passed 120 presentations on Apple M4 Max Metal with HDR output
identity assertions at stable sizes and replacement after resize, alongside
retained depth/motion/raster/primary resources, GGX samples, analytic depth and
reset-motion checks. The radiance_accumulation diagnostic also passed: a reused
output retained its Texture handle and yielded 4; feeding it back as an input
forced distinct storage and yielded 7. Existing wide HDR saturation, accumulation
and reset/null cases passed. Commands: `cargo run --locked --offline -p
voxy_ray_probe --example animated_ray -- --experimental --smoke` and `cargo run
--locked --offline -p voxy_render --example radiance_accumulation`. Strict renderer
library Clippy (--no-deps/-D warnings) and scoped diff checks passed.

Direct/reflected lighting output textures and bindings still allocate per frame.
No speedup, full-image acceptance or new platform hardware coverage is claimed.


### Chunk logical payload accounting (2026-10-02)

ChunkData::payload_bytes now includes paletted block storage and every attached
block-data payload, using checked addition. Shared Arc payloads are charged per
entry. This is a logical charge, excluding allocator, BTreeMap-node and Arc
overhead; it is not process RSS or a global memory limit. Tests independently
check uniform, packed and direct storage, shared metadata and empty metadata.
All 16 voxy_world library tests and strict library/test Clippy passed; scoped
formatting and diff checks passed. Browser retained snapshots remain in RAM;
this API does not implement durable browser storage or resource admission.


### Retained direct GGX radiance output (2026-10-02)

GgxLightingPipeline::create_job_reusing accepts GgxLightingInputs and consumes
a previous SurfaceLightingJob. Device identity is checked before binding use.
Matching dimensions/format retain its output; resize/format changes or aliasing
reflectance inputs allocate replacement storage. Light/camera uniforms and
bindings remain fresh. Previous consumers must run before new writes on the
same queue; cloned output handles obey the same order. Existing constructors
retain fresh-output behavior.

GgxRayLightingState::previous_direct and the common frame's new
into_lighting_resources transfer direct output alongside raster/primary/HDR
resources. Older transfer methods remain available. animated_ray now compares
direct Texture handles each frame and requires reuse at stable dimensions and
replacement on resize, while retaining separate temporal history.

The GPU GGX verifier first encodes zero-intensity direct light, reuses the same
Texture for nonzero light, asserts identity, then compares all 16 pixels against
CPU direct-GGX references across roughness/seed and occluded/unoccluded cases.
The full Metal probe passed on Apple M4 Max. The final animated smoke also passed
120 presentations with direct/HDR/depth/motion/raster/primary resource reuse,
resize, positive GGX/reflection samples and analytic depth/reset-motion checks.
Strict renderer-library Clippy (--no-deps/-D warnings) and scoped diff checks
passed. Commands: `cargo run --locked --offline -p voxy_ray_probe -- --experimental
--backend metal` and `cargo run --locked --offline -p voxy_ray_probe --example
animated_ray -- --experimental --smoke`.

The first animated run hit the existing 60-second smoke limit without a frame
count; the diagnostic now reports presentations on timeout. Sequential rerun
passed without extending that limit. No cause or performance improvement is
claimed. An initial headless build failed on a concurrently changing face.rs
age_transition scope; current source inspection showed it corrected and rerun
passed without edits to that module. Reflection outputs, uniforms/bindings and
acceleration resource reuse remain work.


### Retained GGX reflection radiance and distance pair (2026-10-02)

GgxReflectionPipeline::create_job_reusing accepts GgxReflectionInputs and consumes
a previous SurfaceReflectionJob, retaining RGBA32Float radiance and R32Float hit
distance when size/format match. Resize/format changes replace the pair. Foreign
job devices are rejected before binding use; input aliasing also replaces storage.
New seed/camera/emission uniforms and bindings remain per-job. Old consumers must
precede new writes on the same queue; cloned handles follow that order. Output
selection and camera-uniform creation are isolated helpers.

GgxRayLightingState::previous_reflected transfers the pair through the common
frame. RayLightingResources groups primary/direct/reflected/combined jobs;
into_all_resources transfers that group alongside raster attachments. Older
transfer methods remain available. animated_ray retains the reflection pair,
compares both Texture handles at stable sizes and requires replacement on resize.
Temporal color/previous-depth history remains separate.

The GPU reflection verifier first encodes a different seed, then reuses the same
radiance/distance handles for the requested seed. All 16 values match independent
CPU GGX/triangle references, including null samples, across roughness 0.2/0.5/1
and seeds 0/17/31 in both occluded/unoccluded fixtures. Full Metal ray-probe passed
on Apple M4 Max. Final animated smoke passed 120 presentations with retained
reflection/direct/HDR/depth/motion/raster/primary resources, resize, skeletal BLAS,
positive finite samples, analytic depth and reset-motion checks. Strict renderer
library Clippy (--no-deps/-D warnings) passed after extracting helpers; scoped
diff checks passed. Commands: `cargo run --locked --offline -p voxy_ray_probe --
--experimental --backend metal` and `cargo run --locked --offline -p voxy_ray_probe
--example animated_ray -- --experimental --smoke`.

This retains all primary raster-ray lighting output textures in the animated
path, but does not eliminate uniform/binding/geometry allocation, temporal
output allocation or acceleration replacement. No timing improvement, full-image
quality acceptance or additional physical GPU/platform coverage is claimed.


### CPU-side animated-frame stage measurements (2026-10-02)

animated_ray smoke now records Instant wall-clock durations for presented
frames and reports mean, nearest-rank p50/p95/p99 and maximum. Prepare starts
after resize reconciliation and includes pose/geometry/frame preparation and
diagnostic-buffer allocation. Encode/submit/present measures render_custom,
including diagnostic copy encoding and synchronous surface/driver work. Explicit
GPU readback polling, event-loop pacing, device initialization and resize
reconciliation are outside these intervals. They are CPU-side wall-clock stages,
not GPU timestamp durations or pure CPU execution time.

Final debug-profile Metal run on Apple M4 Max passed 120 presentations, including
resize/reset, all retained resource assertions and the existing ray/HDR/depth
checks. Command: `cargo run --locked --offline -p voxy_ray_probe --example
animated_ray -- --experimental --smoke`. Milliseconds across those 120 samples:

| Stage | Mean | p50 | p95 | p99 | Maximum |
| --- | ---: | ---: | ---: | ---: | ---: |
| Prepare | 3.295 | 3.237 | 3.429 | 3.701 | 8.726 |
| Encode/submit/present | 2.144 | 2.030 | 2.272 | 7.188 | 7.438 |

A preceding run without p99 reporting observed prepare mean/p50/p95/max
3.756/3.680/3.919/10.464 and encode/submit/present
2.312/2.219/2.478/8.357. These are separate observational runs, not a controlled
before/after optimization comparison. They mix first-frame work and window
resolutions, and each smoke frame performs readback after these measurements.
No sustained FPS, speedup or GPU-time claim is supported. Release-profile,
fixed-resolution no-readback trials and GPU timestamp queries remain profiling
work. Formatting and scoped diff whitespace checks passed.


### Fixed-size no-readback animated profile (2026-10-02)

animated_ray --profile presents 30 warmup frames followed by exactly 300 measured
frames, then reports CPU-side stage statistics and exits. It rejects combination
with --smoke, dimension changes and premature completion. Pose progression uses
fixed per-frame phase increments. It does not allocate the diagnostic copy buffer,
encode pixel copies, or poll/read a GPU snapshot. Normal interactive runs also
no longer allocate that unused diagnostic buffer. Smoke retains its explicit
readback path and resize/reset correctness checks.

Apple M4 Max Metal runs passed at fixed physical dimensions 1280x960. Times in
ms for 300 samples after 30 warmup presentations:

| Build/stage | Mean | p50 | p95 | p99 | Maximum |
| --- | ---: | ---: | ---: | ---: | ---: |
| Debug prepare | 3.134 | 3.106 | 3.424 | 3.515 | 3.597 |
| Debug encode/submit/present | 4.285 | 4.193 | 6.319 | 6.911 | 7.194 |
| Release prepare | 0.696 | 0.695 | 0.757 | 0.803 | 0.833 |
| Release encode/submit/present | 6.106 | 6.022 | 8.652 | 8.977 | 9.259 |

Commands: `cargo run --locked --offline -p voxy_ray_probe --example animated_ray
-- --experimental --profile` and the same command with --release before --locked.
These are single observational runs, not controlled proof of a resource-reuse
speedup. Encode/submit/present includes synchronous driver/surface work and may
include backpressure. These intervals exclude initialization/event-loop pacing
and are not GPU timestamps, end-to-end frame time or sustained FPS evidence.

Release correctness also passed all 120 smoke presentations after this change,
including retained-output handles, resize/reset, positive GGX samples and CPU
depth references. Command: `cargo run --release --locked --offline -p
voxy_ray_probe --example animated_ray -- --experimental --smoke`. Direct invocation
with --experimental --smoke --profile failed with the expected mutual-exclusion
error before creating a window. Formatting and scoped whitespace checks passed.
GPU timestamp durations and repeated fixed-protocol trials remain profiling work.

### GPU timestamps on real compute passes (2026-10-02)

`animated_ray --experimental --gpu-profile` profiles the actual HDR composition
and clipped temporal compute passes: 30 warmup presentations, 300 measured
presentations at fixed 1280x960, distinct per-frame queries, aligned resolve slots,
and one final batch readback. No artificial boundary passes or dummy kernels
remain. `encode_with_timestamps` is optional on TemporalResolveFrame and
RadianceComposition; the existing encode API delegates with no timestamps.
RasterRayFrame/RayLightingFrame propagate optional composition timing without
changing pass ordering. Callers supply same-device timestamp queries, valid
indices and resolve/lifetime management.

Verified on M4 Max Metal in release: HDR composition p50 0.184 ms, p95 0.185 ms,
maximum 0.188 ms over 300 strictly positive samples. This is one observational
run with timestamp instrumentation, not FPS or a speedup claim. Temporal had
287/300 zero-elapsed samples; its timing is reported unavailable and no temporal
percentiles are published. Any negative interval or wholly zero stage rejects
the batch before GPU results are printed. The render-pass timing experiment also
returned nonmonotonic values and has been removed. The earlier independent
boundary-pass experiment was unsuccessful; only real compute-pass timestamps
are retained. Remaining work: resolve the Metal temporal counter limitation and
instrument reconstruction, direct GGX, reflections and raster passes.

Release smoke passed all 120 presentations after these API changes, including
resize/resource replacement, animated BLAS, direct/reflected HDR, motion, analytic
current/previous depths and history reset. Scoped formatting and whitespace checks
passed.

Strict renderer-library Clippy was attempted and failed on six diagnostics in the
concurrently developed `lod_witness.rs` (doc-markdown and float-to-u32 casts).
No diagnostics referenced the timestamp API files; this is not a passing whole
library Clippy gate. The unrelated LOD implementation was preserved.

### Direct GGX and reflection GPU pass timing (2026-10-02)

`SurfaceLightingJob` and `SurfaceReflectionJob` now expose optional actual-pass
`encode_with_timestamps`, preserving their original untimed `encode` API.
`RayLightingFrame` and `RasterRayFrame` propagate three optional descriptors in
explicit direct-light/reflection/composition order. Query device/index/lifetime
requirements remain caller-owned; no submission, history commit or CPU wait is
added by the encoder methods. The animated profile uses eight distinct queries
per presentation and resolves a 64-byte payload into each aligned 256-byte slot.

Release profile completed 330 presentations (30 warmup + 300 measured), 1280x960,
on Apple M4 Max Metal. All 300 direct/reflection/HDR intervals were positive and
monotonic within their own real compute passes:

| Actual GPU pass | p50 ms | p95 ms | max ms |
| --- | ---: | ---: | ---: |
| Direct GGX | 0.747 | 0.761 | 0.921 |
| GGX reflection | 0.817 | 0.832 | 0.879 |
| HDR composition | 0.184 | 0.185 | 0.188 |

Temporal returned 290/300 zero-elapsed intervals and is explicitly unavailable;
no temporal percentiles are published. These are one-run timestamp observations,
not whole-frame GPU time, end-to-end FPS, or an optimization speedup. Raster,
primary reconstruction, motion/depth, tone mapping and BLAS costs remain outside
the reported intervals. No artificial boundary passes remain.

Validation after the direct/reflection timestamp API change: release animated
smoke passed all 120 presentations; `cargo clippy --locked --offline -p voxy_render
--lib -- -D warnings` passed; scoped formatting and whitespace checks passed.
Earlier unrelated LOD Clippy errors were no longer present in current source.

### Animated skeletal browser scene (2026-10-02)

The existing Rust/WASM shell now exposes `start_animated_scene` and a presented-pose
counter. `?backend=webgpu&animated=1` (or `backend=webgl`) uses the same native-demo
quad topology, two-joint UNORM16 weights, UVs and sinusoidal upper-joint X/Z
translation. `SkinnedMotionHistory` prepares each deformed mesh and commits its
captured pose only after queue submission/presentation; surface-acquisition skips
return before preparing/committing poses. This mode disables the old rigid-body
rotation. It renders the existing texture and 2D overlay through SceneRenderer,
with the shared resize/device-loss/pause lifecycle. Incompatible voxel/gravity/
water-check query combinations fail startup. It performs CPU skinning and fresh
mesh upload per presented frame; GPU skinning and retained mutable geometry are
still work ahead.

WASM target check and development build succeeded using project-local
wasm-bindgen 0.2.127. Existing startup-failure, pause-lifecycle and device-loss Node
checks passed. Real in-app browser verification on the running localhost:8793
service showed `BrowserWebGpu`, 552 frames/552 presented poses, then manual pause
holding animation time at 10.7749 while frames continued, and resumed time 22.7078.
WebGL2 separately showed `Gl`, 775 frames/775 poses. Both screenshots were inspected:
textured deformed quad and blue 2D overlay visible. WebGPU error/warning logs were
empty. Proofs: `docs/browser-animated-skeleton-proof.jpg` and
`docs/browser-animated-webgl-proof.jpg`. WASM/cache version `animated-skeleton-v1`.

This is the skeletal raster portion of the representative scene, not a full port
of the native ray/HDR demo. Remaining: browser material lighting, shadow raster
path, supported reflection fallback, HDR intermediate/tone mapping, reset/exposure
controls and GPU/retained-buffer deformation. Hardware ray queries and DLSS are
not provided by this browser backend.

Final-source follow-up: camera updating and presented-pose commit were extracted
into focused helpers; the rebuilt final WASM ran 1944 presentations/1944 committed
poses. A temporary real-browser 800x600 resize retained rendering and its viewport
was restored. Existing lifecycle Node checks passed again. The final WebGPU proof
screenshot was refreshed after the rebuild.

Final `cargo clippy --locked --offline -p voxy_web --target
wasm32-unknown-unknown --lib --no-deps -- -D warnings` passed.

### Browser animated HDR intermediate and exposure (2026-10-02)

The animated WebGPU mode now retains a separate RGBA16Float SceneRenderer,
ProcessedColorTarget and shared TextureBlit tone mapper. The animated textured
mesh renders into the floating-point target; SDR tone mapping follows, and the
2D overlay renders afterward through the surface renderer. Display exposure
updates reuse the tone-mapping pipeline. Nonfinite/nonpositive values are rejected
before mutation. The HDR target is replaced only when canvas dimensions change;
ordinary frames retain the renderer, target and display pipeline. WebGL2 keeps
its existing LDR raster path and hides the exposure control.

The demo's unlit vertex radiance is [3.2,1.6,0.8] in HDR mode versus the existing
[0.8,0.4,0.2] in LDR mode; this exercises HDR gain/tone mapping, not a lighting
model. Actual PBR lights, shadows and reflection fallback remain required.
HDR here denotes the intermediate rendering pipeline and SDR output, not an
HDR-display presentation or browser HDR metadata implementation.

WASM target check, final crate-only strict WASM Clippy, development WASM build,
and existing startup/pause/device-loss Node tests passed. Real WebGPU verification
showed HDR=true and changing Exposure 1 -> 0.25 -> 4 while animation time held at
17.0499: inspected screenshots show darker/brighter 3D radiance with stable 2D
colors. Canvas resized from 2560x1440 to 800x600 with continued presentations and
no GPU warning/error logs. Viewport was restored, exposure returned to 1 and
animation resumed. Proofs: docs/browser-animated-hdr-exposure1.jpg,
docs/browser-animated-hdr-exposure025.jpg, docs/browser-animated-hdr-exposure4.jpg.
Cache version: animated-hdr-v1. Remaining includes direct lights/shadow raster,
reflection fallback, GPU deformation, reset controls and pixel readback assertions.

Final-package WebGL2 fallback also ran 3209 presentations with HDR=false and
exposure controls hidden; its raster scene remained available.

### Browser textured GGX point light and retained mesh updates (2026-10-02)

The shared renderer exports TEXTURED_POINT_LIGHT_SHADER using the existing
SceneRenderer vertex/transform/texture ABI. The shader evaluates textured GGX
specular, energy-reduced Lambertian diffuse, inverse-square point-light intensity
and a small constant ambient term in linear HDR. This initial shader fixes
roughness=0.35 and metallic=0.5; world transforms must be rigid or uniformly scaled
for its normal transformation. Zero-length normal/view/half vectors are guarded.
It does not implement light visibility or shadows.

Animated WebGPU initialization now awaits validated atomic shader reload into the
retained HDR renderer. Per-frame uniforms provide actual camera position, identity
world and point light [1.5,1,2.5], default intensity 35. Albedo returns to
[0.8,0.4,0.2] instead of the earlier artificial unlit HDR gain. New Light control
selects intensity 0/35/70; API rejects unavailable HDR, nonfinite/out-of-[0,100]
intensity before mutation. The 2D overlay remains outside lighting/tone mapping.
CPU-skinned poses now update the existing SceneGeometry buffers and normal cache
via its queue-ordered update API, rather than reallocating geometry each frame.
Pose history still commits only after presentation.

Development WASM build and final strict crate-only WASM Clippy passed. Existing
startup/pause/device-loss Node checks passed. In the real WebGPU browser, validated
shader ABI/reload succeeded with empty warning/error logs. At paused time 13.5332,
inspected light35, light0 and light70 screenshots showed specular/illumination,
ambient-only output, and stronger illumination respectively; the 2D overlay was
stable. Controls restored to light35 and animation resumed. Proofs:
docs/browser-animated-ggx-light35.jpg, docs/browser-animated-ggx-light0.jpg,
docs/browser-animated-ggx-light70.jpg. Cache version animated-ggx-v1.

Remaining: shadow map with an actual receiver, configurable material parameters,
reflection fallback, GPU skinning and quantitative pixel/reference verification.
No shadow or complete native-ray-demo parity is claimed by this step.

Final WebGL2 package verification: 5476 frames/5476 presented poses using retained
geometry updates, HDR=false, light controls hidden, and empty warning/error logs.

Shared WGSL inventory passed 33 files, 38 variants, 61 entrypoints, zero failures,
including scene_point_light.wgsl. This is source/Naga validation, separate from the
real WebGPU shader execution above; no new Linux/native GPU run is claimed.

### Shared opaque shadow-map depth pass (2026-10-02)

New ShadowMap/ShadowDraw API creates a retained Depth32Float render/sample/copy
texture, conventional clear=1/Less depth pipeline and immutable per-draw light
clip-from-model uniform snapshots. It renders the existing SceneGeometry indexed
buffers, so CPU-skinned retained updates drive the shadow caster too. Geometry
now retains its actual owner device across ordinary uploads and shared-buffer LOD
variants. Foreign geometry is rejected in prepare, foreign draws are rejected
before opening a pass, and nonfinite light matrices are rejected before allocation.
No submission, CPU wait, presentation or pose-history commit is performed by this
API. Producers and map consumers must be ordered on the same queue; geometry
updates follow submission of earlier encoded draws.

`cargo run --locked --offline -p voxy_render --example shadow_depth` passed on
Apple M4 Max Metal. Actual GPU depth readback checked 192 pixels: near-left caster
0.25 versus full far caster 0.75, an empty map cleared to 1, then a retained caster
Z update yielding 0.5 versus 0.75 with reversed draw order. Zero dimensions,
nonfinite transforms and foreign-device geometry/draws were also rejected.
Final strict renderer-library Clippy passed after the change.

This establishes the raster depth producer, not visible shadow shading. It treats
supplied triangles as opaque, ignores texture alpha/transmission, uses no depth
bias or PCF, and does not yet attach a receiver or compare the map in the browser
GGX shader. Next work: light frustum/receiver and shadow visibility sampling, then
animated browser verification and hardware/software Linux acceptance.

Final WASM compatibility check (`cargo check --locked --offline -p voxy_web
--target wasm32-unknown-unknown`) and scoped whitespace checks passed. This is
compile compatibility; the new shadow pass has not yet executed in a browser.

### Visible raster shadows on the animated browser receiver (2026-10-02)

SceneRenderer now optionally owns a third explicit binding group for a same-device
ShadowMap plus light-from-world/bias/enabled uniforms. The new
`enable_shadowed_point_light` installs validated shadowed GGX pipelines atomically,
restoring previous shadow bindings if shader validation fails; ordinary renderers
keep their existing two-group ABI. All scene draw paths bind the optional group,
including transparent/overlay/MSAA paths. `update_shadow_settings` validates finite
matrix and bias in [0,1] before writing; caller queue/resource ordering is explicit.
The visibility function projects world position to conventional light clip depth,
checks positive W/frustum bounds before texture indexing, and performs a nearest
Depth32Float comparison with constant bias. Outside-frustum pixels are unshadowed;
only direct diffuse/specular is attenuated, ambient remains.

Animated WebGPU HDR scene retains a 1024x1024 map, produces it from the current
CPU-skinned/updated caster buffers, then shades caster and a separate textured
receiver plane at world Z=-0.5 before tone mapping and the 2D layer. A single
90-degree light frustum at [1.5,1,2.5] targets the origin; bias=0.001. Shadows On/Off
changes only uniforms; shadow resources and pipelines remain retained. It is a
hard projective shadow within this frustum, not six-face omnidirectional point-light
shadows, PCF, alpha-tested/transmission shadows, or a complete shadow-quality system.

Real browser inspection at paused time 21.0239 showed a dark displaced silhouette
on the receiver with Shadows On, disappearing with Shadows Off while the caster,
lighting and 2D overlay remained. Screenshots inspected/saved:
docs/browser-animated-shadow-on.jpg and docs/browser-animated-shadow-off.jpg.
Animation resumed with shadows enabled. A real 800x600 resize continued rendering
with empty warning/error logs; viewport reset afterward. Cache animated-shadow-v1.

Numeric Metal GPU verification (`cargo run --locked --offline -p voxy_render
--example shadow_visibility`) passed 64 receiver pixels / 192 RGB pairs: shadowed
left pixels equal ambient 0.01 within RGBA16Float tolerance; unoccluded right pixels
match On/Off, and unshadowed direct light is positive. Invalid bias is rejected.
Final strict renderer and crate-only WASM Clippy passed, development WASM build and
existing startup/pause/device-loss Node tests passed. Native scene_smoke
--shader-test --msaa also passed shader replacement/cache/error rejection and
clear/indexed mesh/depth/texture/overlay pixel assertions. Scoped whitespace checks
passed. RGBA32Float was rejected for blendability by the initial test and replaced
with the production-compatible RGBA16Float fixture; no optional float32 blend
feature is required.

Remaining includes filtered/omnidirectional shadows, material controls, reflection
fallback, GPU deformation, browser numeric readback tests and Linux GPU acceptance.

Final WebGL2 fallback verification: 8991 presentations, HDR=false, shadow controls
hidden, no warning/error logs. WebGL2 shadow parity is not claimed.

### PCF shadow filtering: shared renderer and browser verification

`ShadowSettings.filter` selects `ShadowFilter::Hard`, `Pcf3x3`, or `Pcf5x5`. The shared fragment shader averages respectively 1, 9, or 25 depth comparisons. Samples outside the shadow map count as lit; a receiver outside the light frustum remains lit. The existing 80-byte uniform and depth texture binding remain unchanged; switching the filter updates settings without replacing the pipeline or map. Existing Rust settings literals must now supply the filter field.

The Metal `shadow_visibility` example passed on Apple M4 Max: 64 receiver pixels, 192 Hard On/Off RGB pairs, and 384 filtered RGB values compared against an independent CPU tap-count reference, including map borders. Filtered half-float outputs use tolerance max(abs(expected) * 0.0015, 0.0001). Invalid settings are rejected before updates. Strict library Clippy for voxy_render and wasm32 voxy_web passed; the dev WASM package built with local wasm-bindgen 0.2.127.

The browser animated HDR scene defaults to 3x3 PCF and exposes Hard/3x3/5x5 controls. Real WebGPU UI verification switched Hard and 5x5 at the same paused animation time 11.092600000000067, then restored 3x3 and resumed animation. No error/warning console entries were recorded. Screenshots: browser-animated-pcf-hard.jpg and browser-animated-pcf5.jpg. Shadows and Shadow filter have separate accessible names.

This is a fixed texel-space kernel, not physically varying area-light penumbra. The existing single light frustum remains; omnidirectional cube shadow maps and browser reflections remain future work. Native Metal ray tracing reflection/history work is separate from this browser raster shadow path.

### Shared GGX material parameters

SceneTransform::update_pbr_material(queue, roughness, metallic) sets per-draw parameters in the existing authored.yzw uniform fields without new bindings or pipeline creation. Both inputs must be finite and within [0,1]; rejection happens before GPU writes. Call after update_scene_material, which resets the authored block. The point-light vertex shader passes the parameters flat to the fragment shader; absent the explicit marker, the legacy roughness 0.35 and metallic 0.5 remain. GGX evaluates roughness with a 0.04 numerical floor to avoid singularities at zero. This prepares configurable materials for reflections; it does not itself implement reflected scene radiance.

Metal GPU verification passed on Apple M4 Max: explicit default PBR parameters produced byte-identical readback to the legacy defaults; changing to roughness 1 / metallic 0 changed the rendered image. NaN, infinity and out-of-range updates were rejected and preserved the default image. Existing Hard On/Off and 3x3/5x5 PCF numerical checks also passed. Browser WASM packaging/UI for these new material parameters is not yet updated.

Strict renderer library Clippy did not pass in this concurrent worktree: 12 diagnostics were reported in fluid_screen.rs (useless conversion, default trait access, numeric casts and unused self). No diagnostic was reported for the new PBR setter. Native compilation and Metal readback checks passed; the strict lint gate remains failing and is not claimed green.

### Browser animated mesh material controls

The HDR animated mesh now exposes WebEngine::set_animated_material(roughness, metallic), with finite [0,1] validation before retained state changes. Values reside in HdrScene and are applied to the animated mesh after update_scene_material on every camera update. The receiver keeps its own default material. The UI provides independent roughness and metallic selectors, visible only for the HDR path, and records the selected values in canvas dataset. Startup failure, pause lifecycle and device-loss cleanup Node checks passed after this wiring change. This adds material controls; reflected scene radiance and temporal accumulation remain outstanding.

The dev WASM package build and strict voxy_web wasm32 library Clippy passed. Real WebGPU UI checks compared roughness 0.1 / metallic 1 against roughness 1 / metallic 0 at the same paused time 17.89119999999998 with light 35, exposure 1 and 3x3 PCF. Screenshots browser-material-smooth.jpg and browser-material-rough.jpg show changed mesh illumination and unchanged receiver/shadow geometry; console error/warning log was empty. Defaults 0.35 / 0.5 were restored and animation resumed. Node startup/pause/device-loss checks passed; scoped whitespace checks passed. Native renderer strict Clippy status from the previous section remains separate.

### Linux HDR shadow/material acceptance runner

`tools/linux/shadow-material-smoke.sh` runs the same numeric shadow_visibility example with WGPU_BACKEND=vulkan in voxy-linux-smoke. The source/cache mounts are read-only, networking is disabled during the acceptance run, and build outputs reside in target/linux-docker. The Dockerfile now includes pkg-config and libasound2-dev because renderer example dev-dependencies include the native audio module. The local existing image was extended with these same packages; this does not install packages on macOS. The test covers Hard/PCF readback and PBR material parameter parity/response, not ray-query support or hardware Linux GPU performance.

Linux Vulkan acceptance passed on llvmpipe (LLVM 19.1.7, Mesa 25.0.7-2+deb13u1): 64 receiver pixels, 192 Hard On/Off RGB checks, 384 PCF reference values, PBR default parity/material response/invalid-value preservation. Initial output at pixel (3,3), red was NaN in both On and Off. Clamping GGX dot-product cosines to their physical [0,1] range removed the NaN with unchanged tests/tolerances; this guards roundoff above one before Schlick pow(1-vh,5). This is a software Vulkan proof, not hardware GPU/ray-tracing acceptance. The container build still reports unrelated deprecation/future-incompatibility warnings.

After the cosine-domain fix, the same full numeric receiver/material probe passed again on Apple M4 Max Metal. The dev WASM package rebuilt successfully; browser cache keys advanced to animated-material-v2 so subsequent loads receive the corrected shader.

### Native animated Metal ray material switching

animated_ray accepts --roughness=<finite 0..1> and --metallic=<finite 0..1> before GPU/window creation, defaulting to 0.35/0.5. G cycles roughness 0.1/0.35/1; M cycles metallic 0/0.5/1. The window title reports current values. ReconstructionGuideMesh provides the resulting normal/roughness, diffuse and F0 maps consumed by the same direct GGX and GGX reflection ray passes. Runtime changes discard the temporal radiance history without invalidating skeletal motion history.

The smoke now changes material after 90 presentations and requires history reset on presentation 91. On every temporal reset it checks HDR center radiance against current direct + reflected radiance with relative/absolute tolerance 1e-5, so an old-material accumulation cannot silently pass. Two full 120-presentation Metal M4 Max runs passed: 0.1/1 to 1/0, and 1/0 to 0.1/1, preserving existing resize, retained resource, BLAS deformation, depth/motion and finite positive reflection checks. CLI --roughness=NaN failed before launching the example. These checks cover native ray material changes; browser reflected radiance remains outstanding.

Strict Clippy for voxy_ray_probe --example animated_ray --no-deps passed after targeted example lint cleanup. The final forward-switch smoke was repeated and passed. Dependency voxy_app dead-code warnings remain outside this example lint gate; this is not a whole-workspace clean-lint claim.

### Retained native ray tone mapper

animated_ray now constructs the tone mapping TextureBlit once with the scene's device/format, retains it across frames and resize, and uses with_exposure only when the exposure bit pattern changes. Device initialization replaces it with the new device's pipeline. Normal frame preparation no longer calls tone_mapped or recompiles the display shader. Exposure snapshots retain the pipeline/layout/sampler and allocate an independent parameter buffer through the existing API; per-pass bind groups are still created by encode. The first retained-display Metal smoke passed 120 frames including resize/material switching. Smoke now also sets exposure to 2 after presentation 105 to exercise the retained snapshot update. No FPS gain is claimed from this structural change.

The final retained tone mapper smoke, including exposure 2 after frame 105, passed all 120 Metal presentations with the existing numeric HDR/history/depth/motion/reflection assertions. Strict animated_ray example Clippy passed; dependency voxy_app dead-code warnings remain separate. Scoped whitespace checks passed.

### Display pipeline device ownership

TextureBlit retains its owning Device. with_exposure and with_sdr_white_nits reject a foreign device before allocating parameter buffers; with_auto_exposure rejects an AutoExposureFrame owned by another Device. AutoExposureFrame exposes only a crate-private owner check. encode_checked returns SceneError::DeviceMismatch before bind-group allocation/render-pass encoding when its device argument is foreign. It still requires source/target views and encoder to belong to the supplied device; those opaque resources are validated by wgpu. Legacy encode delegates to the checked path and panics on device-owner misuse. Browser animated HDR and native animated_ray use the recoverable checked path.

The Metal auto_exposure numeric fixture passed existing metering/adaptation/SDR/scRGB/PQ/stereo checks plus two-device rejection. Same-device exposure/PQ snapshots are accepted, foreign exposure/PQ/auto-exposure snapshots are rejected, and an encoder remains submit-able after an encode_checked device mismatch. Strict wasm32 voxy_web library Clippy passed.

Linux llvmpipe Vulkan passed the same auto exposure/display ownership fixture and existing shadow/PBR checks. The Linux shadow-material runner now executes both fixtures. Dev WASM rebuilt successfully and cache keys advanced to animated-material-v3. Node startup/pause/device-loss checks and scoped whitespace checks passed. This validates owner rejection, not real device-loss recovery or hardware Linux ray tracing.

### Browser animated HDR recovery with retained settings

Reload GPU now records current animated HDR settings in its recovery URL: exposure, lightIntensity, shadows, shadowFilter, roughness and metallic. It removes deviceLossCheck to avoid repeated validation destruction. Animated startup restores only values represented by the controls, rejects unsupported values through startup cleanup, and applies restored settings before starting RAF. Existing stop ownership, disabled controls, first diagnostic and no submissions after failure remain intact.

Node animated-recovery checks actual HTML restoration code: all setters receive restored values, absent parameters leave defaults, and unsupported numeric/toggle/kernel URL values throw. device-loss checks confirm current dataset replaces stale URL material settings and destruction injection is removed. Startup failure and pause lifecycle tests also passed.

Real WebGPU acceptance deliberately destroyed the device after 30 presented frames with exposure 4, light 70, shadows On, PCF 5x5, roughness 1 and metallic 0. The failure diagnostic and disabled controls appeared; clicking Reload GPU recreated the scene with all six settings restored and controls enabled. The new owner presented 1272 frames by observation, with an empty error/warning console log. Screenshot: browser-gpu-recovery.jpg. This is page reload recovery, not in-place recovery: animation pose, camera and arbitrary game/world state restart. Full native/mobile device-loss recovery remains unproven.

### Browser pose/time recovery

Animated recovery URLs now also snapshot the last presented animation time and manual pause. Startup restores time before starting RAF and rejects empty, negative, nonfinite or f32-overflow time values; pause must be 0/1. Visibility pause is not persisted as user intent. Recovery remains a page reload and starts with fresh GPU resources/motion history.

Node restoration/loss/pause/startup tests passed. Real WebGPU destruction at frame 30 on paused time 17.8912 produced a recovery link with animationTime=17.8912 and userPaused=1. Clicking Reload GPU recreated the mesh/material/HDR scene at exactly that time; the observed new owner presented 1272 frames while remaining paused, with empty error/warning logs. Screenshot browser-pose-recovery.jpg. Space resumed the animation: time advanced to 17.908800000000003 and userPaused became false. This supersedes the prior limitation that animated pose restarts; arbitrary camera/game/world state and temporal image history are still not persisted.

### PBR state independent of world/light updates

SceneTransform::update_scene_material now writes only world/tint/light and authored.x, preserving authored.yzw PBR settings. update_pbr_material can therefore be called before or after world/light updates. Uninitialized PBR markers still select shader defaults. This supersedes earlier instructions that update_scene_material resets PBR parameters or requires PBR updates afterward.

The numeric shadow_visibility fixture renders a nondefault roughness 1 / metallic 0 material, updates identical world/tint/light, and requires byte-identical GPU readback afterward. Linux llvmpipe Vulkan passed this new persistence check together with default parity/material response/invalid-update checks, Hard/PCF references and the separate auto-exposure/device-ownership fixture.

Metal M4 Max passed the same PBR persistence and shadow checks. Dev WASM rebuilt successfully, with browser cache keys advanced to animated-material-v4. Scoped whitespace checks passed.

### Planar reflection capture camera API

SceneCamera::reflected(plane_point, plane_normal) builds a right-handed capture camera by reflecting eye, target and up around a world-space plane. Projection parameters remain intact; the plane normal need not be unit length but must be finite/normalizable. Source and output cameras are validated. The API allocates no GPU resources and does not clip geometry behind the reflector. Capture UVs must use the reflected view-projection rather than assuming an unchanged horizontal axis. This is the camera prerequisite for a planar reflection pass; browser reflected scene radiance is not implemented by this API alone.

Focused native camera tests passed: 6 tests, including horizontal/oblique plane reflection, signed-distance reversal, tangential preservation, double-reflection identity, normal sign/scaling invariance, projected horizontal parity, orthographic projection preservation and invalid-plane rejection. These are CPU camera invariants, not a GPU/browser reflection rendering acceptance. Scoped whitespace checks passed.

### Planar HDR capture and sampled surface composition

PlanarReflectionCapture retains an RGBA16Float color target and Depth32Float attachment, renders reflected-camera SceneDraws with SceneRenderer, and supports validated shader reload. Unchanged-size resize retains attachments; invalid dimensions preserve them. Geometry/texture owner rejection happens before beginning a pass. Transform/encoder ownership remains subject to wgpu validation. Caller excludes the reflector and clips geometry behind its plane.

material_binding shares the current capture texture with another same-device SceneRenderer, without readback or re-upload. Bindings retain their captured texture across resize; recreate the binding after a successful resize. Sampling the capture while rendering into that same image is prohibited. SceneRenderer internal bind_image now validates its renderer device before creating bindings.

The planar_capture GPU fixture encodes capture then a separate textured surface in one encoder. It checks 384 surface RGB values for original and reflected cameras, including linear HDR values 4 and 2, attachment retention/replacement, old binding retention/new binding identity, and foreign geometry/texture/renderer rejection. Linux llvmpipe LLVM 19.1.7 / Mesa 25.0.7 Vulkan passed the HDR surface fixture plus the existing shadow/PBR and exposure/ownership fixtures. This is software Vulkan evidence; no hardware Linux ray tracing is claimed. Projective reflector coordinates, clipping, rough reflection filtering, temporal accumulation and browser integration remain outstanding.

Apple M4 Max Metal passed the same final HDR sampled-surface fixture (384 RGB references). Targeted formatting and scoped whitespace checks passed. Container compilation still reports unrelated deprecation/future-incompatibility warnings; no whole-workspace strict lint acceptance is claimed.

### Projective planar surface shader

PLANAR_REFLECTION_SURFACE_SHADER computes capture clip coordinates per vertex, interpolates them perspective-correctly, divides by capture W in the fragment stage, and maps NDC to texture coordinates with the framebuffer Y inversion. Samples outside the capture frustum or behind its camera are discarded. Mesh UVs are ignored. Explicit level-zero sampling avoids implicit-derivative requirements inside rejection branches. This preserves linear HDR and multiplies by surface vertex tint; it is not a roughness/Fresnel BRDF.

SceneTransform::update_planar_projection supplies display MVP and capture MVP, both including the surface world transform. The capture MVP occupies the existing previous-MVP ABI slot; this surface transform cannot simultaneously supply motion history, so a motion-vector pass needs a separate transform. Both matrices are validated before writing.

The final Linux llvmpipe Vulkan fixture passed 384 HDR surface references with all authored surface UVs set to zero and a rejected NaN update before rendering. The reflected-camera projection compensates the capture image horizontal parity, preserving world-space left/right surface colors. Existing resize/owner, shadow/PBR and exposure checks also passed. Reflector-plane geometry clipping, perspective/oblique GPU acceptance, roughness/Fresnel composition, temporal history and browser wiring still remain.

Final Metal M4 Max projective fixture passed the same 384 HDR references with degenerate authored UVs and rejected NaN update. Scoped whitespace checks passed. The final shader is verified on orthographic captures; perspective/oblique GPU acceptance is still pending.

### World-plane clipped reflection capture

PLANAR_REFLECTION_CLIP_SHADER clips the negative half-space of a world-space plane in the fragment stage, preserving intersecting triangle portions and conventional depth writes only for retained fragments. SceneTransform::update_planar_clip uploads world/tint and a normalized plane (normal XYZ, -dot(normal,point)), rejecting invalid values before mutation. The plane occupies the light ABI slot, so this shader uses a dedicated capture transform and is currently unlit; it does not combine clipping with point-light PBR. Callers still exclude the reflector itself. Level-zero sampling currently omits material mip selection.

The planar_capture fixture now checks 576 projective HDR surface RGB values across original capture, mirrored capture and world-plane clipped mirrored capture. Plane X=0.25 crosses the quad triangles after the world scale; expected readback removes the red half and the first green pixel column while preserving remaining HDR green=2. A rejected zero-normal update must preserve the valid clipping state. Apple M4 Max Metal and Linux llvmpipe Vulkan both passed the final crossing-triangle fixture and existing retention/ownership checks; Linux shadow/PBR/exposure regression fixtures also passed. Perspective/oblique GPU acceptance, clipped PBR capture, roughness/Fresnel/temporal composition and browser integration remain pending.

### Clipped point-light PBR reflection capture

planar_reflection_pbr_clip_shader derives from the common point-light GGX shader and carries a world-plane signed distance from vertex to fragment. SceneTransform::update_pbr_capture_plane normalizes the plane and stores it in the first previous-MVP column, preserving material world/tint/light, reflected view position and roughness/metallic. MVP/motion updates overwrite that slot: reapply the plane afterward and use a dedicated capture transform without motion history. Unlike the unlit clipped shader, this material preserves the point-light uniform. Uniform visibility remains vertex-only.

The GPU fixture adds a baseline ordinary PBR capture at roughness 0.6 / metallic 0.2 with point-light intensity 35 and reflected camera eye, followed by the clipped PBR variant. 192 RGB checks require zero outside the retained half-space and exact unchanged baseline radiance inside it, alongside finite/nonnegative radiance and invalid-plane state preservation. Linux llvmpipe Vulkan passed this parity fixture plus 576 projective/unlit HDR references and existing shadow/PBR/exposure regressions. Initial direct fragment access to the transform uniform failed pipeline validation; the final interpolated-distance implementation passed. Shadowed capture lighting, roughness/Fresnel surface composition, temporal history, perspective/oblique GPU validation and browser integration remain pending.

Apple M4 Max Metal passed the same final clipped PBR parity fixture. Targeted formatting and scoped whitespace checks passed; container dependency warnings remain separate.

### Browser animated HDR planar reflection integration

HdrScene now retains a planar HDR capture, separate reflected-camera PBR transform, projective surface renderer/transform/material and planar surface geometry. The capture shares the current deformed mesh and texture, clips below world Z=-0.5 and excludes the receiver itself. Point-light intensity, roughness/metallic and reflected eye update alongside the main scene. Capture runs before the main shadowed scene; the projective surface at Z=-0.49 blends at constant alpha 0.35 over the lit receiver with existing depth testing and no depth writes, preserving foreground occlusion. Surface binding is replaced only after capture resize. Shader/material resources are retained across ordinary frames. Shared camera input now passes SceneCamera rather than reconstructing a reflected camera from an opaque VP.

Dev WASM rebuilt with project-local wasm-bindgen 0.2.127; JS/WASM cache keys are animated-reflection-v1. Strict wasm32 voxy_web library Clippy and Node restoration/device-loss/startup/pause checks passed. The helper plane_mesh shares identical plane mesh construction without relaxing Clippy.

Real WebGPU presented 1066 paused frames at restored time 17.8912 with no warning/error logs. Light 70 / roughness 0.1 / metallic 1 controls applied; both material screenshots were viewed. Resize to 1024x768 updated canvas and continued to 13757 presentations; restoring default viewport and resuming advanced animation time from 17.9079 to 42.1786 at 21136 presentations, with empty warning/error logs. Final rebuilt package reloaded and presented 1410 paused WebGPU frames with empty logs; final screenshot browser-planar-reflection.jpg was viewed and saved. The same final package WebGL2 fallback presented 1398 animated frames with HDR=false and empty logs, then its temporary tab was closed.

This is an initial perspective browser integration, not numeric perspective/oblique reflection parity or a physical reflective BRDF acceptance. Capture has point-light PBR without shadow sampling; surface reflection weight is constant, without Fresnel, rough filtering, temporal accumulation or recursion. WebGL2 remains the LDR raster fallback and does not claim reflection parity. Browser camera/window state recovery beyond existing pose/settings remains pending.

### Numeric perspective and oblique planar capture acceptance

planar_capture now adds perspective projection with vertical FOV 2*atan(1/3), camera distance 3 and an oblique variant eye X=0.1 looking at the origin, giving different capture W across the surface. Each adds ordinary capture, mirrored capture and clipped mirrored capture cases. The textured surface uses degenerate authored UVs and its own display projection, so retained world-space red/green placement depends on projective capture coordinates and perspective interpolation. The fixture reloads the default shader explicitly for unclipped unlit cases to prevent previous case state leakage.

Final Apple M4 Max Metal and Linux llvmpipe Vulkan runs passed 1728 analytic HDR RGB references across orthographic/perspective/oblique cameras, plus existing 192 clipped PBR radiance parity references and retention/ownership checks. Linux shadow/PBR and exposure regressions also passed. Initial Vulkan perspective interpolation produced 3.9980469 for expected 4, exactly the preceding half-float representation; the analytic HDR checks now allow at most one half-float ULP, require finite/nonnegative values and retain the same expected colors/clipping boundaries. PBR baseline parity tolerance remains unchanged at 1e-6. Targeted formatting/whitespace checks passed. This covers a modest oblique view with planar geometry, not arbitrary extreme-angle/frustum boundaries, roughness/Fresnel filtering, temporal history or hardware Linux acceptance.

### Scalar Fresnel projective reflection material

PLANAR_REFLECTION_FRESNEL_SHADER extends the projective surface ABI with world/normal, flat eye and scalar F0. Schlick Fresnel controls output blend alpha while linear incoming HDR RGB remains unpremultiplied for the existing alpha blend pipeline. F0 is 0.04 for a dielectric, mixed with average clamped base tint by metallic weight for neutral metallic reflectors. Set world/base tint via update_scene_material, eye via update_view_position, PBR metallic via update_pbr_material and both MVPs via update_planar_projection. Normals require rigid/uniform world transforms. The capture MVP still occupies the motion history slot. This scalar alpha composition does not support physically correct colored-metal RGB Fresnel or roughness filtering.

The GPU planar_capture fixture adds 576 analytic RGB references for dielectric front view, grazing view (eye [3,0,0.1]) and neutral metal F0=0.65. CPU Schlick evaluates each pixel world position and matches the HDR surface after alpha blending over black at relative 0.002 / absolute 0.00002 tolerance for half-float output. Existing 1728 projective HDR and 192 clipped PBR references remain unchanged. Apple M4 Max Metal and Linux llvmpipe Vulkan both passed all final cases; Linux shadow/PBR and auto-exposure/ownership regressions also passed. Targeted formatting/scoped whitespace checks passed. Browser still uses its previous constant-alpha material until this variant is wired and visually validated; roughness/Fresnel RGB composition and temporal filtering remain incomplete.

### Browser Fresnel integration and GPU reload acceptance

Animated HdrScene now uses PLANAR_REFLECTION_FRESNEL_SHADER with full vertex alpha instead of the previous fixed 0.35 weight. Per-frame surface world/tint [0.65,0.65,0.65], main camera eye and current metallic are supplied through the existing material ABI. The reflected object capture retains its own reflected eye and PBR parameters. The shared Metallic control therefore affects both the animated mesh and neutral reflector. Roughness still changes captured material lighting but does not yet blur reflected radiance.

Dev WASM rebuilt with cache key animated-fresnel-v1; strict wasm32 voxy_web Clippy, Node restoration/device-loss/startup/pause tests and scoped whitespace checks passed. Real WebGPU presented 1505 paused frames at time 17.8912 with metallic 0 and empty warning/error logs. Switching Metallic to 1 visibly changed both foreground material and receiver reflection blend; dielectric and metal screenshots were inspected and saved as browser-fresnel-dielectric.jpg and browser-fresnel-metal.jpg. This is scalar Fresnel for neutral surfaces; RGB colored-metal and roughness filtering remain outstanding.

A separate real WebGPU acceptance deliberately destroyed the device after 30 frames with metallic 1, roughness 1 and paused time 17.8912. Diagnostic/disabled controls appeared. Reload GPU removed the injection flag, recreated the reflection resources, retained material/pose/pause settings and presented 2492 frames by observation with empty warning/error logs. Screenshot browser-fresnel-recovery.jpg was inspected and saved; the temporary acceptance tab was then closed. Recovery remains page reload, not in-place GPU reconstruction or arbitrary game-state persistence.

### Retained linear HDR mip pyramid foundation

HdrMipPyramid owns an RGBA16Float texture with a complete mip chain, base render-attachment view, retained per-level views/bind groups and a downsample pipeline. Initialize mip zero via rendering/copy/upload, then encode all lower levels in order in the same encoder. Encoding allocates no GPU resources, submits nothing and uses distinct single-level views, avoiding reading/writing the same subresource. Device/encoder ownership remains subject to wgpu validation. Zero/excessive dimensions return InvalidSurfaceSize before allocation.

Each destination texel averages its integer source rectangle in linear HDR using textureLoad; floor-distributed source partitions include final odd columns/rows and support one-dimensional levels. This is a box/area pyramid with hierarchical local averaging, not a GGX angular prefilter or exact global-area average for all odd dimensions. No sRGB conversion, tone mapping or radiance clamp is applied.

New hdr_mips GPU fixture reads every level for 8x8, 7x5, 1x7 and 1x1 inputs, compares 556 channel values against CPU rectangle averages quantized per level to half-float (at most one half-float ULP), checks constants 4/16 survive and rejects zero size. Apple M4 Max Metal and Linux llvmpipe Vulkan passed. Linux runner now also builds/executes this fixture, with prior shadow/PBR, exposure/ownership and projective/Fresnel checks passing. Initial fixture upload was changed to explicit little-endian half bits because half::f16 has no Pod implementation in the current dependency configuration. Targeted formatting/whitespace checks passed. Capture integration, roughness LOD choice, GGX angular filtering and temporal history remain pending; existing browser reflection output is unchanged by this new utility.

wasm32 voxy_web cargo check passed with the HDR pyramid module included. This is compilation evidence, not browser execution of the new pyramid.

### Planar capture mip-chain integration

PlanarReflectionCapture now owns HdrMipPyramid instead of a single-level ProcessedColorTarget. encode renders mip zero then encodes all dependent lower levels in the same command encoder; ordinary frames reuse views, bind groups and pipeline. Existing view/material_binding expose the base level, while mip_material_binding exposes the complete initialized chain with caller sampling settings. Successful resize replaces the chain/depth; old material bindings retain the old texture. Lower-level generation adds GPU passes; no performance improvement is claimed.

Capture rejects sampling its own texture with InvalidTexture before starting a pass, alongside existing foreign-resource checks. The final fixture verifies this rejection leaves the encoder submit-able. It now binds the full chain and adds a material shader explicitly sampling LOD 3 of the 8x8 capture: all 192 output RGB values must be [2,1,0], the linear HDR average of the red=4/green=2 scene. Existing 1728 projection, 192 clipped PBR and 576 Fresnel references still pass, including retained resize/material identities. Apple M4 Max Metal and Linux llvmpipe Vulkan passed the final self-sampling rejection and mip-surface fixture; Linux shadow/PBR, exposure and standalone 556-value HDR mip checks passed too. Targeted formatting/whitespace checks passed.

The existing browser API continues to bind only mip zero, so roughness selection and GGX angular prefiltering remain incomplete. Live browser execution with the newly integrated capture chain has not yet been performed in this step.

wasm32 voxy_web cargo check passed after capture mip-chain integration.

### Roughness LOD and trilinear planar reflection

PLANAR_REFLECTION_ROUGH_SHADER extends scalar Fresnel with flat roughness from the PBR ABI and samples LOD=roughness^2*(textureNumLevels-1). Full-chain material bindings with linear min/mag/mipmap filtering provide continuous interpolation. This is a screen-space area-mip approximation, not GGX angular prefiltering. It preserves the Fresnel and projected-frustum rejection path.

The GPU fixture adds 576 CPU reference comparisons: roughness zero samples the sharp base, roughness one samples the last HDR average, and roughness sqrt(0.5) samples LOD 1.5 with explicit CPU bilinear values from 4x4/2x2 levels blended equally. Fresnel is applied to each expected value. Linux llvmpipe Vulkan passed this fixture plus previous projection/PBR/Fresnel/mip/retention/feedback checks and shadow/exposure/HDR-pyramid regressions. Strict wasm32 voxy_web Clippy, Node lifecycle/recovery checks and scoped formatting/whitespace checks passed.

Browser HdrScene now uses the rough Fresnel shader and full mip bindings with common reflection_sampling settings at creation and after resize. Cache key advanced to animated-rough-v1. Existing Roughness control drives both captured object material and reflective surface LOD. GGX angular filtering, colored-metal RGB composition and temporal history remain pending.

Final Metal M4 Max passed all roughness/trilinear reference cases. Dev WASM rebuilt with cache key animated-rough-v1. Real WebGPU loaded with empty warning/error logs, then Metallic 1 and Roughness 0.1/1 were selected at the same paused time 17.8912. Both rendered screenshots were inspected and saved (browser-reflection-smooth.jpg, browser-reflection-rough.jpg); reflective receiver changed as well as captured object lighting. The shared material control affects both, so screenshots alone do not isolate filtering; numeric shader references above do. At Roughness 1 the scene presented 4453 frames by observation without errors. Resize to 1024x768 continued to 5913 frames with unchanged pose/settings and empty logs; browser-reflection-resize.jpg was inspected and saved, then viewport override was reset. This confirms full-chain binding replacement after resize. GGX angular prefilter and temporal accumulation are still incomplete.

### GPU projective capture frustum rejection

The planar_capture fixture now renders the rough Fresnel surface with eight deliberately rejected capture projections: negative W, zero W, positive/negative X beyond the frustum, positive/negative Y beyond the frustum, and depth below zero/above one. The display projection and nonzero HDR captured scene remain valid, isolating rejection of projective capture coordinates rather than missing geometry. 1536 RGB readback values must be exactly zero (no numeric tolerance) after discard, checking that invalid projection regions do not clamp-sample capture edges or leak mip averages.

Final Apple M4 Max Metal and Linux llvmpipe Vulkan passed every rejection case alongside prior 1728 HDR, 192 PBR, 576 Fresnel, 192 last-mip and 576 roughness references. Linux shadow/PBR, exposure/ownership and 556 HDR-pyramid checks passed too. Targeted formatting/scoped whitespace checks passed. This strengthens GPU boundary acceptance without changing the rendered browser output. It does not cover nonfinite arithmetic overflow or pixels exactly on every clipping boundary; GGX angular prefilter and temporal history remain pending.

### HDR cube GGX environment prefilter

GgxEnvironmentPrefilter owns distinct six-face input/output RGBA16Float textures, output cube view and retained face/mip bindings/pipeline. Callers initialize all input faces (+X,-X,+Y,-Y,+Z,-Z) before encode. Mip zero performs an unfiltered cube lookup; later levels use roughness=level/(levels-1), alpha=roughness^2, 64 Hammersley GGX NDF half-vector samples with V=N=R, reflected directions, NdotL weights and normalized radiance. This is angular environment convolution, separate from the planar screen-space mip approximation. No resources are allocated or submitted by encode. The IBL importance-sampling basis is documented by Filament: https://google.github.io/filament/main/filament.html (IBL importance sampling annex).

New environment_ggx fixture compares 3576 constant/base channel references for an 8x8 six-face cube: HDR [4,2,8] remains constant across all roughness levels; a cube with distinct red face values 1..6 preserves mip-zero face identity. Rough levels must remain finite/in-range, preserve zero green/blue and opaque alpha, and all six last-mip face centers change versus their original face value. Output identity is retained across two encodes; zero dimension is rejected. This verifies normalization, face response and base mapping, not convergence/accuracy against a high-sample ground-truth environment integral.

Apple M4 Max Metal and Linux llvmpipe Vulkan passed the final fixture. Linux acceptance runner now includes environment_ggx alongside previous shadow/PBR, exposure/ownership, planar/Fresnel/roughness/frustum and HDR mip checks; all passed. Targeted formatting/whitespace checks passed. Full BRDF integration still needs DFG, energy/multiple-scattering handling, visibility and scene material bindings; browser still uses planar mip filtering. No cubemap scene capture or shader-based IBL material integration is claimed by this API alone.

wasm32 voxy_web cargo check passed with the GGX environment module included. This is compilation evidence; the new cube prefilter was not executed in the browser in this step.

### GGX environment quality budget API

GgxEnvironmentPrefilter::with_samples accepts a fixed 1..=4096 sample budget per output texel; new delegates to 64. Invalid counts return RendererError::InvalidPrefilterSamples with the exact submitted value before allocation. Immutable per-face/mip uniforms retain the selected budget across encode. This is a construction-time setting; changing quality requires creating another prefilter.

Metal M4 Max and Linux llvmpipe Vulkan passed the environment fixture at 1, 64 and 256 samples: 3576 constant/base channel references per setting (10728 total), finite bounded rough radiance and retained texture identity. One sample reproduces the deterministic reflected-axis limit and changes zero rough face centers; 64/256 change all six. Counts 0, 4097 and u32::MAX were rejected with exact error values. Linux shadow/PBR, exposure/ownership, planar/frustum and HDR-pyramid regressions also passed. Targeted formatting/scoped whitespace checks passed. These gates are quality-budget behavior and normalization evidence, not a measured performance/convergence comparison or full scene IBL acceptance.

The final fixture additionally exercised the supported maximum 4096 samples on both Metal M4 Max and Linux llvmpipe Vulkan. All four budgets passed 3576 constant/base references each (14304 total per backend), including retained output and bounded finite rough radiance. wasm32 voxy_web cargo check passed after the API change.

### GGX boundary dimensions and split-sum DFG LUT

Final environment acceptance now includes 1x1 and odd 3x3 cubes at 64 samples. Metal M4 Max and Linux software Vulkan passed 48 and 456 additional constant/base references respectively (14808 total per backend including the four 8x8 budgets). A 1x1 cube has only its unfiltered base level; a 3x3 cube has a filtered 1x1 level. Existing Linux shadow, exposure, planar and HDR-pyramid checks passed.

GgxDfgLut provides a retained RGBA16Float integration target and sampling view. Coordinates are NdotV on X and perceptual roughness on Y, sampled at texel centers; RG stores A,B for the single-scattering split-sum specular factor F0*A+B. It uses GGX NDF importance sampling, height-correlated Smith visibility and Schlick Fresnel, consistent with the visibility derivation in https://google.github.io/filament/main/filament.html. new defaults to 256 samples; with_samples accepts 1..4096 and rejects invalid counts before allocation. Encode allocates nothing and submits nothing; callers must initialize by encoding before sampling. The encoder must belong to the owning device (wgpu validation).

The dfg GPU example compares every channel against a separate f64 CPU integral at sizes/budgets 1/1, 3/64, 8/256 and 8/4096, encoding each retained LUT twice: 1104 channel references per backend with absolute tolerance 0.002. Metal M4 Max and Linux llvmpipe Vulkan passed, including retained texture identity and invalid input rejection. This compares discrete integration results, not convergence against an exact hemispherical integral. Native library and wasm32 voxy_web cargo check passed; targeted rustfmt and scoped whitespace checks passed. Linux runner includes dfg and prior regressions. Existing unrelated dependency deprecation/Send recursion warnings remain.

Material bindings, scene IBL composition, diffuse environment lighting, multiple-scattering energy compensation and local visibility remain pending. The new DFG shader has not been executed in the browser; wasm evidence is compilation only. No rendered scene change is claimed in this step.

### Scene specular environment composition

SceneRenderer::enable_environment_lighting attaches retained GGX cube/DFG/sampler bindings at group 3, leaving material bindings intact and group 2 reserved for shadow visibility. Pipeline construction permits the missing shadow group. All scene draw paths bind environment resources when present, including MSAA/overlay/transparent paths (those paths are compiled but not separately GPU-tested here). Both producers now retain their owning device for early foreign-resource rejection. Shader reload failure restores the previous environment bindings. Enabling shadowed point light after IBL preserves the environment shading source.

The shader samples reflected view direction with roughness*(cubeMipCount-1) and multiplies linear HDR radiance by RGB F0*A+B. Metallic F0 uses the base color, retaining colored-metal response; dielectric uses 0.04. Existing direct light and ambient remain. This is single-scattering specular IBL; diffuse environment convolution, multiple-scattering compensation, environment intensity controls and local occlusion remain pending. Producers must be initialized before rendering. Retained bindings refer to the attached textures; recreating producers requires reattachment.

New environment_lighting fixture renders an actual SceneMesh with direct-light intensity zero and HDR cube [4,2,8], checking 768 RGB references against CPU bilinear interpolation of GPU DFG readback and material composition (absolute tolerance 0.008). Cases cover neutral metal without shadows, neutral metal with attached disabled shadow visibility, colored metal and colored dielectric. Shadow sampling uses a separate texture from render depth. Metal M4 Max and Linux llvmpipe Vulkan passed all final cases; prior Linux shadow, exposure, planar, HDR mip, GGX and DFG regressions passed. Native library and wasm32 voxy_web check passed, targeted rustfmt/scoped whitespace checks passed. This proves shader/material integration on a constant environment; angular response in a varying environment, browser animation integration, enabled-shadow IBL interaction and foreign-resource rejection runtime acceptance remain to be added. Browser rendered output is unchanged in this step.

### Browser animated specular IBL integration

HdrScene initialization now creates a 16x16 six-face studio HDR cube with distinct warm/cool/ceiling/floor radiance, convolves GGX at 64 samples and generates a 64x64 DFG LUT at 256 samples. Upload and both producer passes are submitted once before consuming scene frames. Retained bind groups keep produced textures alive; normal animation and resize do not regenerate the environment. Both the shadowed main renderer and planar PBR capture consume the same environment/DFG. PlanarReflectionCapture::enable_environment_lighting installs world-plane-clipped IBL shading; its resource attachment and shader validation use one rollback boundary via enable_environment_source.

Strict wasm32 voxy_web Clippy passed after replacing ambiguous default-trait access in the initializer. Dev WASM rebuilt with cache key animated-ibl-v1. Node restoration, device-loss, startup-failure and pause-lifecycle tests passed, targeted formatting/scoped whitespace passed. Real WebGPU presented 2909 frames at paused time 17.8912 with direct light zero, metallic one and roughness 0.1, with no warning/error logs. Smooth and roughness-one screenshots were inspected and saved as browser-ibl-smooth.jpg/browser-ibl-rough.jpg; foreground material and planar reflection changed. This is a small procedural environment, not scene-generated cubemap capture or imported HDR asset support. Diffuse IBL, multiscattering compensation, independent reflector material controls and environment-specific UI remain outstanding. WebGL fallback retains its LDR path.

Live animation resumed through Space: time advanced from 17.8987 to 42.9828 and presented frames from 4982 to 7992 with direct light still off, no warning/error logs. Final moving-pose screenshot browser-ibl-animated.jpg was inspected and saved. The deliverable browser tab remains open. New environment/DFG device-loss reconstruction and canvas-resize acceptance were not separately exercised in this step.

### IBL foreign-resource and failed-reload GPU acceptance

The environment_lighting fixture now requests a second device from the same adapter and constructs foreign GGX/DFG resources. Each of the four material cases rejects a foreign supplied device, foreign environment, and foreign DFG; shader revision must remain unchanged. It also rejects invalid WGSL, then renders again and compares the entire readback buffer byte-for-byte with the pre-rejection result. Metal M4 Max and Linux llvmpipe Vulkan passed all cases alongside 768 existing numeric material references. This confirms rejection preserves usable original IBL bindings/pipelines, not just an error return.

PlanarReflectionCapture accepts valid initialized environment/DFG, rejects both foreign producers without changing its renderer revision, and encodes/submits an empty clipped capture afterward. This verifies continued pipeline usability but does not compare visible capture geometry after rejection. Linux shadow, exposure, planar, HDR mip, GGX and DFG regressions passed. Targeted rustfmt and scoped whitespace checks passed. Product code was unchanged in this acceptance step; diffuse IBL and multiscattering remain pending.

### Cosine-weighted diffuse HDR cube convolution

DiffuseEnvironmentConvolution owns separate six-face RGBA16Float input/output cubes and retained face pass bindings/pipeline. Output has one mip, storing E/pi rather than E: the Hammersley directions follow the cosine-weighted PDF cos(theta)/pi, so their mean radiance directly yields the normalized irradiance. The basis follows PBRT diffuse sampling/radiometric integration: https://www.pbr-book.org/4ed/Reflection_Models/Diffuse_Reflection . The caller multiplies by diffuse albedo without another pi factor. Fresnel partition, metallic exclusion, local visibility and material integration remain separate. Default budget is 256; with_samples accepts 1..4096. Initialize all source faces before encode; encode allocates/submits nothing and retains output identity. Invalid budgets/dimensions are rejected before allocation.

GPU acceptance tests 8x8 at 1/64/256/4096 samples plus 1x1 and 3x3 at 64 samples. Metal M4 Max and Linux llvmpipe Vulkan passed 6384 constant HDR [4,2,8,1] channel references (one half-float ULP tolerance) per backend. Distinct red face radiances 1..6 remain finite/bounded with zero green/blue and opaque alpha; all six face centers mix at multi-sample settings, while one sample returns the normal direction. Both encodes retain texture identity; invalid size/count rejection passed. This verifies normalization and response, not an independent varying-environment hemispherical integral or numerical convergence. Linux shadow, exposure, planar, HDR mip, GGX, DFG and scene IBL regressions passed. Targeted formatting/scoped whitespace passed. Diffuse material binding and browser output integration are not yet implemented; this producer alone does not add diffuse lighting to scenes.

### Scene diffuse and specular environment composition

SceneRenderer::enable_full_environment_lighting attaches GGX, DFG and normalized diffuse cubes. Existing enable_environment_lighting remains specular-only; the expanded compatible environment layout reserves binding 3 for the diffuse cube (specular-only bindings retain a harmless unused cube view). The material shader adds E/pi * base * (1-metallic) * (1-SchlickFresnel(NdotV,F0)); it does not divide by pi again. This is an explicit single-scattering view-dependent Fresnel partition, not an exact coupled BRDF/environment integral or multiscattering compensation. Existing direct light and ambient remain. The attached diffuse flag is retained when enabling shadow shading; foreign diffuse resources are validated before candidate bindings replace the active configuration. Planar capture currently continues with specular-only IBL.

The environment_lighting fixture produces a constant diffuse HDR cube [4,2,8], renders each existing material with the specular-only path, then attaches full IBL and checks 768 additional RGB references against the prior frame plus the expected diffuse term (absolute tolerance 0.012 including half-float output rounding). Fully metallic cases must contribute zero diffuse; colored dielectric contributes channel-scaled diffuse. Metal M4 Max and Linux llvmpipe Vulkan passed 768 specular + 768 full-IBL references, along with previous foreign-resource/invalid-shader output preservation. Linux shadow, exposure, planar, HDR mip, GGX, DFG and diffuse cube regressions passed. Native check, targeted formatting and scoped whitespace passed. Partial-metallic interpolation, varying-environment diffuse scene accuracy, foreign diffuse runtime acceptance and enabled-shadow/full-IBL transitions require further cases. Browser still uses the specular-only entrypoint; no visual change claimed yet.

### Browser full diffuse/specular IBL

Browser environment initialization now uploads the same six studio radiance faces to GGX and diffuse producers, encodes both plus DFG once, and attaches full environment lighting to the main shadowed renderer and the planar clipped capture. PlanarReflectionCapture::enable_full_environment_lighting installs the clipped diffuse/specular shader through the shared atomic environment-source path. Retained bindings keep textures alive across frames; no environment recomputation on ordinary animation or resize.

Strict wasm32 voxy_web Clippy passed; Node restoration/device-loss tests and scoped formatting/whitespace checks passed. Dev WASM rebuilt under animated-full-ibl-v1. Real WebGPU initialized with no warning/error logs. At paused time 17.8912, roughness one, metallic zero and direct light disabled, the scene presented 1884 frames by observation. The dielectric object and receiver remained lit by the environment; browser-full-ibl.jpg was inspected and saved. This confirms browser execution of the full scene and clipped-capture shader; no pixel-integral ground truth or device-loss/resize acceptance of this new configuration was performed here. Multiple-scattering energy compensation, imported HDR environments, local occlusion and independent receiver material controls remain outstanding.

### Bounded linear HDR image assets

HdrImageAsset adds Radiance RGBE .hdr decoding to linear RGB32F with retained dimensions and pixels. The existing image dependency enables its hdr codec; PNG/JPEG ImageAsset behavior remains separate. No tone mapping, sRGB conversion or 0..1 clamp is applied. Header exposure/color-correction metadata is not applied; stored RGBE radiance is returned. EXR is unsupported. from_rgb rejects negative/nonfinite radiance, inconsistent lengths, zero/excessive dimensions and decoded-buffer limits. Decode bounds source bytes, codec dimensions and combined decoded/conversion buffer sizes before pixel decode; codec scratch limits remain best-effort as with ImageLimits.

Four native tests passed: a 2x1 RGBE fixture preserves [2,1,0.5] and [8,4,2]; source and 47-byte combined-buffer budgets reject, exact 48-byte budget accepts; dimension one and truncated pixels reject; PNG magic is unsupported; CPU NaN/infinity/negative values and wrong lengths reject. wasm32 voxy_web cargo check and targeted formatting/scoped whitespace passed. The initial native process temporarily waited at macOS dyld startup (sample showed _dyld_start), then finished all four tests successfully. Linux acceptance runner now includes the HDR asset tests; its current run is still in progress. This is decoding only: equirectangular-to-cube conversion, GPU upload, environment asset loading UI and imported-environment rendered acceptance remain pending.

### HDR equirectangular panorama conversion

HdrImageAsset::cube_faces resamples a full panorama to six RGB32F faces ordered +X,-X,+Y,-Y,+Z,-Z using the existing GPU cube face directions. Longitude atan2(Z,X) maps +X to U=0.5 and +Z to U=0.75; +Y is the north pole. CPU bilinear sampling wraps longitude across the seam and clamps latitude at poles. Double precision interpolation retains finite HDR values without intermediate f32 overflow; no tone mapping or gamma is applied. Combined source-plus-six-output allocation and face-size limits are checked before allocation. The output remains RGB32F CPU buffers; GPU half-float upload and attachment to loaded environment resources are still pending.

New tests preserve constant HDR [4,2,8] at face sizes 1,3,8, reject zero/excessive dimensions, and exercise exact source-plus-output budgets (456 bytes succeeds, 455 rejects). A longitude/latitude ramp checks +X/+Z/-Z centers, the -X wrapped seam, and north/south clamping independently of the production direction loop. Linux HDR asset tests and the full existing software Vulkan runner completed successfully. The preceding native loader tests also completed successfully after their dyld delay. Native expanded test and WASM checks are still being observed in this step; no imported panorama rendered GPU acceptance is claimed.

wasm32 voxy_web cargo check passed with panorama conversion included. Targeted rustfmt and scoped whitespace checks passed. Native expanded tests remain live at observation; Linux acceptance is completed.

Final native expanded HDR asset suite completed: all six tests passed, including panorama axes/seam and exact combined allocation limits. No process remains pending for this step.

### Imported HDR GPU environment resource

ImportedEnvironment::from_panorama connects HdrImageAsset cube resampling to retained GPU GGX/diffuse/DFG producers. It accounts for retained RGB32F panorama, six resampled RGB32F faces and packed RGBA16Float faces in the supplied CPU pixel budget. Radiance above half-float maximum 65504 returns HalfFloatRange before GPU resource creation rather than clamping or uploading infinity. Input cubes are uploaded once; encode generates all three outputs without allocation/submission; attach installs full scene IBL on the resource's owning device. Queue/encoder ownership still relies on wgpu validation. DFG resolution is currently fixed at 64 with 256 samples; GGX uses 64, diffuse 256. GPU texture allocations remain subject to device validation, separate from the CPU image limit.

New imported_environment acceptance decodes a real RGBE byte fixture for an 8x4 [4,2,8] panorama, constructs the import resource, encodes it and renders material cases. It compares 768 specular + 768 full diffuse/specular HDR RGB references using CPU bilinear lookup of the 64x64 DFG readback. It also rejects radiance 70000 and insufficient packing budget before upload, and repeats existing foreign-resource/invalid-shader output preservation cases. Linux llvmpipe Vulkan passed all final import cases plus HDR asset and previous rendering regressions. Native library check and targeted formatting/scoped whitespace passed. Metal import acceptance and WASM check remain under observation. Varying imported-panorama angular accuracy, asset selection UI, EXR and multiscattering compensation remain outstanding; existing browser demo still uses its procedural studio environment.

Final Metal M4 Max import acceptance passed all four cases (1536 references) including wrapper encode/attach and range/budget rejection. wasm32 voxy_web cargo check passed after moving half to runtime dependencies. No processes remain pending for this step.

### Browser HDR environment loading API

WebEngine::load_hdr_environment accepts Radiance bytes for an existing animated WebGPU HDR scene; other modes return an error. HdrScene decodes with ImageLimits defaults, imports at 32x32 cube resolution, submits environment producers, then attaches full lighting to the main renderer and clipped reflection capture. The scene now retains its active environment producers. If reflection attachment fails, the main renderer is restored from the previous retained resources; successful replacement transfers ImportedEnvironment producer ownership via into_parts. If restoration itself fails, both errors are returned explicitly.

The caller must suspend frame calls for the entire async WASM mutable borrow. This API does not itself manage JS RAF, file selection, progress, resource-operation concurrency or device-loss lifecycle. Imported bytes are not persisted across reload. Strict final wasm32 voxy_web Clippy passed; Node restoration/device-loss tests and scoped whitespace/formatting checks passed. These are API compilation and existing lifecycle tests, not execution of HDR loading in a real browser. The file input and its RAF guard, invalid-file behavior and rendered imported-environment acceptance remain next steps. Existing browser package was not rebuilt in this API-only step, so its displayed procedural environment is unchanged.

### Browser HDR file selection and async borrow guard

Animated WebGPU demo exposes HDR environment file selection. createHdrLoader serializes file operations, checks the 32 MiB source bound before reading and after arrayBuffer, and releases its busy state on success/read/decode/GPU failure. During loading, settings/file controls are disabled and RAF returns before device diagnostics or other engine calls; visibility pause synchronization also avoids the borrowed engine. Time bookkeeping resets so loading delay does not jump animation. Corrupt files preserve the prior environment and permit retry. Input value is cleared after selection so the same file can be selected again. User-facing failures show a readable HDR/size hint; decoder detail is retained only in dataset diagnostics. Uploaded bytes are local to the current scene and are not persisted across reload or GPU recovery.

Node loader tests passed serialization, pre-read source rejection, success and failure release. Updated existing page tests passed device loss, startup failure, settings restoration and visibility/manual pause; additional acceptance verifies RAF and visibility handlers make no engine call during the import borrow. Dev WASM rebuilt with animated-hdr-import-v1; scoped whitespace checks passed. Added deterministic warm-studio.hdr and corrupt invalid-test.hdr fixtures.

Real WebGPU file-chooser acceptance loaded warm-studio.hdr with direct light zero, dielectric material and roughness one: foreground/receiver lighting visibly changed and screenshot was inspected. Corrupt file selection returned a failure, released controls and retained environment=warm-studio.hdr; frame count continued from 3265 to 7554 with empty warning/error logs. After page reload, the friendly error hint was verified and a correct file successfully loaded after a corrupt selection. Some intermediate screenshots in that second paused run showed black canvas despite increasing presentation counts; no rendering cause was established. Resuming animation produced a visibly correct imported scene at 12244 frames/time 54.9331 with no logs. Final browser-hdr-import.jpg is that inspected moving-pose screenshot, replacing intermediate black captures. The deliverable tab remains running. Pause/compositor screenshot reliability after repeated load merits further validation; GPU-loss persistence and varying panorama visual accuracy remain outstanding.

### Repeated paused HDR import black-canvas investigation

The black-canvas screenshot reproduced on the retained tab: ordinary pause kept the visible image, then selecting the same warm HDR again at unchanged time 104.2653 produced black canvas at 20748 presentation counts with empty GPU logs. Changing exposure to four restored visible scene without advancing time (26362 counts). Thus resuming animation is not the only recovery trigger, and repeated import at a frozen pose is the distinguishing scenario.

An experiment refreshing the WebGPU surface configuration after the async loader released its borrow compiled under strict WASM Clippy and was rebuilt/tested. It did not fix the repeated import: first load displayed correctly, second load at paused time 17.8912 again showed black at 5041 counts without logs. The refresh API/call was removed rather than retaining an ineffective workaround. No rendering fix is claimed. Next investigation needs GPU readback of retained HDR scene output versus canvas presentation to localize the defect; screenshot/presentation count alone cannot distinguish compositor behavior from actual render content. Node loader/device-loss/pause tests still pass after removal.

### Asynchronous HDR diagnostic pixel readback

HdrPixelProbe copies one RGBA16Float pixel into a retained aligned readback buffer, maps asynchronously after submission, decodes half floats and returns the result once. Source format, COPY_SRC usage and bounds are rejected before copy. Animated browser imports create a fresh probe; the scene copies its center after HDR rendering and before tone mapping. The page clears old hdrProbe diagnostics at each import and exposes the completed linear values through canvas.dataset.hdrProbe. Normal rendering does not block for readback.

Strict wasm32 voxy_web Clippy and dev WASM build passed. Node HDR loader serialization/limit/release tests passed; scoped whitespace checks passed. Real WebGPU first and repeated warm-studio.hdr imports at paused time 17.8912, direct light zero, roughness one and metallic zero returned identical center [1.5771484375,0.2064208984375,0.05523681640625,1]. Inspected browser-hdr-probe-first.jpg and browser-hdr-probe-repeat.jpg both show a visible scene, at 3843 and 5847 presentations respectively; warning/error logs were empty. This run did not reproduce the prior black canvas. Diagnostic mapping may change timing; no black-canvas fix or compositor cause is established. Native hdr_probe acceptance example was added for nonuniform HDR values, nonzero origin, invalid coordinates and one-shot lifecycle; its cargo run is still waiting behind a confirmed concurrent editor build, not yet a passing native gate.

The initial native hdr_probe example completed on Apple M4 Max/Metal: exact nonzero-origin [6,0.25,16,1] readback and one-shot behavior passed. The source guard now additionally rejects multisampled textures before an illegal texture copy. Expanded example coverage rejects both out-of-bounds axes, RGBA8 format and missing COPY_SRC before the valid copy, ensuring failed validation leaves the probe usable. The Linux shadow/material runner now includes hdr_probe. Shell syntax and scoped whitespace checks passed. Expanded native and standalone Docker software-Vulkan runs are live at this observation; their completion is not yet claimed.

Expanded native HDR probe acceptance completed on Metal M4 Max, including format/usage and both coordinate rejection cases followed by successful exact HDR readback. Docker Vulkan remains live.

Final standalone Docker HDR probe acceptance also completed on llvmpipe/Mesa software Vulkan, with the same exact HDR values and rejection/lifecycle cases. Both expanded runs are terminal and passed. This is software Vulkan evidence; hardware Linux GPU and the paused-repeat black-canvas cause remain unverified. Existing dependency deprecations and voxy_render Send recursion future-compatibility warnings remain.

### Retained HDR loader recovery ownership

createHdrLoader now retains a private copy of the last successfully imported source bytes and name. restore(importer) can attach that environment to a replacement engine without rereading a File. WASM/caller mutation cannot change retained bytes. Failed imports or restores preserve the previous source and release the busy guard; restore and normal import share serialization. The source remains bounded by the existing 32 MiB import cap. Retention is in-memory for the loader lifetime, with one source snapshot; this does not persist across whole-page reload.

Node acceptance passed: no-environment restore, importer mutation isolation, successful source retention, failed replacement preservation, retry after replacement-device error, no File reread, and concurrent restore/import rejection. Existing loader, device-loss and pause lifecycle tests passed. The current Reload GPU link still reloads the page; it has not yet been connected to an in-place engine recreation or a persistent recovery store. Therefore end-to-end GPU-loss preservation of imported HDR remains pending.

### Browser reload HDR transfer wiring

Reload GPU now intercepts recovery clicks when a successfully imported HDR is retained. It commits a bounded source snapshot to origin-local IndexedDB under a random transfer ID before navigation. The recovery URL carries only that ID. New animated HDR startup reads the source, imports it before scheduling the first frame, and deletes the entry/removes the URL ID on successful restoration. Settings/time/manual pause still use the existing recovery URL snapshot. Storage errors leave the recovery link available for retry; source import failures retain the stored entry. Store reads reject missing, oversized or expired (one hour) sources. Expiration is a read guard, not automatic garbage collection; abandoned entries currently remain until site data is cleared.

HDR loader, device-loss, pause lifecycle and startup-failure Node tests passed after integration. Real IndexedDB transaction execution and end-to-end browser GPU-loss reload acceptance are still pending; no rendered recovery proof is claimed. Recovery source is local to this origin and does not leave the browser.

### Live browser imported-HDR GPU-loss recovery acceptance

An explicitly opted-in deviceLossManual=1 URL exposes Lose GPU (diagnostic) for WebGPU. It invokes the existing validation device destroy API through a normal UI button; ordinary pages do not expose it and HDR loading disables it with other controls. Real browser acceptance imported warm-studio.hdr at paused time 17.8912 with direct light zero, exposure one, roughness one, metallic zero and PCF shadows. Before device loss: 2131 presentations and center HDR [1.5771484375,0.2064208984375,0.05523681640625,1]. Clicking the diagnostic produced the expected Destroyed device status, disabled controls and Reload GPU link. Clicking recovery navigated through the IndexedDB transfer and recreated the engine.

After recovery, DOM confirmed warm-studio.hdr, identical center HDR values, unchanged pose/manual pause and all material/light/shadow settings, at 904 new presentations. Controls were enabled, hdrRecovery transfer ID was removed from the URL, and warning/error logs were empty. browser-hdr-gpu-recovery.jpg was inspected: the recovered warm-lit scene is visible. This proves the explicit reload recovery path for a local constant HDR fixture in this browser. It does not prove automatic in-place recovery, arbitrary game-world state, varying-panorama accuracy, browser storage quota failure handling, or resolution of the earlier paused-repeat black-canvas issue. Existing device-loss and HDR loader Node tests passed with the diagnostic UI addition.

### HDR transfer expiry reclamation and real IndexedDB acceptance

Recovery-store open now removes expired, malformed and future-dated records in a readwrite cursor transaction; read also deletes an invalid entry rather than merely returning null. Store initialization closes its connection if cleanup fails. The one-hour transfer lifetime is unchanged. Cleanup occurs on next store access, not in a background timer; fresh abandoned sources within that hour still consume quota.

A standalone browser acceptance page uses a unique temporary database and injected clock with actual IndexedDB transactions. It passed structured-clone ownership, read mutation isolation, expiry reclamation (a raw store count confirms zero physical records before any post-expiry read), save after cleanup, explicit transfer deletion and empty-source rejection. The temporary test database is deleted in finally. Final browser-hdr-store-acceptance.jpg was inspected; no warning/error logs were observed in the first run. Node HDR loader tests and module syntax passed. This complements the prior live GPU recovery check; quota/blocked-open failure acceptance remains outstanding.

### Common scene environment intensity API

SceneRenderer::set_environment_intensity writes a retained 16-byte uniform at IBL group 3 binding 4. Both diffuse and specular environment terms use the scale; direct lighting and existing ambient remain unchanged. It accepts finite nonnegative values and rejects invalid values or absent IBL bindings before queue writes. Queue ownership is validated by wgpu. Default intensity is one; enable_environment_source initializes replacements with the active intensity, preserving it across imported resource reattachment and shader rollback. No environment convolution is repeated.

wasm32 voxy_web cargo check passed; targeted formatting and scoped whitespace passed. Expanded environment_lighting GPU example renders scales zero, half and double for each material case, compares to the independent unscaled readback with ambient removed/readded, rejects negative/NaN/infinity values, and reattaches resources before rendering to check scale retention. The Metal cargo run is currently live behind concurrent workspace compilation; runtime success is not yet claimed. Planar capture and browser controls still require API integration, and current served WASM has not been rebuilt for binding 4.

### Environment intensity reflection/browser API integration

PlanarReflectionCapture exposes the same validated environment intensity update; WebEngine::set_animated_environment_intensity forwards to both HdrScene main renderer and clipped reflection renderer. Reattachment preserves each renderer's active scale. Browser HTML controls/recovery URL persistence for this setting are not yet wired, and served WASM remains the previous build.

The first Metal intensity fixture passed its first material then failed the next baseline because the fixture retained intensity two between material cases. This is the documented retention behavior, not a rendering fix: the test now resets to one after its scale checks before entering the next case. Retried Metal and standalone Linux software-Vulkan acceptance plus strict WASM Clippy are currently live; passing runtime results are pending. Scoped whitespace and targeted formatting passed.

Final Metal M4 Max and Linux llvmpipe Vulkan environment intensity acceptance passed all four material cases: 1536 prior HDR references plus 2304 additional scale references per backend (three scales, 192 RGB values per case). Invalid intensity rejection and resource replacement retention passed. Reflection/browser forwarding compiles but has not yet received separate rendered acceptance.

### Browser environment intensity controls and recovery settings

Animated HDR WebGPU UI exposes independent Environment intensity Off/0.5/1/2. Changes invoke the new main/reflection API without environment recomputation. Initial default is one; validated environmentIntensity URL restoration uses the control options, and GPU recovery captures the current dataset value alongside light/material/shadow settings. HDR loader busy handling disables this select with other controls. Default WebGL/LDR mode keeps it hidden.

Node animated restoration tests passed selected intensity two and rejection of negative/NaN/infinity values; device-loss URL test passed retention of the current intensity. Existing pause and HDR loader tests passed. Scoped whitespace passed. Dev WASM rebuild under animated-environment-intensity-v1 is live; rendered browser scale/recovery acceptance remains pending until rebuild completes.

Final dev WASM rebuild completed. Real WebGPU paused scene with direct light zero showed dark residual ambient at Environment Off and visibly brighter main/receiver scene at intensity two. browser-environment-off.jpg and browser-environment-two.jpg were inspected. DOM confirmed intensity two and paused time 17.8912 at 3754 presentations, with empty warning/error logs. The first immediate screenshot after changing to two still showed the prior Off control/frame; a subsequent fresh DOM/screenshot showed the updated selected control and bright output. No timing-based render defect conclusion is drawn from that intermediate capture. New intensity GPU-loss rendered acceptance remains pending; URL/unit coverage passed.

### Live environment intensity import and GPU recovery acceptance

Real WebGPU imported warm-studio.hdr after selecting environment intensity two, with direct light zero, metallic zero, roughness one and paused time 17.8912. Import preserved the active scale: center HDR readback [3.138671875,0.40478515625,0.1064453125,1] at 7925 presentations. Manual device destruction produced the expected terminal status and a recovery URL carrying environmentIntensity=2. Clicking Reload GPU restored the imported environment, scale, pose/pause and light/material/shadow settings. At 982 new presentations, center HDR exactly matched the pre-loss values; environmentLoading=false and environmentError empty. Warning/error logs were empty, and inspected browser-environment-intensity-recovery.jpg shows the bright warm-lit recovered scene.

This proves selected intensity retention across imported-resource replacement and real browser GPU-loss reload recovery for this fixture. It does not independently verify reflected pixel scaling, arbitrary scene/world reconstruction or varying panorama angular accuracy. The full multi-platform engine goal remains active; Windows NVIDIA, mobile and headset runtime acceptance remain separate outstanding requirements.

### Independent browser planar receiver material

HdrScene now retains reflector_material separately from the animated mesh material, both defaulting to roughness 0.35/metallic 0.5. Animated mesh PBR and its reflected capture use the mesh material; the planar Fresnel/mip surface transform uses the reflector material. WebEngine::set_animated_reflector_material validates finite [0,1] parameters before mutation. Browser HDR UI adds independent reflector roughness/metallic selects; validated URL restoration and GPU recovery snapshot retain both. Changing mesh material no longer implicitly changes receiver Fresnel or blur.

Strict wasm32 voxy_web Clippy passed. Node animated restoration checks independently restore reflector roughness one/metallic zero and reject invalid reflector input; current device-loss tests include reflector setting retention. Dev WASM build under animated-reflector-material-v1 is live; rendered independent reflection controls remain unverified until browser acceptance.

Final dev WASM build completed. Real WebGPU controls set reflector roughness 0.1/metallic one while mesh remained roughness one/metallic zero; DOM confirmed independent state at paused time 17.8912 and 1637 presentations. Inspected browser-independent-reflector.jpg shows the configured scene, with empty warning/error logs. This confirms API/UI execution and retained independent settings, not a reflected-pixel ground-truth comparison or live GPU-loss recovery of the new settings.

### Numeric clipped reflection IBL intensity acceptance

Expanded environment_lighting fixture now renders actual geometry through PlanarReflectionCapture with full diffuse/specular IBL, explicit world clip plane and a separately created capture transform. Each of four material cases renders environment scales zero, one and two, reattaching resources after setting the scale. It compares each capture RGB pixel against the already independently validated main full-IBL readback with residual ambient removed/readded (absolute tolerance 0.025). The clip plane retains this geometry; arbitrary plane clipping is covered by the separate planar fixture.

Metal M4 Max passed 2304 added reflected RGB references alongside 1536 existing base material and 2304 main scale references (6144 total per backend). This verifies capture shader execution, intensity forwarding and scale retention, not reflected camera projection/mip/Fresnel surface numeric composition, which remains the separate planar_capture fixture. Linux software Vulkan run is currently live. Targeted formatting and scoped whitespace passed.

Final Linux llvmpipe software Vulkan clipped-reflection intensity acceptance also passed all four material cases and 6144 total channel references. Both GPU runs completed successfully. Existing dependency deprecation and Send recursion future-compatibility warnings remain; no hardware Linux GPU evidence is claimed.

### Metallic rough planar surface numeric boundaries

The planar_capture fixture adds three receiver material cases to the existing projection/Fresnel/mip tests: metallic one with roughness zero, metallic one with roughness one (last mip), and metallic 0.5 with roughness sqrt(0.5) (halfway between mip levels one/two). CPU expected values combine independently computed red/green mip filtering with scalar Schlick F0 0.65 for metal or 0.345 for partial metal. Frustum rejection remains limited to its eight original explicit cases rather than capturing the added material cases.

Metal M4 Max passed 576 added RGB references and all 4800 previous references (5376 total), including projection, clipping, retained targets and resource ownership checks. The output label was updated to include the new count after that run; no shader/product code changed. Linux software Vulkan run is live. These cases verify the existing neutral-tint planar Fresnel/roughness approximation, not GGX angular convolution of planar reflections, arbitrary colored-metal reflectors or temporal denoising.

Final llvmpipe software Vulkan planar material boundary acceptance passed the same 5376 references and existing retention/ownership checks. Both expanded GPU runs are complete. Native/Linux run output used the older count label because compilation preceded the label-only update.

### Retained temporal resolve destination

TemporalResolve::prepare_into reuses a caller-owned single-mip RGBA32Float storage target with the input extent, with optional existing neighborhood clipping. It validates target format/dimension/sample count/usage and rejects aliases with all five sampled inputs before creating bindings. Existing prepare/prepare_clipped retain allocation behavior. Frame preparation still allocates uniform/bind-group resources; this change removes destination texture allocation only. Device ownership remains wgpu validated.

Metal temporal fixture passed all existing reprojection/depth/reset/HDR/clipping/sanitation checks using prepare_into for the clipped output, with retained texture identity verified. A stronger feedback-alias test using a fully compatible input/output target was added after that run; its repeat and WASM check are currently live. Targeted formatting/scoped whitespace passed. Caller-managed presented-history commit, ping-pong ownership and browser reflection integration remain pending.

### Retained temporal target rejection acceptance

Final stronger compatible feedback-alias test passed on Metal M4 Max and Linux llvmpipe Vulkan alongside the temporal analytic numeric cases and retained output identity. The fixture additionally rejects a wrong width, RGBA16Float output, missing STORAGE_BINDING and multiple mips, before executing/readback-validating the previously prepared valid frame. This expanded rejection suite passed on Metal; its Linux repeat is live. Existing per-channel sanitation, UV/depth rejection, reset and neighborhood clipping remain passing. These are retained-destination safety and numeric tests, not presented-history lifecycle or browser temporal integration.

Expanded retained temporal rejection suite completed successfully on Linux llvmpipe software Vulkan too. Both final expanded runs are terminal and passed; existing unrelated dependency/Send recursion warnings remain.

### Caller-confirmed retained temporal history owner

TemporalHistory owns two RGBA32Float color targets and two R32Float previous-depth textures on one device. color/depth identify the committed read slot; output identifies the distinct pending color slot. encode_depth validates size/format/sample count/COPY_SRC and rejects pending-depth copy aliases. presented explicitly swaps roles and marks history valid; callers must only invoke it after radiance/depth submission and successful presentation. reset invalidates without allocation; changed-size resize recreates both pairs and invalidates, while invalid resize preserves the owner. The owner does not itself observe queue/presentation success or enforce that callers wrote both pending resources.

wasm32 voxy_web check passed. Temporal GPU fixture now resolves clipped output into this owner's retained target and encodes depth copy, while preserving existing numeric checks. It additionally simulates caller commit notifications to verify alternating identity, reset, unchanged resize and failed resize validity preservation. The native run is live. This is a common resource foundation, not browser reflection temporal integration or actual headless presentation evidence.

### Temporal history depth content acceptance

The initial TemporalHistory owner fixture completed on Metal M4 Max: retained-output numerical resolve checks plus simulated commit/reset/resize identity and validity assertions passed. TemporalHistory now exposes output_depth for the pending color/depth pair. The fixture copies all four pending R32Float pixels to readback and expects exact source depth 0.5 before simulated commit, complementing texture identity checks with content evidence. Expanded Metal and Linux software Vulkan runs are currently live. Commit notification remains caller-authoritative and the headless fixture does not claim actual presentation.

Expanded Linux llvmpipe Vulkan history-owner fixture passed numeric resolve, exact depth-copy readback, alias/output rejection and simulated role/reset/resize transitions. The expanded native run remains live at observation; Linux evidence remains software Vulkan.

Final expanded Metal M4 Max run also completed successfully. Both history-owner GPU acceptance runs are terminal and passed, including exact pending-depth content.

### Primary depth attachment to temporal history conversion

Inspection of the windowed temporal surface consumer found Depth32Float attachment inputs, incompatible with the history owner's existing R32Float copy path. TemporalHistory now retains a fullscreen raster pipeline and renderable pending R32Float depth targets. encode_depth_attachment samples exact integer pixel depth from a single-sampled Depth32Float texture into that slot without linearization or texture allocation. It validates size/format/dimension/sample count/TEXTURE_BINDING before encoding. Existing R32Float copy remains available.

wasm32 voxy_web check and scoped formatting/whitespace passed. Expanded native temporal fixture clears a real Depth32Float attachment to 0.25, converts it into pending depth and expects exact four-pixel R32Float readback, followed by the existing numeric/lifecycle assertions; this native run is live. No successful conversion runtime or windowed history-owner integration is claimed until that check completes.

Final Metal M4 Max temporal attachment conversion fixture passed exact 0.25 depth readback and all existing resolve/owner lifecycle checks. Linux conversion and actual windowed presentation integration remain pending.

### Windowed retained history lifecycle integration in progress

The temporal_surface_smoke example adds opt-in VOXY_TEMPORAL_RETAINED=1, requiring VOXY_TEMPORAL_HDR=1 and VOXY_TEMPORAL_RG_MOTION=1. Its real surface consumer records converted primary depth and current HDR radiance into the pending TemporalHistory slot. Resolve intentionally resets history and uses weight zero: this proves current-frame ownership, not temporal accumulation; expected previous-surface depth correspondence is not yet supplied. After the existing outcome/callback verifier confirms successful presentation, the owner commits. The injected consumer failure case checks that its history texture did not switch. Existing surface resize scenarios resize the retained owner on the next consumer frame.

Example check and actual Metal windowed run are live. Compilation/runtime success and frame content after real presentation are not claimed yet. The shared Depth32Float conversion has prior Metal numeric acceptance; Linux conversion and windowed retained integration remain outstanding.

### Windowed history verification build interruption

Native example check/run and Linux conversion repeat initially failed while compiling concurrently edited voxy_editor: parent module called private component_fields::edit_component_field (E0624). Live source reinspection showed pub(super) visibility already applied by concurrent work before a proposed patch could match; no editor mutation was made in this task. The confirmed-terminal native windowed run was restarted with VOXY_TEMPORAL_BACKEND=metal plus HDR/RG/retained flags against current source and is live. Earlier sessions were not restarted on timeout; restart follows their recorded compilation failure. Windowed history content/presentation acceptance and the Linux conversion repeat remain unproven.

Final current-source Metal M4 Max windowed retained-history run completed successfully: HDR source readback, opaque primary depth/motion protection, real prepare/submit/publish, injected consumer-failure history identity preservation and resize/reset scenarios passed. This is actual surface presentation lifecycle evidence for the owner, with reset/zero-weight color resolve. Committed history pixel readback and temporal accumulation with moving-surface previous depth remain outstanding.

### Primary correspondence expected previous depth output

PrimaryMotionPass now emits a second R32Float attachment for previous-camera NDC depth at current primary coverage, using the same selected previous world position as its backward motion vectors. Static, affine-object and supplied deformation-position paths therefore share correspondence. Reset/background/unknown object/invalid correspondence or previous depth outside (0,1) produces zero depth for rejection; the existing RG16 motion output remains available. Construction checks two attachments/eight bytes per sample before allocation. previous_depth exposes the retained texture for TemporalResolveInputs::expected_previous_depth.

WASM compilation is currently live and MRT shader runtime acceptance is pending. Existing windowed retained history consumer remains reset/zero-weight and has not yet been switched to this primary-surface producer. No temporal accumulation or moving-surface depth accuracy claim is made in this step.

Final wasm32 voxy_web check passed with the new primary MRT output. Native Metal relative_motion GPU regression is live; it exercises correspondence/motion and will validate pipeline execution, but does not yet read back the new previous_depth attachment.

### Previous-surface depth correspondence GPU acceptance in progress

Initial Metal relative_motion regression passed with the new primary MRT pipeline, preserving its existing backward UV and zero-background behavior. Expanded fixture moves previous positions by (0.25,0,0.125), leaving its orthographic UV reference unchanged while changing expected previous depth from current 0.5 to previous 0.625. It reads every R32Float previous-depth pixel and an independently reset producer: covered samples must be 0.625, background and reset zero. Large 100000/200000 world-coordinate/local correspondence coverage remains. Expanded Metal and Linux software Vulkan runs are live; depth accuracy is not yet claimed. Targeted formatting and scoped whitespace passed.

Expanded Linux llvmpipe software Vulkan previous-depth fixture passed all 32 added depth references (normal/reset, covered/background) alongside existing motion references. Expanded native run remains live; prior native MRT execution regression passed. This is supplied deformation-position correspondence with an orthographic camera; affine-ID and perspective depth correctness still require additional cases.

### Correspondence depth to temporal resolve end-to-end fixture

Final previous-depth-only Metal fixture completed successfully with the same 0.625 covered/zero background/reset references as Linux. Expanded relative_motion now feeds actual PrimaryMotionPass RG motion and previous_depth into TemporalResolve. Constant current HDR two/history ten must blend to six at valid covered samples when previous presented depth is 0.625; wrong presented depth 0.5 must preserve current two. Background and reset correspondence must also preserve current two. This adds 144 RGB references over three scenarios, independent of the producer depth readback. Metal and Linux expanded runs are live. No production temporal consumer or presented-history accumulation integration is claimed by this fixture.

Final Metal M4 Max and Linux llvmpipe software Vulkan end-to-end correspondence depth/resolve runs passed all 144 added HDR RGB references plus previous motion/depth checks. Both runs completed successfully. The fixture remains orthographic/local supplied deformation correspondence; perspective/affine-ID coverage and production accumulation remain outstanding.

### History-owned resolve preparation

TemporalHistory::prepare_resolve builds the common resolve input bundle from its committed color/depth and pending output. It forces reset_history when the owner is invalid, preserving the caller's explicit reset flag. Current depth copy and successful-presentation confirmation remain explicit caller operations; expected previous depth must come from surface correspondence. No current depth substitution or automatic presentation claim is made.

WASM check passed. Expanded relative_motion fixture adds three sequential retained-owner scenarios: first frame outputs current ten despite requested blending, a following covered matching-depth sample blends current two with committed ten to six, then reset forces current two everywhere. The middle candidate is deliberately not committed in this headless caller simulation. Test source textures were given COPY_SRC for depth storage. Metal and Linux software Vulkan runs are live; numeric owner behavior is pending. Scoped formatting/whitespace passed.

Final Metal M4 Max and Linux llvmpipe software Vulkan history-owned resolve scenarios passed 144 additional RGB references plus all earlier producer depth/motion and resolve rejection cases. Both runs are terminal and passed. Presented confirmation in this fixture remains simulated; the separate windowed owner lifecycle proof remains reset/zero-weight.

### Animated ray scene uses common retained temporal history

animated_ray now owns TemporalHistory instead of retaining each newly allocated TemporalResolveFrame/current-depth pair. It prepares clipped HDR resolve into the common pending color target, converts the actual primary Depth32Float attachment into pending history depth, and commits only inside the successful RenderOutcome::Presented branch. The existing deformation-aware PreviousDepthPass remains the expected previous-surface correspondence input. Invalid owner or invalid skeletal motion resets history; material, camera and resize invalidation paths remain in place.

cargo check --locked --offline -p voxy_ray_probe --example animated_ray passed. cargo run --locked --offline -p voxy_ray_probe --example animated_ray -- --experimental --smoke completed successfully on Apple M4 Max Metal with 120 actual presentations. Existing skeletal BLAS, finite GGX direct/reflected radiance, HDR reprojection, analytic previous/current depth, reset motion, resize and material-switch reset assertions passed. The expanded readback additionally compares pending retained history depth with current-surface depth at the sampled center pixel (tolerance 0.000001). This is an actual animated ray temporal consumer using the common owner; it is not browser integration or whole-image reflection quality/denoising acceptance. Scoped diff whitespace check passed; existing unrelated voxy_app dead-code warnings remain.

### Perspective and affine correspondence depth acceptance

New perspective_motion GPU fixture reconstructs a z=-2 plane from Depth32Float depth 5/9 using a 90-degree perspective camera (near 1, far 10). It exercises static surfaces, two independently moving objects with rotation/nonuniform scale/current-model inverses, unknown R32Uint object IDs and explicit history reset. Every 4x4 pixel is read back for RG16 backward UV plus R32 previous-camera depth: 144 scalar references with tolerance 0.0003, compared to a separate pinhole plane/depth formula and CPU affine transforms.

Initial Metal M4 Max run passed all references. Final source switched the deprecated camera constructor to glam::camera::rh::proj::directx::perspective; Linux llvmpipe software Vulkan passed all 144 references against that source. Final native repeat compiled successfully and is live. Linux temporal-motion-smoke.sh now builds the example and executes its Vulkan path before the existing scenarios; only this added standalone Vulkan invocation was runtime-verified in this step, not the entire runner. Scoped rustfmt, shell syntax and whitespace checks passed. Browser scene correspondence production remains pending; this fixture does not prove browser reflection accumulation, perspective skinned deformation, or hardware Linux GPU support.

Final current-source Metal repeat also completed successfully with all 144 UV/depth references. Both final runs are terminal and passed.

### Browser primary temporal correspondence preparation

Animated WebGPU HdrScene adds opt-in deformation-aware RasterMotionPass and PreviousDepthPass, sharing retained pipeline/output resources through next_frame_reusing. The paired vertices combine the exact PreparedSkinnedFrame correspondence and the stationary receiver plane. Passes encode after matching main-scene primary depth coverage. The browser depth attachment now permits texture sampling. HdrScene tracks current camera and updates previous camera only after queue presentation and skeletal pose commit; resize invalidates camera history. First-frame/invalid skeletal history resets motion. Disabling guides releases resources and invalidates camera history.

WebEngine::set_animated_temporal_guides and the temporalGuides=1 diagnostic query enable preparation for an active animated WebGPU HDR scene. Color temporal accumulation is not yet connected, and no reflection accumulation claim is made. wasm32 cargo check passed; scoped whitespace passed. Browser dev package build is live using the existing wasm-bindgen 0.2.127 executable at /tmp/voxy-bindgen-0.2.127/bin/wasm-bindgen (the default executable version was rejected before compilation). Browser shader execution/data readback and resize/present behavior acceptance remain pending.

Browser development package compilation and wasm-bindgen generation completed successfully. Runtime acceptance remains pending.

### Browser temporal guides execution and resize acceptance

Current generated WASM was exercised in the existing in-app browser at http://127.0.0.1:8793/?backend=webgpu&animated=1&temporalGuides=1. Animated HDR scene rendered visibly with guides enabled and no warning/error console entries. DOM diagnostics recorded 2219 successful animated presentations while moving, 5537 after paused resize to 960x640, 6554 after restoring default 2560x1440 and resuming, then 7717 presentations with animation time advancing from 18.59 to 28.28. Temporary viewport override was reset. Screenshot docs/browser-temporal-guides.jpg preserves the rendered scene and settings; tab remains available paused for inspection.

This proves browser pipeline creation/execution, frame presentation, resize replacement and resumed animation without reported validation errors. It does not numerically verify browser motion/depth texture contents, first-frame reset contents, history accumulation, or reflection denoising. Temporal color consumption and browser GPU readback acceptance are still pending.

### Opt-in browser HDR temporal color integration

The temporalGuides=1 animated WebGPU path now also prepares TemporalResolve/TemporalHistory, writes primary Depth32Float into the pending history slot and resolves linear HDR into retained RGBA32Float before tone mapping. Backward raster motion and deformation-aware previous depth feed clipped resolve (history weight 0.85, depth tolerance 0.001). The owner commits alongside previous camera after presentation and skeletal pose commit. First frame, invalid skeletal history, resize and explicit invalidation reject history. Successful environment replacement/intensity, mesh/reflector material, light and shadow changes invalidate history; exposure remains a display transform. 2D overlays render after temporal tone mapping.

WASM cargo check and development package build passed. Browser ran this opt-in path for 7749 successful presentations without error/warning logs, visibly rendering animation. Changing roughness to 1/light to 70, resizing to 960x640 and restoring the default viewport remained operational. Screenshot docs/browser-temporal-resolve.jpg preserves the rendered result. Device-loss recovery URL now retains temporalGuides=1; Node device-loss and animated-recovery suites passed. The URL change was tested in Node, not a new live GPU-loss cycle. Scoped whitespace passed.

This proves actual browser compute resolve execution and display integration, not numeric browser accumulation/depth acceptance or a denoising quality guarantee. Planar reflection radiance is temporally filtered as part of the surface image; there is no separate reflected-hit motion/reprojection or RTX ray reconstruction. The path remains opt-in while numeric browser readback, disocclusion/ghosting quality and full device-loss recovery acceptance are pending.

### Full float HDR temporal diagnostic readback

HdrPixelProbe now decodes RGBA16Float or RGBA32Float based on the actual encoded source. Existing one-shot asynchronous mapping and format/usage/coordinate rejection remain. The expanded hdr_probe fixture additionally writes a nonuniform RGBA32Float texture and reads pixel (1,1), expecting exact [70000.125, 0.123456, 16.25, 1]. Metal M4 Max and Linux llvmpipe software Vulkan passed both original half-float/lifecycle assertions and this full-float case. The large value exceeds half-float range and detects accidental half decoding. Targeted formatting and scoped whitespace passed.

The browser temporal owner initialization now creates paired one-shot source/output probes for its first frame. Existing hdrProbe records the current linear RGBA16 source; new temporalProbe records the RGBA32 resolved pixel. First-frame history rejection should preserve finite nonnegative source values exactly. WebEngine::take_temporal_probe exposes completed diagnostics; mapping begins after submission. WASM cargo check passed. Updated browser dev package build is live, waiting for the shared Cargo build directory; no numerical browser comparison is claimed yet.

### Browser first-frame temporal HDR numerical acceptance

The existing development package build completed successfully after the shared Cargo lock released. Current generated WASM was reloaded in the temporal-enabled animated WebGPU scene. Source and first resolved center pixels both read back exactly [0.72998046875, 0.359619140625, 0.219970703125, 1]. DOM-backed comparison confirmed all four channels finite and bit-value equal; 2932 presentations were observed, with no warning/error logs. Screenshot docs/browser-temporal-first-frame.jpg records the visible scene; probes refer to the first frame and the screenshot to a later displayed frame. Node device-loss and animated-recovery suites remained passing.

This closes the browser first-frame history-reset color acceptance at one sampled pixel. It does not prove subsequent blending coefficients, motion/depth contents, whole-image correctness, reset after setting changes, GPU-loss restore or disocclusion/ghosting quality; those remain separate required checks.

### Browser retained temporal numerical fixture

New voxy_web temporal_validation invokes the actual common TemporalResolve/TemporalHistory on the WebEngine WebGPU device with controlled 4x1 HDR textures, zero motion and matching/mismatching depth. Five scenarios read back all RGBA channels: invalid initial history preserves ten; matching depth with weight 0.5 blends current two/presented ten to six; enabled neighborhood clipping limits history to current two; expected depth 0.75 versus stored 0.5 rejects history; explicit reset preserves current two. Intermediate candidates remain uncommitted so the fixture also checks retained committed input. Commit is explicitly simulated by the fixture, which has no surface presentation.

WebEngine::validate_temporal exposes the asynchronous browser-event-loop check; temporalCheck=1 runs it before animation startup. WASM check and generated development package build passed. Actual in-app browser WebGPU run reported canvas.dataset.temporalValidation=80 (five scenarios, four pixels, four channels), with tolerance 0.00001 and no warning/error logs. The animated scene then rendered 1175 presentations with existing first-frame source/resolved probes still equal. Screenshot docs/browser-temporal-validation.jpg preserves the subsequently visible scene. Targeted formatting/scoped whitespace passed.

These are numerical browser GPU resolve/owner checks with controlled inputs and simulated commit. They do not numerically validate the animated scene's generated motion/depth textures, actual surface commit blending, multi-pixel disocclusion or ghosting quality. Those production-input/quality checks and live GPU-loss recovery remain outstanding.

### Animated browser production motion/depth numerical acceptance

New temporal_guides_probe samples a 3x3 grid from the actual animated scene RasterMotionPass RG16Float and PreviousDepthPass R32Float outputs. CPU independently projects all paired triangles, selects nearest visible coverage using screen barycentrics/current NDC depth, perspective-correctly interpolates previous positions and derives previous-camera UV/depth. It compares GPU motion X/Y and previous depth with absolute tolerance 0.00005, including background and first-frame zero motion. Reads use asynchronous ComputeDispatch mapping after submission; scene settings and generated skeletal correspondence are the real production inputs.

Opt-in temporal scene captures presented frame indices 0, 30, 60 and 120, up to 108 scalar references. Current generated WASM compiled and ran in the in-app browser: temporalGuideChecks=108, temporalMovingChecks=3 (sampled references with nonzero expected motion above tolerance), 1348 actual animated presentations and no warning/error logs. Existing controlled temporalValidation=80 and first-frame HDR source/output equality remained passing. The moving counter confirms acceptance included deforming surface correspondence rather than only zero-motion receiver/background samples. Screenshot docs/browser-temporal-production-guides.jpg preserves the later displayed scene. Targeted formatting and scoped whitespace passed.

This establishes sampled production correspondence accuracy at four frames. It does not prove full-frame edge/disocclusion coverage, scene color accumulation coefficient after actual presentation, reflected-hit motion, ghosting quality, reset after settings changes, or live GPU-loss restore; those remain outstanding.

### Production edge correspondence and user temporal control

The production guide probe now augments the 3x3 grid with samples two pixels to either side of each projected animated triangle edge (including the internal diagonal). Sample count/readback capacity follows accepted in-bounds positions. Actual browser WebGPU run passed 252 scalar motion/depth references at frames 0/30/60/120, including 24 sampled nonzero motion references, with no warning/error logs. This covers sampled boundaries between animated foreground and stationary receiver, while retaining the independent perspective barycentric reference. Existing controlled temporalValidation=80 remained passing. It is not exhaustive raster edge or disocclusion acceptance.

Animated HDR UI now exposes a Temporal HDR checkbox. It enables/disables the existing shared resolve path through WebEngine API; re-enabling creates fresh history. Device-loss recovery preserves enabled mode and removes a stale temporalGuides=1 URL flag when the user disables it. Node device-loss/animated-recovery tests and scoped whitespace passed. Browser UI toggle off/on remained operational at 2100/3352 presentations; re-enabled first-frame source/resolved center pixels both exactly [0.257080078125, 0.22705078125, 0.2130126953125, 1], proving fresh history preserved the current sampled color. No warning/error logs were reported. Screenshot docs/browser-temporal-control.jpg preserves the checked UI and visible scene.

Generated WASM build and actual browser acceptance passed. Live GPU-loss restore, full-image ghosting/disocclusion quality and scene blending coefficient after actual presentation remain pending.

### Live browser temporal GPU-loss recovery acceptance

The current animated WebGPU scene was exercised with the existing diagnostic device-destroy button (deviceLossManual=1), not just mocked callbacks. With Temporal HDR enabled, roughness 1 and environment intensity 2, real Destroyed loss stopped rendering and disabled controls. Reload GPU URL preserved temporalGuides=1, settings and animation time 16.1834. Reload created a fresh device/scene/history: 996 presentations, 252 production guide references and 16 moving references passed without warning/error logs. First-frame source/resolved center both exactly [1.1884765625, 0.591796875, 0.31298828125, 1], confirming new history preserved current HDR.

A second real device loss followed user disabling Temporal HDR. Recovery URL removed the enabled flag and retained material/environment/pose settings; restored checkbox was unchecked, guide checks zero and rendering resumed for 870 presentations with no warning/error logs. Re-enabling after recovery worked, with first-frame source/resolved both [1.205078125, 0.6005859375, 0.31689453125, 1]. Screenshot docs/browser-temporal-gpu-recovery.jpg preserves the recovered enabled scene. These checks used the procedural environment; imported HDR plus temporal recovery and pause-state combinations are not covered by this run.

This closes real device-destroy recovery for enabled/disabled temporal mode with the tested settings. Full-image ghosting/disocclusion quality, reflected-hit motion and scene blending coefficient after actual presentation remain outstanding.

### Desktop OpenGL temporal portability fixes and full Linux runner

Linux temporal-motion runner now includes temporal_resolve and explicitly selects VOXY_XR_BACKEND alongside WGPU_BACKEND/VOXY_MOTION_BACKEND, preventing the XR GL iteration from silently using an automatic Vulkan device. Expanded full execution exposed three real failures: R32Float motion-depth render targets forbidden by GL downlevel restrictions; direct WGSL textureLoad of depth unsupported by the GLSL translation path; and NaN motion accepted by GLSL float UV comparisons, incorrectly blending history.

PrimaryMotionPass and raster PreviousDepthPass now use renderable R16Float previous-depth outputs on GL (half precision), retaining R32Float on other backends. TemporalResolve accepts sampleable R16Float/R32Float depth and relative_motion decodes the actual readback format. TemporalHistory retains R32Float storage depth and converts Depth32Float through compute rather than an unsupported R32 render attachment. It reuses the existing depth_sample mechanism: nearest sampling normally, comparison-based 24-step depth reconstruction on GL. Construction checks compute/storage limits and dispatch dimensions. Temporal resolve now explicitly rejects nonfinite motion/UV/depth using integer IEEE exponent masks before UV conversion and depth matching.

Final sh tools/linux/temporal-motion-smoke.sh passed completely on Mesa llvmpipe software Vulkan and desktop OpenGL 4.5. Both backend logs confirm temporal depth conversion/resolve numeric cases, correspondence/background, relative previous depth/history resolve, moving-geometry composition, actual skinned surface commit/resize/suspension and explicit-backend XR motion cases. The separate perspective fixture passed Vulkan. Exact pending depth 0.25 conversion and existing sanitation/clipping/reset cases passed in temporal_resolve on both backends. This is software-driver acceptance, not hardware Linux GPU or physical headset proof.

WASM voxy_web cargo check passed, scoped formatting/whitespace and shell syntax passed. An earlier Metal temporal fixture passed after the compute conversion; final current-source Metal repeat remains live. Generated browser WASM/runtime needs rebuilding/rechecking after these shared shader changes. Initial --locked checks encountered a shared manifest/lock mismatch; normal offline library check completed, and subsequent --locked Linux/WASM checks worked. Existing dependency/editor/Send-recursion warnings remain.

Final current-source Metal M4 Max temporal_resolve repeat completed successfully, including depth conversion and all retained-history numerical checks. Browser artifact rebuild/runtime remains the next validation gate.

### Browser acceptance after portable depth/shader fixes

Development WASM package rebuilt successfully with the shared compute depth conversion and explicit nonfinite motion/UV/depth rejection. New cache key temporal-portable-depth-v1 ensured current artifacts. Actual animated WebGPU run passed controlled temporalValidation=80, production temporalGuideChecks=252 and temporalMovingChecks=24; first source/resolved HDR center remained exactly [0.72998046875, 0.359619140625, 0.219970703125, 1]. No warning/error logs were reported. At 3639 presentations, resize to 971x653 (neither dimension divisible by the 8x8 workgroup) remained operational; default viewport was restored without errors. Screenshot docs/browser-temporal-portable-depth.jpg preserves the later rendered scene.

This closes the browser runtime regression gate for these shared shader changes and sampled production inputs. Resize acceptance is operational; border depth/color pixels after resize were not numerically read back. Full-image ghosting/disocclusion quality, reflected-hit motion and scene blending coefficient after actual presentation remain outstanding.

### Production browser temporal color acceptance

The capture-only temporal_color_probe reads actual scene current color, committed history, motion, expected/history depth, resolved output and 3x3 current neighborhoods at presented frames 0/30/60/120. An independent CPU calculation checks clipped blending with weight 0.85 and depth tolerance 0.001; capture does not calculate the expected blend. Current development WASM build completed and actual WebGPU execution passed 252 RGB references, including 28 pixels with a result differing from current color and two sampled depth-mismatch history rejections. Existing production guide checks remained 252 with 24 moving references, and controlled resolve validation remained 80. At 1464 presentations no warning/error logs were reported.

This closes sampled production color blending after actual history presentation and sampled depth rejection. Full-image ghosting/disocclusion quality, reflected-hit correspondence, imported HDR plus temporal device-loss/pause combinations and hardware/platform acceptance remain outstanding.

### Runtime asynchronous completion rejection

BarrierQueue previously returned false for duplicate keys but replaced the accepted value through BTreeMap::insert. It now uses the entry API so duplicate rejection preserves the first accepted completion; stale epochs remain rejected before mutation. A regression test uses distinct accepted, duplicate and stale mesh values and checks the actual drained result. Scoped whitespace passed. The offline locked runtime library test compiled; its test executable remains live without a result at the time of this note, so test acceptance is pending.

The runtime library run subsequently completed: all 12 tests passed, including duplicate/stale rejection preserving the drained mesh value. BarrierQueue now also exposes its current job epoch and advance_epoch for scene/world transitions. Advancing discards pending old results and rejects their late completions; exhaustion preserves the original epoch and queued values. New transition and exhaustion regressions are included. The repeat test process is live, waiting for the shared build-directory lock; acceptance for these new methods is pending. This is a renderer-neutral queue contract, not yet an asynchronous asset-loader or native/browser scene-switch integration.

### Runtime epoch and existing asset path acceptance

The repeat locked/offline runtime library test completed successfully: all 14 tests passed, including transition clearing/late-result rejection and exhaustion preserving pending data. The existing voxy_assets background_import example also passed real off-owner-thread decoding, bounded submission, stale-ticket rejection, file-triggered failed reload/recovery, retained immutable snapshots and joined shutdown. Source inspection of the editor tick confirms it polls source invalidation before publishing imports through complete_observed. This existing asset-ticket/catalog path already supplies import currency checks; the world/chunk BarrierQueue epoch API is not substituted for those asset tickets.

A current-source Metal animated_ray experimental smoke regression was launched after the common temporal depth conversion changes. Its process remains live compiling renderer/editor dependencies, with GPU runtime acceptance pending.

### Shared runtime clock in Metal animated ray

The Metal regression terminated with a timeout after zero presentations; this run does not establish current-source GPU acceptance. The animated ray example now advances animation through the common voxy_runtime::FrameLoop, sharing native scene-shell pause and bounded delta semantics. Smoke/profile retain deterministic 1/60-second input. Resize and initialization reset the clock, and the Space control changes pause immediately. Offline example cargo check passed; scoped whitespace passed. New smoke diagnostics report draw attempts and skipped surface outcomes with preparation/submission times without extending the 60-second deadline. A repeat with the shared clock and diagnostics remains live, waiting for the shared build directory. The cause of the zero-presentation timeout is unproven.

### Metal surface occlusion diagnosis and retry scheduling

The diagnostic repeat terminated after zero presentations and 32291 draw attempts; captured outcomes repeatedly reported SkippedOccluded. This establishes surface occlusion as the immediate skip reason, not numerical temporal acceptance or a shader failure. The example now logs changes in skipped outcome rather than every repeated skip and schedules skipped-frame retries through winit WaitUntil at 100 ms intervals. It also requests window focus during initialization, matching the native scene shell. This prevents immediate redraw spin while retaining retries and the original smoke timeout; it does not establish that focus resolves occlusion on this host. Scoped whitespace passed. Compile validation is live waiting for the shared build lock; GPU acceptance remains pending.

The current locked/offline animated_ray example check completed successfully after the lock released. A smoke repeat with scheduled retries and focus request has been launched; runtime outcome is pending.

### Metal retry runtime and lifecycle repair

The scheduled-retry smoke completed with the original 60-second timeout: zero presentations, 574 attempts, compared with 32291 attempts in the prior immediate-redraw run. Only the initial SkippedOccluded transition was logged. This verifies bounded retry frequency and suppressed repeated logging under actual occlusion. Requesting focus did not resolve acquisition on this run. GPU color/depth/animation acceptance is still unproven.

The example now handles suspension by stopping redraw work, suspending its surface at zero size, discarding pose/camera/temporal history and resetting the shared clock. Resume reuses an existing window/device instead of creating a second window, clears retry scheduling and requests redraw; draw-time size reconciliation restores the surface. Lifecycle source compiled successfully and scoped whitespace passed. A final repeat check covers the added resume ControlFlow::Wait reset. Physical minimize/resume presentation acceptance remains pending. Native UI tooling could not select this unbundled CLI executable as an application; packaging/visible-window acceptance is a separate next step.

### Local macOS bundle packaging and dependency boundary

tools/macos/package-ray-demo.sh now creates interactive VoxyRay.app or VoxyRaySmoke.app with an executable launcher, copied debug Mach-O, high-resolution application metadata and runtime log. Both actual bundles built and passed plutil validation. The ray probe's face application dependency is now optional under default-enabled face-demo; no-default-feature animated ray packaging avoids unrelated editor code. The no-default-feature probe binary cargo check also passed. Default face functionality is retained in source; a shared editor private-method compile failure prevented the initial default build, so default build acceptance is not claimed.

The smoke bundle launched via native UI and registered as io.voxy.ray.smoke. Its window became accessible, but acquisition remained SkippedOccluded and it timed out at zero presentations/549 attempts. The interactive bundle also launched and exposed its titled window; native screenshots showed a blank dark content area before and after title-bar activation. Bundling alone therefore has not resolved occlusion, and no presented ray image is claimed. The bundles are local, unsigned development artifacts, not notarized distributable applications. Script syntax and scoped whitespace passed.

### Current Metal offscreen ray acceptance and native occlusion handling

Installed wgpu-hal 30.0.1 Metal source checks the hosting NSWindow occlusionState before nextDrawable and returns SurfaceError::Occluded if its visible bit is absent. This is the immediate acquisition guard observed in the native runs; why the host reports that state for the accessible blank window is still unproven.

Current locked/offline no-default-feature ray probe ran to completion on Apple M4 Max Metal. Numerical checks passed TLAS transforms/shared-BLAS instances/masks/removal/invalid update retention, shadow visibility, HDR reflection radiance 131008, analytic GGX distance/weight, closest-hit/miss/range/bias, nine roughness/seed configurations with 16 primary pixels and eight-sample CPU means, depth reconstruction, camera/object motion/reset and material-weighted HDR reflection/direct-light composition. The shader fixtures include blocked/unblocked geometry; their occluded boolean is geometric lighting visibility, not NSWindow visibility. This is actual offscreen GPU acceptance and does not close animated surface presentation, temporal ownership after real presentation, or minimize/resume acceptance.

The animated example now handles WindowEvent::Occluded explicitly. Hidden windows defer preparation before skeletal/BLAS updates, retain bounded retries for smoke timeout observation, and reset pose/camera/temporal history and the shared clock on visibility changes. Returning visibility requests redraw. Locked/offline no-default-feature example check and scoped whitespace passed; actual hidden/visible lifecycle runtime acceptance is pending. The no-face build now rejects --face before selecting an adapter instead of failing later through GPU initialization.

### Portable animated ray backend and physical NVIDIA gates

animated_ray no longer hardcodes Metal: --backend selects auto/metal/vulkan/dx12 without silent API fallback. --require-nvidia enumerates adapters and requires vendor 0x10de, a physical device class, window-surface compatibility and experimental ray-query support. Normal selection also rejects an adapter missing ray-query support before device creation. Local macOS launchers explicitly request Metal. Native macOS and x86_64-pc-windows-gnu locked/offline example cargo checks passed; Windows linking/execution is not established by cross-check.

The existing optional CUDA hardware ray-query gates now build both probes without the unrelated default face application dependency and additionally run the animated example with explicit DX12 on Windows or Vulkan on Linux. They require the 120-presentation acceptance marker plus a matching physical NVIDIA adapter/backend log. PowerShell parser validation, shell syntax and scoped whitespace passed. Actual Windows/Linux NVIDIA execution remains pending. A native runtime check of malformed backend selection has been launched and is still compiling; no runtime selection acceptance is claimed yet. These changes do not implement or prove DLSS, frame generation or RTX 5090-specific capability support.

Malformed backend runtime check completed and rejected the invalid token before GPU initialization with the documented allowed values. Physical-NVIDIA rejection on the Metal-only host is being checked separately.

The Metal-host --require-nvidia runtime check completed with the explicit missing-physical-NVIDIA error, confirming it did not fall back to Apple GPU rendering.

### Owned wide HDR Frame Generation scene handoff

SceneFrameGeneration::import_wide_radiance now owns an HdrHalfResolveJob converting wide HUD-less ray color into the half-float HDR input accepted by the existing FG scene importer. Candidate presentation ID, reset state, opaque depth and motion are preserved. Preparation encodes conversion before native shader-read handoff; the returned FG tag guard retains the imported output lease. Depth/motion share render resolution while color can use presentation resolution. Caller still guarantees matching camera/exposure, SDK options, serialized producer/preparation ordering and all native SDK/GPU lifetime contracts.

Initial Windows GNU-target scene-dx12 cross-check passed using existing Streamline 2.14.1 headers. A final check after preserving distinct color/render extents is running. Actual Metal hdr_range completed successfully, including linear HDR range/exposure cases and half conversion clipping, negative RGB clamping and alpha preservation. Scoped whitespace passed. This validates the existing conversion shader numerically and the new interface by compilation; actual DX12 FG tagging/generated presentation is not proven.

Current NVIDIA Streamline FG documentation requires depth, motion and HUD-less color inputs: https://github.com/NVIDIA-RTX/Streamline/blob/main/docs/ProgrammingGuideDLSS_G.md. The developer DLSS 5 announcement was rechecked at https://developer.nvidia.com/blog/whats-new-for-game-developers-dlss-5-with-3d-guided-neural-rendering-nvidia-ace-updates-and-new-rtx-kit-capabilities. Its developer controls announcement does not itself establish a usable NR SDK ABI or this engine's DLSS 5 execution. Neural Rendering options/model/evaluation and physical RTX acceptance remain outstanding.

Final locked/offline Windows GNU-target scene-dx12 check completed successfully with distinct presentation-color/render-depth extents supported.

### Shared HUD-less FG composition input

SceneFrameGeneration now supports import_radiance for a separately composed HDR/SDR color while preserving the scene candidate's depth, motion, presentation ID and reset. Wide HDR import delegates to that same path after its owned conversion. FrameGenerationResources::hudless_color and the scene candidate getter expose the exact retained color for read-only composition, including converted HDR output. This allows a caller to compose the same input tagged for FG rather than manually reconstructing a TemporalFrame or displaying a different intermediate. Preparation/producer ordering and SDK completion ownership still apply; callers must not destroy or overwrite the borrowed texture during consumption.

Locked/offline Windows GNU-target scene-dx12 library check passed with existing SDK 2.14.1 headers; scoped whitespace passed. This is a compiled SDK handoff/composition interface. No Windows GPU execution, proxy presentation, generated-frame count or DLSS 5 evaluation is claimed. A native composition/FG caller and physical acceptance remain required.

### Executable DX12 scene FG preparation fixture

The existing dx12_probe now invokes support/fg_resources when scene-dx12 is enabled, including queue-only mode without loaded NVIDIA DLLs. It creates same-device depth/motion at width two and wide HUD-less HDR color at width four, imports the production SceneFrameGeneration candidate, encodes its owned conversion and preparation, then reads the candidate's exact exposed output. The fixture expects 16 channels covering linear HDR, values above half range, negative RGB clipping and alpha, plus retained ID 17/reset true/output extent/format. It performs no SDK tagging/evaluation or generated presentation.

GPU poll and mapping callback waits are bounded at five seconds. Unproven GPU completion conservatively retains the candidate owners. The final locked/offline Windows GNU-target scene-dx12 dx12_probe check passed against existing Streamline 2.14.1 headers; targeted formatting and scoped whitespace passed. The fixture has not run on Windows hardware here, so neither its PASS marker nor actual FG generation is claimed. Native README documents the required runtime marker and distinguishes preparation from SDK generation.

### DX12 FG fixture validation gate and executable build

The FG resource fixture now wraps creation/preparation/readback in a GPU validation error scope, closes it on ordinary error returns, and reports GPU error plus readback context. The PASS marker moved outside the readback helper and is emitted only after the scope reports no error and numerical checks succeed. Current Windows GNU-target scene-dx12 example check passed after the scope change, including current vendored wgpu-hal. Scoped formatting/whitespace passed. A full Windows executable build is live compiling/linking dependencies; its outcome and physical GPU execution remain pending.

The full locked/offline scene-dx12 Windows GNU-target build subsequently completed successfully. The linked artifact is target/x86_64-pc-windows-gnu/debug/examples/dx12_probe.exe. It is not a Windows runtime result: execute --queue-only on a Windows DX12 host and require the FG SCENE RESOURCES PASS marker before accepting that hardware path.

### OpenXR stereo scene with compositor 2D panel

QuadSubmission now carries a released swapchain subimage, reference space, pose, metre dimensions, eye visibility and composition flags. end_stereo_frame_with_quad submits the existing stereo projection first and the panel second in the same frame. Panel and projection may use different reference spaces from the same runtime instance. Validation rejects nonfinite transforms/dimensions, nonunit orientation, nonpositive dimensions and unknown eye visibility before runtime submission. The existing stereo entry point delegates through the same implementation. end_stereo_frame_with_quad_and_history commits combined history only after successful submission of both layers. GPU completion and released-image ownership requirements still apply.

Native test compilation and targeted formatting passed; library test execution and Windows cross-check are live at this note. No headset/runtime submission, visual HUD rendering or compositor acceptance is claimed.

The locked/offline XR library run completed: all 17 tests passed, including panel geometry/eye visibility and existing submission-history ownership checks. The locked/offline Windows GNU-target XR library cross-check also passed. These are CPU validation and compilation results; physical compositor acceptance remains pending.

### Controller aim picking for OpenXR quad UI

hit_test_quad maps an active tracked controller aim (-Z) to the same reference-space panel pose and metre dimensions used by QuadSubmission. It returns top-left UV coordinates and reference-space distance, rejects back-facing/parallel/out-of-bounds/out-of-range rays, and returns no hit when aim activity or either location validity flag is missing. Nonfinite geometry/range and nonunit valid poses are explicit errors. Internal f64 transforms and normalized validated quaternions preserve metre-distance semantics. Tests cover centre/boundary misses, range, tracking loss, rotated UI coordinates, translated/rotated reference spaces and back faces. This is picking logic, not a renderer or headset acceptance result. The caller must locate aim against the panel space at the pending predicted time and sync actions before using activity/select state.

The locked/offline XR test process is live waiting on shared build-directory activity; test acceptance is pending at this note.

The locked/offline native XR library test completed successfully: all 20 tests passed, including the three controller/panel picking cases and existing quad/stereo/history ownership checks. Targeted formatting passed. No actual controller-runtime or UI click dispatch is established by these CPU tests.

### VR controller to common UI pointer routing

XrPanelPointer now adapts QuadHit coordinates into the existing voxy_ui PointerRouter, using logical UI dimensions rather than swapchain pixels or panel metres. update_hand consumes synced HandInput select/aim activity and located aim validity: inactive select/aim or missing position/orientation cancels pointer capture. A tracked ray leaving the panel retains drag-out release semantics. Initialization, tracking loss and explicit cancellation require observing a released select button before another press; holding a trigger through recovery cannot capture a widget. Switching controller/panel/session/reference-space requires cancellation. Existing pointer semantics handle disabled overlays, repeated down, capture and release/click; no second widget router was introduced.

Application order: sync actions, read_hand, locate aim in the panel reference space at predicted display time, hit_test_quad, then update_hand with the same located aim and hit. Dispatch the returned PointerAction to the existing UI. Update painter-ordered hit regions through PointerRouter before routing. End stereo plus quad only after GPU work and swapchain release. This implements the input bridge; no physical runtime, rendered menu or actual headset clicks are claimed. Test/locked-current-source acceptance is live at this note.

The final locked/offline XR library run completed: all 22 tests passed, including actual common PointerRouter click/drag-out dispatch and tracking-loss cancellation/held-recovery suppression. Final-source update_hand compiled, and targeted formatting passed. Physical OpenXR input/UI rendering acceptance remains pending.

### Executable end-to-end panel input path

The locked/offline panel_input example built and ran successfully, printing XR PANEL INPUT PASS. It exercises actual hit_test_quad -> XrPanelPointer::update_hand -> common PointerRouter with controlled HandInput/SpaceLocation boundary values. Assertions cover topmost overlay selection rather than the full-panel background, repeated hold, clicked release, active ray drag-out, invalid tracking with NaN location data ignored, capture cancellation, held tracking recovery suppression and inactive select cancellation. Runtime-boundary values are deterministic; no runtime calls or physical input are claimed. The crate README now records the executable commands and session/action/aim/panel/render/release/submission order, with reference-space and unit distinctions. Targeted example formatting passed.

### Current vendored Metal window-state diagnostic acceptance

The current no-default-feature animated_ray smoke bundle rebuilt successfully against the active vendored wgpu/wgpu-hal and passed Info.plist validation. A temporary generated launcher enabled the existing VOXY_EDITOR_TRACE_INPUT diagnostic; repository packaging defaults were not changed. Native UI launched the bundle, exposed its titled window, raised it and clicked its title. The screenshot remained blank dark. Runtime reported hosting NSWindow number 25113, occlusionState 8192, isVisible true, isMiniaturized false and isKeyWindow false. The visible occlusion bit (1 << 1) was absent and acquisition returned SkippedOccluded. This distinguishes the visible-property/occlusion-state disagreement and lack of key status from a minimized window; why AppKit reports this state remains unproven. Window activation did not change the logged state in this run.

The process terminated at its original smoke deadline after zero presentations and 572 attempts. Thus current native surface ray acceptance remains failed, despite successful offscreen ray checks recorded earlier. Existing winit documentation/source says launch activation already ignores other applications by default; adding that default again would not establish a fix. No occlusion-guard bypass or vendor mutation was made. Generated diagnostic launcher is temporary and normal packaging overwrites it on the next build. The terminal process is no longer live.

The generated launcher diagnostic export was removed after the run; its normal launch arguments/log path are restored.

### GPU reflected triangle correspondence records

SurfaceReflectionJob now exposes a row-major 48-byte ReflectionHit storage buffer written by the actual reflection ray query. Records contain world hit XYZ/ray distance, instance index/custom data, geometry/primitive indices and triangle barycentrics with explicit validity. Each invocation clears its record before every miss/background/invalid rough-sample early return, avoiding retained hit identity. Records capture geometry independent of emission lookup validity. Instance indices are frame-local: stable scene/epoch matching is still required before previous-pose reconstruction. This is correspondence output, not complete reflected-hit motion or temporal denoising.

GGX reusable jobs retain equal-sized hit storage alongside radiance/distance, and resize allocates a replacement; old readers must execute before reused writes. Capacity validation includes three storage buffers and checked hit-buffer limits. The Metal probe now reads all 16 records for each roughness/seed case, compares hit distance with its independent CPU reference, reconstructs world XYZ from triangle barycentrics and validates cleared miss records and same-sized storage reuse. An initial check found the probe had no direct bytemuck dependency; readback decoding now uses explicit little-endian words without adding a dependency. Current GPU probe build/run remains live at this note.

The current-source locked/offline no-default-feature ray probe built and completed on Apple M4 Max Metal. Reflected-hit correspondence assertions passed all 16 pixels across nine roughness/seed combinations in each of the blocked/unblocked fixtures, alongside retained radiance/distance/lighting/primary-motion/background/TLAS checks. Final RAY SMOKE PASS was emitted. This establishes actual GPU record output, barycentric world reconstruction, miss clearing and storage reuse for these fixtures; previous-frame hit motion, animated-instance correspondence and native presentation remain outstanding.

### Previous deformed reflected-hit world correspondence

ReflectionCorrespondencePipeline prepares an immutable sorted table of PreviousReflectionTriangle values keyed by current hit instance index/custom data/geometry/primitive. Each triangle supplies previous WORLD vertices in unchanged topology/order. The GPU compute pass performs binary search and barycentric reconstruction, producing row-major previous XYZ with W=1 only for valid correspondence. Misses, unmatched full identity and explicit reset write zero. Nonfinite barycentrics/interpolated output are rejected in shader; duplicate input keys and nonfinite vertices are rejected before GPU resource creation. Device ownership, storage and dispatch limits are checked. Empty tables have a dummy binding with zero logical count. The caller must remap stable scene identities to current indices and omit new/topology-changed triangles across scene epochs.

The Metal proof now includes previous triangle deformation (independent vertex XYZ changes), unsorted table input, a decoy instance identity, reset, duplicate rejection and nonfinite geometry rejection, reading three actual compute outputs per pixel. Compile and hardware execution remain live at this note. These are previous reflected world points, not ready-made screen motion: reflecting-surface/camera reprojection and temporal acceptance still require integration.

The final locked/offline no-default-feature Metal ray probe built and completed on Apple M4 Max with RAY SMOKE PASS. All previous-deformed-position, reset, unknown-instance and invalid-table assertions executed for 16 pixels across nine roughness/seed cases in both blocked/unblocked fixtures. Existing ray lighting/HDR/primary motion/TLAS checks also passed. Native check and targeted new-module formatting passed. This proves previous WORLD correspondence for the tested geometry; reflection screen reprojection, temporal filter integration and animated scene-table lifecycle remain outstanding.

### Perfect planar reflection screen reprojection

PlanarReflectionPipeline projects current/previous reflected virtual world points using current/previous normalized world planes and unjittered world-to-clip cameras. It emits RG32 backward top-left UV, R32 expected previous virtual NDC depth and R32 current virtual NDC depth. The current virtual projection must agree with primary pixel coverage within 0.01 pixel; invalid hit/coverage/depth writes zero. Missing previous correspondence or offscreen/behind-camera projection rejects history while retaining valid current depth. This path is specifically perfect opaque planar mirrors; curved or rough reflection reprojection remains a separate required path, and accidental projected coverage agreement is not a material classifier.

The outputs feed the existing TemporalResolve and a separate reflection TemporalHistory; primary/direct history must not use virtual reflection depth. The planar_reflection GPU fixture uses controlled hit/previous-position inputs at 9x3 (workgroup edge coverage), geometry deformation/translation, camera translation and mirror-plane translation. It reads guides, blended color and actual pending history depth copy, comparing 27 pixels to independent analytical values. It does not call presented or claim window presentation. Native library check passed; executable GPU run is live at this note.

The initial example compilation caught the wgpu v30 error-scope guard API (no Device::pop_error_scope); the example now retains the returned guard and awaits guard.pop. The final locked/offline planar_reflection example built and ran on Apple M4 Max Metal, emitting PLANAR REFLECTION PASS after all 27-pixel guide/depth/temporal/pending-depth-copy comparisons and an empty GPU validation scope. New-module/example formatting passed. The existing TemporalResolve and TemporalHistory APIs were used directly, with no substitute filter or synthetic presentation commit. Actual ray-produced planar-frame integration, presentation lifecycle and rough/curved reflection reprojection remain outstanding.

### Production mirror ray to temporal resolve integration

RayLightingFrame exposes its exact SurfaceReflectionJob via reflection_job, retaining production ownership instead of constructing substitute hits. PlanarReflectionPipeline::prepare_reflection derives dimensions from that producer and checks both ray/correspondence job device identities; correspondence jobs retain their device for this check. Existing raw-buffer preparation remains available with its documented same-device contract.

The specular probe now encodes production mirror ray -> previous triangle correspondence -> planar virtual-point guides -> existing TemporalResolve, then pending reflection-history depth copy. It reads actual RGBA16 ray radiance and checks blended RGBA32 RGB against that independently captured current value and controlled history. The previous triangle is translated by 0.2 world units: analytic backward UV is 0.05 for this perspective/mirror fixture. Reset correspondence must reject history while keeping pending current virtual depth. Both blocked/unblocked direct-light cases use the same mirror verification. The hardware build/run is live; no new acceptance is claimed yet.

Before compilation started, the actual-ray fixture was strengthened to a 1.0-world-unit previous triangle translation, giving 0.25 UV (one pixel at width four). History now has spatial/channel gradients, so the resolve must read the adjacent previous pixel rather than merely blend a constant texture. The last column's offscreen history must be rejected while retaining current virtual depth. The earlier 0.2/0.05 description above is superseded by this final fixture.

The initial probe compilation required explicit usize readback-offset closure parameters; corrected before execution. The final locked/offline Metal ray probe completed on Apple M4 Max with RAY PLANAR TEMPORAL PASS in both direct-light cases and final RAY SMOKE PASS. Each case checks 16 actual mirror ray hits/radiance, one-pixel previous-geometry UV, spatial/channel-gradient history lookup, virtual depths, offscreen last-column rejection, reset rejection and pending history depth copy. Targeted new-module formatting passed. Prior geometry/history are controlled fixtures, while current hits/radiance come from production reconstruction/ray shading. Multi-frame animated scene history, successful presentation and rough/curved reflection paths remain outstanding.

### Shared last-presented skeletal source for reflected geometry

PreparedSkinnedFrame::previous_reflection_triangles now derives previous WORLD triangle vertices from the exact SkinnedMotionHistory snapshot already used for raster deformation motion. Keys take current instance/custom data/geometry and a checked primitive offset. First frame/reset returns an empty table, preserving explicit invalid history. The API does not introduce another pose owner or advance on preparation: skipped candidates retain the last committed pose. It validates triangle-list correspondence/finite previous vertices and primitive-index overflow. Caller still supplies compatible current TLAS identity/topology and resets consumers on scene/pose history invalidation.

The existing captured-pose regression now checks reflected triangle positions/keys, first-frame emptiness and preservation across a skipped pose; malformed/nonfinite data and primitive overflow checks are included. The actual Metal planar ray fixture now obtains its previous geometry from this API after a modeled prior successful-present acknowledgment, with a different skipped candidate between prior and current. The fixture remains offscreen: modeled acknowledgment verifies ownership logic but does not establish real presentation. Focused CPU tests and updated ray GPU probe are live at this note.

The updated locked/offline Metal ray probe completed on Apple M4 Max, with RAY PLANAR TEMPORAL PASS in both direct-light cases and RAY SMOKE PASS. Previous triangle input came from the shared prepared skeletal snapshot; the different skipped candidate retained the same previously acknowledged pose. Initial focused captured-pose tests passed. A final focused repeat including additional primitive-overflow/nonfinite correspondence assertions is live; final acceptance for those added assertions remains pending.

The final locked/offline focused skeletal-history repeat completed successfully: both tests passed, including reflected triangle identity/previous-pose preservation and the added primitive-overflow/nonfinite-data assertions. The new PreparedSkinnedFrame implementation region and GPU fixture were formatted without formatting unrelated large modules. Actual multi-frame native presentation and rough/curved reflection filtering remain pending.

### Two-dimensional reflected correspondence dispatch

ReflectionCorrespondenceJob now dispatches 8x8 workgroups over the source image rather than flattening every pixel into a single dispatch axis. Uniform width/height define row-major indexing and explicit partial-workgroup bounds. Preparation checks both dispatch dimensions, u32 indexing, texture extent and buffer capacities; storage-buffer limits still apply independently and no 4K memory/performance acceptance is implied. prepare_hits exposes the same validated stage for raw row-major ReflectionHit buffers, while production prepare retains typed producer device checks.

The 9x3 planar fixture now encodes GPU previous-triangle correspondence before reprojection/temporal resolve. It uses reversed per-pixel-key input to exercise sorting/search, omitted correspondence and multiple rows/workgroup edge bounds. The actual ray probe is also being repeated after the shared correspondence shader/dispatch change. Both runtime handles are live at this note; final acceptance is pending.

Both final locked/offline GPU runs completed on Apple M4 Max Metal. The 9x3 fixture emitted PLANAR REFLECTION PASS after GPU correspondence/reprojection/resolve numerical checks and an empty validation scope. The production ray probe emitted RAY PLANAR TEMPORAL PASS for both lighting cases and RAY SMOKE PASS, retaining the sampled GGX/deformation/miss/reset checks. Targeted formatting passed. This closes the shared shader/dispatch regression for tested dimensions; actual 4K buffer allocation/throughput and native window presentation remain unverified.

### Portable Linux planar reflection temporal acceptance and GL format repair

The planar example now selects an explicit --backend metal/vulkan/gl/dx12 with no API fallback and requests adapter limits. tools/linux/planar-reflection-smoke.sh performs a locked/offline build in the existing isolated Linux image and requires both the matching adapter backend and numerical PASS marker, retaining per-backend logs. The container does not contain rg; the runner uses available grep for those checks after an initial post-Vulkan tooling failure.

Vulkan numerical checks passed, while GL initially returned zero guides. Moving validation-scope reporting before readback assertions exposed the actual pipeline error: GL does not support WriteOnly RG32Float storage. PlanarReflectionPipeline now uses RGBA32Float motion output on GL and RG32Float elsewhere. TemporalResolve accepts RGBA32 motion and reads only XY; the fixture uses actual motion format when decoding. No reprojection math or history rules were relaxed.

The final Linux runner completed successfully on Mesa llvmpipe Vulkan and desktop OpenGL 4.5. Both executed GPU correspondence -> planar reprojection -> temporal resolve and all 27-pixel guide/blend/pending-depth numerical cases with an empty validation scope. This is software-driver acceptance, not physical Linux GPU, hardware ray queries or native presented animation. A current-source explicit Metal regression is live awaiting the shared native build directory. Existing deprecation/Send recursion warnings remain unrelated to this repair.

The final explicit --backend metal repeat built and ran successfully on Apple M4 Max with PLANAR REFLECTION PASS after all numerical comparisons and an empty validation scope. New-source formatting and Linux runner syntax passed. The current pipeline is now verified on Metal and software Linux Vulkan/GL for these sampled controlled inputs.

### Ordered planar reflection temporal frame API

PlanarTemporalPipeline caches correspondence, virtual-point reprojection and the existing temporal resolver. prepare accepts the exact SurfaceReflectionJob, prior triangles/planes/cameras and a separate mutable TemporalHistory. Device identity is checked; invalid history forces both correspondence/reset rejection and resolve reset. PlanarTemporalFrame exclusively borrows history until completion, owns all prepared jobs and encodes correspondence -> guides -> resolve -> pending depth copy in order. It exposes filtered reflection output for composition/display. It supports perfect planar mirrors only; rough/curved surfaces remain outside this reprojection model.

A second encode is rejected. Dropping a candidate or finish with a skipped outcome preserves readable history. Presented before encoding is rejected without commit; only encoded Presented commits the pending pair. The caller must pass the actual host outcome from the submission displaying that output; this API cannot infer presentation from command encoding alone. The ray fixture adds modeled outcomes for unencoded rejection, duplicate encoding, skipped cancellation, encoded commit and dropped candidate retention, plus readback of the first wrapped output to ensure invalid history is not sampled. Modeled offscreen outcomes are logic tests, not native presentation evidence. Native library check passed; Metal GPU probe is live at this note.

The final locked/offline Metal ray probe completed on Apple M4 Max with ordered-frame lifecycle assertions, wrapped first-frame numerical readback, both RAY PLANAR TEMPORAL PASS cases and RAY SMOKE PASS. Native library check and targeted formatting passed. This establishes the integrated frame ordering and modeled commit/discard rules; physical displayed animation and host-outcome-driven runtime use remain unverified.

### Native integrated mirror demo, controls and conservative rough filtering

The planar_scene example now combines the shared SceneSurface/FrameLoop, moving emissive geometry and RayScene, texture/material raster guides, ray lighting, compute reflection filtering, HDR composition/tone mapping and common SceneRenderer sprite overlays in one candidate graph. Three controls use WindowUi pointer/keyboard routing: pause, reset and planar temporal filtering. Input regions are published only for actual host Presented outcomes; resize, occlusion, suspension and resets invalidate input/history. G cycles roughness and C switches to a tessellated curved reflector. Static guide geometry is retained until the surface/material changes. Only the emissive triangle is in the ray acceleration; this is a single-reflector demonstration, not general complete-scene ray tracing.

ReflectionSpatialPipeline is a current-frame 3x3 HDR estimator using production primary surface and ReflectionHit buffers. It rejects incompatible reflected instance/custom-data/geometry, local tangent-plane separation, normals, roughness and ray distances; perfect mirrors and invalid primary surfaces bypass it. Stochastic ray misses on valid rough primary surfaces contribute zero radiance to the same spatial average, including missed centres. Hit identity/distance compatibility applies when both samples hit. Invalid/nonfinite guides/samples are rejected, and incremental convex blending avoids HDR sum overflow. Typed preparation checks producer device/extent ownership; raw preparation validates schemas/capacities and leaves raw device checking to wgpu. This is biased spatial smoothing, with no universal image-quality or performance claim. Rough/curved modes never commit or sample the perfect-planar temporal history. Their general reflection-motion temporal denoising remains outstanding.

The standalone Metal numerical fixture passed 135 channel references on M4 Max, including compatible curved normals, independent noisy-HDR weighted averages, geometry/plane/normal/material/distance/nonfinite rejection, stochastic miss smoothing, perfect-mirror bypass and numerical right/bottom partial-workgroup output checks. The combined demo passed three private-target GPU candidates for a perfect planar mirror, roughness 0.2 and a curved roughness-0.2 reflector: finite emissive HDR centre, exact white display UI pixel and empty GPU validation scope. Offscreen candidates use skipped outcomes and must leave presented pose/reflection history uncommitted. These checks still create a native window/device but never acquire or present its surface.

A deliberate actual device.destroy test passed: the device-loss callback was detected before new GPU preparation, resources were recreated on the same window, history was invalidated and the graph resumed with exactly one recreation. The native-window resize check passed after AppKit accepted 644x438 for a 643x437 request from 1280x960. It re-rendered/read back the graph and UI at the accepted dimensions, and zero-size SceneSurface suspension returned Suspended without calling the encoding closure. This is GPU resource/native-size-event acceptance, not displayed resize or native minimization acceptance. Smoke/check deadlines now also fire while minimized/zero-sized, and retry wakeups restore Wait rather than retaining an expired WaitUntil.

Native presentation remains unresolved: the new mirror bundle and an additional always-on-top diagnostic both timed out with zero presentations. The installed AppKit NSWindow.h confirms NSWindowOcclusionStateVisible is 1UL << 1, so the existing Metal occlusion guard mask is correct; no guard bypass/vendor mutation was made. Visibility on the current Space versus AppKit occlusion still needs live confirmation. Full displayed animation, minimize/restore, actual UI activation and surface-loss recovery are not established by the private-target tests. Device-loss recovery has actual GPU acceptance as recorded above.

The final curved rough scene PNG was captured from the private GPU display target at target/planar-curved-offscreen.png and inspected visually. A matched capture before stochastic-miss smoothing is retained at target/planar-curved-before-spatial-misses.png. On display-RGB crop [400,300]-[900,700], the average absolute adjacent-pixel difference (mean of horizontal/vertical differences, channels normalized from 8-bit to 0..1) changed from 0.0555150326452699 to 0.008328721861302504. Display mean RGB also changed from [0.8885751764698505,0.8503877450979896,0.8177292156862969] to [0.9381959215681925,0.8731460588234339,0.8217125098039367]. This is one display-image smoothness comparison, not temporal ghosting acceptance, unbiased radiance accuracy or a general noise/performance benchmark. Residual grain remains visible near the reflector edges. The final 135-reference numerical repeat and rebuilt VoxyPlanar local bundle completed successfully; source formatting/scoped whitespace checks passed.

Current renderer GPU qualification (2026-10-06): all 162 release library tests
pass with ignored tests explicitly enabled and one test thread. Physical GPU
checks cover skeletal/legacy raster and normal transport, LOD shared streams,
reference rig interpolation/mirroring, multi-view/X-ray and film thickness/near
clipping on the current Apple M4 Max/Metal adapter. The ignored CPU admission
profile is executed too; it does not establish frame rate.

The multi-view readback now additionally rejects malformed WGSL and a valid
module with a missing vertex entry while retaining the accepted custom shader,
revision and MSAA variants. A subsequent GPU render must still show the custom
color permutation. Module and pipeline errors return diagnostics without
replacing the healthy pipelines. Evidence, explicit ignored-test inventory,
Metal capabilities/device limits and source digest:
`artifacts/renderer-current-gpu-2026-10-06/`.

Adapter features remain distinct from enabled device features. This result does
not qualify native presented frames, NVIDIA CUDA/RTX, DirectX12/other devices,
all lighting/material/scene combinations, or complete graphics parity.

### Inverse bind validation (2026-10-06)

Skeleton construction now rejects finite but projective or singular inverse
bind matrices before a skin palette can be published. The homogeneous row
must encode an affine map `(0, 0, 0, h)` with finite nonzero h; all columns are
divided by h before storage, giving canonical `(0, 0, 0, 1)`. The canonical
linear determinant must be nonzero.
The determinant uses f64 for the f32 input coefficients, avoiding determinant
underflow for uniformly small valid bind scales. Reflections remain valid.

All 226 animation tests passed, including rejected zero-scale/perspective
matrices and admitted scales from 1e-20 to 1e20 and reflected transforms.
This does not qualify every ill-conditioned matrix, external skeletal importer
or GPU/editor runtime behavior.

Final qualification: 718 animation/application/gameplay tests passed in total;
28 application tests were ignored. The editor all-target release check and
formatting/diff checks passed. Evidence and source hash are saved in
`artifacts/rig-inverse-bind-validation-2026-10-06/`. Ignored tests do not count
as proof of hardware or runtime coverage.

### Computed homogeneous inverse-bind compatibility (2026-10-06)

A 256-transform regression exposed that a normal TRS matrix inverse can have
homogeneous w=0.99999994 after floating-point inversion. Requiring the input w
to equal one rejected that otherwise affine bind. Skeleton construction now
canonicalizes every affine homogeneous inverse bind by dividing all columns
by its finite nonzero w, preserving the homogeneous map. Source perspective
components are checked before division, so underflow cannot hide perspective.
Unrepresentable canonical matrices and singular linear maps still reject.

Tests cover computed TRS inverses, positive and negative homogeneous scale,
point-map equivalence with homogeneous projection, hidden perspective and
canonicalization overflow. Existing unit-w binds retain their representation.

Final qualification of homogeneous compatibility: all 720 animation,
application and gameplay tests passed; 28 application tests were ignored.
Editor all-target release, formatting and diff checks passed. The initial
reproduction log and repaired-state evidence/source hash are saved in
`artifacts/rig-homogeneous-bind-compatibility-2026-10-06/`. External rig import
and visual/GPU runtime qualification remain separate unfinished work.

### Bind-pose hierarchy preflight (2026-10-06)

Skeleton construction now evaluates its bind pose using the existing shared
double-precision skin-matrix evaluator. Finite local transforms can overflow
through a parent chain or when combined with inverse binds; these initial
failures now return `InvalidJointTransform` for the offending joint before
the skeleton is published. No duplicate hierarchy evaluator was introduced.

Double precision is intentional: rigs whose global matrices exceed f32 but
remain finite in f64 still support wide kinematics. Narrow skin-palette
publication keeps its separate range gate. Regression tests cover global and
palette overflow, valid wide rigs and runtime overflow from a later pose on
an otherwise valid skeleton.

All 229 animation tests and 220 gameplay tests passed; application/editor
all-target release checks and formatting/diff checks passed. Evidence and
source hashes are in `artifacts/rig-bind-pose-preflight-2026-10-06/`. External
import and visual/GPU runtime qualification remain unfinished.

### Palette underflow and scale-independent rank checks (2026-10-06)

Bind validation, narrow skin-palette evaluation and double-precision global/
palette evaluation now share a column-scaled linear rank check. Each column
is divided componentwise by its maximum absolute entry solely for validation;
stored transforms are not altered. This avoids overflowing/underflowing the
determinant of an otherwise representable uniformly scaled matrix. Zero or
linearly dependent columns reject. Componentwise division preserves nonzero
subnormal columns without forming an overflowing reciprocal.

A ten-joint chain with scales 1e-30 produces a representable wide matrix near
1e-300 and remains valid; its narrow palette rejects. An eleven-joint chain
underflows to zero and rejects during skeleton construction at joint 10. A
1e-320 subnormal matrix also passes the scaled check. No epsilon scale clamp
or replacement transform is introduced. Arbitrary ill-conditioned matrices
are not certified by this floating determinant check.

All 230 animation tests and 220 gameplay tests passed; application/editor
all-target release checks and formatting/diff checks passed. Evidence and
source hashes are saved in `artifacts/rig-palette-underflow-2026-10-06/`.

### Exact rank predicate for stored rig matrices (2026-10-06)

A regression showed that independently dividing columns can change exact
dependency through rounding: integer columns `(1,2,3)`, `(4,5,6)`, `(5,7,9)`
were wrongly accepted although the third is the sum of the first two. The
previous normalized-column rank test is therefore replaced by `linear_rank`.

Well-separated finite normal triple products use a conservative floating
filter. Remaining cases decompose stored f64 coefficients into exact signed
dyadic mantissas/exponents and add all six triple products in fixed 100-limb
positive/negative sums. This covers the complete finite f64 coefficient range
without underflow, overflow, division or alteration of the matrix. The result
classifies exact zero for stored coefficients, not conditioning or authored
mathematical intent before rounding.

Tests qualify the dependent-column reproduction, one-ULP near dependency at
unit/large/small scales, extreme/subnormal entries and 2048 independent integer
determinants. All 233 animation tests and 220 gameplay tests passed, as did
application/editor all-target release and formatting/diff checks. Reproduction
and final qualification evidence/source hashes are in
`artifacts/rig-exact-linear-rank-2026-10-06/`. GPU/editor visual and external
import qualification remain unfinished.

### Full character owner playback qualification (2026-10-07)

The pinned CesiumMan GLB now passes through the editor's actual ModelPlayback
owner for 240 frames at 60 Hz, crossing two authored clip cycles. Every frame
validates the full skin palette and deforms the entire mesh (over 1000
vertices), requiring finite output and actual motion. A second owner shares
the same imported asset at zero speed and retains identical mesh positions.
Eight deliberately rejected publication attempts are retried and compared
exactly against a control owner, proving no clock drift on this fixture.

The targeted test and complete model playback test group passed. Logs and
source hash are in `artifacts/character-owner-playback-2026-10-07/`. This is
CPU owner playback and mesh deformation evidence; it does not establish
editor visual quality, GPU rendering of the character or all external rigs.

### Current full character GPU capture (2026-10-07)

The existing body_motion_snapshot entrypoint completed its full two-second
CesiumMan clip on Apple M4 Max / Metal with 41 saved frames and no GPU
validation rejection. The simulation uses the existing 1/240-second step.
The complete run took 37.361641542 seconds (offline, not realtime).
The image was inspected: the entire neutral character is visible and moves.
The illustrative soft tissue attachment covers only 16 of 3273 skin vertices;
this is not full anatomical attachment qualification. CPU deformation and
GPU rasterization are used, not compute skinning. Captures, animation and
energy receipt CSV are in `artifacts/character-gpu-current-2026-10-07/`.
No editor UI, CUDA or other hardware qualification is inferred.

### Character stage timing (2026-10-07)

The existing capture entrypoint now reports separate wall-time accumulators
for simulation, receipts, mesh deformation, combined upload/encode/GPU
readback, and capture I/O. These counters do not alter the simulation step
or admission limits. A full 480-step CesiumMan run with four checkpoints
completed: simulation 46.063372791 s, receipts 0.000130917 s, deformation
0.006581708 s, upload/encode/readback 0.050648625 s, capture I/O 0.000403708 s.
Total was 46.121587208 s. Simulation dominates this fixture; these are host
wall times, not GPU timestamps or a comparison establishing a speedup.
The prior 41-capture run is a different capture schedule and host timing.
Evidence and source hash: `artifacts/character-stage-profile-2026-10-07/`.

### Constitutive profiling and rejected optimization (2026-10-07)

A five-second native stack sample of the full character fixture identifies
Ogden-Maxwell response, strain, matrix multiplication and pow among active
CPU stacks. Sleeping helper-thread samples must not be counted as simulation
CPU time. A candidate reused the strain's already computed volume coefficient
in response; 18 constitutive/dynamic tests passed and the full clip image and
step counts matched exactly. However, the candidate run took 72.211539250 s
and the retained before binary took 58.230919125 s under changing shared-host
load. This does not establish a causal regression or speedup. The candidate
was reverted; the original production material implementation is retained.
Profile, trial logs and rejected patch are preserved in
`artifacts/constitutive-strain-reuse-2026-10-07/`.

### Full clip rejection causes (2026-10-07)

Opt-in `VOXY_REFINEMENT_DIAGNOSTICS` now aggregates original errors by
refinement depth on the calling simulation thread. Diagnostics are drained
after capture and do not replace admission rules, budgets or state rollback.
The complete 480-step character run recorded 58,416 rejected explicit trials:
all were `finite-deformation inertial support work defect`. Depth counts
were 25, 13, 16, 31, 208, 8702, 23250 and 26171 at depths 0 through 7.
Accepted steps remain 208,168 and maximum depth 8. The four-checkpoint image
SHA256 exactly matches the pre-diagnostic image. No inference is made about
other solver paths or all characters. This fixture motivates comparing an
implicit moving-support integration path under the same work/heat budget,
rather than loosening energy admission. Evidence:
`artifacts/character-refinement-reasons-2026-10-07/`.

### Implicit moving-support mechanics (2026-10-07, qualification in progress)

A new support-only implicit midpoint path reuses material path evaluation
and existing limited-memory secant algebra without manufacturing an external
contact surface. It validates complete support targets, continuous volume and
gap feasibility, inertia-scaled residual and impulse work, and independent
endpoint work balance. Support reaction and pin kinetic work remain separate;
unrepresentable summed work rejects before publication. The Maxwell/thermal
transaction now exposes this path with rollback of the complete candidate.

All 20 material/inertial tests passed, including driven supports with a free
node, failed/inverted-target rollback and independent analytic free-body
ballistics. The demo offers opt-in `VOXY_IMPLICIT_SUPPORTS` for comparison;
default integration is unchanged. Full CesiumMan capture was started and
remains unqualified until terminal results are inspected. No speedup or
production-readiness claim is made. Test evidence and source hashes are in
`artifacts/implicit-moving-supports-2026-10-07/`.

Full implicit character qualification completed: all 480 source steps, 2067
accepted mechanical substeps, 147 rejections and maximum depth 1. Rejections
were 24 line-search failures and 123 nonlinear nonconvergence cases. Total
time was 210.253948625 s. Fewer substeps do not establish a speedup: this
run is slower than prior explicit captures. All 21 material/inertial tests
passed, including temporal refinement convergence. The nonlinear search
currently preconditions only with inertia; material stiffness must be
considered before promoting this path. Final logs and image are retained
in the same artifact directory.

### Material-aware implicit support search (2026-10-07)

The support-only implicit solver now uses inertia plus the existing rest
material stiffness diagonal for its search metric, with the shared secant
Rayleigh scaling. The inverse is formed with scaled sums to avoid overflowing
an otherwise finite inertia/stiffness combination. Admission continues to
use the original inertia-scaled residual, impulse work, path checks and
endpoint work budget. No material or energy law was changed.

The first search-metric trial exposed free-body ballistic position error
against the analytic test. Initial free midpoint displacement now includes
constant acceleration (dt*v/2 + dt²*a/4). The same strict ballistic test
then passed, together with all 21 constitutive/inertial tests and temporal
refinement convergence. The original failed test log is retained.

The full 480-step CesiumMan clip completed in 23.685676458 s: 1934 accepted
substeps, 10 rejections, maximum depth 2. Prior inertia-only implicit run
was 210.253948625 s with 147 rejections. These are individual shared-host
runs, not a broad benchmark or realtime qualification. Output was visually
inspected; trajectories can differ between numerical integration choices.
Default demo mode is unchanged. Final evidence and source hashes:
`artifacts/material-preconditioned-supports-2026-10-07/`.

### Complete-character motion accuracy audit (2026-10-07)

The existing capture can now export every continuum node's position and
velocity plus energy receipts via `VOXY_CAPTURE_NODE_STATE`. Motion CSV is
also saved beside the output image. `VOXY_SUPPORT_MIN_DEPTH` (0..8) sets a
minimum mechanical subdivision for diagnostic refinement; the source clip
and outer support sampling remain fixed at 240 Hz. Default is unchanged.

Seven complete implicit captures (minimum depths 0..6) and a complete
explicit capture were compared at 0, 0.5, 1 and 2 seconds, all 76 tissue nodes.
Early adjacent refinements did not converge monotonically despite closing
energy receipts. At the final checkpoint depths 1/2 differed by 2.842846 mm
and 1.293559 m/s. At depths 5/6 differences fell to 0.346359 mm and
0.133274 m/s. Finest implicit versus explicit was 0.278649 mm and
0.078741 m/s. This is motion accuracy evidence, not a continuous-path bound
or production qualification at an established application tolerance.

Depth 6 took 29.724654583 s (122880 accepted, no rejected steps); explicit
took 29.767419458 s (208168 accepted, 58416 rejected). These individual
shared-host measurements do not establish a general speedup. Fast coarse
implicit runs must not be promoted solely by energy acceptance. The next
admission work must control position and velocity error as well as energy.
Complete numerical traces, counts and comparisons are retained in
`artifacts/full-character-mechanical-refinement-2026-10-07/`.

### Support motion accuracy admission (2026-10-07, full fixture pending)

`step_viscoelastic_implicit_with_support_accuracy` compares a full step with
two half steps at every coarse endpoint along the original linear support
trajectory. It sums the maximum nodal position/velocity differences across
the interval, publishes the finer state, partitions the unchanged energy
budget and independently checks full-interval work/heat. Differences are
raw error indicators, not certified global trajectory bounds. Limits and
invalid inputs preserve coordinates, velocities, Maxwell histories and heat.
Receipt addition is shared with the existing adaptive skin/contact path.

The initial strict motion test exposed premature energy-only nonlinear
stopping. Accuracy calls now also require displacement and velocity
kinematic residual corrections to fit a share of the motion budgets. Tiny
objective changes may use roundoff-sized acceptance only when the residual
decreases; endpoint energy admission is unchanged. Default calls retain
their previous nonlinear criteria. All 25 relevant tests passed: independent
2048-step unforced Verlet reference, 4096-step prescribed-support reference,
invalid-input/limit rollback including thermal state, and existing adaptive
skin/contact interval rollback.

Opt-in `VOXY_SUPPORT_MOTION_BUDGETS=position_m,velocity_m_s` allocates
complete-frame indicator budgets across support segments; source sampling
remains 240 Hz. `VOXY_SUPPORT_ACCURACY_TRACE` records actual segment dt,
committed substeps, all trial calls, measured differences and budgets.
Estimator trial work is separate from admitted-step counters.
The first full-character diagnostic was intentionally stopped (exit 130):
its warm-start depth did not actually try a coarser physical step after
publishing half steps. Accuracy-mode warm start now subtracts two depth
levels; every newly tried step still passes the same motion/energy admission.
The corrected full run is in progress, not qualified. Tests, original
failed/stopped diagnostics and hashes:
`artifacts/support-motion-accuracy-2026-10-07/`.

### Coupled rest-material search metric (2026-10-07, candidate runtime pending)

Implicit moving supports now use the coupled rest isotropic elastic operator plus inertia as the secant base metric. Element displacement gradients produce deviatoric shear and volumetric stresses, with pinned columns and rows excluded. Ogden and Maxwell small-strain shear contributions enter the metric; nonlinear material forces, committed memory, heat and independent endpoint admission continue to use the full original constitutive evaluator. This approximation does not replace anisotropic physical behavior.

The existing contact preconditioned conjugate-gradient iteration was extracted into a shared helper rather than adding a parallel solver. Its diagonal fallback and finite descent check remain. The candidate passed all 14 viscoelastic inertia tests, the independent affine-energy and pinned coupled-system residual check, and the existing rank-one contact inverse check. Full fixture performance and motion accuracy remain unqualified.

The previous diagonal-metric accuracy run was intentionally interrupted (owned session 10542, exit 130) after 4090 committed diagnostic intervals totalling 0.077783203125 s. Maximum observed position and velocity indicator budget fractions were 0.9927346076639307 and 0.996589059533212. This is partial evidence only. A replacement full CesiumMan run uses the same 1e-7 m and 1e-4 m/s per-frame indicator budgets, session 5940; its build/runtime result is pending. Evidence is retained in artifacts/coupled-material-search-2026-10-07.

The candidate compiled and began emitting admitted full-fixture intervals (session 5940 remains live). The existing adaptive implicit Maxwell parallel-contact and atomic-limit regression also passed. Candidate partial measurements are timestamped in result.json; no full trajectory or speed claim is admitted yet.

Expanded candidate validation completed with exit 0: physics library 112 passed and 7 ignored, viscoelastic constitutive integration 10 passed, viscoelastic inertia integration 14 passed. A two-second sample of the active full-fixture main thread observed 1537 of 1640 samples under path material evaluation; this short observation is retained as profiling evidence, not a full-run timing claim. Full accuracy runtime remains live and unqualified.

### Streamed character checkpoints (2026-10-07)

VOXY_CAPTURE_NODE_STATE now writes and flushes each committed checkpoint to a nodes.jsonl sidecar while retaining the final nodes.json aggregate on successful capture completion. This makes already captured states available without waiting for the full trajectory; partial files do not imply full qualification. The existing CesiumMan GPU example with --capture-steps=3 completed with exit 0 and emitted four 76-node checkpoints at the unchanged 240 Hz clock. Every streamed JSON row exactly matched the final aggregate. Evidence: artifacts/streamed-character-checkpoints-2026-10-07. The live coupled-metric full run predates this diagnostics edit and continues under its original binary.

Full coupled-metric session 5940 and owned PID 70977 were absent on the next authoritative revalidation; its temporary log was also missing. No terminal result is available, and previous running status must not be interpreted as completion. Persisted partial evidence and tests remain; a fresh full run is required.

### Independent early trajectory reference (2026-10-07, partial candidate)

Fresh candidate prefix session 94511 covers the first 24 original rig ticks (0.1 s), with the same support-motion indicator budgets as the full session 28155. Three independent explicit Verlet prefix captures completed: default adaptation, fixed minimum depth 7 and depth 8. At 0.03333333333333333 s, depth 7 versus depth 8 differs by at most 2.628569598608118e-10 m and 4.715556841654225e-7 m/s across all 76 nodes; candidate versus depth 8 differs by 1.278651659546991e-7 m and 0.00023196873318696073 m/s. The default explicit capture itself differs by 2.687568720715588e-6 m and 0.002579880660649394 m/s, so it is not the qualified reference for this comparison. These are checkpoint differences, not certified errors or a full trajectory result. Both candidate prefix and full capture remain live. Reference files and partial comparisons are retained under artifacts/coupled-material-search-2026-10-07/prefix-comparison.

### Rotating prescribed supports: roundoff diagnosis and correction (2026-10-07)

A new rotation plus translation test at the unchanged strict 1e-10 m and 1e-8 m/s interval indicators exposed refinement exhaustion. Diagnostic trials showed the velocity indicator hitting the budget at counts 1024 and 2048 despite much smaller position differences. Prescribed-node diagnostic velocities are endpoint differences divided by dt, so rounding of the same prescribed path grew under temporal subdivision and was incorrectly accumulated as integration error. The velocity estimator now covers free integrated nodes; the position estimator still covers all nodes, prescribed endpoints remain exact, and the independent work/heat admission is unchanged. The new test compares against both 2048 and 4096 explicit steps, checks free motion, exact endpoints, energy balance and complete mechanical/memory/thermal rollback on exhaustion. All 15 viscoelastic inertia tests passed. Failure, diagnosis and passing logs are retained.

The pre-fix coupled prefix completed with exit 0: 0.1 s of animation took 211.704291375 wall seconds under concurrent host workloads. At 0.1 s it differs from depth-8 explicit reference by 2.0014903119048004e-7 m and 0.0002304134084898288 m/s across all 76 nodes. Depth-7 versus depth-8 reference differences are 3.590215652908454e-8 m and 1.748545221398975e-5 m/s. These checkpoint comparisons demonstrate early trajectory agreement but do not qualify the full clip or real-time performance. Live full session 28155 predates the estimator correction.

### Search cancellation scale correction (2026-10-07, full qualification running)

The implicit support search subtracts the initial material energy in every quadrature sample. Its resolved-descent roundoff allowance previously used only the small residual objective and lost the cancellation scale. It now also includes initial absolute energy weighted by the original sum of w/(2t). Epsilon is multiplied before potentially large energies to retain a finite allowance. This allowance is used only when motion controls are enabled and the residual norm strictly decreases; independent endpoint work/heat admission and all accuracy budgets remain unchanged.

All 15 inertia tests passed after the final overflow-safe expression. Two same-budget 0.1 s CesiumMan GPU prefixes completed in 2.180956625 and 2.271928041 s, compared with the recorded pre-fix 211.704291375 s; these are shared-host individual observations, not a general speedup claim. Final prefix states exactly match the first cancellation candidate. At 0.1 s maximum position difference from depth-8 explicit reference is 4.970551019252834e-7 m and velocity difference 0.00022099885479687494 m/s. Maximum observed checkpoint energy closure residual is 9.391558838154547e-15 J. Accepted fine substeps reduced from 8698 to 4202 under unchanged indicator budgets. This is still offline, not real-time qualification.

Old full session 28155 was deliberately interrupted with confirmed exit 130 after the substantive search repair. Current full cancellation-fixed session 2243 and explicit reference depths 7/8 (sessions 98584/37834) are active; outputs and incremental nodes.jsonl remain inside artifacts/coupled-material-search-2026-10-07. Full trajectory comparison is pending.

Both full independent explicit references completed with exit 0: depth 7 took 23.393557458 s (271960 accepted, 26200 rejected leaves); depth 8 took 39.343547583 s (491520 accepted, no rejected leaves). Current candidate session 2243 remains live, with 0.5 and 1.0 s checkpoints. At 1.0 s candidate versus depth 8 differs by 2.5236141248596375e-6 m and 0.0049380496028146315 m/s; depth 7 versus depth 8 differs by 3.6905433910692706e-7 m and 0.0005848605039959032 m/s. These full-character partial comparisons reinforce that local indicator budgets are not global accuracy certificates.

### Complete implicit support accuracy capture and shared explicit controller (2026-10-07)

Full cancellation-fixed implicit capture completed with confirmed exit 0 in 117.109676291 wall seconds for 2 s of CesiumMan motion. All 55588 committed estimator intervals satisfied their local indicators, using 207320 accepted fine steps and 455028 trial calls. At the 2 s checkpoint all-node differences from the depth-8 independent explicit reference are 7.950583048099286e-5 m and 0.023256453627126916 m/s. Depth-7 versus depth-8 reference differences are 1.0806504317994608e-5 m and 0.003360383675582553 m/s. Maximum checkpoint energy closure residual is 5.433996650250719e-14 J. The full comparison, source hashes, images and traces are retained in full-comparison.json. This establishes the full fixture outcome and remaining accuracy/performance limits, not real-time readiness or a continuous global error certificate.

The support accuracy transaction is now shared by both implicit midpoint and explicit velocity Verlet through one const-generic owner method. The existing implicit public API retains its behavior; step_viscoelastic_with_support_accuracy exposes explicit mechanics with the same coarse/fine indicators, summed energy budgets, exact prescribed targets and whole-interval history/thermal commit. Explicit path-collapse and work-defect trials refine under the same finite limit. The rotating support test now qualifies both variants against 2048/4096 independent explicit steps and checks exact endpoints plus failed-limit rollback. All 15 inertia tests passed. VOXY_EXPLICIT_SUPPORT_ACCURACY selects the explicit alternative only with VOXY_SUPPORT_MOTION_BUDGETS; default demo behavior stays unchanged. Same-budget explicit prefix runtime is pending in session 68077.

The explicit accuracy prefix completed with exit 0 in 8.161380708 s, using 7628 accepted fine steps. It was slower than the recorded implicit prefix (2.271928041 s, 4202 steps), so no explicit speedup claim is admitted. At 0.1 s its differences from depth-8 reference are 4.728982396167896e-8 m and 3.5544111616365765e-5 m/s, smaller than the implicit prefix. Identical raw local indicators therefore do not imply identical global accuracy across integrators. The explicit option remains an alternate accuracy/cost tradeoff rather than replacing the default. Full same-budget explicit capture is now starting; result is pending.

### Completed integrator/reference comparison (2026-10-07)

The explicit accuracy full clip completed with exit 0 in 145.018263333 s: 428986 accepted fine steps, 718069 trial calls, maximum depth 9, all local indicators admitted. Its checkpoint energy closure residual is at most 2.4275931339902423e-13 J. The opt-in minimum mechanical depth diagnostic now allows 0..10 within the unchanged depth-12 mechanical cap, enabling a depth-9 independent reference without altering the 240 Hz rig clock or defaults. That reference completed with exit 0 in 123.750864250 s (983040 accepted steps, no rejected steps).

Against depth 9 at the 2 s endpoint, implicit accuracy differs by 7.671713980281915e-5 m and 0.02235791782324781 m/s; explicit accuracy differs by 1.0892910622442865e-5 m and 0.0030211570389216745 m/s. The depth-8/depth-9 reference difference is 2.79423641945123e-6 m and 0.0009108757550130546 m/s, approximately one quarter of the previous adjacent reference position difference. Comparison includes all 76 nodes at 0, 0.5, 1 and 2 s. Final data is in integrator-comparison-depth9.json. All captures are complete; no live handles remain from this qualification. These are numerical convergence and checkpoint results, not real-time, continuous-error or anatomical full-skin qualification. The broader engine goal remains incomplete.

### Reuse admitted material metric (2026-10-07, full runtime pending)

The Ogden exponent-2 energy now uses the already computed isochoric metric Cbar instead of rebuilding F-transpose F inside the stable invariant. Existing callers without a metric retain the original wrapper and computation. The determinant power, strain law, material parameters, force derivative and all admission budgets remain unchanged. This is separate from the earlier rejected determinant-power reuse candidate.

The existing 1000-step viscoelastic CPU benchmark was run with copied before/after executables in ABBA order three times (18 samples per variant). Median measured times were 0.020039604 s before and 0.0187483125 s after, ratio 0.9355630231016542; all logged physical outputs matched. This is a single controlled workload on the current host, not a full-engine or real-time speedup claim. Material/inertia tests passed 25, other biomechanical/myocardial invariant users passed 17, total 42. The same 0.1 s CesiumMan explicit-accuracy GPU prefix completed and its four 76-node checkpoint JSON objects (including energy receipts) exactly match the pre-change capture. A full clip is now starting; its outcome remains pending. Evidence and source hashes are in artifacts/material-metric-reuse-2026-10-07.

The material-metric reuse full CesiumMan capture completed with exit 0 in 71.696072167 observed wall seconds. All four 76-node checkpoint JSON objects, including energy receipts, exactly equal the prior full explicit accuracy capture; accepted fine steps remain 428986 at depth 9. This verifies checkpoint identity for the complete 2 s fixture. The wall time differs from the prior shared-host run, but these were not paired full runs; only the ABBA CPU benchmark supports the scoped 6.4 percent measurement. Real-time and continuous trajectory certification remain unproved.

### Share validated deformation during material memory update (2026-10-07, GPU prefix pending)

Ogden-Maxwell response now has an internal response_at_strain evaluator. Public response still validates dt and computes the admitted strain before evaluating force and energy. Backward-Euler memory advance and exact held-pose relaxation now reuse the same strain for response validation and their memory update instead of recomputing it. The helper remains private so callers cannot provide stale metrics. All former response overflow and history/heat finite guards remain before commit.

All 25 viscoelastic material and inertia tests passed. The existing 1000-step physical benchmark was measured in ABBA order three times, 18 samples per variant, against a copied binary from the metric-reuse baseline. Median time changed from 0.0185416665 s to 0.0175526455 s, ratio 0.9466595410935689; all logged physical outputs matched. This is a scoped additional measurement, not a summed full-engine speedup claim. GPU character prefix is building/running; its result is pending. Evidence is in artifacts/relaxation-strain-reuse-2026-10-07.

The relaxation strain-reuse 0.1 s GPU character prefix completed with exit 0; all four 76-node checkpoint JSON objects exactly equal the preceding metric-reuse prefix, including energy receipts. Observed wall time was 1.216577917 s; this single capture is identity evidence, not a paired full-character speed measurement. Full runtime after this second change has not yet been qualified.

The relaxation strain-reuse full capture completed with confirmed exit 0 in 64.522368833 observed wall seconds. All four 76-node checkpoint JSON objects exactly equal the previous material-metric reuse capture, including energy receipts. Every SUPPORT_ACCURACY_INTERVAL diagnostic line also matches exactly, including all substep/trial counts, difference indicators and budgets. Fine accepted steps remain 428986 at depth 9. This verifies unchanged checkpoint states and estimator decisions over the full fixture; it is not a continuous per-node trajectory certificate. The full timing is an unpaired shared-host observation, so only the ABBA benchmark supports the scoped additional 5.3 percent measurement. No live process remains for this qualification.

### Cache the admitted isochoric scale (2026-10-07, full runtime pending)

The private strain result is now StrainState, carrying the metric, deviator, volume ratio and the already computed J^(-2/3). Response and memory updates reuse the same admitted scalar rather than computing the identical determinant power again. Public APIs, input guards, physical formulas, history/thermal commit and integration budgets remain unchanged. This revisits the old determinant-power candidate using paired measurements rather than its previously inconclusive unpaired character run.

All 25 material/inertia tests passed. Copied before/after executables measured the existing admitted 1000-step fixture in ABBA order three times, 18 samples per variant. Median time decreased from 0.0177030625 s to 0.01628625 s, ratio 0.9199679433996238. All logged endpoint, energy, heat, work and defect outputs match. This supports only a scoped additional 8 percent observation, not a summed full-engine or real-time claim. Full CesiumMan explicit-accuracy capture is starting with unchanged 1e-7 m / 1e-4 m/s per-frame indicators; runtime and full checkpoint identity remain pending. Evidence: artifacts/isochoric-scale-reuse-2026-10-07.

The isochoric-scale reuse full capture completed with confirmed exit 0 in 58.547580167 observed wall seconds. All four 76-node checkpoint JSON objects and every one of the 150308 committed interval diagnostic lines exactly match the preceding relaxation strain-reuse capture, including energy receipts and estimator decisions. Fine steps remain 428986 at depth 9. Full PNG byte identity is recorded in result.json. This verifies unchanged checkpoint/diagnostic behavior for the complete 2 s fixture, not a continuous trajectory certificate. Full timing is an unpaired observation; only the ABBA measurement supports the scoped additional 8 percent benchmark result. Real-time readiness remains unproved.

Повторное использование определителя при инверсии деформации проверено и отвергнуто (`artifacts/material-inverse-reuse-2026-10-07`). В парном ABBA ×3 сравнении существующего теста 1000 принятых шагов (18 измерений каждой версии) медианы составили 0.016379333 с для исходной реализации и 0.0172393335 с для кандидата: отношение 1.0525052. Напечатанные физические результаты совпали; 25 тестов вязкоупругости/инерции и 17 тестов биомеханики/миокарда прошли. Кандидат удалён из исходников, прежняя реализация сохранена. Это измерение локального теста, не оценка скорости всего движка; причина замедления не установлена.

Добавлен `TetraMesh::from_star_shaped_surface(points, boundary, interior)`: общий радиальный построитель теперь допускает вогнутую замкнутую поверхность, если каждая грань видима из заданной внутренней точки. Вершины и исходные грани сохраняются, к каждой грани добавляется тетраэдр с общим центром. Выпуклый API сохраняет прежние ограничения; новый путь дополнительно проходит существующие проверки связности границы и пересечения ячеек. VXTM, привязка кожи и перенос сил используют существующие механизмы. 15 тестов прошли: вогнутая форма с независимым объёмом 0.75 м³/массой 750 кг, виртуальная работа нагрузок, перенос аффинного движения, сериализация, непригодная геометрия и регрессии выпуклых/решёточных объёмов и пересечений. У плотной исходной границы 98 вершин/192 треугольника все вершины привязаны; максимальная ошибка компоненты положения в покое 1.1102230246251565e-16 м. Источник и протокол: `artifacts/star-shaped-tissue-surface-2026-10-07`. Это подготовка звёздных региональных объёмов, не общий тетраэдрализатор и не анатомическая калибровка. Покрытие демонстрационного Cesium остаётся 16 из 3273; новая объёмная сетка персонажа ещё не подготовлена.

Авторский импорт тканей версии 2 теперь принимает `mesh_format: "star-shaped-surface-v1"` наряду с прежним VXTM (по умолчанию). Файл `mesh` содержит JSON с полями `points`, `boundary`, `interior` в системе координат манифеста; `mesh_blake3` фиксирует байты исходной поверхности. Построение использует `TetraMesh::from_star_shaped_surface`, затем прежние владельцы динамики, наблюдения входов, материалы, поддержки и контракт покрытия. Отсутствующие необходимые поля, неизвестные форматы, некорректная геометрия и несовпадающий хеш отклоняются. Отчёт регионов сохраняет фактический формат. Два теста импорта прошли. Сохранённый пример в `artifacts/star-surface-import-2026-10-07/fixture/star-regions.json` прошёл существующий Metal запуск с `--cesium --contact --capture-steps=3`: 0.0125 с моделируемого времени, 12 принятых подшагов, 0 отказов, 17 узлов/28 ячеек и одна исходная вершина кожи (666), 6 физических контактных треугольников. Независимое замыкание изменения механической энергии, работы и тепла в конечной точке 9.907938449419817e-11 Дж; заявленный дефект из него не вычитается. Параметры контрольного объёма не анатомически калиброваны; это интеграционная проверка прямого импорта поверхности, а не квалификация полной кожи персонажа или всего клипа.

Проведён исходный геометрический аудит Cesium через существующий нативный `scene_surfaces64` (`artifacts/character-volume-preflight-2026-10-07`). Экспорт включается только диагностическим `VOXY_SOURCE_SKIN_FIXTURE_DIR` в тесте существующей привязки: сохранены исходные 3273 вершины/4672 грани и проверены точные координатные совпадения на 481 равномерной фазе полного импортированного клипа. 935 дополнительных вершин входят в 654 группы; максимальное расхождение внутри этих групп во всех выборках 0 м. Это выборочное доказательство, не сертификат непрерывной эквивалентности швов; модель не изменялась. При диагностическом точном сопоставлении координат остаётся 2338 геометрических вершин, одна связная замкнутая ориентированная граница без граничных/неманифолдных рёбер и некорректных связей вершин. Для полного исходного контура найден и независимо перепроверен точными рациональными операциями сертификат несовместимости четырёх полупространств (грани 3641, 1192, 4272, 4140): неотрицательные веса дают нулевую сумму нормалей и строго отрицательную сумму смещений плоскостей. Поэтому единого центра радиального заполнения нет для этих нативных координат/ориентации; дальнейшая работа требует регионального разбиения или общего тетраэдрализатора, сохранения соответствия исходным вершинам и полной привязки кожи. Этот аудит не строит объём персонажа; покрытие демонстрации остаётся 16 вершин.

Для общего невыпуклого объёма добавлен `TetraMesh::from_medit_volume`: текстовый геометрический профиль Medit v1, выдаваемый fTetWild (`MeshVersionFormatted 1`, `Dimension 3`, `Vertices`, пустые `Triangles`, `Tetrahedra`, `End`). Однобазовые индексы преобразуются; отрицательный порядок ячейки нормализуется без перемещения координат. Ненулевые ссылки регионов/материалов и неизвестные секции отклоняются вместо потери метаданных. Новый общий `from_tetrahedra` восстанавливает наружные грани и проходит существующие проверки объёма, манифолдности и пересечений; решёточный построитель также использует его. Манифест версии 2 принимает `mesh_format: "medit-volume-v1"` с прежними наблюдениями входов/хешем, материалами и контрактом покрытия. 17 геометрических тестов и один расширенный тест импорта прошли; контрольная текстовая сетка совпала с VXTM по точкам, ячейкам, границе и членству кожи.

Как внешний инструмент подготовки ассетов локально собран [fTetWild](https://github.com/wildmeshing/fTetWild) на коммите `8118f810478e0e65a7bf2d8cecdc5e203a876e97`, MPL 2.0, без TBB. Совместимость установленного CMake 4 с рецептом загрузки Eigen в libigl исправлена только во временной внешней сборке; патч сохранён в `artifacts/general-tissue-mesher-2026-10-07`. Механика движка этим инструментом не заменяется. На вход передан исходный контур Cesium после точного сопоставления совпадающих координат с сохранением отображения всех 3273 исходных вершин в 2338 геометрических (4672 исходные грани); модель рендера не менялась. Запущена подготовка полного объёма с шагом 0.06 м, относительным геометрическим конвертом 1e-6 и отключённым упрощением. На момент записи процесс ещё выполняется; выходная сетка, нативное принятие, полнота привязки, поддержки и динамика ещё не квалифицированы. Поиск RAG не дал релевантных результатов по TetWild (wiki истёк по бюджету); решение и формат проверялись по первичным исходникам.

Добавлена проверка `body_motion_snapshot --check-volume-json VOLUME.mesh`: файл объёма и исходный GLB наблюдаются через существующий `ImportInputs`; сетка проходит нативное принятие, затем все 3273 исходные вершины проверяются/привязываются общим `TetrahedralEmbedding`. Отчёт содержит хеши, принятие объёма, число узлов/ячеек/граней, фактические привязанные и внешние вершины, полноту привязки и ошибку положения в покое. Код выхода 0 допустим только при полной привязке; частичный объём не квалифицируется как готовый. Контрольный регион (1 привязанная вершина, 3272 внешних) проверен: геометрия принята, полнота false, команда завершилась с кодом 1. Неверный файл также получает явный отказ в отчёте; тест импортера прошёл. Это проверка исходной позы, не полного движения или анатомической калибровки.

Первый полный прогон fTetWild остаётся активным (сессия 60651). Профиль процесса показал затраты в `insert_triangles`/`mark_surface_fs` и `sample_triangle_and_check_is_out` при заданном относительном допуске 1e-6. Для проверки другого предусмотренного библиотекой алгоритма конверта подготовлена отдельная сборка `FLOAT_TETWILD_WITH_EXACT_ENVELOPE=ON` с тем же допуском. Конфигурация прошла, первоначальная сборка выявила выбор x86 SSE по разрядности указателя в IndirectPredicates на ARM64. Во временной внешней зависимости исправлен аппаратный guard: SSE включается при поддержке SSE2/x64; иначе используется существующий скалярный интервальный код. Патч сохранён, новая сборка ещё выполняется (сессия 50639). Этот вариант пока не квалифицирован; первый процесс не перезапускался и не останавливался.

Вариант fTetWild с предусмотренным точным конвертом собран на ARM64 (терминальный код 0) после аппаратного guard в IndirectPredicates. Контрольная вогнутая L-образная поверхность из трёх ячеек 0.01 м обработана построителем с кодом 0; полученный Medit объём принят нативным Voxy: 278 узлов, 992 ячейки, 376 наружных граней. Независимая сумма объёмов ячеек 3.000000000000005e-6 м³ совпала с аналитическими 3e-6 м³ до 5e-21 м³. Полный Cesium запуск отдельного варианта начат с тем же исходным контуром, шагом 0.06 м, относительным конвертом 1e-6 и отключённым упрощением (сессия 78136, PID 14292). Он дошёл до оптимизации сетки; промежуточные числа узлов/ячеек ещё включают наружную область и не являются принятым объёмом. Первая выборочная версия тоже остаётся активной (60651/PID 98952). Полная геометрия персонажа, конечный размер файла и привязка всей кожи пока не подтверждены. Протокол, контрольная сетка и нативное принятие сохранены в `artifacts/general-tissue-mesher-2026-10-07`; контрольный регион по-прежнему связывает только вершину 666 модели, что корректно даёт отказ полной проверки.

Неограниченная оптимизация полного точного-конвертного варианта остановлена по явной команде SIGINT после наблюдаемого разрастания до 514013 промежуточных узлов/2928315 ячеек и ухудшения `max_energy` с 51.7343 до 184.282; сессия 78136 терминально завершилась с кодом 130. Это не истечение наблюдения и не успешный объём. Запущен отдельный диагностический кандидат с пределом `--max-its 2`, прежними входом/геометрическим конвертом/шагом. Предел итераций сам по себе не подтверждает качество: требуются конечный результат, нативное принятие и полная привязка. Производственное качество всей сетки остаётся незавершённым; первый выборочный прогон не перезапускался.

Ограниченный диагностический прогон полного объёма завершился с кодом 0 (сессия 73829): 2477 узлов, 7322 ячейки. Нативный `--check-volume-json` завершился с кодом 1 и отказом `nonmanifold tetrahedral boundary vertex`; volume_admitted=false, complete_skin_binding=false. Файл и отчёт сохранены, отказ не ослаблялся. Производственная сетка персонажа остаётся незавершённой. При подготовке Git-доставки первый выборочный построитель остаётся активным; журнал зафиксирован отдельным снимком, активный файл оставлен вне коммита.

Полный объём Cesium: независимый аудит локализовал два неманифолдных узла, в каждом граничный link содержит раздельные циклы из 3 и 4 вершин. Проверены три завершённых с кодом 0 варианта fTetWild: winding по исходному контуру, штатный manifold-surface, general winding по исходному контуру. Все нативно отклонены с кодом 1, тот же дефект сохраняется; связность всего объёма по граням одна. Штатная коррекция экспортируемой поверхности не гарантирует правильную границу ячеек. Локальная проверка исходного OBJ точными рациональными операциями на binary-f64 координатах выявила 12 строгих поперечных пересечений (свидетелей segment/triangle; не обязательно 12 различных пар) среди 167 bbox-кандидатов у 12 исходных граней. Это доказывает локальное самопересечение входной поверхности, но не является полным аудитом всей поверхности или доказательством единственной причины дефекта. Исправление физического контура и независимая полная привязка кожи остаются необходимыми; допуски принятия не менялись. Артефакты: artifacts/character-volume-filtering-2026-10-07.

Первый выборочный прогон (60651/PID 98952) намеренно остановлен SIGINT после доказательства локального самопересечения входа; терминальный код 130, окончательный журнал сохранён в character-volume-filtering-2026-10-07/sampled-mesher-final.log. Это решение по проверенным свойствам входа, не по истечению наблюдения. Запущенных построителей этого эксперимента больше нет.

Полный объём в нативной опорной позе скелета (2026-10-07): opt-in экспорт существующего теста теперь сохраняет bind_pose64 с теми же 3273 вершинами/4672 треугольниками; локальная точная проверка прежних 12 граней не нашла строгих поперечных пересечений (это не глобальный сертификат). fTetWild в этой позе даёт нативно допустимые объёмы, но покрывает лишь 2666/3273 при двух итерациях и 2729/3273 без оптимизации. Для подготовки конформного ассета отдельно проверен TetGen commit 9ce1cea8b9ac236aab3d15f2a15d0fc260006348 (AGPL-3.0, внешний инструмент; не включён в Rust runtime), штатная сборка и -pYJC завершились с кодом 0. 2338 геометрических узлов, 6845 тетраэдров, 4672 наружные грани; независимое сравнение подтверждает точное сохранение координат всех исходных вершин и всех граней после явного сопоставления швов. Объём 0.05371328527883002 м³, разница с ориентированным исходным контуром 6.94e-18 м³; это геометрия, не анатомическая калибровка. Новый --check-bind-volume-json явно проверяет skeleton-bind-pose, прежний --check-volume-json по умолчанию сохраняет animation-phase-zero; оба используют один нативный gate/ImportInputs и публикуют reference_pose. Нативный bind gate: код 0, volume_admitted=true, complete_skin_binding=true, 3273/3273, ошибка восстановления 1.11e-15 м. Регрессия требует полного bind покрытия и отказа полноты при подстановке phase-zero; контрольный регион покрывает 1 вершину в phase-zero и 0 в bind, обе проверки прошли, экспорт тоже прошёл. Производственные поддержка костями, материалы, работа динамики и привязка runtime к явной опорной позе ещё не завершены. Источники/команды/контуры/объёмы/проверки: artifacts/character-bind-pose-2026-10-07.

Авторский импорт теперь сохраняет явную опорную позу Regions.reference: version 2 допускает scene_bind_pose_metres, прежний scene_phase_0_metres и version 1 сохраняют семантику. Один VolumeReference выбирает существующий native sample_pose_phase64 (clip None или Some(0)); отчёт публикует reference_pose. Полный bind-regions.json наблюдает manifest/объём/GLB через ImportInputs, создаёт существующий единый владелец динамики и связывает 3273 вершины; identity palette + текущая опорная поза восстанавливают кожу с погрешностью <2e-15 м. Все 3 теста импортера прошли, включая несовместимую позу покрытия и неверные coordinate_space. Материал полной контрольной конфигурации задан явно, без калибровки; закреплений нет, запуск движения не квалифицирован. Выявлен следующий архитектурный предел: GlobalSkinBinding::posed_reference применяет один joint ко всем узлам региона, что подходит прежним локальным областям, но не целому персонажу с несколькими костями. Опорные положения узлов полного объёма должны выводиться из существующего нативного skin provider с сохранением отображения геометрических узлов в исходные вершины; копировать логику LBS или считать всё тело одной костью нельзя. Entry point захвата пока сохраняет начальную phase-zero и полную bind-конфигурацию не выдаёт за готовую динамику.

Опорная кинематика полного объёма теперь использует source-skin-nodes вместо одной кости на регион, по явному полю version 2 kinematic_reference; прежняя region-joints и отсутствие поля сохраняют старый путь. bind_skin_source_reference сопоставляет каждый узел объёма точной группе исходных координат, сохраняет все отдельные швы и отклоняет узел без соответствия. На каждом кадре группа должна оставаться совпадающей; скрытого nearest fallback и копии LBS нет. Один GlobalSkinBinding::posed_reference используется рендером и staged embedded contact; неправильные размеры, матрицы, нечисловые координаты и расхождение шва дают отказ. Нативная проверка всех 2338 узлов/3273 исходных вершин в 481 позе подтвердила точное совпадение с существующим model.scene_surfaces64; при неподвижном физическом состоянии опорное скелетное движение отменяется ровно один раз с максимальной ошибкой 5.27e-16 м. Отдельный тест неравномерного affine shear + физического смещения проверяет перенос 1 мм без двойного движения; тест контактного расхождения швов сохраняет всё состояние до отказа. 3 теста переноса, full-clip, новые ограничения manifest и прежний импорт прошли (6 тестов); форматирование/diff check прошли. Существующий body_motion_snapshot теперь выбирает заявленную reference pose для палитр, исходного контакта и bind_skin; нового runner нет. Запущен full-volume захват Metal M4 Max (57222/PID35083), 3273 bound и 4672 responsive треугольника, единственный владелец динамики. Первый нулевой checkpoint готов, короткий интервал 0.0125 с ещё вычисляется; образец профиля находится в implicit quadrature/material/contact evaluations. Материал контрольный без калибровки, закреплений к костям нет; self-contact остаётся отдельной задачей. Динамическое завершение, анимационное закрепление, производительность и полная аппаратная поддержка не подтверждены. Артефакты: artifacts/source-node-reference-2026-10-07.

Профиль живого полного объёма подтвердил работу в implicit quadrature/material/contact evaluation. В PrescribedTriangleSurface введён приватный BodyContactDomain с производным immutable has_enabled; пустая маска группы не может включить пару и позволяет пропустить broad-phase поиск для её граней после прежней validate_faces. Проверки геометрии, активный закон, порядок активных пар, unlisted triangles и Arc owner не изменялись. До/после прошла одинаковая проверка нулевой энергии/всех градиентов, активного unlisted face, rebinding owner, staged same-owner и неверной геометрии. ABBA x3 (6 измерений на вариант), 200 response по 512 граням: медиана 22.5400712295 с → 0.1964392085 с, отношение 0.00871511037. Это узкий all-masked benchmark на совместно используемом хосте, не скорость всего движка. 113 lib тестов прошли, 8 ignored; интеграционный embedded contact suite ещё работает (13734), полной квалификации suite пока нет. Старый full-volume захват 57222/PID35083 по-прежнему активен с нулевым checkpoint; не перезапускался. Независимый аудит 6845 тетраэдров: mean-ratio min 0.007358, median 0.352715, 2 ячейки ниже 0.01, min объём 2.260323e-9 м³; это качество формы, само по себе не доказательство причины медленного решателя. Артефакты: artifacts/inactive-contact-domain-2026-10-07 и source-node-reference-2026-10-07/volume-quality-audit.json.

Отдельный prescribed_surface_contact завершился с кодом 0: 34 интеграционных теста прошли, включая CCD, энергию движущегося контакта и Галилееву ковариантность. Итого подтверждены 147 lib/контактных тестов и 8 ignored; embedded_skin_contact suite остаётся активным, результаты оставшихся длительных случаев пока не утверждаются.

Source-node закрепления теперь используют тот же один reference_nodes вектор, который передаётся в staged embedded skin: после прежних проверок матриц/индексов целевые позиции закреплённых узлов берутся непосредственно из нативного source skin pose. Дублирование bone blending и отдельная однокостная аппроксимация целей отсутствуют. Прежний region-joints путь сохранён. Source policy отклоняет authored joint overrides/weights при binding, чтобы они не игнорировались молча. Контроль nonrigid shear с 4 закреплениями прошёл нативный solver/work/history путь: закрепления точно совпали с source pose, render не добавил смещение второй раз; support_work=1.6241246909e-10 Дж, независимая невязка mechanical-support+heat=-1.076276484e-11 Дж при абсолютном gate 1e-10 Дж. Все 4 skin_tests прошли, включая conflict/alias отказ без изменения состояния. Это протокол общего механизма, не авторское/анатомическое размещение полного скелета. Материал без калибровки. Старый полный свободный объём (57222/PID35083) остаётся активен; наблюдение не объявлялось терминальным. Начат отдельный вариант full-volume с уже проверенным кешем пустых контактных доменов (15657), теми же входами/геометрией/шагом и без закреплений: новый прогон для проверки новой реализации, а не перезапуск из-за времени ожидания. Embedded integration 13734 ещё выполняется. Стартовая привязка clip phase-zero отличается от bind reference; для реального анимационного закрепления нужны явная подготовка начального механического состояния/контролируемый переход и проверка контактов, а не телепорт закреплений. Артефакты: artifacts/source-node-supports-2026-10-07.

Cached full-volume вариант 15657/PID53608 подтверждён живым; его нулевой checkpoint точно равен исходному 57222/PID35083 (узлы и энергетические receipts), оба процесса пока не выдали ненулевой checkpoint. Это проверка одинакового старта, не конечной динамики и не измерение ускорения полного расчёта.

Добавлен Pose64::blend: исходный rig/число локальных TRS проверяются, finite weight clamp сохраняет конечные позы побитно, quaternion идёт shortest path, нечисловые/сингулярные/чужие позы отвергаются. AnimationClip::try_sample_phase64_with_startup использует тот же authored phase clock: smoothstep убирает bind-вклад в первые startup_seconds, после перехода возвращается обычный target sample точно. Таймер и LBS не дублировались. Version 2 bind manifest явно разрешает startup_seconds от 0 до длины клипа; остальные схемы, null/ошибочный тип/выход за диапазон отвергаются. Bind source supports без ненулевого startup duration не публикуются, чтобы закрепления не начинали с мгновенного скачка в clip pose. Main использует один imported_pose64 для рендера и contact/pin callback; compatibility wrappers и отсутствие startup сохраняют старую семантику. Сохранён отдельный startup-regions.json со временем 0.5 с, старые running manifests не менялись. 235 animation lib тестов прошли; полный Cesium кinematic/contact audit 481 позы/3273 вершин подтвердил точную начальную bind pose, совпадение skin/contact, швов и обычного клипа после phase 0.25. Первый шаг меняет компонент положения не более 1.15301125e-4 м. Проверка manifest прошла. Это sampled kinematics, не completed full driven mechanics/self-contact. Ранее запущенный physics suite 13734 завершился с кодом 0: 113 lib +33 embedded +34 prescribed теста прошли, 8 ignored. Оба full-volume capture процесса остаются активными и пока не подтвердили ненулевой checkpoint; результат динамики не утверждается. Артефакты: artifacts/rig-startup-transition-2026-10-07.

Source-node support reversal qualification (2026-10-07): seven sequential physical steps, including forward/zero/reverse prescribed shear, retain the same native contact owner and exactly impose only the four pinned nodes. Rendered embedded vertices match physical node positions within 1e-14 m; free nodes deviate from prescribed skin by up to 1.33971965295964779e-5 m, so they remain dynamic. Maximum independently recomputed work/heat/mechanical energy balance is 3.74167992353562316e-11 J (gate 1e-10 J per step). Evidence: `artifacts/source-support-reversals-2026-10-07/`. This is an illustrative one-cell mechanical fixture, not anatomical calibration or full-character startup qualification.

Imported rig startup mechanical subset (2026-10-07): the largest positive-volume tetrahedron in the admitted character volume (original nodes 1282, 167, 1856, 488; source vertices 1383, 167, 2284, 494) receives three native clip startup poses at phases 1/480, 2/480, 3/480 with the existing 0.5 s bind transition. Three pinned nodes track source skin within 1e-13 m; the fourth remains dynamic with up to 2.70177711018337874e-4 m component offset from the prescribed skin. Maximum independent work/heat/mechanical balance is 1.03688392310024846e-10 J against a 1e-9 J per-step gate. Evidence: `artifacts/imported-startup-mechanics-2026-10-07/`. This proves native clip-to-physical-support coupling on a tetrahedron from the original asset, not full-character dynamics or anatomical material calibration.

Full-volume nonlinear search metric (2026-10-07): native contact/skin implicit directions now include the existing rest isotropic material action and rest stiffness diagonal, shared with the support-only solver; actual material forces, nonlinear convergence, inertia-scaled work and geometry admission remain unchanged. Six implicit tests pass, including independent affine-stress plus rank-one-contact inversion and pinned DOFs. Physics library: 114 pass, 8 ignored; wider contact regressions remain running. Optimized full character free-body control completes three physical steps through 0.0125 s (2338 nodes, 6845 tets, 3273 skin vertices), with exact same time-zero checkpoint as the optimized baseline. Independent gravity position error <=2.1450383136389917e-11 m, velocity error <=8.640035651236166e-7 m/s, final mechanical/work/heat balance ~1.0125362545936236e-13 J. Baseline still fails nonlinear convergence on its first interval and refines quadrature; all baseline processes retained. Evidence: `artifacts/contact-material-metric-2026-10-07/` and `artifacts/full-volume-release-2026-10-07/`. This is zero-support free fall with no active contact pairs, not rig-driven full-body dynamics, dynamic self-contact, anatomical calibration or real-time performance qualification.

The same optimized candidate also completes the full-volume rig-driven startup through 0.0125 s with three illustrative source-node supports (1282, 167, 1856) from the largest-volume tetrahedron. A manual native audit (`VOXY_RIG_SUPPORTED_CAPTURE=<absolute checkpoint path> cargo test -p voxy_app --example body_motion_snapshot audits_completed_full_rig_supported_capture_against_native_clip -- --ignored --nocapture`) compares all four runtime checkpoints against the actual imported clip/startup provider: pinned positions match exactly, maximum independent mechanical/work/heat balance is 4.62058754159642288e-11 J (<1e-9 J), and free node component motion reaches 8.58970377027201692e-4 m. Two imported-startup and four skin regressions also pass under the changed metric. This is the first short full-volume rig-driven result; the arbitrary three-pin fixture is not calibrated anatomical attachment, continuous full-clip dynamics, active self-contact, or real-time evidence. Wider contact regressions still running.

Full rig clip continuation (2026-10-07): existing optimized capture entrypoint is now running all 480 authored time steps through 2 s with the same three-source-pin manifest, outputting checkpoint/Metal frame evidence every 0.05 s. Process 88501 (exec session 75968) was verified live at 0.1 s; all three observed checkpoints have 2338 physical nodes and independent cumulative energy balance below 1e-9 J. The manual native checkpoint auditor now derives expected checkpoint times/counts from the existing capture scheduler using `VOXY_RIG_SUPPORTED_CAPTURE_STEPS` (default 3; full clip 480), so partial captures cannot be reported as complete; its short-capture regression passes. Full-clip and wider contact tests remain nonterminal. Evidence: `artifacts/full-rig-clip-2026-10-07/`.

Scene/prefab current-state audit (2026-10-07): 100 existing `voxy_scene` unit/integration tests and five existing editor prefab integration tests pass. Current native implementation covers durable object IDs, nested instance composition, source/reference remapping, member and identified-collection patches, structural deletion/reparenting, historical whole-component compatibility, bounded validated file replacement, and editor save/reload/undo/dependency-failure retention. No parallel persistence system was introduced. Evidence: `artifacts/scene-prefab-audit-2026-10-07/`. This qualifies the tested source/format/editor integration paths, not a new interactive graphical-editor acceptance or platform crash-durability guarantee. RAG `wiki://voxy-engine-roadmap` is historical 2026-09-05 context; catalog update timestamps do not make its old body current, and current files/tests govern this audit.

Engine corpus/source audit (2026-10-07): saved evidence integrity verifies 1428 unique candidates and pinned README/root records, 500 classified engine identities (52 derived), and now six focused mechanism manifests with 28 source/license files. All corpus records remain README/root classification depth; this is not 500 architecture reviews or builds. New Stride animation review contrasts render-context clip advancement and mutable SkeletonUpdater ownership with Voxy fixed physical time and immutable pose publication. Evidence: `artifacts/engine-corpus-audit-2026-10-07/` and `docs/engine-research/mechanisms/stride-animation/`. No external code was imported.

Correction to unconditional contact material metric (2026-10-07): the full debug embedded suite completed with 32 pass/1 fail; adaptive parallel contact velocity refinement regressed (fine ~8.997e-7 versus coarse ~4.712e-7 m/s), although position improved. Unconditional active-contact material search is rejected. Current solver conservatively enables the rest material metric only when immutable authored outer AND embedded domains exclude every possible pair; unlisted/eligible faces preserve the original active-contact metric, irrespective of apparent separation at a sampled pose. Corrected release suites: 114 library pass/9 ignored, 33 embedded pass, 34 prescribed pass (181 pass). Targeted debug regression remains running.

Native long-run profile also identified repeated BTree topology validation of fixed embedded faces. Binding-time full admission is retained; private embedded responses now call the identical contact kernel through dynamic geometry admission (finite/bounded coordinates, defensive indices, nondegenerate areas), without rebuilding a duplicate-face set. Public arbitrary-face calls retain full admission. Eight ABBA paired tests of 200 inactive native embedded responses (729 vertices, 768 faces) measured median 0.482243021 s before and 0.357206708 s after (~26% less elapsed time); this is not a full-engine or real-time result. Raw evidence: `artifacts/embedded-topology-validation-2026-10-07/`. Full clip process remains the prior executable and is not restarted due elapsed observation time.

Full-clip observation at 0.3 s: seven saved checkpoints, process still live; independent cumulative energy residual reaches 4.8762149873482485e-9 J, exceeding the manual whole-clip auditor gate of 1e-9 J. The three-step result remains valid, but full-clip conservation is not qualified. Preserve the failure observation and investigate the configured per-step/cumulative conservation contract; do not silently raise the gate to label the run passed. New domain eligibility and nonfinite/collapsed dynamic-skin rejection tests pass in release.

Explicit tissue numerical energy policy (2026-10-07): version 2 manifests may author finite positive `energy_budget_rate_j_s`; absence preserves the legacy 0.0024 J/s policy. Each mechanical interval receives rate × dt, including adaptive subdivisions, and global region assembly preserves the strictest rate. Invalid settings and underflow-driven failed steps retain the previous state. A moving source-support fixture at 5e-10 J/s has independent balance -1.58830134464398734e-14 J against a 2.08333333333333341e-12 J frame budget. Policy atomicity/assembly and manifest tests pass; app release suite passes 247 with 28 ignored, runtime release build passes. Strict full-volume runtime and full-clip conservation remain unqualified. The previously failing contact refinement debug regression now passes after the domain-gated metric correction. Evidence: `artifacts/tissue-energy-policy-2026-10-07/` and `artifacts/contact-material-metric-2026-10-07/active-contact-correction.log`.

Strict full-volume runtime qualification (2026-10-07): the explicit 5e-10 J/s mechanical policy completes three native steps (2338 physical nodes, 6845 tetrahedra, 3273 embedded skin vertices) through 0.0125 s. The existing native clip checkpoint auditor now optionally checks each saved interval and cumulative balance against the authored rate without subtracting reported defect or adding a rounding allowance. Four checkpoints pass: exact three source pins, maximum interval independent balance 4.79192138773797279e-14 J versus 2.08333333333333341e-12 J frame budget; maximum cumulative balance 7.07385945609637079e-14 J; free component motion 8.58970383242674274e-4 m. Evidence: `artifacts/tissue-energy-policy-2026-10-07/strict-native-audit.log`. A separate complete 480-step strict clip is running (PID 5623, session 45883) under the unchanged native runtime; the legacy failed-gate run remains preserved. This is still illustrative attachment and uncalibrated material with no eligible contact pairs; full-clip completion and real-time performance are not established.

Native strict checkpoint auditor rejection qualification (2026-10-07): the actual compiled native auditor rejects the completed three-step capture presented as a 480-step clip, an independently changed mechanical receipt exceeding the authored interval rate while still below the legacy whole-clip gate, and a 1e-5 m displaced source pin. Original runtime checkpoints are unchanged; altered copies and rejection output are saved separately in `artifacts/tissue-energy-policy-2026-10-07/negative-audits.json`. These checks qualify rejection paths, not the continuing full clip.

Committed-frame energy evidence (2026-10-07): `VOXY_CAPTURE_NODE_STATE` now also writes `.energy.jsonl` with initial and every accepted nominal physical frame, independently flushed before later steps. Node/render checkpoints retain their prior schedule. The native manual auditor optionally consumes `VOXY_RIG_SUPPORTED_ENERGY_CAPTURE`, requires exactly end+1 monotonically indexed frames, checks independently recomputed frame and cumulative work/heat balance against the authored rate, and exactly matches corresponding node checkpoint receipts. A fresh strict three-step runtime and native audit pass (four records; maximum frame balance 4.79192138773797279e-14 J). The auditor rejects a missing frame, a compensated intermediate energy error, and a trace differing from the node checkpoint. Evidence: `artifacts/tissue-energy-policy-2026-10-07/per-frame-native-audit.log` and `per-frame-negative-audits.json`. Existing live full clips continue using their original mapped executable and have no retrospective per-frame trace; no claim that sparse checkpoints prove every intermediate frame.

Native jet/impact global momentum ownership (2026-10-07): liquid demo now retains both capture momentum and rebound substrate impulse from existing event receipts, plus the independently integrated gravity impulse of unmarked carrier particles. Existing ballistic marked-drop and finite-gas wall receipts complete the total emitted-fluid/gas/substrate impulse balance. Every candidate nominal frame validates finite global residual <=1e-9 kg m/s before publication, retaining the existing full-frame rollback. Six liquid demo release tests pass (one ignored), including finite-source recoil/exhaustion, species/thermal/gas inventories and reset. In the existing 120-step water/oil impact fixture the maximum independently assembled momentum residual is 1.86517468137026299e-14 kg m/s; a corrupted substrate impulse rejects the entire next frame and preserves both liquids, films, gas, source and clock. Evidence: `artifacts/liquid-global-momentum-2026-10-07/`. Momentum deposited into the overdamped film is assigned to its prescribed stationary substrate; this does not introduce moving-substrate dynamics or calibrate impact fragmentation.

The existing `liquid_snapshot --impacts --finite-source --optical` entrypoint also completes with the changed impulse ownership on Apple M4 Max/Metal, passes its native finite-source/film/gas verification, and produces a four-phase GPU-readback composite with 205 optical film cells in the final phase. Both finite source energy ledger residuals print zero. Its color heuristic reports zero water pixels, so runtime/render completion is not treated as proof of every desired optical appearance. Evidence: `artifacts/liquid-global-momentum-2026-10-07/finite-runtime.log` and `finite-impacts.png`.

Native liquid optical presence admission (2026-10-07): the snapshot color heuristic classified clear water as missing (zero saturated blue pixels) despite actual visible refraction/highlights. Optical snapshots now render the identical opaque scene into a separate GPU reference target in the same encoder/frame and count RGB differences >2, checking both halves of the paired overview fixture (combined view for water close-up). Empty impact frames must match the background exactly; later fluid frames require nonzero contribution. Diagnostic/reference modes retain their existing semantics. Three actual Metal runtimes pass: finite-source impacts (active water/oil halves 1293/1989 changed pixels; final deposited film 3607/4778), reservoir flow, and water close-up. Raw audits are saved next to each image. Evidence: `artifacts/liquid-optical-presence-2026-10-07/`. This proves visible optical contribution in these fixture views, not calibrated hydrodynamics/optics or every hardware backend.

The existing real-GPU analytic `fluid_optics::sphere_path_scale_additivity_and_opaque_occlusion` regression also passes on this Metal adapter: sphere ray-integral scale/additivity and opaque clipping remain qualified within that test scope. Raw log: `artifacts/liquid-optical-presence-2026-10-07/gpu-analytic-regressions.log`.

Surface-film to wet solid ownership (2026-10-07): new native `Body::advance_surface_film_supplies` consumes an explicitly selected water species from canonical `FilmMixture` mass through the existing implicit finite-supply network. One film cell has one authored material supply link; distinct cells may target the same material cell. Other components retain their canonical mass. Material and film commit together; receipts contain actual representable film removal and independent combined mass defect, rejecting unresolved transfers. No new diffusion solver or secondary persistent pool was introduced. Three new bridge tests plus 28 existing finite-supply/vapor/film-withdrawal tests pass. A capped-network analytic fixture then dries into existing finite vapor, closing water and latent-energy balances. The existing wet FEM preview now uses actual film inventory rather than detached scalar supply copies, then applies its existing wet bulk/cohesive transaction. Its native wet/heated motion test passes and rejects a late repeated-wet topology failure atomically. The existing `wet_fem_snapshot --motion` runtime passes on Apple M4 Max/Metal: initial film water 0.2 kg partitions into 0.1998001998001998 kg retained water and 0.0001998001998069765 kg remaining film, fragments increase 1→2 and exposed faces 6→8; subsequent 0.25 s fragment motion reaches rendered geometry with energy defect 6.118473783178757e-12 J. Evidence: `artifacts/moisture-film-transfer-2026-10-07/`. This is authored isothermal unit-activity uptake and diagnostic calibrated wet weakening, not automatic contact, a solution-activity law, sensible heat/impulse transfer, swelling or general material calibration.

Adaptive nonlinear retry ownership (2026-10-07): the live strict full-volume baseline reaches 0.2 s then repeatedly fails nonlinear/impulse-work admission while increasing path panels through 64/128. Preserve this as a live observation, not a terminal failure. A new internal retry policy lets the adaptive temporal caller attempt at most one nonlinear quadrature rescue (1→2 panels) when immutable authored outer and embedded domains exclude every contact pair; on continued nonlinear failure, the existing temporal subdivision controller receives the original rejection. Fixed-step callers and any eligible contact pair retain the original rescue policy. Actual material/contact path quadrature refinement, nonlinear/CCD gates, independent energy admission, Maxwell staging and full rollback remain unchanged. Release regression suites pass 181 with nine ignored. A fresh strict full-volume three-step runtime completes, and all four parsed node/energy checkpoints exactly equal the baseline; its native per-frame/source-pin audit passes. A separate 480-step candidate (PID14525/session49153) is live with every committed energy frame retained. No speedup or full-clip success is asserted. Evidence: `artifacts/implicit-temporal-retry-2026-10-07/` and `artifacts/full-rig-temporal-retry-2026-10-07/`.

Live native energy-prefix qualification (2026-10-07): extracted the existing per-frame energy admission into one shared native audit used by both completed clips and explicitly labelled immutable live-prefix observations. Completion still requires the full scheduled node checkpoints and end+1 energy frames; the prefix path never asserts full-clip qualification. A byte snapshot of 35 committed frames from live candidate PID14525 passes at time 0.14583333333333334 s: maximum independent frame energy defect 1.05304653885696098e-13 J versus 2.08333333333333341e-12 J, final cumulative defect 2.13162820728030056e-14 J. Existing completed three-step native/source-pin audit also passes after sharing the validator. Evidence: `artifacts/full-rig-temporal-retry-2026-10-07/observed-native-energy-audit.log` and immutable `observed-committed-energy.jsonl`. The long candidate and prior baseline remain live, so no full-clip or speedup claim follows. RAG hardware archive `wiki://voxy-archive-01a0f478-2ce6-7143-b83b-1a8b07048f71` was consulted as historical guidance; current source confirms compute-managed storage and direct scene texture/mesh allocations still have separate admission coverage, not an engine-wide device memory cap.

Shared mesh/device memory admission (2026-10-07): extended the existing per-device ComputeMemoryBudget rather than adding another ledger. Geometry upload/reservation admits all buffers atomically; eager LOD admits four shared vertex/normal/material streams plus every index variant in one batch. Streamed ordinary and skeletal LOD index allocations use the same ledger. Shared streams retain an Arc accounting owner, so dropping a base/level cannot retire data still owned elsewhere. SceneError::MemoryBudget preserves prior published geometry and avoids partial candidate allocation/retirement. A physical Apple M4 Max Metal fixture verifies unchanged GPU vertex bytes after replacement rejection, shared compute exhaustion, eager/reserved admission, unique LOD accounting, retained shared owners, and explicit retirement/reuse. Existing real-GPU LOD and skeletal deformation/CPU oracle tests pass; the browser build passes. Streamed admission additionally rejects competition from compute and retired index buffers until explicit cleanup, then recovers. Evidence: `artifacts/shared-mesh-memory-budget-2026-10-07/`. This remains opt-in payload admission; textures, transform uniforms, skeletal source/palette/parameter buffers, staging/CUDA/driver allocations and automatic retirement coordination are not yet covered. Native editor builds, but its current LodSmoke run terminates with zero presented frames and SkippedOccluded; window acceptance is unverified. No engine-wide cap or complete hardware claim follows.

The preserved full-volume candidate remains live and its immutable 55-record energy prefix reaches 0.225 s. The native shared audit verifies all 54 committed physical intervals: maximum independent frame defect 1.82076576038525673e-13 J, final cumulative defect 1.81188397618825547e-13 J under the same authored 5e-10 J/s policy. This is partial observation, not completion of the requested two-second/480-step clip or a speed measurement. Raw prefix/audit: `artifacts/shared-mesh-memory-budget-2026-10-07/observed-rig-energy-prefix.jsonl` and `rig-prefix-audit.log`.

Shared skeletal/device memory admission (2026-10-07): extended the same geometry batch admission to include per-instance joint palette and shader parameters, and routed immutable skin source attributes through the existing device ledger. SceneSkinSource retains one managed owner shared by its instances; parameters now have an explicit lifetime owner instead of relying only on a bind group. Instance logical bytes derive from actual retained buffers. Physical Metal acceptance uses a 256-byte shared source/two 440-byte instances and injects 36 bytes of competing compute, causing a last-parameter-capacity shortfall. The whole candidate rejects with no partial charges, while the original instance still matches CPU deformation after rejection. Explicit retirement enables retry; source lifetime spans both instances, retained LOD streams survive final instance destruction, and final retirement returns charged bytes to zero. All 148 default renderer tests and 19 explicitly enabled GPU tests pass, including existing full reference rig and Fox clip/normal comparisons; the wasm browser build passes. Evidence: `artifacts/shared-skeletal-memory-budget-2026-10-07/`. This closes the SceneSkinner source/palette/parameter accounting gap described above, not textures/transform buffers, default engine retirement, CUDA/all-backend qualification, native-window acceptance or calibrated full-volume tissue completion. No push was performed during these implementation continuations.

Frozen implicit evaluation reuse (2026-10-07): a live full-volume solver sample showed repeated material/path evaluations. The coupled surface/skin implicit attempt now reuses the existing midpoint evaluation for its objective, the same trial evaluation for objective and residual fallback, the accepted trial for the next iteration, and the converged evaluation for the final midpoint impulse. Cache lifetime is strictly one immutable quadrature/history attempt at its exact midpoint; no result survives temporal/path retry or Maxwell history staging. Independent endpoint energy/work, geometry/CCD, Armijo/noise-band criteria, nonlinear/impulse-work gates and rollback remain unchanged. Release suites pass 181 tests with nine ignored. Eight measured ABBA native three-frame full-volume captures on Apple M4 Max/Metal have median simulation+render time 3.0711970835 s before and 2.000006521 s after (34.8786% less elapsed). All node JSON/JSONL, every-frame energy JSONL, CSV and composite PNG outputs match the baseline bytes. The independent native audit passes four checkpoints at 0.0125 s: 2338 nodes/6845 cells, three exact source pins, maximum cumulative energy defect 7.07385945609637079e-14 J and per-frame defect 4.79192138773797279e-14 J under the same authored 5e-10 J/s policy. Evidence: `artifacts/implicit-evaluation-reuse-2026-10-07/`. A separate 480-step candidate (PID26986/session63828) is live at `artifacts/full-rig-evaluation-reuse-2026-10-07/`; prior captures remain alive. The short benchmark does not prove full-clip speedup, real-time production readiness or general material calibration.

Shared managed colour resources (2026-10-07): SceneRenderer uploaded colour mip chains/sampled colour and HdrMipPyramid now share the existing per-device compute/geometry ledger. A common managed-resource enum carries both buffer and texture retirement, preserving conservative cancellation/fence behavior. Material and planar reflection bindings retain one Arc allocation owner. Logical mip/layer bytes use checked 2D single-sample RGBA8/BGRA8/RGBA16Float dimensions; no driver VRAM claim. Physical Metal readback verifies all three previously uploaded mip colours after shared capacity rejects image, sampled-colour and HDR allocation; 44-byte colour/56-byte NPOT HDR payloads retain charges across views and mixed retirement, then release to zero. Tests: 151 default renderer, 20 explicit GPU, ten focused memory; wasm browser compilation and independent HDR mip/material mip/planar numeric fixtures pass. Evidence: `artifacts/shared-colour-texture-budget-2026-10-07/`. Depth/MSAA/3D/compressed/environment paths, transforms, automatic/default retirement, CUDA and other backends remain outstanding.

Authoritative full-volume terminal observation (2026-10-07): temporal-retry candidate PID14525/session49153 actually exits 1, rejecting nominal frame 55 at time 0.229166667 s with "implicit midpoint work defect"; last committed frame is 54 at 0.225 s. This replaces prior live status, not prior immutable prefix audit results. No gate was raised and the full clip is unqualified. Original trace/energy/nodes remain preserved in `artifacts/full-rig-temporal-retry-2026-10-07/`; result.json records the terminal error and step. Reuse candidate PID26986/session63828 and original baseline PID5623 remain live. Further full-clip work must reproduce and reduce the independent work defect rather than reinterpret incomplete traces as completion.

Delivery terminal update (2026-10-07): evaluation-reuse candidate PID26986/session63828 exits 1 at the same nominal frame 55/time 0.229166667 s with implicit midpoint work defect. All 55 committed energy records and saved node-state bytes equal the temporal-retry baseline. This supersedes earlier live observations; the full 480-step clip remains unqualified. New optional VOXY_IMPLICIT_WORK_TRACE reports independent endpoint potential, factored kinetic change, gravity and reaction arithmetic on rejection without changing any admission gate. Diagnostic regression suites pass 181 tests with nine ignored and the native capture runtime builds. The new diagnostic full-length reproduction has not been run.

Shared transform uniform admission (2026-10-07): each SceneTransform now retains its unchanged 256-byte GPU ABI through existing ComputeStorage ownership and the same per-device ledger as compute, geometry, skin and managed colour. Matrix/view/PBR updates stay in-place. Invalid/exhausted creation leaves the existing object and ledger unchanged; dropped transforms remain charged until explicit retirement. A physical Metal fixture fills capacity with mesh, colour, compute and two transforms, rejects new allocations, then independently reads all 128 uniform floats on GPU after updating one object. Both transforms match their own expected values. Default renderer suites pass 152 tests; all 22 explicitly enabled GPU tests pass and voxy_web wasm compilation passes. Evidence: artifacts/shared-transform-memory-budget-2026-10-07. Depth/MSAA/environment/3D/compressed/driver allocations, automatic retirement and CUDA/all-backend qualification remain outside this milestone.

Compensated gravity endpoint energy (2026-10-07): native rejection diagnostics show repeated world-gravity accumulation contributes arithmetic remainders near 1e-13 J; some rejected trials have alternative independent endpoint balances inside the unchanged budget, while others still exceed it. Inertial diagnostics now compensate the sum of independently evaluated material/contact energy and nodal gravity terms, preserving their representable total instead of repeatedly rounding each contribution into a larger material energy. A binary-exact one-tetrahedron/unit-nodal-mass fixture proves four individually sub-ULP terms sum to one ULP, which the former loop erased, and nonfinite acceleration still rejects. Release physics suites pass 182 tests with nine ignored; native capture builds. A separate full 480-step strict candidate runs at artifacts/full-rig-compensated-gravity-2026-10-07, preserving the original diagnostic run. No tolerance, force model, iteration/refinement gate or rollback was relaxed; full-clip success remains unqualified pending completion and native audit. Evidence: artifacts/compensated-gravity-energy-2026-10-07 and artifacts/full-rig-work-arithmetic-2026-10-07.

Compensated material endpoint energy (2026-10-07): Body endpoint evaluation now uses the same private EnergySum owner as inertial gravity diagnostics for pore aggregate, each tetrahedron, cohesive/gap/surface/embedded contact aggregate, individual cavity faces and dead loads. The force/gradient assembly and all gates remain unchanged; individual constitutive/contact/pore sub-evaluations retain their own arithmetic. A 65-disjoint-tetrahedron scale fixture evaluates a large cell and 64 individually sub-ULP contributions through independent single-cell evaluations. The old sequential sum loses the small contribution; the new result matches the power-of-two oracle in both element orders with identical gradients. Eight native regression suites pass 240 tests with nine ignored, covering biomechanics, embedded/prescribed contacts, pore coupling, muscles and viscoelastic dynamics. Runtime builds; a fresh 480-step strict candidate runs at artifacts/full-rig-compensated-material-2026-10-07 while prior runs remain preserved. Full clip and material calibration remain unqualified. Evidence: artifacts/compensated-material-energy-2026-10-07.

Additional material-energy acceptance: seven coupled suites pass another 35 tests (275 physics tests in total), covering wet cohesion/wear, poroelastic cells/body, internal surface contact, HGO body viscosity and muscle activation. The current native three-step full-volume capture and independent source-pin/per-frame energy audit pass at the original 5e-10 J/s authored rate: four checkpoints, exact three source pins, maximum interval energy defect 4.76902303785507894e-14 J and maximum cumulative independent balance 7.14602395269700597e-14 J. This short qualification does not establish full-clip success.

Scene budget owner persistence (2026-10-07): SceneRenderer and SceneSkinner retain the same device admission owner while empty. This prevents last-resource retirement and external handle destruction from erasing configuration/history while a scene producer is still alive. Skeletal source allocation uses its retained owner directly. Tests prove empty configured renderer/skinner still enforce the original limit and reject a different limit; empty formerly unbounded renderer still rejects retroactive configuration. Default renderer suites pass 154 tests, all 22 explicit physical GPU tests pass, and voxy_web builds. Evidence: artifacts/scene-memory-owner-persistence-2026-10-07. This is producer lifetime ownership, not a finite default, automatic retirement, driver VRAM accounting or permanent raw-device configuration after every managed owner disappears.

Terminal diagnostic reproduction (2026-10-07): original VOXY_IMPLICIT_WORK_TRACE run PID36385/session23131 exits 1 on nominal frame55; all committed energy frames equal the previous failed baseline. Its 71 rejected-trial arithmetic records include 21 alternative endpoint balances within the same budget, but other trials have independently evaluated material/path-work residuals exceeding it. This proves arithmetic loss exists without proving it alone closes the clip. Original logs remain preserved. The compensated material candidate and gravity-only candidate remain live and unqualified.

Factored implicit kinetic change (2026-10-07): coupled surface/skin and material-only implicit steps now measure free-node endpoint kinetic change as 0.5*m*(v_new-v_old)*(v_new+v_old), summed with the existing compensated energy owner. This avoids subtracting squared velocities; force work is not used as an energy oracle. A binary-exact polynomial fixture detects the old 0.5 J rounding error at v=2^27+1 -> v+1, verifies cancelling endpoint changes and pin exclusion. Five native regression suites pass 209 tests with nine ignored, including contacts, muscle and viscoelastic support accuracy. Pin work and explicit/support Verlet arithmetic retain their existing implementation. No tolerance/iteration/refinement/geometry/rollback gate was relaxed. Evidence: artifacts/factored-kinetic-energy-2026-10-07.

Previous failure boundary passed (2026-10-07): the preserved material/gravity-compensated 480-step runtime PID42743/session8152 commits nominal frames55 and56, beyond the original terminal frame55 rejection. An immutable 57-record energy snapshot (initial plus 56 committed intervals) passes the native independent prefix audit under unchanged authored 5e-10 J/s: maximum frame balance 7.18314296932476282e-14 J, final cumulative balance 6.30606677987088915e-13 J at 0.23333333333333334 s. The runtime remains live; full480 scheduled checkpoints/source-pin/full-clip audit remain required. This candidate excludes later kinetic-factor and scene-owner edits, so the boundary evidence applies specifically to material/gravity compensation. The separately current factored-kinetic three-step runtime/native audit passes. Raw evidence: artifacts/full-rig-compensated-material-2026-10-07/observed-through-step56-energy.jsonl and observed-through-step56-audit.log.

Native live node/energy prefix audit (2026-10-07): extracted the existing completed-rig audit into one shared verifier with an explicitly separate ignored manual prefix entrypoint. Prefix requires an explicit incomplete committed endpoint, derives required saved node checkpoints from the original requested full capture schedule, matches them to every-frame energy receipts, verifies native clip source pins/finite full-node states/free motion and independent authored interval/cumulative budgets. Completion mode still requires every scheduled node checkpoint and end+1 energy records. The immutable first60 intervals of the ongoing 480-step material-compensated run pass: 61 energy records, six node checkpoints through0.25 s, 2338 nodes/6845 cells, maximum source-pin error4.44089209850062616e-16 m and maximum per-frame balance7.18314296932476282e-14 J. Five native negative fixtures reject missing node/energy records, a displaced pin, incomplete data presented as complete and a full endpoint presented as prefix; current completed three-step audit still passes. Prefix output explicitly states full_clip_qualified=false. Source: body_motion_snapshot.rs; evidence: artifacts/full-rig-compensated-material-2026-10-07/observed-through-step60-audit-result.json. A one-second live process sample highlights repeated geometry/domain validation, material path evaluation and rest search-metric action; this is a profiling observation, not a measured speedup or real-time claim.

Immutable contact path topology admission (2026-10-07): PreparedPrescribedContactPath response retains full topology/geometry validation for both endpoints and reuses that immutable borrowed face-slice admission within its quadrature samples. Every sample still validates finite coordinates, indices, budgets and nondegenerate triangle area; no cache crosses a call/owner and no force/contact/CCD/work gate is changed. The legacy full-validation branch remains a native test oracle. Active barrier outputs at1/2/4/8/16 panels match potential and both gradients exactly; duplicate/bad-index/nonfinite/changed-count inputs reject identically, and a triangle collapsing at the middle sample still rejects even under an inactive domain. Release suites pass185 tests with10 ignored. Eight ABBA/BAAB native microbenchmark trials of1024 domain-inactive faces/16panels/10responses have median0.0386919165 s legacy vs0.0179530415 s admitted (53.6000% less elapsed in this isolated validation fixture only). Current native three-step capture/node-energy audit passes; node JSON/JSONL, every-frame energy JSONL, CSV and PNG equal the previous factored-kinetic baseline bytes. A fresh full480-step current-version runtime runs at artifacts/full-rig-admitted-topology-2026-10-07 while prior candidates remain preserved. Full-clip speedup/real-time readiness and all-backend coverage remain unqualified. Evidence: artifacts/admitted-path-topology-2026-10-07. Native audit logs now distinguish completion of a requested short capture from full480-step clip qualification.

Prefix72 qualification and gravity-only terminal observation (2026-10-07): gravity-only PID40488/session72979 exits1 at nominal frame55, whereas the material+gravity compensated run passes the immutable first72 intervals and seven scheduled full-node checkpoints through0.3 s under original5e-10 J/s. Native clip pin error remains4.44089209850062616e-16 m. Evidence: artifacts/full-rig-prefix72-2026-10-07. This isolates gravity compensation as insufficient without asserting complete480-step success.

Immutable contact stiffness topology admission (2026-10-07): prepared path normal stencils now reuse immutable face admission just as the path response does. Public instantaneous normal-stencil inputs still undergo full validation; both path endpoints remain fully validated, and all dynamic sample coordinates/indices/areas remain checked. Native legacy-oracle tests now compare all normal-stencil fields alongside path energies/gradients for active and inactive domains, duplicate/bad-index/changed-size/nonfinite inputs and an inactive triangle collapsing mid-step.185 release regression tests pass with10 ignored. Eight balanced native trials of1024 domain-inactive faces/16panels/10combined response+stiffness calls have median0.084062021 s legacy both vs0.0362636455 s admitted both (56.860845% less elapsed in this isolated validation fixture only). Native current three-step capture/audit passes and node/energy/CSV/PNG bytes equal the prior response-admitted version. Browser build passes. Existing long runs remain preserved; this fixture does not establish full-rig speedup, real-time operation or general hardware coverage. Evidence: artifacts/admitted-contact-stiffness-2026-10-07.

Current-version first56 runtime qualification (2026-10-07): the independently launched factored-kinetic + response-topology-admitted runtime PID51794/session3242 passes the immutable prefix audit under original5e-10 J/s:57 energy records and all five scheduled full-node checkpoints through0.2 s, committed time0.23333333333333334 s, native source-pin error4.44089209850062616e-16 m. Its independent energy/node audit passes beyond the original failed frame55, but raw energy and node traces differ from the material/gravity-only baseline; exact long-prefix parity is not established. Structured checkpoint deltas are recorded in result.json. This does not qualify the whole480-step clip or every unsaved node frame. This runtime predates the later normal-stencil topology optimization; both long runs remain live. Evidence: artifacts/current-rig-prefix56-2026-10-07.

Structured current-prefix comparison: all five saved physical node position/velocity arrays through0.2 s match the material/gravity-only baseline exactly; all57 first-three-column mechanical/heat/work receipt values match as well. Raw JSON bytes differ because the fourth reported solver-defect column changes with factored kinetic arithmetic. This is scoped physical-state/ledger parity, not every unsaved node state or full480-step qualification.

### Extended live rig prefix qualification (2026-10-07)

The material-compensation runtime passed the native node/energy prefix audit through step 84 (0.35 s): 85 committed energy records, eight scheduled node checkpoints, 2338 nodes, 6845 cells and three source pins. The factored-kinetic/response-topology runtime passed through step 60 (0.25 s), with 61 energy records and six scheduled node checkpoints. Maximum source-pin error was 4.440892098500626e-16 m in both audits; the authored energy rate remains 5e-10 J/s. Immutable inputs and native logs are retained in `artifacts/material-prefix84-2026-10-07` and `artifacts/factored-prefix60-2026-10-07`.

All six saved physical node position/velocity arrays through 0.25 s and the first three energy receipt columns through step 60 are exactly equal between these runtimes. The fourth reported solver-defect column differs after kinetic factorization; this is not raw trace parity or proof of unsaved physical frames. Both requested 480-step runs were confirmed live during observation. Neither is full-clip qualified, and neither includes the later normal-stencil topology optimization. A one-second macOS sample of PID 51794 is retained alongside the factored prefix; it still observes repeated normal-stencil face validation in that older executable. This sample is diagnostic, not a throughput measurement or a claim about the latest executable.

The coupled implicit search operator now directly constructs either the material-plus-inertia output or the inertia-only output. It previously allocated an inertia-only vector and discarded it whenever a material body was present. This removes one unused node-sized output allocation per operator application without changing the selected operator or arithmetic. Release library tests passed (118, 10 ignored), plus 67 embedded-skin and prescribed-surface integration tests, including coupled material/contact analytic solutions, moving-surface work accounting and transactional rejection. No speedup or full-clip parity is inferred from this allocation change. Logs are retained in `artifacts/implicit-material-allocation-2026-10-07`.

Native runtime verification after removing the discarded material-search allocation passed on the full 2338-node/6845-cell rig for three nominal frames (0.0125 s). Node JSON/JSONL, every-frame energy JSONL, motion CSV and rendered PNG are byte-identical to the preceding normal-stencil topology version. The independent native audit passed with maximum per-frame balance error 4.769023037855079e-14 J and exact source-pin positions. This verifies the short trajectory and output equivalence only, not the full requested 480-frame clip or an elapsed-time improvement. Evidence is in `artifacts/implicit-material-allocation-2026-10-07`.

A separate requested 480-step full-rig runtime was launched from the latest executable, including both prepared-path topology optimizations and the discarded material-search allocation removal. PID 60406/session 12591 was confirmed live; its executable SHA-256 and version scope are recorded in `artifacts/full-rig-current-allocation-2026-10-07/launch.json`. Prior material-only and response-only processes were not stopped. This launch is not evidence of completion, full-clip parity or throughput; later committed prefixes must be independently audited.

The latest executable passed native prefix auditing through step 15 (0.0625 s), with two physical node checkpoints through 0.05 s. Its complete 16 energy records and both checkpoint JSONL rows are byte-identical to the response-only runtime at the same interval. The per-frame independent balance maximum remains 4.769023037855079e-14 J, with source-pin error 4.440892098500626e-16 m. Immutable inputs and audit are retained in `artifacts/current-rig-prefix15-2026-10-07`. A one-second sample of live PID 60406 observes 204 collapsed leaf samples in `rest_material_action`, 108 in prescribed contact response and 27 in the validated normal-stencil core. This is an early-trajectory diagnostic; it cannot establish acceleration versus older samples at different physical times. It identifies the material search operator as the next measured investigation target. Full480 qualification remains open.

### Prepared material moduli candidate (2026-10-07)

The coupled contact search and support search now prepare rest shear/bulk moduli once per immutable internal solve rather than summing Ogden/Maxwell branch moduli on every conjugate-gradient operator application. Element order, moduli sum order and subsequent action arithmetic are preserved. Preparation is rebuilt at each solve and has no persistent body cache. Release regression coverage passed: 118 library tests (10 ignored), 33 embedded-skin tests and 34 prescribed-surface tests, including exact prepared-versus-uncached affine operator output and independent analytic coupled material/contact solutions. The tradeoff is an element-count array (16 logical bytes per element). This remains a performance candidate: timing and current native trajectory equivalence are still required before claiming a benefit. Live PID60406 predates this candidate. Logs are in `artifacts/prepared-material-moduli-2026-10-07`.

The prepared-moduli candidate was subsequently rejected and removed from production call sites. In eight balanced ABBA/BAAB trials of 1024 disjoint viscoelastic tetrahedra, 100 solves with 64 operator applications each (charging preparation), uncached median elapsed time was 0.175995604 s versus 0.1771051045 s prepared. Concurrent live full-rig workloads introduce noise; these measurements do not prove a reliable speedup, so the additional preparation allocation is not retained. The uncached production operator and earlier discarded-output allocation removal remain. The prepared reference and ignored manual benchmark are retained only for testing/future investigation, with raw trials in `artifacts/prepared-material-moduli-2026-10-07/benchmark.log`.

Latest launched rig prefix verification now covers 48 steps (0.2 s): 49 energy records and five physical node checkpoints. Both raw streams are byte-identical to the prior response-topology-only runtime over the matching prefix. Independent native auditing passed with maximum per-frame balance 7.183142969324763e-14 J, cumulative node balance 6.075140390748857e-13 J and source-pin error 4.440892098500626e-16 m. The unchanged authored rate is 5e-10 J/s. Evidence is retained in `artifacts/current-rig-prefix48-2026-10-07`; PID60406 was confirmed live and had committed 51 steps at observation. This has not yet qualified the previous step55 failure boundary or the full480 clip.

### Existing wet FEM preview with finite drying (2026-10-07)

`wet_fem_snapshot --dry --motion` now runs dry intact, wet fractured, finite isothermal drying and fragment motion stages through the existing native water/vapor/cohesive transaction. Material water decreases from 0.1998001998001998 to 0.18196089624661058 kg; vapor receives 0.017839303553589276 kg and latent exchange is 42814.32852861426 J. Combined water, accounted latent energy, mechanical mass and broken topology are checked before publishing the preview clone. Two native preview tests and the actual Apple M4 Max/Metal four-frame readback passed; motion changes fragment pixels. Evidence is in `artifacts/wet-fem-drying-2026-10-07`.

Repeated drying is explicitly incomplete: its second parameter migration rejects with `cohesive parameter change would heal damage`. A regression verifies complete rollback including water, vapor, film, mechanics and history. The no-healing gate is retained; accepted damage migration needs further work for repeated drying cycles. First drying at fixed pose need not change pixels; the runtime compares the actual later motion frame. The scenario is authored finite isothermal exchange, not gas flow, sensible heat, swelling or general calibration.

### Irreversible terminal fracture across drying cycles (2026-10-07)

The earlier repeated-drying failure is fixed for completely fractured frictionless interfaces. `cohesive::State` now records accepted terminal fracture independently of the current calibrated failure separation. Subsequent laws retain damage one, zero tensile/shear cohesive traction and tangent, unchanged measured maximum separation, and ordinary compression closure. Fracture-energy offsets continue to preserve accumulated accepted dissipation; partial-history parameter changes that would heal remain rejected. Frictional law migration remains explicitly unsupported.

An independent regression runs ten wet/dry history cycles, checks exact zero tensile traction/tangent, fracture-work retention, unchanged physical maximum and compression traction, plus partial-damage healing rejection. Release library (118, 11 ignored), cohesive/wet/topology suites (16) and preview tests (2) passed. The existing Metal `wet_fem_snapshot --dry --motion` now performs two accepted drying exchanges, ending with 0.16603294664519164 kg material water and 0.033767253155008274 kg vapor, while retaining two fragments/eight exposed faces and passing actual GPU motion readback. Combined water, latent energy and mechanical mass remain checked before each transaction publishes. Evidence: `artifacts/irreversible-drying-history-2026-10-07`. This qualifies terminal-fracture drying cycles in the authored isothermal preview, not all partial-damage/thermal/frictional cycles or general material calibration.

Extended terminal-history qualification passes 33 additional quadratic cohesive, friction/dynamic friction, wet-update, wet-material and wear/wet tests. Integrated histories now explicitly verify successful drying of terminal fractures with fragment count two, retained accumulated fracture dissipation and corresponding water-mass loss; invalid late thermal/cohesive mixing still rolls back the full transaction. Historical assertions that all terminal law restoration must reject were replaced by these stronger no-healing physical invariants. Partial-damage healing rejection remains covered by the independent history-cycle regression.

The live allocation/path-topology rig version also passed a native prefix audit through 60 steps (0.25 s), beyond the original failed step55: 61 energy records and six saved physical checkpoints are byte-identical to the earlier response-only runtime. Independent per-frame balance peaks at 7.183142969324763e-14 J, and source-pin error at 4.440892098500626e-16 m. Evidence: `artifacts/current-allocation-rig-prefix60-2026-10-07`. PID60406 remains a pre-terminal-cohesive-fix executable; this prefix is not full480 or latest-worktree qualification.

### Partial irreversible damage across drying (2026-10-07)

Partial frictionless interface histories now retain accepted damage as a lower bound when a stronger moisture/thermal calibrated law is installed at the same accepted pose. Migration no longer rejects solely because the new geometric damage would be lower: it carries prior damage, verifies no healing and preserves accepted fracture work through the existing offset. Physical maximum separation is never rebased. Remaining stiffness is bounded by the retained damage; its loading tangent resumes only when the new law exceeds that bound. The associated internal fracture progress is obtained from the inverse bilinear damage law, so held-damage loading adds elastic work without artificial fracture dissipation. Signed stored-energy changes remain explicit parameter work.

An independent analytic piecewise traction integral closes stored-plus-fracture work across the crossover; finite force differences verify tangents on both sides. Ten wet/dry fixed-pose cycles preserve partial damage, dissipation, stored energy and actual maximum separation. 170 release tests passed across library, cohesive/wet/quadratic, topology/friction/wear and the native preview (11 ignored). Evidence is in `artifacts/partial-drying-history-2026-10-07`. This supersedes the earlier partial-history rejection limitation; frictional law migration and general calibration remain open. No full-scene or complete rig claim follows from these tests.

### Cohesive drying with unchanged friction law (2026-10-07)

Cohesive history migration now accepts exactly unchanged optional Coulomb parameters and an unchanged accepted contact history. Friction material value equality compares its validated coefficient and both penalty stiffnesses. Old-law fixed-pose re-evaluation must retain the entire accepted friction state; the new-law response must retain it as well. Changed coefficients/stiffness, enabling/disabling friction, new opening or changed contact history remain explicit errors. The existing damage floor/fracture offset handles changed cohesive properties independently.

A binary-exact regression covers accepted sticking and slipping transitions, compares before/after queries of the same accepted history for traction, tangent, mode, physical friction dissipation, numerical dissipation and released energy, and verifies changed-law/new-contact rejection. The original sliding transition tangent is deliberately not compared against a repeated fixed-pose query tangent, which may select stick at the yield boundary. 156 unique release tests passed across library, cohesive/friction/mesh, quadratic friction/dynamics and wet updates (11 ignored). Evidence: `artifacts/friction-drying-history-2026-10-07`. This supports retaining an unchanged friction history, not migrating arbitrary changed friction parameters or complete scene acceptance.

Current drying/history code passes `voxy_web` compilation for `wasm32-unknown-unknown` and the native Apple M4 Max/Metal repeated-drying/motion runtime. The terminal-fracture four-frame PNG remains byte-identical to the pre-partial-floor preview; water/vapor acceptance and GPU fragment motion gates pass. Evidence is in `artifacts/drying-platform-check-2026-10-07`. This is browser compile evidence, not browser/GPU execution or NVIDIA/CUDA qualification. Live rig processes 60406/42743/51794 were confirmed with 68/99/87 committed steps respectively; full480 remains open and their executable versions remain as recorded in launch manifests.

Migrated partial-damage qualification now covers mixed mode: retained-damage tensile/shear loading, renewed softening beyond the floor crossover, and compressive normal closure with shear damage. All three traction components agree with independent finite differences of stored-plus-fracture energy, all nine tangent entries agree with force differences, and a non-axis-aligned rigid rotation preserves damage/energy and transforms force/tangent covariantly. All nine cohesive integration regressions passed. Evidence: `artifacts/mixed-drying-history-2026-10-07`. This verifies local constitutive derivatives/objectivity; full dynamic stability and general calibration still require separate evidence.

Partial history now passes a native volumetric preview: the existing wet FEM fixture accepts an authored 0.006 m preload, remains one fragment with six exposed faces after wet weakening, and executes five finite vapor drying transactions. Each transaction checks water, latent-energy and mechanical mass; pre-existing positive integrated fracture dissipation stays unchanged and render-mesh positions remain exactly fixed. Three preview tests passed, with an isolated serial partial-volume trace retained in `artifacts/partial-volume-drying-2026-10-07`. The default snapshot still requires its original two-fragment fracture after wetting. Drying now requires unchanged before/after fragment count and full topology instead of hardcoding two fragments. This proves fixed-pose native two-cell coupling, not loaded dynamic stability or partial-preview GPU/browser execution.

Partial drying now passes three native loaded intervals through existing `advance_vapor_loaded`: each 1e-5 s interval atomically exchanges finite vapor, changes mass/cohesive properties and advances real gravity-loaded motion. The mesh changes positions, retains one fragment/six faces and closes water/mechanical-mass/latent receipts. Maximum reported dynamic absolute defect is 4.2188031302249824e-14 J; maximum combined water defect is 4.343976706258157e-17 kg and latent defect is zero. A subsequent longer interval with one permitted attempt fails specifically at the adaptive attempt limit and restores the entire preview and vapor debug state exactly, including prior sources/history/positions. Four preview regressions pass. Evidence: `artifacts/partial-drying-motion-2026-10-07`. Qualification covers 3e-5 s of this authored two-cell split model, not long-time stability, step convergence or GPU scene acceptance.

### Loaded partial-drying timestep qualification (2026-10-07)

The native two-cell partial-drying/gravity transaction now passes 0.1 s runs at 1e-5, 5e-6 and 2.5e-6 s steps (10000/20000/40000 accepted intervals). Water is compared against an independent closed activity-network ODE, with equal 0.1 kg material capacities, 1 kg vapor capacity and 0.01 kg/s conductance per cell: W/6+(5W/6)exp(-0.12t). Final-water errors decrease 1.184452841e-9 →5.922187740e-10 →2.970375335e-10 kg. Successive maximum node-position differences decrease 7.873251417e-10 →3.729780929e-10 m; every interval retains one fragment/six exposed faces and checks finite inventories. Raw runs and the earlier0.01s trial are retained in `artifacts/partial-drying-convergence-2026-10-07`.

This demonstrates water/position refinement for the authored scenario. It does not demonstrate mechanical-energy convergence: maximum reported local dynamic defects increase 1.178175637e-9 →2.582584917e-9 →8.433706858e-9 J under the unchanged1e-8 J per-step tolerance. Independent cumulative mechanical accounting and time-scaled tolerance qualification remain necessary before broader long-time claims.

### Independent cumulative partial-drying mechanics audit (2026-10-07)

The0.1s refinement scenario now measures mechanical energy directly from kinetic, elastic, cohesive stored and accumulated fracture energies (all other physical channels are explicitly checked zero for this fixture). Boundary accounting independently sums gravity work from public consistent-mass row weights and measured displacements, carried water kinetic energy minus transfer loss, and signed bulk/cohesive parameter work. Reported solver defects are never subtracted. Maximum cumulative imbalances decrease 1.447417759e-7 →3.394772818e-8 →6.645990425e-9 J for10000/20000/40000 intervals; final balances are -9.634106846e-10, -6.042171208e-10 and +6.260762575e-9 J. Raw measurements are retained in `artifacts/partial-drying-energy-audit-2026-10-07`. Maximum cumulative refinement is observed, but the final fine-step balance worsens under the constant per-step solver tolerance; time-scaled solver precision qualification remains open. This is diagnostic independent accounting, not an authored production energy-rate acceptance gate.

### Time-scaled partial-drying energy qualification (2026-10-07)

Repeating the same0.1s fixture with rate0.001 J/s preserves the original1e-8 J budget at1e-5 s and tightens finer intervals to5e-9/2.5e-9 J. No production defaults or full-rig authored rate were changed. Independent final balances now refine -9.634106846e-10 →-5.229594535e-10 →-2.094395768e-10 J; maximum cumulative balances refine1.447417759e-7 →3.534185566e-8 →8.047479128e-9 J. Water and node-position refinement also pass. The native qualification explicitly asserts decreasing maximum and final independent balances, with no solver-defect subtraction. Raw qualified run: `artifacts/partial-drying-time-budget-2026-10-07/qualified-native.log`. This rate is anchored to the previous example tolerance, not substituted for the distinct5e-10 J/s full-rig policy or claimed as general material calibration.

### Shared quadratic interval energy-rate API (2026-10-07)

`QuadraticAdvanceLimits::with_interval_energy_rate(interval_s, maximum_defect_j_s)` now creates the explicit interval budget for both existing quadratic advance consumers. It retains sampling/attempt limits, rejects invalid inputs, overflow and zero-rounded products, and adds no energy floor. An exact binary partition regression verifies that64 child budgets preserve the original total. The authored partial-drying convergence example uses this helper instead of multiplying the rate locally; all native receipt markers remain exactly equal to the prior qualified manual policy. Seventeen rate/wet/dynamic-friction tests and the0.1s native refinement qualification passed. Evidence: `artifacts/quadratic-energy-rate-2026-10-07`. Existing defaults and the distinct full-rig energy-rate policy remain unchanged.

### Current allocation rig: immutable 84-step audit (2026-10-07)

The still-running PID 60406 capture now has a separately saved and natively audited prefix through step 84 (0.35 s), under `artifacts/current-allocation-rig-prefix84-2026-10-07`. All 85 per-frame energy receipts and eight physical node checkpoints passed for 2338 nodes, 6845 cells and three source pins. Maximum pin error was 4.44089209850062616e-16 m; maximum independently reconstructed cumulative energy balance was 6.07514039074885659e-13 J. Energy and node JSONL bytes exactly match the response-only optimization runtime over this same prefix. This qualifies the allocation and normal-stencil changes over the longer completed prefix, not the requested 480-step capture, real-time throughput, editor behavior or later cohesive drying changes. The active processes remain running.

### Full physics regression after drying changes (2026-10-07)

The complete native release physics test suite passed: 1642 tests, 0 failures, 15 ignored/manual tests across 217 test reports. The preserved log and result are in `artifacts/full-physics-regression-2026-10-07`. This broadens compatibility evidence for irreversible cohesive drying/history, friction migration, interval-rate budgets and the removed implicit allocation. It does not qualify ignored/manual experiments, physical CUDA execution, browser runtime, editor acceptance or the unfinished 480-step full rig capture.

### Already-dry preview equilibrium acceptance (2026-10-07)

The finite isothermal drying preview now accepts exactly zero water transfer rather than requiring evaporation to be strictly positive. Three repeated steps on the already-dry fixture preserve all reported mechanical energy channels, positions, velocities, material water, topology and finite vapor/heat inventories exactly. The rejection oracle now uses an actually insufficient latent heat inventory after wetting, and confirms whole-owner rollback. Five preview tests pass; two manual convergence tests are ignored here. Evidence is in `artifacts/drying-equilibrium-2026-10-07`. This fixture does not qualify nonzero wet equilibrium, variable-temperature gas flow or a general drying model. No core transport tolerance or energy budget changed.

### Exact wet activity equilibrium without spurious transport (2026-10-07)

A regression reproduced nonzero water changes at exact activity equilibrium from dense transport solve/reconstruction roundoff. The canonical moisture advancement now preserves accepted inventories when every active link and bath has exactly zero activity difference and all sources are zero. It still rejects nonfinite interval operators and invalid steps; no tolerance, clipping, energy-floor or near-equilibrium cutoff was added. Ten successive exchanges preserve all inventories and zero water/latent receipts exactly in 1-, 3- and 129-material-cell networks. Three wet partial-damage FEM preview exchanges preserve physical energy, positions, inventories and topology. An independent small-flux oracle remains nonzero. 55 focused physics/preview tests passed, 3 manual tests ignored. Logs, the original reproduction and checksums are under `artifacts/wet-equilibrium-2026-10-07`. This establishes exact edge-zero equilibrium, not general thermal/gas equilibrium or arbitrary cancelling boundary fluxes.

### Native compatibility after exact moisture equilibrium change (2026-10-07)

The current source passed the 0.1 s partial-drying motion refinement at 10,000/20,000/40,000 intervals: every `PARTIAL_DRYING_` receipt line exactly matches the pre-change interval-rate qualification. The wet fracture, two drying exchanges and 0.25 s separated-fragment motion also passed an actual Apple Metal render/readback run; its montage PNG bytes match the previous fixture exactly. Evidence is under `artifacts/wet-equilibrium-native-2026-10-07`. These fixtures confirm unchanged non-equilibrium behavior over their tested paths, not general transport equivalence, full character/editor acceptance or CUDA execution.

### Current rig profile around committed step 93 (2026-10-07)

A fresh one-second sample of live PID 60406 is preserved under `artifacts/current-rig-profile-2026-10-07`. Collapsed leaf samples include rest-material search action 196, prescribed response 142, BTree insertion 71, viscoelastic response 50 and validated normal stencils 23. This directs the next isolated benchmarks toward the search material operator and remaining endpoint admission work. It is a stage-specific diagnostic, not an end-to-end performance or speedup measurement. The rejected prepared-moduli cache stays rejected; no physics, work, contact or error gates were changed in this profiling step. The observed executable predates later cohesive/equilibrium changes.

### Shared immutable topology admission across path endpoints (2026-10-07)

Prepared contact response, normal-stencil and indexed rejection paths now validate immutable topology once at the start endpoint and geometry at the end endpoint after verifying equal vertex counts. End coordinates, bounds and areas are still validated; intermediate quadrature geometry and all contact/path rejection checks remain. 185 library/contact tests pass. An isolated 1024-face/1000-admission benchmark in eight balanced trials measured median full second admission 0.0320996665 s versus geometry-only 0.0035029375 s. This is not full-rig throughput or a causal end-to-end speedup. Native full-rig trace/image parity for this new change remains pending. Evidence: `artifacts/contact-endpoint-admission-2026-10-07`.

### Endpoint topology optimization: native rig parity (2026-10-07)

The current native body snapshot executable passed a completed three-step Metal rig capture with 2338 nodes, 6845 cells and three source pins. Node aggregate JSON, node checkpoint JSONL, per-frame energy JSONL, motion CSV and montage PNG exactly match the preceding allocation-only executable capture. An independent native audit confirms zero pin error, maximum per-frame balance 4.76902303785507894e-14 J and cumulative balance 7.14602395269700597e-14 J. This closes short startup parity for the endpoint-admission change; the requested 480-step clip, late-contact phases and real-time performance remain unqualified. The first capture invocation omitted the node-state environment switch, so it was rerun correctly; both initial logs are preserved. Existing long calculations were not restarted. Evidence: `artifacts/contact-endpoint-admission-2026-10-07`.

### Full endpoint-admission rig runtime launched (2026-10-07)

A new 480-step/2-second requested capture runs as PID 90673, tool session 19071, with current executable/source hashes and exact configuration in `artifacts/full-rig-endpoint-admission-2026-10-07/launch.json`. Existing historical runtime processes remain running. A separate immutable prefix through step 12 (0.05 s) passed native per-frame and node audits, and node/energy JSONL bytes match the prior allocation runtime over that prefix. Evidence is in `artifacts/endpoint-rig-prefix12-2026-10-07`. Launch and initial prefix are not full clip or late-contact qualification; no existing run was restarted. Current executable includes later cohesive/moisture fixes, but this fixture does not exercise drying.

### Current browser/CUDA qualification and tissue routing gap (2026-10-07)

Current contact endpoint and exact-moisture-equilibrium source passes `voxy_web` wasm32 compilation. CUDA feature compilation and 18 native unit tests pass; the explicit clang host harness additionally executes the actual f64 CUDA gravity arithmetic for 257 bodies x128 steps and 2560 orbit steps, matching CPU exactly and verifying singular/overflow rollback. This is host arithmetic evidence, not NVIDIA driver/device execution or browser runtime. Source audit of `voxy_cuda/src`, `physics/src`, and application CUDA routing finds gravity/projectile/box sweep/voxel/water operations but no connection of the finite tissue/implicit material solver to CUDA. The tissue CUDA backend therefore remains a concrete implementation gap in the original full hardware objective. Logs and checksums: `artifacts/current-platform-qualification-2026-10-07`.

### Canonical tissue search backend input ownership (2026-10-07)

`Body::tissue_search_snapshot` exports an explicit immutable owned input for one selected rest-material search backend. It preserves element order, reference gradients/volume, pins and supplied inertia weights; effective Ogden/Maxwell search moduli come from the canonical native material law. Invalid weight lengths, nonfinite/negative weights, zero free-node weights and nonfinite rest data reject without body mutation. Tests verify the analytic tetrahedron volume/moduli and that snapshots retain prior ownership after body material/support changes. All 120 library tests pass, 12 manual tests ignored. The existing native operator and physical evaluator are unchanged; no persistent cache or automatic allocation per solver iteration was added. CUDA kernel execution/connection, nonlinear forces, history, contacts and endpoint-work qualification remain open. Evidence: `artifacts/tissue-search-backend-boundary-2026-10-07`.

### CUDA canonical tissue search metric operation (2026-10-07)

`CudaCompute::tissue_search_action` now consumes the canonical native tissue snapshot and launches two f64 kernels: element stress-action evaluation and ordered CSR node assembly. It preserves native element/corner accumulation order without floating atomics; weights, pinned DOFs and current Ogden-Maxwell search moduli remain native-owned. Existing CUDA allocation accounting covers the combined input, element scratch and node output, with synchronized retirement on all allocated-buffer exit paths. Readback rejects nonfinite, truncated or displaced pinned output before return; CUDA-disabled calls explicitly return Disabled without a native fallback. `Body::tissue_search_action` exposes the unchanged native search operator for independent backend comparisons, without changing the production implicit solver.

Actual CUDA source executed through a strict clang host harness matches native output f64 bits in 15 fixtures (1/17/257 tetrahedron pairs times five vector modes), including signed zero, pins, shared nodes, heterogeneous elastic and effective Ogden-Maxwell moduli. Native library 120 tests, CUDA-enabled 20 tests and disabled 21 tests pass; the explicit host parity test passes. A physical NVIDIA test is present but ignored pending an explicitly selected device. Cargo feature compilation and host arithmetic do not prove NVRTC device compilation, NVIDIA execution or speed. Connection to the implicit solver, persistent device input ownership, nonlinear force/history/work integration and complete CUDA tissue support remain open. Evidence: `artifacts/cuda-tissue-search-2026-10-07`.

### Fallible search algebra for an explicit GPU backend (2026-10-07)

Shared CG action, L-BFGS inverse action and inverse quadratic scaling now have fallible variants. The default native wrappers use Infallible callbacks over the same implementation, retaining arithmetic and existing numerical curvature/descent fallback. Backend callback failure propagates immediately, including a second-CG-iteration failure, without returning a diagonal direction as success. Secant failures leave retained history unchanged; successful callback paths match native f64 bits. All 122 library tests pass; 12 manual tests ignored. This supplies necessary error propagation for backend integration, not yet physical-step rollback under a CUDA driver failure or solver/device qualification. Evidence: `artifacts/fallible-tissue-search-2026-10-07`.

Separately, the endpoint-admission runtime PID 90673 has an immutable native-audited prefix through step 60 (0.25 s), passing the original failure55 point. Node and energy JSONL exactly match the prior allocation-only prefix60. Evidence: `artifacts/endpoint-rig-prefix60-2026-10-07`. This mapped executable predates the CUDA search operation and new fallible primitives; full480 and CUDA execution are still unqualified.

### Explicit tissue backend integrated into implicit transactions (2026-10-07)

`InertialBody::set_tissue_search_backend` selects a native-owned `TissueSearchBackend`/prepared `TissueSearchOperation`; `Arc<CudaCompute>::tissue_search_backend` supplies the CUDA adapter. Support and contact-free surface implicit paths prepare the canonical snapshot once per solve and send metric vectors through the fallible CG/L-BFGS callbacks. Nonlinear material forces, contacts, history, line search, work and final transaction admission stay native. Invalid-length/nonfinite/nonzero pinned operator output rejects before search publication; backend errors return immediately without a native metric fallback. Shared regional backend identity is retained during assembly, mixed identities reject. Disabled CUDA rejects already at preparation, including predictor-only cases with no CG applications.

Proof-backend tests compare the entire accepted support and contact-free surface reports/physical owners against native, and confirm full physical-owner rollback for preparation, first/second application and nonfinite readback failures. Native library 125 tests, contact/embedded/viscoelastic integration 82 tests, CUDA-enabled 21 tests and disabled 23 tests pass; both crate configurations compile. A physical NVIDIA full-step parity/budget test is implemented and ignored, with an actual operator call counter to exclude predictor-only admission. This does not prove NVIDIA execution, rig/editor selection, speed, persistent GPU input ownership, active-contact search acceleration or nonlinear CUDA forces/history. Active-contact inertia-only search retains its existing native policy. Evidence: `artifacts/tissue-backend-transaction-2026-10-07`.

### Native rig CUDA selection and delivery snapshot (2026-10-07)

The existing body_motion_snapshot example accepts paired explicit --cuda-tissue-device=N and --cuda-tissue-budget-bytes=N with --cesium --contact. Selection reaches the assembled continuum owner; malformed/duplicate arguments and incompatible regional owners reject. Default-disabled CUDA fails before renderer allocation or capture creation. The example suite passes 67 tests (6 ignored), CUDA feature compilation passes, and the three-step default native PNG, CSV, node JSON/JSONL and energy JSONL exactly match the preceding endpoint-admission capture. NVIDIA execution and the full 480-step rig remain unqualified. Evidence: artifacts/native-rig-cuda-selection-2026-10-07.

Delivery-6 preserves 136 files (133142957 bytes) from ongoing captures with SHA-256 manifests in artifacts/session-snapshot-2026-10-07-delivery-6. Original live files and processes are retained; later appended capture data is outside this immutable delivery snapshot.

### CUDA failures stop adaptive tissue refinement (2026-10-07)

The native rig adaptive contact step now returns CUDA adapter infrastructure/input/readback errors before temporal subdivision. The adapter exports a classifier alongside its existing legacy string mapping; this is a scoped compatibility bridge, not a fully typed solver error API. Seven mapped failure classes are injected into the contact-free continuum path: each prepares exactly once, returns the original error and leaves the entire physical owner unchanged. Mechanical energy rejection remains eligible for existing refinement. Three backend-selection tests pass and CUDA-feature crate compilation passes. Evidence: artifacts/cuda-refinement-rejection-2026-10-07/verified-tests.log. Active contact inertia-only policy and NVIDIA execution remain outside this proof.

### Whole-frame CUDA failure rollback and rig prefix84 (2026-10-07)

The CUDA failure regression now stages a genuinely advancing first mechanical frame and injects a backend failure in region two of the second frame. Both serial and four-worker regional execution return the original error after one backend preparation and preserve the complete original demo including clock, accumulator, regional mechanics and ledgers. All four backend-selection tests pass. Evidence: artifacts/cuda-frame-rollback-2026-10-07/tests.log. This is a proof-backend failure injection, not a physical NVIDIA driver test.

Endpoint-admission PID90673 is confirmed live with 86 energy steps. Its immutable prefix84 (0.35s; 85 energy records, eight node checkpoints) passes the native independent audit and exactly matches allocation-only prefix84 node and energy bytes. Evidence: artifacts/endpoint-rig-prefix84-2026-10-07. This executable predates CUDA selection and the full requested 480-step clip is still unqualified.

### CUDA search preparation admits layout and kernels before solving (2026-10-07)

CUDA backend preparation checks the shared packed/scratch/output layout against configured capacity without allocating vectors, then compiles/loads both kernels through the same cache used by applications. A predictor-only solve therefore cannot hide a configured byte-limit failure or an unavailable compiler/module. Actual concurrent device-budget reservations and buffer allocation still occur per application; this does not reserve persistent device inputs during preparation. The extracted checked-arithmetic layout rejects empty/overflow/u32 ABI cases and accepts the exact byte threshold; packing uses the identical layout. CUDA feature tests 22 pass (4 ignored), default disabled tests 24 pass (2 ignored). Evidence: artifacts/cuda-search-preparation-2026-10-07. Host checks do not prove NVRTC execution on NVIDIA.

### CUDA preparation checks occupied shared capacity (2026-10-07)

Search preparation now probes the shared allocation ledger after checked layout validation and before kernel loading. A predictor-only solve cannot accept an operator whose working buffers already exceed currently available shared capacity. The probe owns no GPU buffers and is immediately released: actual applications still reserve atomically again, so this is readiness admission rather than a persistent guarantee across concurrent work. The regression occupies one byte at the exact operator capacity, verifies rejection without changing the existing reservation, releases it, then checks sixteen successful zero-retention probes and full-budget occupation. CUDA-feature tests 23 pass (4 ignored), disabled tests 25 pass (2 ignored). Evidence: artifacts/cuda-preparation-capacity-2026-10-07. Physical NVIDIA execution remains unverified.

### Per-step jet/film/source mass admission (2026-10-07)

The existing liquid demo now derives initial fluid and finite-source inventories from constructed owners and validates total liquid plus film mass against initial fluid plus emitted mass before each fixed-step publication. Finite sources also validate remaining source plus emitted mass against their initial inventory. Existing absolute 1e-9kg demo admission applies; solver transport and impact laws are unchanged. Corrupted emitted-fluid accounting rejects the entire candidate frame in all four source/impact modes; corrupted remaining finite-source mass rejects both finite modes and preserves fluid, film, gas, emitter, source, ledgers and clock. All eight liquid snapshot example tests pass (one timing test ignored), including existing 120-step water/oil spray/deposition and finite-source exhaustion cases. Evidence: artifacts/liquid-frame-mass-admission-2026-10-07/qualified-tests.log. This does not establish complete liquid constitutive/phase calibration or general smoke/fog dynamics.

### Moving-surface film and receipt publication are atomic (2026-10-07)

FilmPreview now stages the film alongside source and self-contact receipts, validates the preexisting source-adjusted inventory and admits finite post-step source/contact accounting plus conserved mass before publishing geometry, volumes or receipt history. Corrupted source inventory and post-transport contact receipts both reject without publishing moved substrate or new liquid. A self-contact fixture now explicitly records its direct initial deposit; transfer and stopping after separation remain unchanged. All 20 surface-film preview tests pass, with six manual tests ignored, covering source distributions, checkpoint restoration, morph/remap and refined body transfer. The additional staged film clone has not been performance-qualified; no throughput claim is made. Evidence: artifacts/film-receipt-admission-2026-10-07.

### One film transaction owns physical and caller receipt admission (2026-10-07)

SurfaceFilm::advance_on_geometry_with_contact_admitted exposes a read-only admission callback over the staged physical layer and its volume receipts. Callback rejection returns before either numerical publication or taking/refitting the old contact cache. The existing contact method delegates through an accepting callback, retaining its interface and arithmetic. FilmPreview performs its external mass/contact accounting inside this transaction, removing the extra full-film clone introduced by the previous safety patch. Native tests verify a rejected owner receipt leaves geometry/volumes unchanged and the next accepted contact or no-contact result exactly matches the reference owner, including receipts. Native surface-film tests 25 pass (2 ignored), atomic-contact tests 4 pass, preview rejection tests 4 pass. Evidence: artifacts/film-single-transaction-2026-10-07. No measured runtime speedup is claimed.

### Current finite water/oil jet rendered on physical Metal (2026-10-07)

The current liquid_snapshot example runs both diagnostic and optical finite-source impact modes on Apple M4 Max/Metal for 120 fixed steps (one second), capturing initial, emission, impact and deposited-film stages. Existing per-step mass/momentum admission and final finite-source energy/species/gas/exhaustion verification pass. Emitted water 5kg and oil 4.0000000000000036kg, both printed source energy defects zero. The optical last frame renders 205 film cells; differential from same-frame background is 3607/4778 pixels in the respective water/oil halves. Physical GPU validation scopes return no errors. Evidence and source/output hashes: artifacts/current-liquid-native-render-2026-10-07. These are stylized test fixtures, not calibrated optics, realtime editor or NVIDIA qualification.

### Spatial pure-vapor / exposed-particle exchange (2026-10-07)

Liquid::exchange_vapor_grid connects explicit exposed-particle/interface selections to their containing FiniteDropletGasGrid cells, reusing exchange_vapor for mass, donor momentum, latent/sensible heat and mixing kinetic conversion. Particle membership is frozen; duplicate indices, outside cells, invalid controls and exchange budget overflow reject. A late thermodynamic pair failure preserves the complete liquid and every gas cell. Shared-cell exchanges retain explicit caller order as operator splitting. Spatial tests cover separate/shared cells, exact sequential-pair parity, evaporation and condensation with independently assembled global mass/momentum/energy checks, duplicate/budget rejection and late-failure rollback. Evidence: artifacts/spatial-vapor-exchange-2026-10-07. This models pure vapor around existing drops; carrier-air mixtures, nucleation, coupled cloud equilibrium, advection and fog rendering remain separate requirements.

### Atomic vapor transport / heat / phase composition (2026-10-07)

Liquid::exchange_vapor_grid_with_transport stages existing gas Euler transport, Fourier conduction and spatial phase exchange with explicit SpatialVaporTransportControl. Interfaces must share latent reference and match the transport gas constant. Particle motion remains separate and the composition is first-order operator splitting. A nonuniform gas fixture verifies earlier flow/heat stages genuinely change physical state, then incompatible phase capacities reject and roll back both owners. Fifty coupled periodic steps (0.05s) independently conserve global mass, momentum and thermal/kinetic/latent energy; metrics in artifacts/spatial-vapor-transport-2026-10-07/result.json. Five spatial tests plus six grid and eleven evaporation tests pass. This establishes coupled pure-vapor feedback, not carrier-air mixing, nucleation or production fog rendering.

### Temporal refinement of composed vapor transport and current rig prefix96 (2026-10-07)

The nonuniform two-cell pure-vapor fixture now compares complete liquid mass/velocity/temperature and gas mass/velocity/temperature after 0.05s at 25/50/100/200 steps against an 800-step reference. The componentwise absolute differences are divided by max(abs(reference),1) and summed; every refinement reduces the normalized error by more than 30%. This is numerical time convergence for the fixed two-cell splitting fixture, not an analytical solution or spatial convergence certificate. All six spatial phase/transport tests pass. Metrics: artifacts/spatial-vapor-time-refinement-2026-10-07/result.json.

Endpoint-admission runtime PID90673 remains live and commits step97. An immutable prefix96 (0.4s;97 energy records and nine node checkpoints) passes native independent energy, support-pin and free-motion audit. Evidence: artifacts/endpoint-rig-prefix96-2026-10-07. It predates CUDA selection; full480 remains open.

### Stronger continuum heat spatial qualification (2026-10-07)

The existing gas-grid continuum heat test now evolves an exact cell-average cosine mode for 0.2s at 300K base temperature on 8/16/32/64 periodic cells, refining time together with dx squared. Analytical Fourier diffusion with alpha=0.05m2/s predicts the cell averages. L1 temperature errors fall from 0.08625018K to 0.001417956K (60.83x), each mesh doubling improving by more than 3.5x. Total gas mass remains unchanged and thermal energy residual stays below 1e-8J. All six gas heat tests pass. Evidence: artifacts/gas-fourier-spatial-refinement-2026-10-07. This qualifies heat conduction; spatial convergence of coupled droplets, phase interfaces and compressible gas remains separate.

### Derived droplet extinction field and exact CPU optical reference (2026-10-07)

Liquid::droplet_extinction_grid derives cell extinction in m^-1 from explicitly marked physical spheres using caller-supplied dimensionless extinction efficiency Q and radii: sum(Q*pi*r^2)/cell_volume. Ordinary SPH samples do not contribute; outside marked drops are counted explicitly. The immutable derived grid exposes layout and coefficients for future renderer upload. Exact piecewise-constant segment integration supplies optical depth and direct unscattered transmission exp(-tau); this CPU reference scans all cells and is not a production GPU traversal. Tests compare analytic cross-section totals, forward/reverse/diagonal rays, shared-face ownership, clipping, zero-length and transparent rays; malformed/overflow inputs reject and physical owners remain unchanged. Two optics, six spatial phase and six gas-grid tests pass. Evidence: artifacts/droplet-extinction-field-2026-10-07. No wavelength-dependent Mie calibration, in-scattering, GPU fog renderer or carrier-air composition is claimed.

### Portable GPU droplet extinction through existing compute ownership (2026-10-07)

DropletExtinctionComputeInput transports an immutable generic ExtinctionGridView and finite ray segments to DROPLET_EXTINCTION_SHADER, reusing ComputeProgram device/memory/readback ownership. Render code has no production dependency on physics; the native example supplies the physical field. Packing checks dimensions, finite/nonnegative coefficients, f32 representability, finite rays and explicit byte capacity. Decode checks complete finite nonnegative optical depth, bounded transmission and exp(-tau) consistency, rejecting uncomputed output. The WGSL cell-integral shader runs 131 rays across three workgroups on Apple M4 Max/Metal, including clipping, reverse, diagonal, shared-face and zero-length cases; max depth error 1.75755e-8 and transmission error 4.25997e-8 against native f64. Validation scopes pass; byte-budget, NaN and truncated-result cases reject. Evidence: artifacts/droplet-extinction-gpu-2026-10-07. This is exact cell-integration GPU qualification, scanning every cell per ray; optimized traversal, scene composition, in-scattering and calibrated fog/NVIDIA remain open.

### GPU extinction grid-plane traversal (2026-10-07)

DROPLET_EXTINCTION_SHADER now clips rays to the field and visits crossed cells by grid-plane traversal, updating every tied axis at edge/corner crossings. The original dense shader remains exported as DROPLET_EXTINCTION_REFERENCE_SHADER for comparison. Input dimensions admit bounded integer traversal and reject f32-unresolvable first/last cell spacing. Physical Metal qualification covers 131 analytic rays plus 1027 deterministic random/boundary/corner rays through a 7x5x3 heterogeneous sphere-derived field. Maximum depth deviation from f64 native integration is 4.59860e-7; from dense GPU integration 1.19209e-7. Validation and malformed-readback gates pass. Evidence: artifacts/droplet-extinction-traversal-2026-10-07. This establishes traversal correctness on the fixture; no measured speedup, large-world-coordinate certification, scene composition, in-scattering or NVIDIA execution is claimed.

### Direct extinction composition with depth-ended camera rays (2026-10-07)

DropletExtinctionCompositeInput/DROPLET_EXTINCTION_COMPOSITE_SHADER compose linear-light scene RGB with exp(-tau), retaining alpha, through the existing compute owner. Traversal source is shared with the standalone kernel. Packed colors and complete readback are admitted; decode also checks computed transmission and expected RGB attenuation. extinction_segments_from_depth unprojects supplied WebGPU 0..1 depths at top-left pixel centers; perspective rays originate at eye and orthographic rays at their near-plane pixel. Unit checks reproject endpoints and reject malformed depth data. Physical Metal fixture uses depths projected from explicit foreground/background positions: foreground before the cloud remains exact, background transmission 0.7304026910486455 matches native and alpha stays fixed. Prior 1158 traversal rays still pass. Evidence: artifacts/droplet-extinction-composite-2026-10-07. Actual scene depth-texture integration, entirely GPU-resident scene compositing, in-scattering, spectral calibration and NVIDIA remain open; this fixture supplies projected depth data rather than reading a rendered opaque scene.

### Extinction verified against rendered opaque color and depth (2026-10-07)

The existing droplet_extinction_smoke now renders foreground/background opaque meshes through SceneRenderer into Rgba8Unorm and Depth32Float, reads both textures, reconstructs actual pixel rays and runs the extinction composition on Metal. Across 16384 pixels, all 4992 classified foreground pixels remain exactly unchanged and 1560 background pixels attenuate; maximum independent linear RGB deviation is 1.26156e-7. Validation scopes pass and the before/after image is saved in artifacts/droplet-extinction-rendered-scene-2026-10-07/comparison.png. The earlier analytic and heterogeneous traversal checks continue to pass. This closes actual rendered-depth correctness for the fixture, not GPU-resident scene integration: color/depth pass through CPU readback before compute upload. In-scattering, calibrated fog, runtime editor attachment and NVIDIA remain open.

### Scene color/depth stay on GPU through extinction computation (2026-10-07)

ComputeProgram::with_scene_textures extends the existing storage ownership with read-only 2D color/depth bindings. create_scene_job validates bind resources asynchronously; incompatible texture types reject, ordinary create_job rejects a missing scene ABI, shader reload retains the selected ABI and existing jobs retain their pipeline. DropletExtinctionSceneInput packs field/camera/layout, while the shader loads actual opaque textures, reconstructs rays and performs direct transmission entirely on GPU using 8x8 pixel dispatch. Malformed dimensions/depth/unprojection and uncomputed or nonfinite readback reject.

Apple M4 Max/Metal qualifies 16384 rendered pixels before any CPU scene readback; maximum difference from the independent CPU-upload control path is 1.78814e-7. Foreground/occlusion checks and prior 1158 traversal rays pass. Default compute smoke also passes 1042 exact GPU results and shared budget/readback cancellation/retirement gates. Evidence: artifacts/droplet-extinction-resident-scene-2026-10-07. Final result is read back only for qualification; direct presentation, editor attachment, 4K memory qualification, in-scattering and NVIDIA are still open.

### Resident extinction output to graphics (2026-10-07)

`StorageColorBlit` reads row-major linear RGBA f32 words directly from resident
storage in the fragment stage. It records a fullscreen render pass after the
producing compute pass; no staging allocation, CPU pixel mapping, or CPU pixel
upload is required for composition. `DropletExtinctionSceneInput::color_output`
provides the bounded output range. The producer's allocation owner must remain
alive through submission/completion. Storage range, usage, attachment dimensions,
sample count, and supplied pipeline device are admitted before recording the
pass; backend validation scopes report resource/pipeline incompatibilities.
The attachment must be a base-level single-sample 2D view on that device.

The existing `droplet_extinction_smoke` now executes rendered opaque scene →
scene-texture compute → storage-to-color render pass on Metal. Qualification
CPU mappings occur after composition and never feed the display pass. For all
16,384 pixels, linear RGBA8 and sRGB outputs match independent quantization and
sRGB encoding within one byte. Foreground/depth and CPU reference comparisons
remain intact. A separate 13×3 fixture with a five-word prefix checks every
RGBA16F bit against independently converted values, including RGB above one
and varying alpha. The captured comparison PNG now contains the actual color
attachment bytes, rather than a host reconstruction of the compute output.

Evidence: `artifacts/droplet-extinction-presentation-2026-10-07/`.
This proves offscreen GPU composition on Apple M4 Max/Metal. It does not prove
window/editor presentation, frame-time performance, NVIDIA execution, or
scattered light. HDR can feed the existing `ProcessedColorTarget`/tone-mapping
pipeline; its end-to-end window integration is still outstanding.

### Extinction pipeline in native surface submission (2026-10-07)

`DropletExtinctionPass` caches the scene-texture compute pipeline and HDR storage
composition pipeline. `prepare` admits texture dimensions/usages/sample count,
output bytes against an explicit limit, and compute storage against the shared
device budget. `DropletExtinctionFrame` owns the job and RGBA16F target; callers
encode it after opaque scene production and use its linear output with the
existing exposure/tone-mapping passes. Physics owners and input textures remain
external. Preparation failures return before presentation. Frame owners must
remain alive through completion; output limits are per frame, not a global
texture-residency accounting claim.

The existing `temporal_surface_smoke` accepts `VOXY_TEMPORAL_EXTINCTION=1` and
uses `SceneSurface::render_scene_with_temporal_hooks`. The fixture's orthographic
camera matches its geometry transform; a one-metre segment through sigma=0.5/m
must produce linear center RGBA `[4*exp(-0.5), exp(-0.5), 0, 1]`. Center readback
is qualification only and is not uploaded into the compute/display path. The
same lifecycle test injects a discarded consumer frame, verifies that its
presentation ID is not consumed and temporal history is invalidated, and
changes the window/attachment dimensions.
The fixture renders world draws only and does not qualify editor UI ordering.

Direct background launch returned only `SkippedOccluded`. Launching the same
locally built executable as a macOS application produced four presentations:
three at 320×240 and one at 321×241. Each admitted frame checks a rejected zero
output budget and the analytic HDR center. No occluded acquisition is counted
as a presented frame. Retry redraws now use 16ms event-loop deadlines rather
than rapidly exhausting the attempt counter. Evidence is preserved in
`artifacts/droplet-extinction-window-2026-10-07/`.

This qualifies the native surface hook, not an authoring fog component or an
editor play scene. Camera changes, UI ordering, optical fluid composition,
scattered illumination, whole-scene resource budgets, and NVIDIA runtime
qualification remain outstanding.

The unchanged ordinary temporal-window path also completed with exit code zero
using the same final executable; the render library passed 155 tests with 22
explicitly ignored cases.

### Directional single scattering (2026-10-07)

Extinction now optionally adds radiance from one infinitely distant directional
source through the same scene-texture compute and storage composition path.
`DirectionalScatteringOptions` supplies irradiance (on a plane normal to the
beam outside the medium), scalar single-scattering albedo, HG asymmetry, and
sample count. Directions are normalized with scaling to avoid overflow or
subnormal normalization errors. Optical calibration is explicit; water/oil
spectral coefficients, particle-size-dependent Mie phase fits, and scene
photometric units are not inferred.

For each clipped primary subsegment, the shader integrates its extinction
exactly, then weights the incident source at its midpoint by a medium shadow
ray and the preceding camera-path transmission. The normalized HG function
uses photon travel directions: positive g favors forward scattering. A stable
polynomial evaluates `1-exp(-tau)` for thin subsegments. The GPU input admits
1–128 samples and checks a caller-supplied total DDA visit budget; failure is
`ComputeError::WorkBudget`, distinct from byte capacity. The HDR frame facade
rejects source bounds above RGBA16F range. These are admission limits, not a
frame-time or general quadrature-error guarantee.

Reference foundations: [beam transmittance](https://www.pbr-book.org/4ed/Volume_Scattering/Transmittance)
and [phase functions](https://www.pbr-book.org/4ed/Volume_Scattering/Phase_Functions).
The native f64 reference uses independent all-cell line integration, leaves
physical owners unchanged, and supports refinement through 4096 samples under
an explicit cell-test budget. Uniform-slab tests compare both light directions
against closed integrals; refinement decreases error approximately fourfold.
Numerical HG integration independently checks normalization and mean cosine.
Validation also covers zero/missing segments, zero albedo, invalid calibration,
scale-invariant directions, and transparent fields under intense illumination.

Metal qualification retains the existing depth/transmission/HDR regressions.
Eight slab cases test isotropic and forward scattering in both light directions
at optical thickness 0.5 and 1e-6. Maximum closed-solution RGB error is
8.32e-7; thin-medium scattering remains nonzero. A rendered 128×128 scene checks
all 16,384 pixels against the native reference: 1,792 receive scattered light,
maximum native RGB difference is below 8.1e-7, foreground is unchanged, and
alpha is preserved. The comparison PNG contains actual GPU attachment bytes.

`VOXY_TEMPORAL_EXTINCTION=1 VOXY_TEMPORAL_SCATTERING=1` in the existing native
surface test checks the analytically predicted scatter contribution through
RGBA16F and tone mapping. A macOS application launch completed four presented
frames (320×240, then 321×241), including consumer-error/history invalidation
and resize recovery; its exit status is zero. The render library passed 155
tests (22 ignored) and optical physics passed four tests. Evidence is in
`artifacts/droplet-single-scattering-2026-10-07/`.

This remains deterministic single-scattering quadrature. Multiple scattering,
solid-object shadows on incident light, optically thick accuracy, adaptive
quadrature, spectral calibration, dynamic optical/thermal coupling, editor
component authoring/UI ordering, high-resolution performance and NVIDIA
execution remain unqualified. The analytic fixtures prescribe optical
coefficients; they do not qualify a calibrated real water/oil cloud.


### Dense directional transport qualification (2026-10-07)

The current CPU and WGSL paths replace midpoint source sampling with analytic
integration of linearly interpolated shadow optical depth on each primary
subsegment. Optical depth is accumulated rather than multiplying prefix
transmissions. The shader snaps clipped entry/exit coordinates to the exact
intersected grid plane; this corrects a measured Metal error at optical
thickness 10,000 without relaxing the observable-value tolerance. Work admission
now bounds primary and two shadow traversals: pixels × (3 × samples + 1) ×
(nx + ny + nz + 3). General heterogeneous shadow paths remain approximate.

Five optical physics tests pass. Uniform slabs match closed solutions through
optical thickness 20,000 in f64. A heterogeneous shadow fixture refines from
2.264e-4 error at four samples to 2.239e-7 at 128 samples against a 4096-sample
reference. Metal passes 20 directional slab cases (sigma 1e-6 through 10,000,
two light directions, two asymmetries), with maximum absolute RGB error
5.332e-7. Highly attenuated values below the GPU normal range may flush to zero;
the fixture explicitly allows a 1e-37 absolute floor. The 16,384-pixel native
scene comparison has maximum RGB error 3.297e-7 and preserves foreground/alpha.
Initial failure and corrected results are retained in
`artifacts/dense-scattering-2026-10-07/`. Earlier window evidence qualifies the
preceding implementation; the endpoint revision has offscreen Metal evidence.
Multiple scattering, solid-object incident shadows, calibrated water/oil,
editor integration, high-resolution performance and NVIDIA remain unqualified.


### Endpoint transport in a native window (2026-10-07)

The current endpoint-integrating shader is now qualified through the existing
native temporal surface hook. `VOXY_TEMPORAL_EXTINCTION_SIGMA` selects a finite
coefficient in [0, 10000] for the qualification slab; malformed values are
rejected. The sigma=10000 case completed four presented frames, resizing from
320×240 to 321×241, with exact nearest-RGBA16F agreement with the saturated
analytic center RGB [0.127323955, 0.063661978, 0.031830989] and alpha 1.
Consumer failure still discards the submission, invalidates history and leaves
the presented identifier unconsumed. No CPU scene color/depth upload is used.

The same final executable also passes the sigma=0.5 baseline with its original
0.002 absolute center tolerance. An attempted exact-half assertion failed that
baseline by one half step; the failed log is preserved. Exact agreement is only
asserted for the saturated fixture, whose transport result is insensitive to
small surface-path differences. The finite-depth discrepancy has not been
causally qualified. Logs, executable hash and result metadata are in
`artifacts/dense-scattering-window-2026-10-07/`. This is a native window fixture,
not editor authoring integration, general error certification or NVIDIA proof.


### Opaque object shadows in directional fog (2026-10-07)

`DropletExtinctionPass::with_directional_shadow` now reuses `ShadowMap` and
`ShadowSettings` from surface lighting. The shadow depth and uniform bindings
are explicitly exposed to compute through a retained additional bind-group
layout; shader reload retains that ABI. The pass retains a settings snapshot
and the frame retains its shadow bindings through completion. Rasterize the
opaque map before fog on the same queue. Scattering calibration and the map's
light direction must match; this remains the caller's responsibility. Shadowed
passes reject inputs without directional scattering before output allocation.

Both paths share the exact depth comparison, bias, outside-projection lit
policy and Hard/PCF3×3/PCF5×5 filter implementation. Medium shadow optical depth
is still endpoint-integrated; opaque visibility is sampled at the primary
subsegment midpoint. Therefore arbitrary shadow discontinuities along a ray
remain approximate and need refinement, not a general error guarantee. The
DDA work budget excludes PCF texture-tap cost; overall GPU time is unqualified.

Metal passes twelve raster cases: empty, full, half and disabled shadows for
each filter. Actual opaque geometry generates the shadow texture and actual
scene attachments feed fog. Interior HDR pixels agree with independently
predicted blocked/unblocked slab radiance within 1e-4, with alpha preserved;
the two split boundary columns are excluded from analytic classification.
No CPU shadow-depth or scene color/depth pixels are uploaded. Existing 20 slab
cases, 16,384-pixel scene comparison and 155 library tests still pass (22 ignored).
Evidence is in `artifacts/solid-shadow-scattering-2026-10-07/`.
Transparent casters, directional-map/source consistency validation, moving
shadow snapshots, cascade coverage, along-ray discontinuity certification,
window/editor shadow integration, multiple scattering and NVIDIA remain open.


### Moving opaque fog shadows and source admission (2026-10-07)

A cached shadow-enabled fog pass now has Metal evidence across repeated
empty/full/split/moved/cleared maps. The half-width caster moves by 0.5 world
metres without recompiling the fog pipeline. Twenty-four rendered cases
(eight sequential states per Hard/PCF3×3/PCF5×5 filter) match analytic interior
blocked/unblocked slab radiance. Twenty-one subsequent map updates preserve
all RGBA16F bits of the previous completed output attachment. This is completed
output preservation: a prepared frame samples the live map at execution time;
it does not freeze the map's texels. Encode map writes before its consumers
and subsequent updates afterward on the same queue.

Source consistency is now admitted explicitly. A directional shadow projection
must be finite, invertible and affine with positive homogeneous W; perspective
and singular projections are rejected. Inverse clip Z identifies photon
travel in world space, including affine shear. An enabled pass rejects a
scattering direction whose dot product with the direction toward that light
is below 1-1e-5 before output allocation. Three filters exercise the reversed
source and two invalid projections (nine rejected cases total). This replaces
the previous caller-only direction-consistency boundary. Bias and enabled
settings remain validated by the shared surface-shadow bindings.

Evidence is in `artifacts/moving-shadow-scattering-2026-10-07/`. Moving light
transforms, map-generation identity, window/editor shadow integration, PCF
boundary error, along-ray shadow discontinuity certification, transparent
casters, cascades, multiple scattering and NVIDIA remain unqualified.


### Durable fog authoring and frame extraction (2026-10-07)

`voxy_scene::FogVolume` is a renderer-independent authored uniform medium in
its owner's local [0,size] box. `scene.fog.v1` is registered in the editor's
existing component registry, so documents and generic inspector/history
codecs retain it. Calibration supplies extinction per world metre, albedo,
HG asymmetry and sample count; light remains a separate input. GPU handles,
physical state and optical density inferred from arbitrary liquid radii are
not stored in the component. Deserialization rejects invalid calibration and
unknown fields. Editor save/load validation also checks mutated live data.

FrameStyles now owns active/enabled fog snapshots with world bounds under its
existing extraction schedule. Capacity and calibration errors abort snapshot
preparation. Exact axis-aligned signed scales and axis permutations preserve
the local box; arbitrary rotation/shear is explicitly rejected instead of
inflating the medium into an incorrect AABB. GPU-unrepresentable bounds and
positive extinction that underflows to zero in f32 are rejected. Extinction
per metre is unchanged by owner scale, while the path length changes naturally.

The scene library passes 71 tests, and the editor passes 176 (13 ignored).
New evidence covers typed document round-trip, stable canonical repeated
capture, undo/redo and rejected-edit preservation, inherited activity,
disabling/removing fog, capacity, prior snapshot independence and unsupported
transforms. Raw decimal JSON is normalized to the declared component numeric
types; the initial equality-test failure and corrected checks are retained in
`artifacts/fog-authoring-2026-10-07/`.

This completes durable data and extraction only. The ordinary editor's SDR,
multiple viewport, UI-overlay and optical-liquid compositor still needs to
consume these snapshots through the HDR fog pass. No editor fog pixels are
claimed. Interpolated physics poses, rotated media, multiple-volume transport,
light/shadow authoring and editor GPU acceptance remain open.


### HDR composition into editor view regions (2026-10-07)

`TextureBlit::encode_viewport` maps a whole processed source into a specified
base-level target region, loading the initialized attachment and scissoring
writes instead of clearing neighbouring views. The original full-target
composition path retains its clear. Existing identity, exposure and tone-map
pipelines share the region encoder; global/per-view overlays can follow it.
Empty, overflowing and out-of-bounds regions are rejected before binding
allocation or command recording. Callers retain texture/encoder device and
base-level view ownership contracts.

Metal qualification composes a synthetic GPU RGBA16F storage-produced 13×3
source twice into an initialized 37×11 HDR target at distinct offsets. All
78 destination pixels are checked: identity RGBA bits are exact, tone-mapped
RGB differs by at most one half ULP, and alpha is preserved. All 329 outside
pixels remain bit-identical; four invalid viewport admissions are rejected.
The existing 16,384-pixel native scene, 20 slab and 24 moving-shadow cases
still pass, as do 155 render library tests (22 ignored). Evidence is in
`artifacts/fog-viewport-composition-2026-10-07/`.

This is the compositor prerequisite, not completed editor fog presentation.
Actual UI overlays, HDR scene draw pipelines, compatible optical-fluid output
and merged optical depth, multiple-volume transport, per-view allocation and
editor acquisition/lifetime acceptance still need to be connected and verified.


### Editor fog consumer and native acceptance (2026-10-07)

The existing editor draw loop now consumes active `scene.fog.v1` snapshots in
its ordinary and Play presentation path. FogDraw owns a cached HDR scene
renderer using the editor's material shader, the existing optical compute
pass, display pass and view-sized color/ordinary/X-ray depth targets. Each
view rasterizes world geometry into RGBA16F, transports radiance through the
medium, tone maps into its window region, then draws local controls and global
UI with the ordinary display renderer. Scene pixels/depth are not uploaded
from CPU. Existing DirectionalLight supplies the separate incident-light input
in scene intensity units; real photometric calibration is not asserted.

Models and fog now share the same render-world resolver, including simulation
interpolation. Completed optical frames are retired only after device
completion; disabling fog releases its graphics owner. This first lifetime
implementation waits on the device, so frame-time/performance is unqualified.
Fog image/storage bytes participate in the scene resource accounting. Resize
checks planned/peak bytes and GPU storage binding limits before target
allocation. The first DPI=2 native run exposed an inappropriate fixed 64 MiB
input cap; input admission now derives from device binding capacity under the
already-admitted scene budget. Failure evidence is preserved.

A real opaque/fog two-view GPU test checks all 2560 output pixels, including
128 local overlay pixels and 512 global UI pixels, against slab/tone-map or
untouched-overlay expectations. The second camera misses the medium and is
not attenuated. The editor library passes 177 tests (14 ignored), and this
physical Metal test passes explicitly. Native Scene3D acceptance on a temporary
project copy completed 19 presented fog frames at physical window size
1280×1360, including selection, edits, 26 Play ticks, Stop and reload, exit zero.
The additional fog-only node exposed model picking's assumption that every
object had geometry; picking now skips meshless objects, with a regression test.

The initial launch against the Documents project remains observed in a
filesystem-open wait before UI creation (PID 68705). Its timeout is not treated
as completion or a dead process. Native acceptance uses copied synthetic
fixture assets in /tmp; no OS permission setting was changed. Evidence,
initial failures, scene snapshots and executable hash are in
`artifacts/editor-fog-presentation-2026-10-07/`.

This initial consumer supports one axis-aligned uniform fog volume and a
single-sample scene camera. Multiple media, MSAA and simultaneous screen-space
optical liquids are explicitly rejected until their physical composition is
implemented. The editor's fog does not yet consume opaque scene shadow maps;
those have separate renderer-level evidence. Rotated volumes, editor light
shadow authoring, fog/liquid merged optical depth, asynchronous retirement,
high-resolution performance, spectral calibration, multiple scattering and
NVIDIA execution remain open. The full engine objective is still unfinished.
