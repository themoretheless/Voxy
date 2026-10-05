# Skeletal motion and soft tissue inspection

`preview.html` displays the previously completed 241-frame neutral walk/jump/settle
run from `../contact-initialization-2026-10-05/neutral-frames/`.
`imported.html` displays the previously completed 41-frame imported rig run from
`../contact-frame-diagnosis-2026-10-05/imported-frames/`.

Both previews have pause, frame selection, playback speed, per-frame displacement
receipts, and synchronized displacement plots. Offsets measure each free center
against the corresponding rigid bone transform. These are separate tissue
volumes, not deformation of the imported character's skin.

Regenerate with `python3 tools/export_motion_preview.py FRAME_DIRECTORY OUTPUT.html`.
Open the HTML locally, or serve the repository with a local HTTP server.

The fresh debug render in `frames/` was deliberately interrupted after observing
slow execution. It is partial and does not qualify a new full run. Its log is
`/tmp/voxy-neutral-animation-20261006.log`. No mechanics changes were made in this
preview update. Browser pause, frame selection, and half-speed controls were
checked; generated JavaScript passed `node --check`.
