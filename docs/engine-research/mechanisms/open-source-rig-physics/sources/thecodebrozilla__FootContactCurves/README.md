# Foot Contact Curves

An Unreal Engine **Animation Modifier** that bakes per-foot `contact_l` / `contact_r` curves onto your AnimSequences from foot bone height and speed thresholds. Drop-in compatible with **Game Animation Sample Project (GASP)**, motion-matching pipelines, and any system that expects 0/1 foot-contact gates for foot-lock IK, stride warping, or pose-search features.

![Foot Contact Curves in action](screenshots/main.png)

---

## Why this exists

Epic's GASP locomotion clips ship with `contact_l` and `contact_r` curves baked in — but **the tool that generates them was never released**. Epic confirmed on a livestream that the curves were authored by an internal unreleased script.

If you bring custom locomotion animations into a GASP-based project (or any motion-matching setup that consumes contact curves), you're stuck either hand-keying every clip or writing the tool yourself.

This plugin closes that gap. Drop new clips into your project, multi-select them in the Content Browser, right-click → Apply, and every clip gets GASP-shaped contact curves.

---

## Installation

1. Copy the entire `FootContactCurves` folder into your project's `Plugins/` directory so you end up with:
   ```
   YourProject/Plugins/FootContactCurves/FootContactCurves.uplugin
   ```
2. Right-click your `.uproject` → **Generate Visual Studio project files**.
3. Build your editor target (in Visual Studio, or via your usual UBT command).
4. Open the project. The plugin auto-enables.

No engine source modifications. No extra plugin dependencies. Editor-only — nothing ships in your packaged builds.

---

## Usage

1. In the Content Browser, **multi-select** the AnimSequences you want to bake.
2. Right-click → **Animation Asset Actions → Apply Animation Modifier**.
3. Pick **Foot Contact Curves**.
4. Adjust settings if you need to (defaults work for the UE5 mannequin), click **Apply**.

Open any of the baked sequences and check the **Curves** panel — `contact_l` and `contact_r` will be there.

---

## Settings

| Property | Default | Notes |
|---|---|---|
| **Bake Left Foot** | `true` | Skip the left foot when off. |
| **Foot Bone Name L** | `foot_l` | Change for non-mannequin rigs (e.g. `LeftFoot`, `Bip01_L_Foot`). |
| **Curve Name L** | `contact_l` | Standard for GASP / Motion Matching. |
| **Bake Right Foot** | `true` | Skip the right foot when off. |
| **Foot Bone Name R** | `foot_r` | Same notes as left. |
| **Curve Name R** | `contact_r` | |
| **Sample Rate** | `30` | Sampling FPS (range 10–240). Higher = more accurate, slower bake. |
| **Ground Height Threshold** | `6.0` | Centimeters. A foot is "planted" when its height above the lowest foot Z over the clip is below this. |
| **Vertical Speed Threshold** | `30.0` | cm/sec. A foot must be moving slower than this vertically to count as planted. |
| **Interpolation Mode** | `Linear` | `Linear` for smooth ramps, `Constant` for sharp 0/1 plateaus, `Cubic` for eased transitions. Pick to match your downstream consumer. |
| **Overwrite Existing Curves** | `true` | Removes the existing curve with the configured name before baking. |

---

## How it works

Per foot, per clip:

1. **Pass 1 — find the ground.** Sample the foot bone every frame in component (world) space; the lowest Z it ever reaches over the clip is the ground level. This handles slopes and uneven authored heights without manual tuning.
2. **Pass 2 — classify each frame.** Mark the foot as planted (`1`) when both:
   - foot height above ground < `GroundHeightThreshold`, **AND**
   - foot vertical speed (`|ΔZ|` between consecutive frames) < `VerticalSpeedThreshold`
   - Otherwise mark as in-air (`0`).
3. **Write the curve** through `IAnimationDataController::SetCurveKeys` with the chosen interpolation mode. Transactional, so it shows up in the editor's undo stack.

Each bake logs to `LogFootContactCurves` (Output Log) with sample count, planted percentage, detected ground level, and the active thresholds — useful for tuning.

---

## Tuning

If the baked curve doesn't look right, check the **Output Log** for the bake stats and try:

| Symptom | Likely cause | Fix |
|---|---|---|
| Curve is flat 0 (nothing registers as planted) | `GroundHeightThreshold` too low, or wrong foot bone name | Bump threshold to 8–10 cm; verify bone name |
| Curve is flat 1 (everything is "planted") | Threshold too high, or `VerticalSpeedThreshold` too lenient | Lower `GroundHeightThreshold` to 4–5 cm; tighten speed |
| Curve flickers between 0 and 1 mid-step | Sampling aliasing on subtle motion noise | Raise **Sample Rate** to 60; widen thresholds slightly |
| Curve plateaus too soft / too sharp | Interpolation mode mismatch | Switch between `Linear` / `Constant` / `Cubic` |

---

## Reverting

Modifiers are reversible. With the AnimSequence open, find the **Animation Modifiers** panel in the asset Details; press **Revert** on the Foot Contact Curves entry to delete the curves it added. Or just re-apply with `Overwrite Existing Curves = true` to overwrite.

---

## Compatibility

- Tested on **Unreal Engine 5.7**. Should work on 5.4+ (relies on `UAnimationModifier`, `IAnimationDataController`, and `UAnimPoseExtensions`, all stable since 5.2).
- Requires only the engine modules `AnimationModifiers` and `AnimationBlueprintLibrary` — both ship with the editor.
- Plugin module is `Type: Editor`, so it never compiles into shipping/cooked builds. Zero runtime cost.

---

## License

Released under the [MIT License](LICENSE). Copyright (c) 2026 Shane Beal.
