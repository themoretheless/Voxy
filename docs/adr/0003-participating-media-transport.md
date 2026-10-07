# ADR 0003: transport through liquid and participating media

Status: proposed; implementation and acceptance incomplete.
Date: 2026-10-07.

## Current gap

The editor explicitly rejects simultaneous fog and optical liquid. This protects
against incorrect independent composition. The current droplet transport has a
scalar extinction grid and global albedo/phase calibration; screen-space liquid
integrates RGB absorption, reconstructs a nearest surface and samples a shifted
background. Its studio reflection environment and transverse refraction offset
are approximations. Neither path provides a complete boundary-aware medium ray.

Authoritative sources: `crates/voxy_editor/src/lib.rs`,
`crates/voxy_render/src/droplet_extinction.rs`,
`crates/voxy_render/src/droplet_extinction_scene.wgsl`, and
`crates/voxy_render/src/fluid_composite.wgsl`.

## Ownership and representation

Simulation owns mass, momentum, species, temperature and phase inventories.
Render consumes a versioned immutable physical snapshot and explicit optical
calibration; optical grids and interface geometry are derivatives, never extra
mass reservoirs. Retain the existing resource admission and submitted-frame
retirement boundaries. A rejected preparation preserves the previous accepted
snapshot; encoding failure discards the encoder and unsubmitted resources.

Keep spatial coefficients separate from interface response and integration:

- Each calibrated constituent supplies absorption, scattering and phase data in
  SI units, including its spatial support. Emission/source radiance is explicit.
- Coexisting constituents in one medium add absorption/scattering coefficients.
  Scattering source terms retain each constituent's phase function; averaging
  their anisotropy parameter is not equivalent to a phase mixture.
- Surface boundaries specify inside/outside optical media and refractive index.
  Interfaces replace occupancy across the boundary; water and exterior fog must
  not be indiscriminately added through a solid liquid volume. Suspended droplets
  may coexist with carrier gas where the physical snapshot actually specifies it.
- Ordered ray segments carry RGB transmittance and accumulated source radiance.
  The immutable segment result has a defined direction and domain; it cannot be
  reused for an unrelated reflected or refracted ray.

## Transport and composition

For a segment, use `L_out = L_segment + T_segment * L_background` in linear
radiance. A front segment and a back segment compose as
`T = T_front * T_back`, `L = L_front + T_front * L_back`.
This is associative in exact arithmetic but generally not commutative.
Overlapping spatial media instead add local coefficients before integration;
stacking their independently rendered radiance is incorrect.

A one-metre homogeneous overlap with extinction 1/m for each constituent and
constant source terms (3,0,0) and (0,0,2) has transmittance exp(-2) and source
radiance `(1.5,0,1)*(1-exp(-2))`. Treating those constituents as two ordered slabs
produces a different result in each order. Exact values are preserved in
`artifacts/participating-media-design-2026-10-07/overlap-counterexample.json`.
These fixed source terms illustrate composition, not general lit fog scattering.

Trace actual geometric segments from the camera to the first interface, through
its transmitted medium and along reflected/transmitted outgoing directions.
Apply interface Fresnel/Snell response and appropriate radiance transport
scaling. Include medium attenuation on light-source segments. Handle total
internal reflection, camera-inside-medium cases, opaque termination and medium
exit. Screen-space missing geometry needs an explicit fallback contract; it
cannot be silently treated as a physically traced hit. Work/depth limits reject
or visibly report incomplete transport instead of dropping optical contributions.

## Backend and integration constraints

Use the same transport contract for CPU reference, GPU implementation and
hardware ray capabilities. Negotiate texture/storage/format limits before
allocation; retain the existing GLSL portability checks and low-storage profile.
Expose bounded ray, segment, interface and grid memory/work budgets. Do not add a
second simulation owner or a parallel application runner. Snapshot revision,
per-view camera and prepared frame ownership remain coupled through submission.

## Acceptance gates before removing the editor rejection

1. Independent homogeneous and heterogeneous CPU references, including vacuum,
   absorption-only, overlapping constituents and ordered emitting/scattering slabs.
2. GPU parity for RGB transmittance/source radiance across thin/dense media,
   spatial boundaries, camera projection and SI scale; preserved alpha/HDR behavior.
3. Interface cases: normal/oblique transmission, refractive-index identity,
   entering/exiting water, total internal reflection and camera inside liquid.
4. A fog/water scene with independently checked camera and lighting segments;
   opacity before/inside/behind water, crossing volumes, multiple liquid species.
5. Source inventory conservation and rejected-frame rollback; resize/device loss,
   pending-submission lifetime and complete CPU/GPU memory admission.
6. Native editor presentation and visual acceptance on actual hardware. Metal
   proof does not qualify Vulkan, GL, DX12, CUDA or universal device support.

## Sources and applicability

[PBRT 4e, Transmittance](https://www.pbr-book.org/4ed/Volume_Scattering/Transmittance)
defines path transmittance and segment multiplication.
[PBRT 4e, Media](https://pbr-book.org/4ed/Volume_Scattering/Media) separates spatial
medium properties from integration.
[PBRT 4e, Dielectric BSDF](https://www.pbr-book.org/4ed/Reflection_Models/Dielectric_BSDF)
describes refractive interface response and total internal reflection.
These are physical references; their class hierarchy and Monte Carlo integrator
are not adopted wholesale. The ownership and backend decisions above are Voxy
proposals inferred from current source and the engine goal.
