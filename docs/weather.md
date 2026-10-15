# Procedural weather

The native `street_walk` example accepts `--weather clear|cloudy|rain|snow`.
During the walk, keys **1–4** select those four presets. `--sun-elevation 25`
sets the sun elevation in degrees; the directional light and geometry shadow
map use the same direction.

Example: `cargo run -p voxy_app --example street_walk -- --weather rain --sun-elevation 35`.

`voxy_render::weather::Weather` exposes cloud coverage/density, layer height and
thickness (metres), rain/snow intensity and horizontal wind (metres/second).
`ArchiveSceneRenderer::set_weather`, `set_weather_time` and `set_sun` update
these independently of imported geometry. Clear is the compatibility default.

Clouds use 32 Beer–Lambert ray integration steps through a moving, three-octave
noise volume with approximate directional self shading. Rain and snow use 64
world-space sampling steps within 36 metres. Scene depth clips both effects.
Cloud coverage reduces direct sunlight. This is an initial bounded renderer:
there are no precipitation collisions, accumulated snow, wet surfaces, lightning,
or spatial cloud shadows on geometry. Dense weather increases fullscreen cost,
particularly with spatial AA. Imported panorama lighting remains authored.

## Wind

Use `--wind-speed 12 --wind-direction 45 --wind-gust 0.6` to set speed in
metres/second, direction in degrees (0 toward +X, 90 toward +Z), and gust
amplitude from 0 to 1. Direction is where air moves, not a compass bearing
for where it comes from. F5 toggles calm/wind, F6 rotates direction by 45°,
F7/F8 decrease/increase speed by 2 m/s. Switching weather preserves wind.

Smooth gusts advect the cloud volume, slant rain streaks and carry snow with
lateral flutter. Calm removes horizontal drift and wind-driven flutter.
This atmosphere wind does not yet apply forces to bodies or bend vegetation.
