# Local Metal ray application

From the repository root, run `sh tools/macos/package-ray-demo.sh` to build
`target/macos/VoxyRay.app`. Launch the bundle from Finder. Space pauses,
G changes roughness, M changes metallic, +/- changes exposure and Escape exits.

Use `sh tools/macos/package-ray-demo.sh --smoke` for
`target/macos/VoxyRaySmoke.app`. This variant runs the existing 120-presentation
GPU numerical/resize/material verification and exits. Its result is recorded in
`Contents/Resources/runtime.log`; require `ANIMATED RAY PASS`, not merely an
application launch or clean build, for acceptance.

The package builds the animated example with `--no-default-features`, avoiding
the unrelated face-preview application's editor dependency. The face probe remains
available through the default `face-demo` Cargo feature.

These are local development bundles with a debug executable. They are not signed,
notarized, distributable release installers, or proof of support on other GPUs.
The ray query path remains experimental and requires a compatible Metal adapter.

The unpackaged animated example also accepts `--backend auto|metal|vulkan|dx12`
and `--require-nvidia`. On a Windows NVIDIA machine, use `--backend dx12
--require-nvidia --smoke`; on Linux use `--backend vulkan --require-nvidia --smoke`.
The existing CUDA hardware acceptance scripts include these animated presentation
checks when their ray-query option is enabled. They require a visible native
window and physical compatible hardware; cross-compilation is insufficient.

For the mirror demo, use `sh tools/macos/package-ray-demo.sh --planar` or
`--planar --smoke`. These produce `VoxyPlanar.app` and `VoxyPlanarSmoke.app`.
The three top-left controls are pause, reset history and planar temporal filtering;
Tab plus Enter/Space uses the common keyboard router. Space pauses, R resets,
T toggles planar filtering, G cycles roughness and C switches between a flat
mirror and a tessellated curved surface. Rough/curved modes use current-frame
spatial filtering and disable the planar temporal control. Their general temporal
reprojection is still pending.

The display-independent GPU graph check still creates a native window/device,
but renders to a private texture without acquiring/presenting a surface:

```sh
cargo run --locked --offline --no-default-features -p voxy_ray_probe --example planar_scene -- --experimental --backend metal --offscreen-check
cargo run --locked --offline --no-default-features -p voxy_ray_probe --example planar_scene -- --experimental --backend metal --offscreen-check --curved --roughness 0.2
```

Add `--recovery-check` to the offscreen check to intentionally destroy its device
and require one resource recreation on the same window. Offscreen checks require
HDR/UI readbacks, an empty GPU validation scope and no presentation-history commit.
They cannot establish native animation, minimization or resize acceptance. Native
smoke requires `PLANAR SCENE PRESENT PASS` after 120 actual host presentations.
`--always-on-top` is an optional window-visibility diagnostic, not an occlusion
guard bypass or an acceptance substitute.

`--resize-check` requests a different native size, validates the GPU graph at the
actual AppKit size and checks zero-size surface suspension. Retina/AppKit may
round requested physical dimensions; the current M4 Max run changed 1280x960 to
644x438 after requesting 643x437. This does not prove native minimization.
`--capture target/demo.png` saves the final offscreen display texture as PNG;
it is a GPU image rather than a screenshot of a presented window.

The standalone numerical spatial-filter check is
`cargo run --locked --offline -p voxy_render --example reflection_spatial`.
It covers noisy HDR averaging, compatible curved normals and geometry/plane/
normal/material/distance/miss/nonfinite rejection. Spatial filtering is a biased
current-frame estimator; general moving rough/curved temporal filtering remains
pending. The demo's ray acceleration contains its emissive triangle, while the
reflecting mesh supplies raster guides; this is a single-reflector scene.

## Packaged game application

Build `voxy_app` and export a resource package with its existing `--export-game`
entrypoint, then create a local app without modifying the source scene:

```sh
tools/macos/package-game.sh /absolute/path/to/voxy_app /absolute/path/to/game.vpak /absolute/path/to/VoxyGame.app
```

Add `--smoke` for the existing native game acceptance mode. The builder refuses
an existing output, validates the plist, and includes the executable and package.
The launcher writes unique logs under `~/Library/Logs/Voxy/Game`, outside the
application bundle; `VOXY_GAME_LOG_DIR` can override that location. This is a
local application bundle, with no distribution signing or notarization implied.
