#!/bin/sh
# Full-density native CPU qualification; this does not measure rendered FPS.
set -eu
repo=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
cd "$repo"
out=${1:?Supply a fresh absolute capture directory}
case "$out" in /*) ;; *) printf '%s\n' 'Capture directory must be absolute' >&2; exit 2 ;; esac
mkdir -p "$out"
if [ -e "$out/qualification.log" ]; then
    printf '%s\n' 'Capture directory already contains a run; preserve its evidence' >&2
    exit 2
fi
exec env \
    VOXY_HAIR_CONTINUOUS_MESH_ADMISSION=1 \
    VOXY_HAIR_MESH_SWEEP_TRACE=1 \
    VOXY_HAIR_ARTICULATED_JUMP=1 \
    VOXY_HAIR_QUALIFICATION_FRAMES="${VOXY_HAIR_QUALIFICATION_FRAMES:-720}" \
    VOXY_HAIR_CPU_CONTROL=1 \
    VOXY_HAIR_NATIVE_ONLY=1 \
    VOXY_HAIR_JOINT_CONTACT_POSITIONS=1 \
    VOXY_HAIR_SWEPT_STRAND_POSITIONS=1 \
    VOXY_HAIR_RECOVER_FRICTION_PRESSURE=1 \
    VOXY_HAIR_JOINT_CONTACT_VELOCITIES=1 \
    VOXY_HAIR_SAMPLE_COLLIDER_MOTION=1 \
    VOXY_HAIR_QR_PROFILE=1 \
    VOXY_HAIR_SWEEP_REFINEMENT_TRACE=1 \
    VOXY_HAIR_QR_PROFILE_INPUT_EXPORT="$out/slow-contact-input.vqc" \
    VOXY_HAIR_QR_INPUT_EXPORT="$out/contact-input.vqc" \
    VOXY_HAIR_PROJECTION_FAILURE_EXPORT="$out/projection.vqp" \
    VOXY_HAIR_CONTACT_REPLAY_EXPORT="$out/contact.vhr" \
    VOXY_HAIR_ADMISSION_REPLAY_EXPORT="$out/admission.vhr" \
    VOXY_HAIR_REJECTED_GUIDE_EXPORT="$out/rejected-guide.json" \
    VOXY_HAIR_REJECTED_CONTACT_DIRECTION_EXPORT="$out/direction.json" \
    VOXY_HAIR_NEWTON_FAILURE_EXPORT="$out/newton.vjn" \
    VOXY_HAIR_ROOT_MOTION_FAILURE_EXPORT="$out/root-motion.vjr" \
    VOXY_HAIR_MESH_SWEEP_REJECT_EXPORT="$out/mesh-motion.json" \
    VOXY_HAIR_FAILURE_TRACE_EXPORT="$out/failure-trace.txt" \
    CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/voxy-release-120fps-20261007}" \
    cargo test -p voxy_app --release --lib \
    gpu_linear_solver_tracks_full_model_jump_with_contacts \
    -- --ignored --nocapture > "$out/qualification.log" 2>&1
