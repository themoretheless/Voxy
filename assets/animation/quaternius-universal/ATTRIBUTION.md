# Quaternius Universal Animation Library — Standard

Author: Quaternius. Licence: CC0 1.0 Universal; see LICENSE.
Publisher: https://quaternius.com/packs/universalanimationlibrary.html
The publisher identifies the library as CC0. Retrieved 2026-10-10 from the glTF mirror pinned in UPSTREAM.json. Original glTF and binary buffer are unmodified; each has a SHA-256 hash.

This pinned Standard snapshot contains 46 clips, not the full paid/source collection. CATALOG.json inventories the actual clip names and source skeleton nodes. Native Voxy import and sampled skinning are tested by `crates/voxy_render/tests/quaternius_animation_library.rs`.

Still required: explicit retarget corrections for the Voxy character, contact event authoring, playback policy per clip, gait transitions and full scene/physics/FPS qualification. Importing the library does not establish these behaviors.

Voxy's explicit gait-source preparation leaves these raw files unchanged. It
prepares Jog_Fwd_Loop by appending a one-frame (24 Hz) return to its first
pose, retaining every original key and all samples at their original absolute
times. See GAIT_PREPARATION.json. The prepared cycle is longer; normalized phases
must use the prepared duration. This establishes matching endpoint poses. Original STEP channels retain steps; this does not prove
matching velocities or physical stance. Other clip allocations are unchanged.
