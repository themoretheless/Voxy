# Pinned complete character import

Cesium Man from KhronosGroup/glTF-Sample-Assets at the commit recorded in
assets/animation/cesium-man/SOURCE.md. Model license and attribution preserved.
Regression loads actual GLB through existing ModelAsset importer, requires
multiple joints and >1000 vertices, samples six authored phases, checks finite
palette/positions and actual deformation across phases. Focused test passed.
This is CPU import/skinning evidence, not native GPU pixel or tissue coupling
proof. Separate native review staged for subsequent visual qualification.
Local uncommitted work. Existing rigged-figure and fox fixtures also remain.

Native review: textured complete character visible through existing editor.
Pointer phase drag to 0.250 shows deformed arms and legs; to 0.750 changes
pose. F6 start/stop issued, but runtime motion was not captured for independent
pixel proof. F5 saved scene equals seed JSON. PID 87982 left open. Camera
wheel exhibited excessive zoom: physical pixel deltas were treated as lines.
Current source normalizes pixels by monitor scale and 40 logical pixels per
sensitivity unit. DPI equivalence/event-splitting/invalid-input regression and
full editor suite passed: 180 tests, zero failed/ignored. Camera fix not yet
rebuilt into this open native review. Tissue coupling remains pending.
