# Velocity precursor capture

Paired full-density articulated-jump run, original 720-frame accuracy gates, no native fallback permitted. This is numerical qualification and diagnosis, not a rendered FPS benchmark.

Original-island `.vqc` snapshots select rod 227 and tolerance >= 1e-9, up to 1024 observations per backend. This targets the normal-velocity projection (position rows normally use 1e-11); the tolerance filter alone is not a phase identity certificate. Native/external filenames count observations independently and must not be assumed to identify identical rows without checking input correspondence. Snapshots contain original systems, columns and shifted bounds for same-input replay; they do not contain the entire global trajectory or every free rod velocity.

The older bounded-readback capture failed at frame 6 after amplification during the frame-5 final velocity projection. This run uses packed multi-source readback and records the precursor inputs. Launch settings and source hashes are recorded in launch.json. Status must be read from the process and qualification.log, never inferred from this README.
