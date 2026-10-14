//! Crowd skeletal pose evaluation on CUDA, sharing the packed word ABI and
//! exact `f32` operation order with `voxy_gpu/src/pose_eval.wgsl` and
//! `voxy_animation::gpu_layout::cpu_reference_palette`.

use crate::{CudaCompute, CudaError};

/// Header word count for a packed pose evaluation buffer.
pub const POSE_EVAL_HEADER_WORDS: usize = 8;
/// Maximum joints per skeleton supported by the GPU/CUDA pose evaluator.
pub const POSE_EVAL_MAX_JOINTS: usize = 256;
/// Maximum blend jobs per launch.
pub const POSE_EVAL_MAX_JOBS: usize = 65_536;
/// Maximum resampled clips in one table.
pub const POSE_EVAL_MAX_CLIPS: usize = 4_096;
/// Sentinel parent value for root joints.
pub const POSE_EVAL_NO_PARENT: u32 = u32::MAX;

/// Packs clip headers, resampled key floats (`12` floats = `3` vec4s per key),
/// blend jobs, skeleton parent indices, and column-major inverse bind matrices
/// into the shared word ABI.
///
/// # Errors
/// Rejects empty or oversized counts, parent cycles, nonfinite keys/weights/matrices,
/// or out-of-range clip/instance indices.
pub fn pack_pose_eval_job(
    joint_count: usize,
    instance_count: usize,
    clip_headers: &[[u32; 8]],
    key_floats: &[f32],
    jobs: &[[u32; 8]],
    parents: &[u32],
    inverse_binds: &[[f32; 16]],
) -> Result<Vec<u32>, CudaError> {
    if joint_count == 0
        || joint_count > POSE_EVAL_MAX_JOINTS
        || instance_count == 0
        || instance_count > POSE_EVAL_MAX_JOBS
        || clip_headers.is_empty()
        || clip_headers.len() > POSE_EVAL_MAX_CLIPS
        || jobs.is_empty()
        || jobs.len() > POSE_EVAL_MAX_JOBS
        || parents.len() != joint_count
        || inverse_binds.len() != joint_count
        || key_floats.is_empty()
        || !key_floats.len().is_multiple_of(12)
        || key_floats.iter().any(|v| !v.is_finite())
        || inverse_binds.iter().flatten().any(|v| !v.is_finite())
    {
        return Err(CudaError::InvalidPoseEvalInput);
    }

    let total_key_vec4s = key_floats.len() / 4;
    let palette_words = instance_count
        .checked_mul(joint_count)
        .and_then(|n| n.checked_mul(16))
        .ok_or(CudaError::InvalidPoseEvalInput)?;
    let total_words = POSE_EVAL_HEADER_WORDS
        .checked_add(clip_headers.len() * 8)
        .and_then(|n| n.checked_add(key_floats.len()))
        .and_then(|n| n.checked_add(jobs.len() * 8))
        .and_then(|n| n.checked_add(joint_count))
        .and_then(|n| n.checked_add(joint_count * 16))
        .and_then(|n| n.checked_add(palette_words))
        .ok_or(CudaError::InvalidPoseEvalInput)?;

    let mut words = Vec::with_capacity(total_words);
    words.push(jobs.len() as u32);
    words.push(joint_count as u32);
    words.push(clip_headers.len() as u32);
    words.push(total_key_vec4s as u32);
    words.push(instance_count as u32);
    words.extend([0u32; 3]);

    for header in clip_headers {
        words.extend_from_slice(header);
    }
    words.extend(key_floats.iter().map(|v| v.to_bits()));
    for job in jobs {
        words.extend_from_slice(job);
    }
    words.extend_from_slice(parents);
    for mat in inverse_binds {
        words.extend(mat.iter().map(|v| v.to_bits()));
    }
    words.extend(std::iter::repeat_n(0u32, palette_words));

    validate(&words, usize::MAX)?;
    Ok(words)
}

/// Unpacks the evaluated skin palettes (`instance_count * joint_count` matrices,
/// each 16 column-major `f32`s) from a completed pose evaluation word buffer.
///
/// # Errors
/// Rejects malformed buffers or nonfinite output matrices.
pub fn unpack_pose_eval_palettes(words: &[u32]) -> Result<Vec<[f32; 16]>, CudaError> {
    validate(words, usize::MAX)?;
    let job_count = words[0] as usize;
    let joint_count = words[1] as usize;
    let clip_count = words[2] as usize;
    let total_key_vec4s = words[3] as usize;
    let instance_count = words[4] as usize;

    let palettes_base = POSE_EVAL_HEADER_WORDS
        + clip_count * 8
        + total_key_vec4s * 4
        + job_count * 8
        + joint_count
        + joint_count * 16;
    let matrix_count = instance_count * joint_count;
    let mut out = Vec::with_capacity(matrix_count);
    for m in 0..matrix_count {
        let slice = &words[palettes_base + m * 16..palettes_base + (m + 1) * 16];
        let mat: [f32; 16] = std::array::from_fn(|i| f32::from_bits(slice[i]));
        if mat.iter().any(|v| !v.is_finite()) {
            return Err(CudaError::InvalidPoseEvalInput);
        }
        out.push(mat);
    }
    Ok(out)
}

/// Evaluates the packed pose buffer on the CPU using the exact shader/CUDA
/// floating-point arithmetic order.
///
/// # Errors
/// Rejects malformed buffers.
pub fn cpu_pose_eval_words(words: &[u32]) -> Result<Vec<u32>, CudaError> {
    validate(words, usize::MAX)?;
    let mut out = words.to_vec();
    let job_count = out[0] as usize;
    let joint_count = out[1] as usize;
    let clip_count = out[2] as usize;
    let total_key_vec4s = out[3] as usize;

    let headers_base = POSE_EVAL_HEADER_WORDS;
    let keys_base = headers_base + clip_count * 8;
    let jobs_base = keys_base + total_key_vec4s * 4;
    let parents_base = jobs_base + job_count * 8;
    let inv_binds_base = parents_base + joint_count;
    let palettes_base = inv_binds_base + joint_count * 16;

    for job_idx in 0..job_count {
        let job = &out[jobs_base + job_idx * 8..jobs_base + (job_idx + 1) * 8];
        let instance = job[0] as usize;
        let clip_a = job[1] as usize;
        let clip_b = job[2] as usize;
        let phase = f32::from_bits(job[3]);
        let weight = f32::from_bits(job[4]).clamp(0.0, 1.0);

        let header_a: [u32; 8] = out[headers_base + clip_a * 8..headers_base + (clip_a + 1) * 8]
            .try_into()
            .unwrap();
        let header_b: [u32; 8] = out[headers_base + clip_b * 8..headers_base + (clip_b + 1) * 8]
            .try_into()
            .unwrap();

        let mut globals = [[0.0f32; 16]; POSE_EVAL_MAX_JOINTS];
        for joint in 0..joint_count {
            let sa = sample_clip_words(&header_a, &out[keys_base..jobs_base], phase, joint);
            let sb = sample_clip_words(&header_b, &out[keys_base..jobs_base], phase, joint);
            let t = [
                sa.0[0] + (sb.0[0] - sa.0[0]) * weight,
                sa.0[1] + (sb.0[1] - sa.0[1]) * weight,
                sa.0[2] + (sb.0[2] - sa.0[2]) * weight,
            ];
            let r = quat_nlerp(sa.1, sb.1, weight);
            let s = [
                sa.2[0] + (sb.2[0] - sa.2[0]) * weight,
                sa.2[1] + (sb.2[1] - sa.2[1]) * weight,
                sa.2[2] + (sb.2[2] - sa.2[2]) * weight,
            ];
            let local = trs_matrix(t, r, s);
            let parent = out[parents_base + joint];
            let global = if parent == POSE_EVAL_NO_PARENT {
                local
            } else {
                mat4_mul(globals[parent as usize], local)
            };
            globals[joint] = global;

            let inv_slice = &out[inv_binds_base + joint * 16..inv_binds_base + (joint + 1) * 16];
            let inv_bind: [f32; 16] = std::array::from_fn(|k| f32::from_bits(inv_slice[k]));
            let skin = mat4_mul(global, inv_bind);
            let dst_offset = palettes_base + (instance * joint_count + joint) * 16;
            for k in 0..16 {
                out[dst_offset + k] = skin[k].to_bits();
            }
        }
    }
    Ok(out)
}

impl CudaCompute {
    /// Launches the CUDA crowd pose evaluation kernel (`pose_eval_crowd`) on a
    /// validated packed word buffer and returns the updated buffer.
    ///
    /// # Errors
    /// Rejects malformed inputs or over-budget buffers before touching the driver;
    /// returns `CudaError::Disabled` when built without the `cuda` feature.
    #[allow(unsafe_code)]
    pub fn pose_eval(&self, words: &[u32]) -> Result<Vec<u32>, CudaError> {
        validate(words, self.max_bytes)?;
        #[cfg(feature = "cuda")]
        {
            use cudarc::driver::{LaunchConfig, PushKernelArg};
            let allocation_bytes = words.len().checked_mul(4).ok_or(CudaError::BufferLimit)?;
            let reservation = self.allocation_budget.reserve(allocation_bytes)?;
            let function = {
                let mut cache = self
                    .pose_eval
                    .lock()
                    .map_err(|_| CudaError::KernelCachePoisoned)?;
                if cache.is_none() {
                    let ptx = cudarc::nvrtc::compile_ptx_with_opts(
                        include_str!("pose_eval.cu"),
                        self.compiler_options("pose_eval.cu")?,
                    )
                    .map_err(CudaError::Compile)?;
                    let module = self.context.load_module(ptx).map_err(CudaError::Driver)?;
                    *cache = Some(
                        module
                            .load_function("pose_eval_crowd")
                            .map_err(CudaError::Driver)?,
                    );
                }
                cache
                    .as_ref()
                    .ok_or(CudaError::KernelCachePoisoned)?
                    .clone()
            };
            let stream = self.context.default_stream();
            let job_count = words[0];
            let mut buffer = stream.clone_htod(words).map_err(CudaError::Driver)?;
            let result = (|| {
                let mut launch = stream.launch_builder(&function);
                launch.arg(&mut buffer);
                // SAFETY: `validate` verified all header counts, clip key offsets,
                // job indices, parent indices, and output palette offsets against
                // the exact word length of `buffer`.
                unsafe {
                    launch.launch(LaunchConfig {
                        grid_dim: (job_count.div_ceil(64), 1, 1),
                        block_dim: (64, 1, 1),
                        shared_mem_bytes: 0,
                    })
                }
                .map_err(CudaError::Driver)?;
                let output = stream.clone_dtoh(&buffer).map_err(CudaError::Driver)?;
                Ok::<_, CudaError>(output)
            })();
            reservation.release((buffer, function), &self.context)?;
            result
        }
        #[cfg(not(feature = "cuda"))]
        {
            let _ = words;
            Err(CudaError::Disabled)
        }
    }
}

fn validate(words: &[u32], max_bytes: usize) -> Result<(), CudaError> {
    if words.len() < POSE_EVAL_HEADER_WORDS {
        return Err(CudaError::InvalidPoseEvalInput);
    }
    let job_count = words[0] as usize;
    let joint_count = words[1] as usize;
    let clip_count = words[2] as usize;
    let total_key_vec4s = words[3] as usize;
    let instance_count = words[4] as usize;

    if job_count == 0
        || job_count > POSE_EVAL_MAX_JOBS
        || joint_count == 0
        || joint_count > POSE_EVAL_MAX_JOINTS
        || clip_count == 0
        || clip_count > POSE_EVAL_MAX_CLIPS
        || total_key_vec4s == 0
        || !total_key_vec4s.is_multiple_of(3)
        || instance_count == 0
        || instance_count > POSE_EVAL_MAX_JOBS
        || words[5] != 0
        || words[6] != 0
        || words[7] != 0
    {
        return Err(CudaError::InvalidPoseEvalInput);
    }

    let palette_words = instance_count
        .checked_mul(joint_count)
        .and_then(|n| n.checked_mul(16))
        .ok_or(CudaError::InvalidPoseEvalInput)?;
    let expected_words = POSE_EVAL_HEADER_WORDS
        .checked_add(clip_count * 8)
        .and_then(|n| n.checked_add(total_key_vec4s.checked_mul(4)?))
        .and_then(|n| n.checked_add(job_count * 8))
        .and_then(|n| n.checked_add(joint_count))
        .and_then(|n| n.checked_add(joint_count * 16))
        .and_then(|n| n.checked_add(palette_words))
        .ok_or(CudaError::InvalidPoseEvalInput)?;
    if words.len() != expected_words {
        return Err(CudaError::InvalidPoseEvalInput);
    }

    let headers_base = POSE_EVAL_HEADER_WORDS;
    let keys_base = headers_base + clip_count * 8;
    let jobs_base = keys_base + total_key_vec4s * 4;
    let parents_base = jobs_base + job_count * 8;
    let inv_binds_base = parents_base + joint_count;
    let palettes_base = inv_binds_base + joint_count * 16;

    // Validate clip headers.
    for c in 0..clip_count {
        let h = &words[headers_base + c * 8..headers_base + (c + 1) * 8];
        let key_offset = h[0] as usize;
        let sample_count = h[1] as usize;
        let clip_joints = h[2] as usize;
        let looping = h[3];
        let duration = f32::from_bits(h[4]);
        let needed_vec4s = sample_count
            .checked_mul(clip_joints)
            .and_then(|n| n.checked_mul(3))
            .ok_or(CudaError::InvalidPoseEvalInput)?;
        if sample_count < 2
            || clip_joints != joint_count
            || looping > 1
            || !duration.is_finite()
            || duration <= 0.0
            || key_offset
                .checked_add(needed_vec4s)
                .is_none_or(|end| end > total_key_vec4s)
        {
            return Err(CudaError::InvalidPoseEvalInput);
        }
    }

    // Validate keys are finite and quaternions are non-degenerate.
    for key_chunk in words[keys_base..jobs_base].chunks_exact(12) {
        if key_chunk.iter().any(|&w| !f32::from_bits(w).is_finite()) {
            return Err(CudaError::InvalidPoseEvalInput);
        }
        let qx = f32::from_bits(key_chunk[4]);
        let qy = f32::from_bits(key_chunk[5]);
        let qz = f32::from_bits(key_chunk[6]);
        let qw = f32::from_bits(key_chunk[7]);
        let len_sq = qx * qx + qy * qy + qz * qz + qw * qw;
        if !len_sq.is_finite() || len_sq < 1e-6 {
            return Err(CudaError::InvalidPoseEvalInput);
        }
    }

    // Validate jobs and unique instance targets.
    for j in 0..job_count {
        let job = &words[jobs_base + j * 8..jobs_base + (j + 1) * 8];
        let instance = job[0] as usize;
        let clip_a = job[1] as usize;
        let clip_b = job[2] as usize;
        let phase = f32::from_bits(job[3]);
        let weight = f32::from_bits(job[4]);
        if instance >= instance_count
            || clip_a >= clip_count
            || clip_b >= clip_count
            || !phase.is_finite()
            || !weight.is_finite()
        {
            return Err(CudaError::InvalidPoseEvalInput);
        }
    }

    // Validate parent-before-child ordering.
    for (joint, &parent) in words[parents_base..inv_binds_base].iter().enumerate() {
        if parent != POSE_EVAL_NO_PARENT && (parent as usize) >= joint {
            return Err(CudaError::InvalidPoseEvalInput);
        }
    }

    // Validate inverse binds are finite.
    if words[inv_binds_base..palettes_base]
        .iter()
        .any(|&w| !f32::from_bits(w).is_finite())
    {
        return Err(CudaError::InvalidPoseEvalInput);
    }

    if words
        .len()
        .checked_mul(4)
        .is_none_or(|bytes| bytes > max_bytes)
    {
        return Err(CudaError::BufferLimit);
    }
    Ok(())
}

fn quat_nlerp(a: [f32; 4], b_in: [f32; 4], t: f32) -> [f32; 4] {
    let dot = a[0] * b_in[0] + a[1] * b_in[1] + a[2] * b_in[2] + a[3] * b_in[3];
    let b = if dot < 0.0 {
        [-b_in[0], -b_in[1], -b_in[2], -b_in[3]]
    } else {
        b_in
    };
    let x = a[0] + (b[0] - a[0]) * t;
    let y = a[1] + (b[1] - a[1]) * t;
    let z = a[2] + (b[2] - a[2]) * t;
    let w = a[3] + (b[3] - a[3]) * t;
    let len = (x * x + y * y + z * z + w * w).sqrt();
    [x / len, y / len, z / len, w / len]
}

fn sample_clip_words(
    header: &[u32; 8],
    keys_words: &[u32],
    phase_in: f32,
    joint: usize,
) -> ([f32; 3], [f32; 4], [f32; 3]) {
    let key_offset = header[0] as usize;
    let sample_count = header[1];
    let joint_count = header[2] as usize;
    let phase = phase_in.clamp(0.0, 1.0);
    let scaled = phase * (sample_count - 1) as f32;
    let lower = (scaled.floor() as u32).min(sample_count - 1);
    let upper = (lower + 1).min(sample_count - 1);
    let t = scaled - lower as f32;

    let ia = (key_offset + (lower as usize * joint_count + joint) * 3) * 4;
    let ib = (key_offset + (upper as usize * joint_count + joint) * 3) * 4;

    let ta = [
        f32::from_bits(keys_words[ia]),
        f32::from_bits(keys_words[ia + 1]),
        f32::from_bits(keys_words[ia + 2]),
    ];
    let ra = [
        f32::from_bits(keys_words[ia + 4]),
        f32::from_bits(keys_words[ia + 5]),
        f32::from_bits(keys_words[ia + 6]),
        f32::from_bits(keys_words[ia + 7]),
    ];
    let sa = [
        f32::from_bits(keys_words[ia + 8]),
        f32::from_bits(keys_words[ia + 9]),
        f32::from_bits(keys_words[ia + 10]),
    ];

    let tb = [
        f32::from_bits(keys_words[ib]),
        f32::from_bits(keys_words[ib + 1]),
        f32::from_bits(keys_words[ib + 2]),
    ];
    let rb = [
        f32::from_bits(keys_words[ib + 4]),
        f32::from_bits(keys_words[ib + 5]),
        f32::from_bits(keys_words[ib + 6]),
        f32::from_bits(keys_words[ib + 7]),
    ];
    let sb = [
        f32::from_bits(keys_words[ib + 8]),
        f32::from_bits(keys_words[ib + 9]),
        f32::from_bits(keys_words[ib + 10]),
    ];

    (
        [
            ta[0] + (tb[0] - ta[0]) * t,
            ta[1] + (tb[1] - ta[1]) * t,
            ta[2] + (tb[2] - ta[2]) * t,
        ],
        quat_nlerp(ra, rb, t),
        [
            sa[0] + (sb[0] - sa[0]) * t,
            sa[1] + (sb[1] - sa[1]) * t,
            sa[2] + (sb[2] - sa[2]) * t,
        ],
    )
}

fn trs_matrix(translation: [f32; 3], rotation: [f32; 4], scale: [f32; 3]) -> [f32; 16] {
    let [qx, qy, qz, qw] = rotation;
    let x2 = qx + qx;
    let y2 = qy + qy;
    let z2 = qz + qz;
    let xx = qx * x2;
    let xy = qx * y2;
    let xz = qx * z2;
    let yy = qy * y2;
    let yz = qy * z2;
    let zz = qz * z2;
    let wx = qw * x2;
    let wy = qw * y2;
    let wz = qw * z2;
    [
        (1.0 - (yy + zz)) * scale[0],
        (xy + wz) * scale[0],
        (xz - wy) * scale[0],
        0.0,
        (xy - wz) * scale[1],
        (1.0 - (xx + zz)) * scale[1],
        (yz + wx) * scale[1],
        0.0,
        (xz + wy) * scale[2],
        (yz - wx) * scale[2],
        (1.0 - (xx + yy)) * scale[2],
        0.0,
        translation[0],
        translation[1],
        translation[2],
        1.0,
    ]
}

fn mat4_mul(a: [f32; 16], b: [f32; 16]) -> [f32; 16] {
    let mut out = [0.0f32; 16];
    for col in 0..4 {
        let bx = b[col * 4];
        let by = b[col * 4 + 1];
        let bz = b[col * 4 + 2];
        let bw = b[col * 4 + 3];
        for row in 0..4 {
            out[col * 4 + row] = a[row] * bx + a[4 + row] * by + a[8 + row] * bz + a[12 + row] * bw;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_validate_cpu_eval_and_unpack_two_joint_chain() {
        let joint_count = 2;
        let instance_count = 1;
        // 1 clip with 2 samples and 2 joints -> 4 keys -> 12 vec4s -> 48 floats
        let clip_headers = [[0, 2, 2, 1, 1.0f32.to_bits(), 0, 0, 0]];
        let key_floats = [
            // sample 0, joint 0 (root at y=1)
            0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0,
            // sample 0, joint 1 (child at y=-0.5)
            0.0, -0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0,
            // sample 1, joint 0 (root at y=2)
            0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0,
            // sample 1, joint 1 (child at y=-0.5)
            0.0, -0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0,
        ];
        let jobs = [[0, 0, 0, 0.5f32.to_bits(), 0.0f32.to_bits(), 0, 0, 0]];
        let parents = [POSE_EVAL_NO_PARENT, 0];
        let identity = [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        ];
        let inverse_binds = [identity, identity];

        let words = pack_pose_eval_job(
            joint_count,
            instance_count,
            &clip_headers,
            &key_floats,
            &jobs,
            &parents,
            &inverse_binds,
        )
        .unwrap();

        let evaluated = cpu_pose_eval_words(&words).unwrap();
        let palettes = unpack_pose_eval_palettes(&evaluated).unwrap();
        assert_eq!(palettes.len(), 2);
        // At phase 0.5, root y = 1.5, child global y = 1.5 - 0.5 = 1.0
        assert!((palettes[0][13] - 1.5).abs() < 1e-6);
        assert!((palettes[1][13] - 1.0).abs() < 1e-6);

        // Parent cycle rejection
        let mut bad_parents = words.clone();
        let parents_offset = POSE_EVAL_HEADER_WORDS + 8 + 48 + 8;
        bad_parents[parents_offset] = 0;
        assert!(matches!(
            validate(&bad_parents, usize::MAX),
            Err(CudaError::InvalidPoseEvalInput)
        ));
    }
}
