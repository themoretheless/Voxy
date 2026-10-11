# Corrected full jump with cooperative GPU coordinate batches

Three exact-contact alias corrections, all 469 guides, original 720-frame accuracy gates and zero native fallback required. The runtime flag is VOXY_HAIR_JOINT_COORDINATE_BATCHES=1. BATCH_COMPARE is a separate captured-operator test option, not the full-trajectory runtime flag.

The earlier exact-contact-aliases-full-capture was intentionally interrupted after its log confirmed batching disabled. Its artifacts remain preserved; the interruption is not physical rejection. The older pre-fix velocity-input-capture continues independently. This run is numerical trajectory qualification, not a rendered FPS benchmark. Check live process and qualification.log for actual status.
