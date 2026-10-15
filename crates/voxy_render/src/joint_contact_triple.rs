//! Experimental lossless input packing for a future three-float GPU kernel.
//! This is an input codec; it neither dispatches nor admits physical solutions.
use super::JointDotError;
#[derive(Debug)]
pub struct ExperimentalTripleContactInput {
    pub rows: usize,
    pub width: usize,
    pub column_scale: f64,
    pub bound_scale: f64,
    pub columns: Vec<[u32; 3]>,
    pub bounds: Vec<[u32; 3]>,
    pub tolerance: [u32; 3],
}
fn binary_scale(value: f64) -> f64 {
    let bits = value.to_bits();
    let exponent = bits & 0x7ff0_0000_0000_0000;
    if exponent != 0 {
        f64::from_bits(exponent)
    } else {
        f64::from_bits(1u64 << bits.ilog2())
    }
}
fn pack_exact(value: f64) -> Result<[u32; 3], JointDotError> {
    if !value.is_finite() {
        return Err(JointDotError::NumericRange);
    }
    if value == 0. {
        return Ok([(value as f32).to_bits(); 3]);
    }
    let hi = value as f32;
    let lo = (value - f64::from(hi)) as f32;
    let tail = ((value - f64::from(hi)) - f64::from(lo)) as f32;
    if [hi, lo, tail]
        .iter()
        .any(|v| !v.is_finite() || v.is_subnormal())
    {
        return Err(JointDotError::NumericRange);
    }
    let decoded = (f64::from(hi) + f64::from(lo)) + f64::from(tail);
    if decoded.to_bits() != value.to_bits() {
        return Err(JointDotError::NumericRange);
    }
    Ok([hi.to_bits(), lo.to_bits(), tail.to_bits()])
}
impl ExperimentalTripleContactInput {
    /// Pack both native components before rounding to the shader's three-f32
    /// arithmetic. Never collapse the source to a single f64 first.
    pub fn new_expanded(columns: &[Vec<[f64; 2]>], bounds: &[f64], tolerance: f64) -> Result<Self, JointDotError> {
        let mut canonical=Vec::with_capacity(columns.len());
        for column in columns {
            let mut values=Vec::with_capacity(column.len());
            for &[hi,lo] in column {
                if !hi.is_finite() || !lo.is_finite() || hi+lo!=hi {return Err(JointDotError::NumericRange);}
                values.push(hi);
            }
            canonical.push(values);
        }
        let mut input=Self::new(&canonical,bounds,tolerance)?;
        for _ in 0..=32 {
            let mut packed=Vec::with_capacity(input.columns.len());
            for &[hi,lo] in columns.iter().flatten() {
                let mut high=hi/input.column_scale; let mut low=lo/input.column_scale;
                if !high.is_finite() || high.abs()>2f64.powi(48) || !low.is_finite()
                    || (lo!=0. && (low==0. || low*input.column_scale!=lo)) {break;}
                let mut words=[0;3]; let mut valid=true;
                for word in &mut words {
                    let component=high as f32;
                    if !component.is_finite() || component.is_subnormal() {valid=false;break;}
                    *word=component.to_bits();
                    let residual=high-f64::from(component); let next=residual+low;
                    let virtual_low=next-residual;
                    low=(residual-(next-virtual_low))+(low-virtual_low); high=next;
                }
                if !valid || !(high+low).is_finite() || (high+low).abs()>(hi/input.column_scale).abs()*2f64.powi(-68) {break;}
                packed.push(words);
            }
            if packed.len()==input.columns.len() {input.columns=packed;return Ok(input);}
            input.column_scale*=0.5;
            if input.column_scale==0. {break;}
        }
        Err(JointDotError::NumericRange)
    }
    pub fn new(
        columns: &[Vec<f64>],
        bounds: &[f64],
        tolerance: f64,
    ) -> Result<Self, JointDotError> {
        let rows = columns.len();
        let width = columns.first().map_or(0, Vec::len);
        if rows == 0
            || width == 0
            || bounds.len() != rows
            || !tolerance.is_finite()
            || tolerance <= 0.
            || columns
                .iter()
                .any(|c| c.len() != width || c.iter().any(|v| !v.is_finite()))
            || bounds.iter().any(|v| !v.is_finite())
        {
            return Err(JointDotError::Shape);
        }
        let count = rows
            .checked_mul(width)
            .filter(|&n| rows <= 1024 && n <= 16 * 1024 * 1024 / 3)
            .ok_or(JointDotError::Capacity)?;
        let maximum = columns.iter().flatten().map(|v| v.abs()).fold(0., f64::max);
        if maximum == 0. {
            return Err(JointDotError::NumericRange);
        }
        let mut column_scale = binary_scale(maximum);
        let bound_scale = binary_scale(bounds.iter().map(|v| v.abs()).fold(tolerance, f64::max));
        let scaled = |value: f64, scale: f64| {
            let normalized = value / scale;
            if !normalized.is_finite()
                || normalized * scale != value
                || (value != 0. && normalized == 0.)
            {
                return Err(JointDotError::NumericRange);
            }
            pack_exact(normalized)
        };
        let mut packed = Vec::new();
        packed
            .try_reserve_exact(count)
            .map_err(|_| JointDotError::Capacity)?;
        // Keep every coefficient exact and every uploaded nonzero component
        // normal on FTZ devices. At most 32 binary shifts keep the largest
        // normalized coefficient below 2^33; even the maximum allowed vector
        // length leaves its squared norm below the f32 overflow range.
        let mut complete = false;
        for shift in 0..=32 {
            packed.clear();
            for &value in columns.iter().flatten() {
                match scaled(value, column_scale) {
                    Ok(value) => packed.push(value),
                    Err(_) => break,
                }
            }
            if packed.len() == count {
                complete = true;
                break;
            }
            if shift == 32 || column_scale * 0.5 == 0. {
                break;
            }
            column_scale *= 0.5;
        }
        if !complete {
            return Err(JointDotError::NumericRange);
        }
        let bounds = bounds
            .iter()
            .map(|&v| scaled(v, bound_scale))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            rows,
            width,
            column_scale,
            bound_scale,
            columns: packed,
            bounds,
            tolerance: scaled(tolerance, bound_scale)?,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn decode(words: [u32; 3]) -> f64 {
        let [a, b, c] = words.map(|v| f32::from_bits(v) as f64);
        (a + b) + c
    }
    #[test]
    fn triple_expanded_input_keeps_sub_f64_ulp_source() {
        let low=2f64.powi(-60);
        let expanded=ExperimentalTripleContactInput::new_expanded(&[vec![[1.,low]]],&[1.],1e-14).unwrap();
        let legacy=ExperimentalTripleContactInput::new(&[vec![1.]],&[1.],1e-14).unwrap();
        assert_ne!(expanded.columns,legacy.columns);
        let words=expanded.columns[0].map(|v|f64::from(f32::from_bits(v)));
        assert_eq!(words[0],1.); assert_eq!(words[1]+words[2],low);
        assert!(ExperimentalTripleContactInput::new_expanded(&[vec![[1.,f64::NAN]]],&[1.],1e-14).is_err());
    }
    #[test]
    fn triple_input_preserves_original_binary_coefficients() {
        let columns = vec![
            vec![1., f64::from_bits(1f64.to_bits() + 1), -0.125],
            vec![-0., -1.2345678901234567, 1e-20],
        ];
        let bounds = [-1e-8, 3.141592653589793e-8];
        let input = ExperimentalTripleContactInput::new(&columns, &bounds, 1e-14).unwrap();
        for (&v, &packed) in columns.iter().flatten().zip(&input.columns) {
            assert_eq!((decode(packed) * input.column_scale).to_bits(), v.to_bits());
        }
        for (&v, &packed) in bounds.iter().zip(&input.bounds) {
            assert_eq!((decode(packed) * input.bound_scale).to_bits(), v.to_bits());
        }
        assert_eq!(
            (decode(input.tolerance) * input.bound_scale).to_bits(),
            1e-14f64.to_bits()
        );
    }
    #[test]
    fn triple_input_rescales_small_original_coefficients_without_subnormal_components() {
        let tiny = f64::from_bits(4203036788651473745);
        let columns = vec![vec![3979.2098911488238, tiny, -0.]];
        assert!(pack_exact(tiny / 2048.).is_err());
        let input = ExperimentalTripleContactInput::new(&columns, &[1.], 1e-14).unwrap();
        assert!(input.column_scale < 2048.);
        for (&value, &packed) in columns.iter().flatten().zip(&input.columns) {
            assert_eq!(
                (decode(packed) * input.column_scale).to_bits(),
                value.to_bits()
            );
            assert!(packed.iter().all(|v| !f32::from_bits(*v).is_subnormal()));
        }
        let ordinary =
            ExperimentalTripleContactInput::new(&[vec![3979.2098911488238, 1.]], &[1.], 1e-14)
                .unwrap();
        assert_eq!(ordinary.column_scale, 2048.);
        assert!(pack_exact(f64::from(f32::from_bits(1))).is_err());
    }
    #[test]
    fn triple_input_rejects_lost_small_terms_and_invalid_shapes() {
        assert!(matches!(
            ExperimentalTripleContactInput::new(&[vec![1., 1e-200]], &[1.], 1e-14),
            Err(JointDotError::NumericRange)
        ));
        assert!(ExperimentalTripleContactInput::new(&[vec![f64::NAN]], &[1.], 1e-14).is_err());
        assert!(ExperimentalTripleContactInput::new(&[vec![0.]], &[1.], 1e-14).is_err());
        for maximum in [f64::from_bits(1), 1e-300, 1e300, f64::MAX] {
            let input =
                ExperimentalTripleContactInput::new(&[vec![maximum]], &[maximum], maximum).unwrap();
            assert_eq!(
                (decode(input.columns[0]) * input.column_scale).to_bits(),
                maximum.to_bits()
            );
        }
    }
    #[test]
    #[ignore = "requires original VQC1 operator capture"]
    fn triple_input_preserves_captured_original_operator() {
        let bytes = std::fs::read(std::env::var_os("VOXY_HAIR_QR_INPUT_FIXTURE").unwrap()).unwrap();
        assert_eq!(&bytes[..4], b"VQC1");
        let integer = |i| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap()) as usize;
        let rows = integer(4);
        let width = integer(8);
        let scalar = |i| f64::from_le_bytes(bytes[i..i + 8].try_into().unwrap());
        let tolerance = scalar(20);
        let bounds = (0..rows).map(|i| scalar(28 + 8 * i)).collect::<Vec<_>>();
        let columns = (0..rows)
            .map(|i| {
                (0..width)
                    .map(|j| scalar(28 + 16 * rows + 8 * (i * width + j)))
                    .collect()
            })
            .collect::<Vec<Vec<_>>>();
        let input = ExperimentalTripleContactInput::new(&columns, &bounds, tolerance).unwrap();
        for (v, packed) in columns.iter().flatten().zip(&input.columns) {
            assert_eq!(
                (decode(*packed) * input.column_scale).to_bits(),
                v.to_bits()
            );
        }
        for (v, packed) in bounds.iter().zip(&input.bounds) {
            assert_eq!((decode(*packed) * input.bound_scale).to_bits(), v.to_bits());
        }
        assert_eq!(
            (decode(input.tolerance) * input.bound_scale).to_bits(),
            tolerance.to_bits()
        );
        eprintln!(
            "TRIPLE ORIGINAL INPUT rows={rows} width={width} bit_exact=true scope=input_only_no_GPU_arithmetic_or_physical_admission"
        );
    }
}

/// Experimental WGSL arithmetic source; no kernel, dispatch or physical admission.
pub const EXPERIMENTAL_TRIPLE_CONTACT_ARITHMETIC: &str = include_str!("joint_contact_triple.wgsl");
#[cfg(test)]
mod arithmetic_tests {
    use super::*;
    fn source() -> String {
        format!(
            "{}\n{}",
            EXPERIMENTAL_TRIPLE_CONTACT_ARITHMETIC,
            r"
@group(0) @binding(0) var<storage,read_write> words:array<u32>;
@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id:vec3<u32>) {
    if id.x>=words[0] {return;}
    let base=1u+id.x*18u;
    let a=vec3<f32>(bitcast<f32>(words[base]),bitcast<f32>(words[base+1u]),bitcast<f32>(words[base+2u]));
    let b=vec3<f32>(bitcast<f32>(words[base+3u]),bitcast<f32>(words[base+4u]),bitcast<f32>(words[base+5u]));
    let sum=triple_add(a,b);let product=triple_mul(a,b);let quotient=triple_div(a,b);var magnitude=a;if a.x<0.0 {magnitude=-a;}let root=triple_sqrt(magnitude);
    for(var i=0u;i<3u;i++) {words[base+6u+i]=bitcast<u32>(sum[i]);words[base+9u+i]=bitcast<u32>(product[i]);words[base+12u+i]=bitcast<u32>(quotient[i]);words[base+15u+i]=bitcast<u32>(root[i]);}
}
"
        )
    }
    #[test]
    fn triple_arithmetic_shader_validates() {
        let module = naga::front::wgsl::parse_str(&source()).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap();
    }
    #[test]
    #[ignore = "requires actual GPU; experimental triple arithmetic only"]
    fn gpu_triple_arithmetic_retains_cancellation_and_product_tail() {
        let instance = crate::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let program = pollster::block_on(crate::ComputeProgram::new(&device, &source())).unwrap();
        let mut cases = vec![
            (1. + 2f64.powi(-52), -1.),
            (1. + 2f64.powi(-52), 1. - 2f64.powi(-52)),
            (1e8 + 1e-8, -1e8),
            (1e-12, 1e-12),
            (0., 1.),
            (-1.25, 0.8),
        ];
        if let Some(path) = std::env::var_os("VOXY_HAIR_QR_INPUT_FIXTURE") {
            let bytes = std::fs::read(path).unwrap();
            assert_eq!(&bytes[..4], b"VQC1");
            let integer = |i| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap()) as usize;
            let rows = integer(4);
            let width = integer(8);
            let coefficient = |row, col| {
                let offset = 28 + 16 * rows + 8 * (row * width + col);
                f64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
            };
            for row in 0..rows {
                let nonzero = (0..width)
                    .map(|col| coefficient(row, col))
                    .filter(|v| *v != 0.)
                    .collect::<Vec<_>>();
                for pair in nonzero.windows(2) {
                    cases.push((pair[0], pair[1]));
                }
            }
        }
        let mut words = vec![cases.len() as u32];
        let scales = cases
            .iter()
            .map(|&(a, b)| binary_scale(f64::max(a.abs(), b.abs())))
            .collect::<Vec<_>>();
        for ((a, b), scale) in cases.iter().copied().zip(scales.iter().copied()) {
            words.extend(pack_exact(a / scale).unwrap());
            words.extend(pack_exact(b / scale).unwrap());
            words.extend([0; 12]);
        }
        let job = program
            .create_job(&device, bytemuck::cast_slice(&words))
            .unwrap();
        let mut encoder = device.create_command_encoder(&Default::default());
        job.encode_step(&mut encoder, [(cases.len() as u32).div_ceil(64), 1, 1])
            .unwrap();
        let snapshot = job.encode_snapshot(&mut encoder).unwrap();
        let submission = queue.submit([encoder.finish()]);
        let mut read = snapshot.begin_read();
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .unwrap();
        let bytes = read.try_read().unwrap().unwrap();
        let output = bytes
            .chunks_exact(4)
            .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
            .collect::<Vec<_>>();
        if let Some(path) = std::env::var_os("VOXY_TRIPLE_ARITHMETIC_EXPORT") {
            let mut audit = String::new();
            for (i, &(a, b)) in cases.iter().enumerate() {
                use std::fmt::Write;
                let base = 1 + 18 * i;
                writeln!(
                    &mut audit,
                    "{} {} {} {} {:?}",
                    i,
                    a.to_bits(),
                    b.to_bits(),
                    scales[i].to_bits(),
                    &output[base + 6..base + 12]
                )
                .unwrap();
            }
            std::fs::write(path, audit).unwrap();
        }
        for (i, (a, b)) in cases.iter().copied().enumerate() {
            let base = 1 + 18 * i;
            assert_eq!(&output[base..base + 6], &words[base..base + 6]);
            let decode = |offset| {
                let a = f32::from_bits(output[offset]) as f64;
                let b = f32::from_bits(output[offset + 1]) as f64;
                let c = f32::from_bits(output[offset + 2]) as f64;
                // Preserve the first addition residual before rounding the tail.
                let sum = a + b;
                let virtual_b = sum - a;
                let error = (a - (sum - virtual_b)) + (b - virtual_b);
                sum + (error + c)
            };
            assert_eq!(decode(base + 6) * scales[i], a + b, "sum case {i}");
            assert_eq!(
                (decode(base + 9) * scales[i]) * scales[i],
                a * b,
                "product case {i}"
            );
            assert_eq!(decode(base + 12), a / b, "quotient case {i}");
            assert_eq!(
                decode(base + 15),
                (a.abs() / scales[i]).sqrt(),
                "sqrt case {i}"
            );
        }
        let base = 1 + 18;
        let parts = output[base + 9..base + 12]
            .iter()
            .map(|&v| f32::from_bits(v) as f64)
            .collect::<Vec<_>>();
        assert_eq!(parts[0], 1.);
        assert_eq!(
            parts[1] + parts[2],
            -2f64.powi(-104),
            "product cancellation tail was lost"
        );
        eprintln!(
            "TRIPLE GPU ARITHMETIC cases={} power_of_two_input_scaling=true original_inputs_unchanged=true product_tail_2pow_minus104=true division_checked=true sqrt_checked=true scope=arithmetic_only_not_contact_solver_or_FPS",
            cases.len()
        );
    }
    #[test]
    #[ignore = "requires actual GPU and original VQC1 capture; vector norm qualification"]
    fn gpu_triple_original_column_norms() {
        let bytes = std::fs::read(std::env::var_os("VOXY_HAIR_QR_INPUT_FIXTURE").unwrap()).unwrap();
        assert_eq!(&bytes[..4], b"VQC1");
        let integer = |i| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap()) as usize;
        let rows = integer(4);
        let width = integer(8);
        let scalar = |i| f64::from_le_bytes(bytes[i..i + 8].try_into().unwrap());
        let columns = (0..rows)
            .map(|row| {
                (0..width)
                    .map(|col| scalar(28 + 16 * rows + 8 * (row * width + col)))
                    .collect()
            })
            .collect::<Vec<Vec<f64>>>();
        let bounds = (0..rows)
            .map(|row| scalar(28 + 8 * row))
            .collect::<Vec<_>>();
        let input = ExperimentalTripleContactInput::new(&columns, &bounds, scalar(20)).unwrap();
        let shader = format!(
            "{}\n{}",
            EXPERIMENTAL_TRIPLE_CONTACT_ARITHMETIC,
            r"
@group(0) @binding(0) var<storage,read_write> words:array<u32>;
fn load_triple(offset:u32)->vec3<f32> {
 return vec3<f32>(bitcast<f32>(words[offset]),bitcast<f32>(words[offset+1u]),bitcast<f32>(words[offset+2u]));
}
@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id:vec3<u32>) {
 let rows=words[0];let width=words[1];if id.x>=rows {return;}
 var squared=vec3<f32>(0.0);
 for(var col=0u;col<width;col++) {
  let v=load_triple(2u+3u*(id.x*width+col));
  squared=triple_add(squared,triple_mul(v,v));
 }
 let norm=triple_sqrt(squared);let offset=2u+3u*rows*width+3u*id.x;
 for(var part=0u;part<3u;part++) {words[offset+part]=bitcast<u32>(norm[part]);}
}
"
        );
        let instance = crate::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let program = pollster::block_on(crate::ComputeProgram::new(&device, &shader)).unwrap();
        let mut words = vec![rows as u32, width as u32];
        words.extend(input.columns.iter().flatten().copied());
        let output_start = words.len();
        words.resize(output_start + rows * 3, 0);
        let job = program
            .create_job(&device, bytemuck::cast_slice(&words))
            .unwrap();
        let mut encoder = device.create_command_encoder(&Default::default());
        job.encode_step(&mut encoder, [(rows as u32).div_ceil(64), 1, 1])
            .unwrap();
        let snapshot = job.encode_snapshot(&mut encoder).unwrap();
        let submission = queue.submit([encoder.finish()]);
        let mut read = snapshot.begin_read();
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .unwrap();
        let bytes = read.try_read().unwrap().unwrap();
        let output = bytes
            .chunks_exact(4)
            .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
            .collect::<Vec<_>>();
        assert_eq!(&output[..output_start], &words[..output_start]);
        let mut worst = 0f64;
        for (row, column) in columns.iter().enumerate() {
            let mut sum = 0.;
            let mut error = 0.;
            for &v in column {
                let v = v / input.column_scale;
                let term = v * v - error;
                let next = sum + term;
                error = (next - sum) - term;
                sum = next;
            }
            let reference = sum.sqrt();
            let offset = output_start + 3 * row;
            let a = f32::from_bits(output[offset]) as f64;
            let b = f32::from_bits(output[offset + 1]) as f64;
            let c = f32::from_bits(output[offset + 2]) as f64;
            let hi = a + b;
            let virtual_b = hi - a;
            let low = (a - (hi - virtual_b)) + (b - virtual_b);
            let norm = hi + (low + c);
            assert!(norm.is_finite() && norm > 0.);
            let relative = (norm - reference).abs() / reference;
            worst = worst.max(relative);
            // Reference products and final square root are rounded f64 operations.
            assert!(
                relative <= 4. * f64::EPSILON,
                "row {row}: norm={norm} reference={reference} relative={relative}"
            );
        }
        eprintln!(
            "TRIPLE COLUMN NORMS rows={rows} width={width} worst_relative_to_compensated_f64={worst} input_unchanged=true scope=norm_only_not_QR_or_physical_admission"
        );
    }
}

#[cfg(test)]
#[path = "joint_contact_triple_qr_tests.rs"]
mod qr_tests;

#[path = "joint_contact_triple_active_codec.rs"]
mod active_codec;
pub use active_codec::{
    ExperimentalTripleActiveContactBatch, ExperimentalTripleActiveContactInput,
    ExperimentalTripleActiveFactors, ExperimentalTripleCompletedFactorReadback,
    ExperimentalTripleExpandedCandidate, ExperimentalTripleExpansion,
};
