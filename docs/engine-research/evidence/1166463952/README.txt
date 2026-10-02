<p align="center">
<pre>
██████╗  █████╗ ██╗      █████╗ ████████╗██████╗  ██████╗    ████████╗██╗   ██╗██╗
██╔══██╗██╔══██╗██║     ██╔══██╗╚══██╔══╝██╔══██╗██╔═══██╗   ╚══██╔══╝██║   ██║██║
██████╔╝███████║██║     ███████║   ██║   ██████╔╝██║   ██║      ██║   ██║   ██║██║
██╔══██╗██╔══██║██║     ██╔══██║   ██║   ██╔══██╗██║   ██║      ██║   ██║   ██║██║
██████╔╝██║  ██║███████╗██║  ██║   ██║   ██║  ██║╚██████╔╝      ██║   ╚██████╔╝██║
╚═════╝ ╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝   ╚═╝   ╚═╝  ╚═╝ ╚═════╝       ╚═╝    ╚═════╝ ╚═╝
</pre>
</p>

<p align="center">
  <b>Play <a href="https://www.playbalatro.com/">Balatro</a> in your terminal. No modifications to game code.</b>
</p>

<p align="center">
  <a href="https://github.com/4RH1T3CT0R7/balatro-port-tui/releases"><img src="https://img.shields.io/github/v/release/4RH1T3CT0R7/balatro-port-tui?style=flat-square&color=blue" alt="Release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue?style=flat-square" alt="License"></a>
  <img src="https://img.shields.io/badge/rust-1.75%2B-orange?style=flat-square&logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/lines-~9800-informational?style=flat-square" alt="Lines of Code">
</p>

---

<p align="center"><img src="assets/demo.gif" alt="Demo" width="800"></p>

<p align="center">
  <img src="assets/screenshot-title-menu.png" alt="Title screen" width="800">
</p>

<p align="center">
  <img src="assets/screenshot-sixel.png" alt="Gameplay (Sixel mode)" width="700">
</p>

---

## What is this?

A Rust reimplementation of the **LOVE2D game engine** that renders entirely in a terminal. It runs **unmodified Balatro Lua code** — the game thinks it's talking to a real LOVE2D runtime, but all graphics go through a software pixel buffer and are displayed via the Sixel graphics protocol (true pixels), Unicode octant characters (2×4 sub-pixels per cell), or half-block characters.

The result: a fully playable Balatro in your terminal.

## Quick Start

### Prerequisites

- **Rust 1.75+** with a C compiler (for vendored Lua 5.1)
- **A copy of [Balatro](https://store.steampowered.com/app/2379780/Balatro/)** (`Balatro.exe`)
- **A modern terminal** with 24-bit color support (Windows Terminal 1.22+, WezTerm, iTerm2, kitty, Alacritty)

### Build

```bash
cargo build --release
```

> **Windows tip:** If your project is on OneDrive, use a separate build directory to avoid sync lag:
> ```bash
> CARGO_TARGET_DIR="C:/tmp/balatro_build" cargo build --release          # Git Bash
> ```
> ```powershell
> $env:CARGO_TARGET_DIR="C:/tmp/balatro_build"; cargo build --release    # PowerShell
> ```

### Run

**Linux / macOS / Git Bash:**
```bash
cargo run --release -- "path/to/Balatro.exe"
```

**Windows PowerShell:**
```powershell
cargo run --release -- "C:/Program Files (x86)/Steam/steamapps/common/Balatro/Balatro.exe"
```

> Sixel mode is used by default. Set `TUI_RENDER=octant` or `TUI_RENDER=halfblock` to switch (see [Configuration](#configuration)).

### Download Pre-built Binaries

Pre-built binaries for Windows, Linux, and macOS are available on the [Releases](https://github.com/4RH1T3CT0R7/balatro-port-tui/releases) page.

```bash
./love-terminal "path/to/Balatro.exe"                  # Linux / macOS
```
```powershell
.\love-terminal.exe "path\to\Balatro.exe"              # Windows PowerShell
```

## Controls

| Key | Action |
|-----|--------|
| **Arrow keys** | Navigate menus |
| **Enter** | Confirm / Play hand |
| **Escape** | Back / Cancel |
| **E** | Select / deselect cards |
| **Tab** | Switch between hand and jokers |
| **Space** | Sort hand |

> Balatro's UI is designed for gamepad input. Keyboard keys are automatically mapped to `gamepadpressed`/`gamepadreleased` events for full compatibility.

## How It Works

### Rendering Pipeline

```
Balatro Lua code
    │
    ▼
love.graphics.* API calls (Rust)
    │
    ▼
Software pixel buffer + shader emulation
    │
    ▼
Terminal output
    ┌──────────────────────────────────────┐
    │ Sixel: true pixel graphics           │ ← best quality
    │ Octant: 2×4 sub-pixels/cell          │ ← default
    │ Half-block: ▀ fg/bg per cell         │ ← fallback
    └──────────────────────────────────────┘
```

The project embeds a **Lua 5.1 VM** via [mlua](https://github.com/khvzak/mlua) and implements ~80 `love.*` API functions. All graphics pass through a software rasterizer — rectangles, ellipses, polygons, sprites, text — with anti-aliasing, bilinear filtering, and proper alpha blending.

**Sixel mode** renders actual pixels via the [Sixel graphics protocol](https://en.wikipedia.org/wiki/Sixel). The internal canvas matches the terminal's pixel area (e.g. 1080×600 on a 120×30 terminal), giving true pixel-level fidelity with 256-color quantization per frame. **Octant mode** (default) uses Unicode octant characters (🬀–🬻) to achieve 2×4 sub-pixel resolution per cell with gamma-correct downsampling. **Half-block mode** uses `▀` (U+2580) with fg=top/bg=bottom as a universal fallback.

### Architecture

```
balatro_port_tui/
├── love-terminal/          # Binary crate — entry point + game loop
│   └── src/
│       ├── main.rs         # CLI parsing, path resolution
│       └── runner.rs       # Terminal setup, frame loop, auto-input
├── love-api/               # Library crate — LOVE2D API implementation
│   └── src/
│       ├── lib.rs          # LoveRuntime: Lua VM + module registration
│       ├── state.rs        # SharedState (Arc<Mutex> fields)
│       ├── graphics.rs     # love.graphics.* (3,400+ lines)
│       ├── filesystem.rs   # love.filesystem + zip loader + require
│       ├── event.rs        # love.event (keyboard/mouse → events)
│       ├── keyboard.rs     # love.keyboard.isDown
│       ├── window.rs       # love.window (terminal as display)
│       ├── timer.rs        # love.timer (delta, FPS, sleep)
│       ├── system.rs       # love.system (getOS → "Linux")
│       ├── stubs.rs        # audio, thread, mouse, joystick stubs
│       └── lua_util.rs     # Color parsing helpers
└── sprite-to-text/         # Library crate — pixel buffer + renderer
    └── src/
        ├── pixel_buffer.rs # RGBA framebuffer (2,000+ lines)
        └── renderer.rs     # Downsampling → ratatui terminal output
```

**~9,800 lines of Rust** across 16 source files.

## Features

### Graphics Engine
- **Software rasterizer** — rectangles, rounded rects, ellipses, polygons, thick lines, all anti-aliased
- **Sprite rendering** — `love.graphics.draw` with images, quads, sprite batches, bilinear filtering
- **Text rendering** — TTF via [fontdue](https://github.com/mooman219/fontdue), colored segments, word-wrapped `printf`
- **Transform stack** — translate, scale, rotate with matrix composition
- **Canvas system** — offscreen render targets with premultiplied alpha
- **Stencil buffer** — write + test operations for masked rendering
- **Blend modes** — alpha, replace, additive, multiply, premultiplied

### Shader Emulation

Since there's no GPU, all shaders are emulated per-pixel in Rust:

| Shader | Effect |
|--------|--------|
| **Background** | Procedural swirl — 60×45 grid, paint distortion, 3-color mixing |
| **CRT** | Bloom extraction + Gaussian blur + contrast + vignette |
| **Dissolve** | Per-pixel noise threshold with burn-edge coloring |
| **Foil** | Radial + angular shimmer, silvery-blue metallic (from GLSL) |
| **Holographic** | HSL rainbow shift + hexagonal grid + noise field (from GLSL) |
| **Polychrome** | HSL hue rotation via animated noise, boosted saturation (from GLSL) |
| **Negative Shine** | 5-component sine wave shimmer with blue tint (from GLSL) |
| **Gold Seal** | Animated golden sine-wave highlight sweep (from GLSL) |
| **Debuff** | Desaturation + reddish tint + diagonal stripes (from GLSL) |
| **Played** | HSL desaturation + darkening |
| **Flash / Shadow / Flame** | Overlay effects (white flash, dark shadow, gradient flame) |

### Compatibility

Balatro runs unmodified thanks to several compatibility tricks:

- **`love.system.getOS()` → `"Linux"`** — skips Steam initialization
- **`require "luasteam"` stub** — returns `{ init = function() return false end }`
- **`require "bit"` in Rust** — Lua 5.1 lacks the `bit` library that LuaJIT provides; implemented as `bxor/band/bor/bnot/lshift/rshift`
- **Custom `love.run()`** — Balatro returns a per-frame closure instead of using standard callbacks
- **Keyboard → Gamepad mapping** — keys automatically fire `gamepadpressed`/`gamepadreleased` events

## Configuration

All settings are controlled via environment variables. None are required — defaults work well for most setups.

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `TUI_RENDER` | `sixel` | Rendering mode: `sixel`, `octant`, or `halfblock` |
| `TUI_PIXELBUDGET` | `250000` | Max canvas pixels for Sixel mode. **Main FPS vs quality knob.** Lower = faster, higher = sharper. See details below |
| `TUI_SCALE` | `2` | Canvas divider. For octant/halfblock: canvas = terminal_cells × scale. For Sixel: overrides PIXELBUDGET, canvas = sixel_target / scale |
| `TUI_CELL` | auto-detect | Cell pixel size for Sixel, format `WxH` (e.g. `9x20`, `11x24`). Use if the image doesn't fill the terminal or overflows |
| `TUI_COLS` | auto | Override terminal column count |
| `TUI_ROWS` | auto | Override terminal row count |
| `TUI_DEBUG` | off | Enable timing logs every 120 frames (set to any value to activate) |
| `TUI_AUTOPLAY` | off | Auto-play mode: progresses through title → blind select → play hand → shop automatically |
| `TUI_SNAPSHOT` | off | Save PPM screenshots of the pixel buffer every 200 frames (frames 200–12000) |

### How TUI_PIXELBUDGET works

This is the most important performance setting for Sixel mode. It controls how many pixels the internal canvas has.

In Sixel mode, the **Sixel target** = terminal columns × 10 × terminal rows × 20 (VT340 virtual cell size). On a large terminal (e.g. 280×70) that's 2800×1400 = **3.92 million pixels** — far too many for CPU rendering at 60 FPS.

The canvas is automatically downscaled so its total pixel count stays within the budget:

```
scale = sqrt(sixel_target_pixels / budget)
canvas = sixel_target / scale
```

| Budget | Canvas (at 280×70 term) | Quality | FPS (typical) |
|--------|------------------------|---------|---------------|
| `100000` | ~450×225 | Low | 60+ |
| `250000` (default) | ~707×354 | Good | 50-60 |
| `500000` | ~1000×500 | High | 30-45 |
| `1000000` | ~1414×707 | Very high | 15-25 |

The canvas is drawn by Lua/shaders at this reduced resolution, then the Sixel encoder scales it up to the full terminal pixel area using nearest-neighbor interpolation. Lower budget = fewer pixels to shade = faster frames.

### How TUI_SCALE works

**Octant/Half-block mode:** canvas width = terminal columns × 2 × scale, canvas height = terminal rows × 4 × scale (octant) or rows × 2 × scale (halfblock). Default `2` gives good text readability. `3` is smoother but smaller text.

**Sixel mode:** if set, overrides PIXELBUDGET. Canvas = sixel_target / scale. E.g. `TUI_SCALE=4` on a 2800×1400 target gives a 700×350 canvas.

### How TUI_CELL works

Sixel mode needs to know how many pixels each terminal cell occupies. Auto-detection chain:

1. `TUI_CELL` env var (if set, uses this directly)
2. Window pixel area via process tree walk (Windows Terminal)
3. CSI 14 t terminal query (Linux/macOS)
4. GetConsoleWindow + ConsoleFontEx (Windows legacy console)
5. crossterm terminal size query
6. DPI-based estimation (Windows — assumes Cascadia Mono at 9.6×20 per 96 DPI)
7. Fallback: 9×20

If the rendered image is too small or too large for your terminal, set `TUI_CELL` manually. To find your cell size: divide your terminal window's pixel dimensions by column × row count.

### Rendering Modes

| Mode | Resolution | Quality | Requirement |
|------|-----------|---------|-------------|
| **`sixel`** (default) | Terminal pixel area (e.g. 1080×600) | Excellent | Terminal with Sixel support |
| **`octant`** | 2×4 sub-pixels/cell (~560×280) | Good | Cascadia Code 2404.23+ |
| **`halfblock`** | ▀ top/bottom per cell | Fair | Any terminal |

**Sixel mode** renders actual pixels via the Sixel graphics protocol, bypassing the 2-color-per-cell limitation. 256-color quantization per frame. Requires: Windows Terminal 1.22+, WezTerm, foot, mlterm, xterm.

**Octant mode** uses Unicode octant characters (🬀–🬻) for 2×4 sub-pixel resolution per cell with gamma-correct downsampling. Requires Cascadia Code 2404.23+ font.

**Half-block mode** uses `▀` (U+2580) with fg=top pixel, bg=bottom pixel. Works everywhere.

### Examples

**Linux / macOS / Git Bash:**
```bash
cargo run --release -- "path/to/Balatro.exe"                                      # Sixel (default)
TUI_CELL=11x24 cargo run --release -- "path/to/Balatro.exe"                       # Sixel + manual cell size
TUI_PIXELBUDGET=500000 cargo run --release -- "path/to/Balatro.exe"               # Sixel + higher quality
TUI_PIXELBUDGET=100000 cargo run --release -- "path/to/Balatro.exe"               # Sixel + max FPS
TUI_RENDER=octant cargo run --release -- "path/to/Balatro.exe"                    # Octant mode
TUI_RENDER=halfblock cargo run --release -- "path/to/Balatro.exe"                 # Half-block fallback
TUI_DEBUG=1 cargo run --release -- "path/to/Balatro.exe"                          # Show frame timing
TUI_AUTOPLAY=1 cargo run --release -- "path/to/Balatro.exe"                       # Auto-play demo
```

**Windows PowerShell:**
```powershell
cargo run --release -- "path/to/Balatro.exe"                                               # Sixel (default)
$env:TUI_CELL="11x24"; cargo run --release -- "path/to/Balatro.exe"                        # Sixel + manual cell size
$env:TUI_PIXELBUDGET="500000"; cargo run --release -- "path/to/Balatro.exe"                # Sixel + higher quality
$env:TUI_PIXELBUDGET="100000"; cargo run --release -- "path/to/Balatro.exe"                # Sixel + max FPS
$env:TUI_RENDER="octant"; cargo run --release -- "path/to/Balatro.exe"                     # Octant mode
$env:TUI_RENDER="halfblock"; cargo run --release -- "path/to/Balatro.exe"                  # Half-block fallback
$env:TUI_DEBUG="1"; cargo run --release -- "path/to/Balatro.exe"                           # Show frame timing
$env:TUI_AUTOPLAY="1"; cargo run --release -- "path/to/Balatro.exe"                        # Auto-play demo
```

## Known Limitations

- **No audio** — `love.audio` is fully stubbed (play/stop/setVolume are no-ops)
- **No threading** — `love.thread` is stubbed
- **Mouse precision** — mouse events map terminal coordinates to canvas, but precision is limited by cell size
- **Shader approximations** — CPU-based shader emulation is faithful but not pixel-identical to GPU GLSL
- **Sixel color limit** — Sixel mode quantizes each frame to 256 colors, which is sufficient for Balatro's art style but not lossless
- **Cell size detection on Windows** — Windows Terminal doesn't expose pixel dimensions; use `TUI_CELL` env var if the image doesn't fill the screen

## Dependencies

| Crate | Purpose |
|-------|---------|
| [mlua](https://crates.io/crates/mlua) | Lua 5.1 VM embedding |
| [ratatui](https://crates.io/crates/ratatui) | Terminal UI framework |
| [crossterm](https://crates.io/crates/crossterm) | Cross-platform terminal I/O |
| [fontdue](https://crates.io/crates/fontdue) | TTF font rasterization |
| [zip](https://crates.io/crates/zip) | Reading game files from exe |
| [flate2](https://crates.io/crates/flate2) | Deflate compress/decompress |
| [parking_lot](https://crates.io/crates/parking_lot) | Fast mutexes for shared state |
| [anyhow](https://crates.io/crates/anyhow) | Error handling |

## License

This project is licensed under the [Apache License 2.0](LICENSE).

**Disclaimer:** [Balatro](https://www.playbalatro.com/) is a game by [LocalThunk](https://x.com/LocalThunk). This project is an independent engine reimplementation for educational purposes — it does not include any game assets. You must own a legitimate copy of Balatro to use this software.
