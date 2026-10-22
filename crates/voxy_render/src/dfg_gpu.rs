//! Direct bounded GPU preintegration using the existing device-wide compute
//! storage and readback pools. No queue submission or blocking wait is hidden.
use super::{GgxEnergyPatchData, GgxEnergyRefinementProbe};
use voxy_gpu_core::{
    ComputeDispatch, ComputeError, ComputeJob, ComputeProgram, PendingComputeReadback,
};
pub const GGX_ENERGY_GPU_BATCH_LIMIT: usize = 32;
const SAMPLES: u32 = 524288;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GgxEnergyBatchKey {
    pub generation: u64,
    pub refinement: u32,
    pub offset: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GgxEnergyBatchStage {
    IntegrateNodes,
    InterpolateProbes,
}
#[derive(Debug)]
pub struct GgxEnergyGpuPrecompute {
    device: wgpu::Device,
    nodes: ComputeProgram,
    probes: ComputeProgram,
}
#[derive(Debug)]
pub struct EncodedGgxEnergyBatch {
    input: BatchInput,
    device: wgpu::Device,
    job: ComputeJob,
    dispatch: ComputeDispatch,
}
#[derive(Debug)]
pub struct PendingGgxEnergyBatch {
    input: BatchInput,
    device: wgpu::Device,
    job: Option<ComputeJob>,
    pending: PendingComputeReadback,
}
#[derive(Debug)]
pub struct GgxEnergyBatchOutput {
    pub key: GgxEnergyBatchKey,
    pub stage: GgxEnergyBatchStage,
    pub coordinates: Vec<[f32; 2]>,
    pub moments: Vec<[f32; 4]>,
}
#[derive(Debug)]
struct BatchInput {
    key: GgxEnergyBatchKey,
    stage: GgxEnergyBatchStage,
    coordinates: Vec<[f32; 2]>,
    words: Vec<u32>,
    stride: usize,
    result: usize,
}
fn invalid(message: &str) -> ComputeError {
    ComputeError::Validation(message.into())
}
fn coordinate_ok([nv, r]: [f32; 2]) -> bool {
    nv.is_finite()
        && r.is_finite()
        && (0. ..=1.).contains(&nv)
        && (0. ..=1.).contains(&r)
        && (r == 0. || (r * r) * (r * r) > 0.)
}
fn batch_count(count: usize) -> Result<(), ComputeError> {
    if count == 0 {
        Err(ComputeError::InvalidBuffer)
    } else if count > GGX_ENERGY_GPU_BATCH_LIMIT {
        Err(ComputeError::WorkBudget)
    } else {
        Ok(())
    }
}
impl BatchInput {
    fn nodes(key: GgxEnergyBatchKey, coordinates: &[[f32; 2]]) -> Result<Self, ComputeError> {
        batch_count(coordinates.len())?;
        if coordinates.iter().any(|&p| !coordinate_ok(p)) {
            return Err(invalid("unsupported GGX coordinate or precision"));
        }
        let mut words = vec![0; 1 + 8 * coordinates.len()];
        words[0] = coordinates.len() as u32;
        for (i, p) in coordinates.iter().enumerate() {
            let b = 1 + 8 * i;
            words[b] = p[0].to_bits();
            words[b + 1] = p[1].to_bits();
            words[b + 2] = SAMPLES;
        }
        Ok(Self {
            key,
            stage: GgxEnergyBatchStage::IntegrateNodes,
            coordinates: coordinates.to_vec(),
            words,
            stride: 8,
            result: 3,
        })
    }
    fn probes(
        key: GgxEnergyBatchKey,
        probes: &[GgxEnergyRefinementProbe],
        moments: &[[f32; 4]],
    ) -> Result<Self, ComputeError> {
        batch_count(probes.len())?;
        let mut words = vec![0; 1 + 162 * probes.len()];
        words[0] = probes.len() as u32;
        for (i, p) in probes.iter().enumerate() {
            if !coordinate_ok(p.uv) {
                return Err(invalid("unsupported GGX probe coordinate"));
            }
            let mut values = [[0.; 4]; 36];
            for (j, &index) in p.node_indices.iter().enumerate() {
                values[j] = *moments.get(index).ok_or(ComputeError::InvalidBuffer)?;
            }
            GgxEnergyPatchData::try_new(p.bounds, p.view_nodes, p.roughness_nodes, values)
                .map_err(|e| invalid(&e.to_string()))?;
            let b = 1 + 162 * i;
            words[b] = p.uv[0].to_bits();
            words[b + 1] = p.uv[1].to_bits();
            for j in 0..6 {
                words[b + 2 + j] = p.view_nodes[j].to_bits();
                words[b + 8 + j] = p.roughness_nodes[j].to_bits();
            }
            for j in 0..36 {
                for c in 0..4 {
                    words[b + 14 + 4 * j + c] = values[j][c].to_bits();
                }
            }
        }
        Ok(Self {
            key,
            stage: GgxEnergyBatchStage::InterpolateProbes,
            coordinates: probes.iter().map(|p| p.uv).collect(),
            words,
            stride: 162,
            result: 158,
        })
    }
    fn accept(&self, bytes: &[u8]) -> Result<GgxEnergyBatchOutput, ComputeError> {
        if bytes.len() != self.words.len() * 4 {
            return Err(ComputeError::InvalidBuffer);
        }
        let words = bytes
            .chunks_exact(4)
            .map(|b| u32::from_le_bytes(b.try_into().expect("four bytes")))
            .collect::<Vec<_>>();
        if words[0] != self.words[0] {
            return Err(invalid("GGX readback count mismatch"));
        }
        let mut moments = Vec::with_capacity(self.coordinates.len());
        for i in 0..self.coordinates.len() {
            let b = 1 + i * self.stride;
            let result = b + self.result;
            if words[b..result] != self.words[b..result]
                || (self.stride == 8 && words[b + 7] != self.words[b + 7])
            {
                return Err(invalid("GGX immutable input changed"));
            }
            if words[result + 3] != 1 {
                return Err(invalid("GGX producer or interpolation rejected a result"));
            }
            let m = [
                f32::from_bits(words[result]),
                f32::from_bits(words[result + 1]),
                f32::from_bits(words[result + 2]),
                1.,
            ];
            if !m[..3]
                .iter()
                .all(|v| v.is_finite() && (0. ..=1.).contains(v))
                || (m[0] + m[1] + m[2] - 1.).abs() > 5e-6
            {
                return Err(invalid("GGX invalid energy moments"));
            }
            moments.push(m);
        }
        Ok(GgxEnergyBatchOutput {
            key: self.key,
            stage: self.stage,
            coordinates: self.coordinates.clone(),
            moments,
        })
    }
}
impl GgxEnergyGpuPrecompute {
    pub(super) fn belongs_to(&self, device: &wgpu::Device) -> bool {
        &self.device == device
    }

    /// Compile once on the retained device. Existing global storage/readback
    /// budgets apply to both pipelines and all their outstanding batches.
    pub async fn new(device: &wgpu::Device) -> Result<Self, ComputeError> {
        let nodes = ComputeProgram::new(
            device,
            &format!(
                "{}\n{}",
                include_str!("material_energy_integrate.wgsl"),
                r"
@group(0) @binding(0) var<storage,read_write> data:array<u32>;
@compute @workgroup_size(32) fn cs_main(@builtin(global_invocation_id) id:vec3<u32>){
 if id.x>=data[0]{return;}let b=1u+8u*id.x;
 let m=integrate_ggx_energy(bitcast<f32>(data[b]),bitcast<f32>(data[b+1u]),data[b+2u]);
 for(var c=0u;c<3u;c++){data[b+3u+c]=bitcast<u32>(m.value[c]);}data[b+6u]=m.valid;
}
"
            ),
        )
        .await?;
        let probes=ComputeProgram::new(device,&format!("{}\n{}",include_str!("material_energy_patch.wgsl"),r"
@group(0) @binding(0) var<storage,read_write> data:array<u32>;
@compute @workgroup_size(8) fn cs_main(@builtin(global_invocation_id) id:vec3<u32>){
 if id.x>=data[0]{return;}let b=1u+162u*id.x;var region:GgxEnergyPatch;
 for(var i=0u;i<6u;i++){region.view_nodes[i]=bitcast<f32>(data[b+2u+i]);region.roughness_nodes[i]=bitcast<f32>(data[b+8u+i]);}
 for(var i=0u;i<36u;i++){for(var c=0u;c<4u;c++){region.moments[i][c]=bitcast<f32>(data[b+14u+4u*i+c]);}}
 let m=interpolate_ggx_energy_patch(region,vec2<f32>(bitcast<f32>(data[b]),bitcast<f32>(data[b+1u])));
 for(var c=0u;c<3u;c++){data[b+158u+c]=bitcast<u32>(m[c]);}data[b+161u]=select(0u,1u,m.w==1.0);
}
")).await?;
        Ok(Self {
            device: device.clone(),
            nodes,
            probes,
        })
    }
    fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        input: BatchInput,
    ) -> Result<EncodedGgxEnergyBatch, ComputeError> {
        let (program, width) = match input.stage {
            GgxEnergyBatchStage::IntegrateNodes => (&self.nodes, 32),
            GgxEnergyBatchStage::InterpolateProbes => (&self.probes, 8),
        };
        let bytes = input
            .words
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>();
        let job = program.create_job(&self.device, &bytes)?;
        job.encode_step(
            encoder,
            [input.coordinates.len().div_ceil(width) as u32, 1, 1],
        )?;
        let dispatch = job.encode_snapshot(encoder)?;
        Ok(EncodedGgxEnergyBatch {
            input,
            device: self.device.clone(),
            job,
            dispatch,
        })
    }
    /// Caller submits this encoder on the retained device queue before submitted().
    /// On any encode error discard the encoder; no current atlas is replaced.
    pub fn encode_nodes(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        key: GgxEnergyBatchKey,
        coordinates: &[[f32; 2]],
    ) -> Result<EncodedGgxEnergyBatch, ComputeError> {
        self.encode(encoder, BatchInput::nodes(key, coordinates)?)
    }
    pub fn encode_probes(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        key: GgxEnergyBatchKey,
        probes: &[GgxEnergyRefinementProbe],
        moments: &[[f32; 4]],
    ) -> Result<EncodedGgxEnergyBatch, ComputeError> {
        self.encode(encoder, BatchInput::probes(key, probes, moments)?)
    }
}
impl EncodedGgxEnergyBatch {
    /// Start mapping only after successful queue submission. Ownership preserves
    /// the job and pooled snapshot until completion or cancellation.
    #[must_use]
    pub fn submitted(self) -> PendingGgxEnergyBatch {
        PendingGgxEnergyBatch {
            input: self.input,
            device: self.device,
            job: Some(self.job),
            pending: self.dispatch.begin_read(),
        }
    }
}
impl PendingGgxEnergyBatch {
    /// Drive the original device with PollType::Poll or yield to the browser loop.
    /// Returns None while pending, rejects stale generations after draining, and
    /// consumes every result exactly once. Never waits or writes an atlas.
    pub fn try_read(
        &mut self,
        device: &wgpu::Device,
        current: GgxEnergyBatchKey,
    ) -> Result<Option<GgxEnergyBatchOutput>, ComputeError> {
        if device != &self.device {
            return Err(ComputeError::DeviceMismatch);
        }
        let bytes = match self.pending.try_read() {
            Ok(None) => return Ok(None),
            Ok(Some(bytes)) => bytes,
            Err(error) => {
                self.job.take();
                return Err(error);
            }
        };
        self.job.take();
        if current != self.input.key {
            return Err(invalid("stale GGX energy batch"));
        }
        self.input.accept(&bytes).map(Some)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn key() -> GgxEnergyBatchKey {
        GgxEnergyBatchKey {
            generation: 1,
            refinement: 1,
            offset: 0,
        }
    }
    fn complete(input: &BatchInput) -> Vec<u8> {
        let mut w = input.words.clone();
        for i in 0..input.coordinates.len() {
            let b = 1 + i * input.stride + input.result;
            w[b] = 0.5f32.to_bits();
            w[b + 1] = 0.25f32.to_bits();
            w[b + 2] = 0.25f32.to_bits();
            w[b + 3] = 1;
        }
        w.iter().flat_map(|v| v.to_le_bytes()).collect()
    }
    #[test]
    fn rejects_invalid_and_over_budget_inputs() {
        assert!(BatchInput::nodes(key(), &[]).is_err());
        assert_eq!(
            BatchInput::nodes(key(), &[[0.5, 0.5]; 33]).unwrap_err(),
            ComputeError::WorkBudget
        );
        assert!(BatchInput::nodes(key(), &[[0.5, 1e-20]]).is_err());
        assert!(BatchInput::nodes(key(), &[[f32::NAN, 0.5]]).is_err());
    }
    #[test]
    fn immutable_inputs_and_validity_are_required() {
        let input = BatchInput::nodes(key(), &[[0.5, 0.5]]).unwrap();
        let bytes = complete(&input);
        assert_eq!(
            input.accept(&bytes).unwrap().moments,
            vec![[0.5, 0.25, 0.25, 1.]]
        );
        let mut bad = bytes.clone();
        bad[4] ^= 1;
        assert!(input.accept(&bad).is_err());
        let mut bad = bytes;
        bad[28..32].copy_from_slice(&0u32.to_le_bytes());
        assert!(input.accept(&bad).is_err());
    }
    #[test]
    fn probe_indices_and_moment_quality_reject_before_encoding() {
        let regions =
            super::super::GgxEnergyRefinementPlan::regions_from_seeds(&[[0.5, 0.5]]).unwrap();
        let plan = super::super::GgxEnergyRefinementPlan::try_new(&regions, 1).unwrap();
        assert!(BatchInput::probes(key(), plan.probes(), &[]).is_err());
        let mut moments = vec![[0.5, 0.25, 0.25, 1.]; plan.nodes().len()];
        moments[0][3] = 0.;
        assert!(BatchInput::probes(key(), plan.probes(), &moments).is_err());
    }
    #[test]
    #[ignore = "requires physical GPU; nonblocking DFG batch lifecycle"]
    fn native_gpu_batch_uses_current_core_and_rejects_stale_results() {
        let instance = crate::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let gpu = pollster::block_on(GgxEnergyGpuPrecompute::new(&device)).unwrap();
        let request = key();
        let coordinates = [[0., 0.], [1., 0.], [1., 1.], [1., 0.0001]];
        let mut encoder = device.create_command_encoder(&Default::default());
        let encoded = gpu
            .encode_nodes(&mut encoder, request, &coordinates)
            .unwrap();
        queue.submit([encoder.finish()]);
        let mut pending = encoded.submitted();
        let (other, _) = wgpu::Device::noop(&Default::default());
        assert_eq!(
            pending.try_read(&other, request).unwrap_err(),
            ComputeError::DeviceMismatch
        );
        let started = std::time::Instant::now();
        let output = loop {
            device.poll(wgpu::PollType::Poll).unwrap();
            if let Some(output) = pending.try_read(&device, request).unwrap() {
                break output;
            }
            assert!(
                started.elapsed().as_secs() < 30,
                "nonblocking callback did not complete"
            );
            std::thread::sleep(std::time::Duration::from_millis(1));
        };
        assert_eq!(output.coordinates, coordinates);
        assert_eq!(output.moments[0], [0., 1., 0., 1.]);
        assert_eq!(output.moments[1], [1., 0., 0., 1.]);
        assert!((output.moments[2][2] - std::f32::consts::LN_2).abs() < 1e-6);
        assert!(output.moments[3][1] > 0. && output.moments[3][1] < 1e-18);
        assert_eq!(
            pending.try_read(&device, request).unwrap_err(),
            ComputeError::Consumed
        );
        let mut encoder = device.create_command_encoder(&Default::default());
        let encoded = gpu
            .encode_nodes(&mut encoder, request, &[[1., 0.]])
            .unwrap();
        queue.submit([encoder.finish()]);
        let mut pending = encoded.submitted();
        let changed = GgxEnergyBatchKey {
            generation: request.generation + 1,
            ..request
        };
        let started = std::time::Instant::now();
        loop {
            device.poll(wgpu::PollType::Poll).unwrap();
            match pending.try_read(&device, changed) {
                Ok(None) => {
                    assert!(started.elapsed().as_secs() < 30);
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                Err(ComputeError::Validation(message)) => {
                    assert!(message.contains("stale"));
                    break;
                }
                other => panic!("unexpected stale result {other:?}"),
            }
        }
    }
}
