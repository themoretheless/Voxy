#!/bin/sh
# Full-density native/GPU paired qualification; this does not measure rendered FPS.
set -eu
repo=/Users/themoretheless/Documents/ChatGPT/Voxy
cd "$repo"
out=${1:?Supply a fresh absolute capture directory}
case "$out" in /*) ;; *) printf '%s\n' 'Capture directory must be absolute' >&2; exit 2 ;; esac
mkdir -p "$out"
if [ -e "$out/qualification.log" ]; then
    printf '%s\n' 'Capture directory already contains a run; preserve its evidence' >&2
    exit 2
fi
unset VOXY_HAIR_CPU_CONTROL VOXY_HAIR_NATIVE_ONLY
exec env \
    VOXY_HAIR_CONTINUOUS_MESH_ADMISSION=1 \
    VOXY_HAIR_MESH_SWEEP_TRACE=1 \
    VOXY_HAIR_ARTICULATED_JUMP=1 \
    VOXY_HAIR_QUALIFICATION_FRAMES=720 \
    VOXY_HAIR_JOINT_QR=1 \
    VOXY_HAIR_JOINT_COORDINATE_BATCHES=1 \
    VOXY_HAIR_RESIDUAL_REFINEMENTS=1 \
    VOXY_HAIR_CONTACT_RESIDUAL_REFINEMENTS=1 \
    VOXY_HAIR_REUSE_REFINEMENT_FACTORS=1 \
    VOXY_HAIR_CONTACT_RESPONSE_BATCHES=1 \
    VOXY_HAIR_BATCH_RESPONSE_WAVES=1 \
    VOXY_HAIR_COMPACT_RESPONSE_READBACK=1 \
    VOXY_HAIR_GPU_RESPONSE_TRANSPORT=1 \
    VOXY_HAIR_JOINT_CONTACT_POSITIONS=1 \
    VOXY_HAIR_SWEPT_STRAND_POSITIONS=1 \
    VOXY_HAIR_RECOVER_FRICTION_PRESSURE=1 \
    VOXY_HAIR_JOINT_CONTACT_VELOCITIES=1 \
    VOXY_HAIR_SAMPLE_COLLIDER_MOTION=1 \
    VOXY_HAIR_QR_PROFILE=1 \
    VOXY_HAIR_SWEEP_REFINEMENT_TRACE=1 \
    VOXY_HAIR_QR_PROFILE_INPUT_EXPORT="$out/slow-contact-input.vqc" \
    VOXY_HAIR_QR_INPUT_EXPORT="$out/contact-input.vqc" \
    VOXY_HAIR_ACCELERATOR_REJECTION_EXPORT="$out/rejected-accelerator.vqc" \
    VOXY_HAIR_JOINT_FAILURE_COORDINATES_EXPORT="$out/rejected-coordinates.json" \
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
    /tmp/voxy-mesh-feature-normal-gpu-tests \
    gpu_linear_solver_tracks_full_model_jump_with_contacts \
    --ignored --nocapture > "$out/qualification.log" 2>&1
