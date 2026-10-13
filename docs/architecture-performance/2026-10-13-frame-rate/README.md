# Frame rate of the voxel app (2026-10-13)

Headless harness: `xvfb-run -a -s "-screen 0 640x360x24" target/release/voxy_app --autopilot`
on the container's software Vulkan driver (llvmpipe, LLVM 20, four vCPUs). The autopilot
runs a fixed 600 ticks at 60 Hz, so one run is 10 s of real time and the presented frame
count is the frame rate.

## Startup

The app did not start at all on this driver: `Renderer::new` requested
`EXPERIMENTAL_RAY_QUERY` whenever the adapter advertised it, and wgpu 30 refuses that
feature unless the device opts into experimental features (an `unsafe` call the render
crate denies). Every adapter that advertises ray query (llvmpipe, current desktop GPUs)
failed device creation with `ExperimentalFeaturesNotEnabled`. The request is removed;
`has_ray_query` still reports the capability for devices created with the opt-in (the ray
probe crate does that). After the fix the autopilot passes.

## Where the frame goes

| measurement | result |
| --- | ---: |
| 640×360, surface default (fifo) | 835 / 811 / 840 frames per 10 s |
| 1280×720, surface default | 850 frames per 10 s |
| 640×360, `--present-mode immediate` | 897 |
| 640×360, `--present-mode mailbox` | 919 |
| callgrind, whole process | app code 3.3% of instructions; llvmpipe + LLVM JIT + JIT'd code 95% |
| strace `-c`, whole process | 97% of syscall time in `futex` (main thread waiting on driver threads) |

PROBES

The frame rate is set by the software driver, not by the app: the app's own code is a few
percent of the instructions, resolution barely matters, and presentation pacing moves the
count by less than 10%. On a hardware GPU the same loop will be bounded by vertical sync
unless an uncapped present mode is chosen, which is what the new option provides.

## What changed

- `GraphicsOptions::present` (`PresentPreference::{Default, Fifo, Mailbox, Immediate}`)
  applied at surface configuration when the surface supports it; the app parses
  `--present-mode auto|fifo|mailbox|immediate`. Default behaviour is unchanged.
- The ray-query request at device creation is gone (startup fix above).

## Gates

`cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check`
clean; `voxy_app` graphics option parser tests 2 passed; headless autopilot passed in
all three present modes.
