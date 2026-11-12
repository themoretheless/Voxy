//! Five-component GPU route under the existing physics-owned final admission.
use super::*;
use std::collections::BTreeMap;

const QR_PACKET_CACHE_BYTES: usize = 8 * 1024 * 1024;
use voxy_render::{
    ExperimentalFiveGapInput as Gaps, ExperimentalFiveQrInput, ExperimentalFiveQrValue,
    ExperimentalFiveRelease, ExperimentalFiveReleaseInput as Release, ExperimentalFiveSelection,
    ExperimentalFiveSelectionInput as Select,
};
#[derive(Debug)]
struct CachedQrPacket {
    input: ExperimentalFiveQrInput,
    factors: Option<Vec<u8>>,
}

#[derive(Debug)]
pub(super) struct FivePrograms {
    qr: ComputeProgram,
    gaps: ComputeProgram,
    select: ComputeProgram,
    release: ComputeProgram,
    operator: Vec<Vec<[f64; 2]>>,
    qr_packets: BTreeMap<Vec<usize>, CachedQrPacket>,
    qr_packet_bytes: usize,
    qr_jobs: BTreeMap<usize, voxy_render::ComputeJob>,
    qr_job_bytes: usize,
    pub(super) qr_submissions: usize,
    pub(super) qr_dispatches: usize,
}
impl FivePrograms {
    pub(super) async fn new(device: &wgpu::Device) -> Result<Self, voxy_render::ComputeError> {
        Ok(Self {
            qr: ComputeProgram::new(device, &ExperimentalFiveQrInput::shader_source()).await?,
            gaps: ComputeProgram::new(device, &Gaps::shader_source()).await?,
            select: ComputeProgram::new(device, &Select::shader_source()).await?,
            release: ComputeProgram::new(device, &Release::shader_source()).await?,
            operator: Vec::new(),
            qr_packets: BTreeMap::new(),
            qr_packet_bytes: 0,
            qr_jobs: BTreeMap::new(),
            qr_job_bytes: 0,
            qr_submissions: 0,
            qr_dispatches: 0,
        })
    }
    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    pub(super) fn solve(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        request: &physics::hair::HairExpandedCoordinateRequest<'_>,
    ) -> Result<(Vec<physics::hair::HairResponseCoefficient>, Vec<f64>), Box<dyn std::error::Error>>
    {
        let profile = std::env::var_os("VOXY_HAIR_JOINT_STAGE_PROFILE").is_some();
        let solve_started = profile.then(std::time::Instant::now);
        let mut qr_time = std::time::Duration::ZERO;
        let mut release_time = std::time::Duration::ZERO;
        let mut gap_time = std::time::Duration::ZERO;
        let mut select_time = std::time::Duration::ZERO;
        if self.operator.len() != request.columns.len()
            || self
                .operator
                .iter()
                .zip(request.columns)
                .any(|(cached, incoming)| {
                    cached.len() != incoming.len()
                        || cached
                            .iter()
                            .zip(incoming)
                            .any(|(a, b)| a.map(f64::to_bits) != b.parts().map(f64::to_bits))
                })
        {
            self.operator = request
                .columns
                .iter()
                .map(|column| column.iter().map(|value| value.parts()).collect())
                .collect();
            self.qr_packets.clear();
            self.qr_packet_bytes = 0;
        }
        let columns = &self.operator;
        let mut qr_packet_reuses = 0;
        let mut qr_job_reuses = 0;
        let mut qr_factor_reuses = 0;
        let bounds = request.bounds;
        let tolerance = request.tolerance;
        let rows = columns.len();
        let width = columns.first().map_or(0, Vec::len);
        let mut active: Vec<usize> = Vec::new();
        let mut state = vec![zero(); width];
        let mut reactions = vec![zero(); rows];
        let mut gaps = Gaps::new_with_runtime_state(columns, &state, 1., bounds, tolerance)?;
        let gap_job = self.gaps.create_job(device, gaps.bytes())?;
        let mut need_qr = false;
        let mut completed = false;
        for iteration in 0..512usize.max(rows + 1) {
            if need_qr {
                let started = profile.then(std::time::Instant::now);
                let targets = active.iter().map(|&i| bounds[i]).collect::<Vec<_>>();
                let mut temporary;
                let packet = if let Some(packet) = self.qr_packets.get_mut(&active) {
                    packet.input.replace_bounds(&targets, tolerance)?;
                    qr_packet_reuses += 1;
                    packet
                } else {
                    let selected = active
                        .iter()
                        .map(|&i| columns[i].as_slice())
                        .collect::<Vec<_>>();
                    let input = ExperimentalFiveQrInput::new(&selected, &targets, tolerance)?;
                    let bytes = input.bytes().len() + input.factor_range().len();
                    let packet = CachedQrPacket {
                        input,
                        factors: None,
                    };
                    if bytes <= QR_PACKET_CACHE_BYTES {
                        if self.qr_packet_bytes + bytes > QR_PACKET_CACHE_BYTES {
                            self.qr_packets.clear();
                            self.qr_packet_bytes = 0;
                        }
                        self.qr_packet_bytes += bytes;
                        self.qr_packets.entry(active.clone()).or_insert(packet)
                    } else {
                        temporary = packet;
                        &mut temporary
                    }
                };
                let input = &packet.input;
                let factorized;
                let (bytes, steps) = if let Some(factors) = &packet.factors {
                    factorized = input.factorized_bytes(factors)?;
                    qr_factor_reuses += 1;
                    (factorized.as_slice(), 1)
                } else {
                    (input.bytes(), input.steps())
                };
                self.qr_submissions += 1;
                self.qr_dispatches += steps;
                let temporary_job;
                let job = if let Some(job) = self.qr_jobs.get(&bytes.len()) {
                    queue.write_buffer(job.buffer(), 0, bytes);
                    qr_job_reuses += 1;
                    job
                } else if bytes.len() <= QR_PACKET_CACHE_BYTES {
                    if self.qr_job_bytes + bytes.len() > QR_PACKET_CACHE_BYTES {
                        self.qr_jobs.clear();
                        self.qr_job_bytes = 0;
                    }
                    let job = self.qr.create_job(device, bytes)?;
                    self.qr_job_bytes += bytes.len();
                    self.qr_jobs.entry(bytes.len()).or_insert(job)
                } else {
                    temporary_job = self.qr.create_job(device, bytes)?;
                    &temporary_job
                };
                let collect_factors = packet.factors.is_none();
                let (mut candidate, factors) =
                    dispatch_job(device, queue, job, steps, [1, 1, 1], |bytes| {
                        let candidate = input.candidate(bytes)?;
                        let readback_factors = &bytes[input.factor_range()];
                        if packet
                            .factors
                            .as_deref()
                            .is_some_and(|expected| expected != readback_factors)
                        {
                            return Err(voxy_render::JointDotError::Output);
                        }
                        let factors = collect_factors.then(|| readback_factors.to_vec());
                        Ok::<_, voxy_render::JointDotError>((candidate, factors))
                    })??;
                if let Some(factors) = factors {
                    packet.factors = Some(factors);
                }
                scale(&mut candidate.coordinates, input.state_scale())?;
                scale(&mut candidate.reactions, input.reaction_scale())?;
                if let Some(started) = started {
                    qr_time += started.elapsed();
                }
                let started = profile.then(std::time::Instant::now);
                let current = active
                    .iter()
                    .map(|&i| reactions[i].clone())
                    .collect::<Vec<_>>();
                let step = Release::new_with_state(
                    &current,
                    &candidate.reactions,
                    &state,
                    &candidate.coordinates,
                )?;
                let release = dispatch(
                    device,
                    queue,
                    &self.release,
                    step.bytes(),
                    1,
                    [1, 1, 1],
                    |bytes| step.result(bytes),
                )??;
                if let Some(started) = started {
                    release_time += started.elapsed();
                }
                match release {
                    ExperimentalFiveRelease::Accept => {
                        state = candidate.coordinates;
                        reactions.fill(zero());
                        for (&id, value) in active.iter().zip(candidate.reactions) {
                            reactions[id] = value;
                        }
                    }
                    ExperimentalFiveRelease::Release {
                        row,
                        fraction: _,
                        reactions: updated,
                        coordinates,
                    } => {
                        let id = active[row];
                        state = coordinates;
                        for (&id, value) in active.iter().zip(updated) {
                            reactions[id] = value;
                        }
                        active.remove(row);
                        reactions[id] = zero();
                        need_qr = !active.is_empty();
                        continue;
                    }
                    ExperimentalFiveRelease::Uncertain => {
                        return Err(format!(
                            "uncertain release at iteration {iteration}, active {active:?}"
                        )
                        .into());
                    }
                }
            }
            let started = profile.then(std::time::Instant::now);
            gaps.replace_runtime_state(&state, 1.)?;
            for (offset, bytes) in gaps.runtime_uploads() {
                queue.write_buffer(gap_job.buffer(), offset, bytes);
            }
            let (residuals, decisions) =
                dispatch_job(device, queue, &gap_job, 1, gaps.workgroups(), |bytes| {
                    gaps.results(bytes)
                })??;
            let decisions = decisions.into_iter().map(|v| v as u32).collect::<Vec<_>>();
            if let Some(started) = started {
                gap_time += started.elapsed();
            }
            let started = profile.then(std::time::Instant::now);
            let mut flags = vec![false; rows];
            for &id in &active {
                flags[id] = true;
                if decisions[id] != 3 {
                    return Err(format!(
                        "active equality not certified at row {id}, iteration {iteration}"
                    )
                    .into());
                }
            }
            let input = Select::new(&residuals, &decisions, &flags)?;
            let selection = dispatch(
                device,
                queue,
                &self.select,
                input.bytes(),
                1,
                [1, 1, 1],
                |bytes| input.selection(bytes),
            )??;
            if let Some(started) = started {
                select_time += started.elapsed();
            }
            match selection {
                ExperimentalFiveSelection::Enter(id) => {
                    active.push(id);
                    active.sort_unstable();
                    need_qr = true;
                }
                ExperimentalFiveSelection::Feasible => {
                    completed = true;
                    break;
                }
                ExperimentalFiveSelection::Uncertain => {
                    return Err(format!("uncertain entering row at iteration {iteration}").into());
                }
            }
        }
        if !completed {
            return Err("unchanged active-set iteration budget exhausted".into());
        }
        if let Some(started) = solve_started {
            eprintln!(
                "FIVE SOLVE STAGES rows={rows} width={width} qr_packet_reuses={qr_packet_reuses} qr_job_reuses={qr_job_reuses} qr_factor_reuses={qr_factor_reuses} total_ms={:.3} qr_ms={:.3} release_ms={:.3} gaps_ms={:.3} select_ms={:.3}",
                started.elapsed().as_secs_f64() * 1000.,
                qr_time.as_secs_f64() * 1000.,
                release_time.as_secs_f64() * 1000.,
                gap_time.as_secs_f64() * 1000.,
                select_time.as_secs_f64() * 1000.,
            );
        }
        let coordinates = state.iter().map(native).collect::<Result<Vec<_>, _>>()?;
        let reactions = reactions
            .iter()
            .map(|v| {
                native(v).map(|v| {
                    let [h, l] = v.parts();
                    h + l
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok((coordinates, reactions))
    }
}
fn zero() -> ExperimentalFiveQrValue {
    ExperimentalFiveQrValue {
        words: [0; 5],
        exponent: 0,
        error_bound: 0.,
    }
}
fn scale(
    values: &mut [ExperimentalFiveQrValue],
    scale: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    if !scale.is_normal() || scale <= 0. || scale.to_bits() & 0x000f_ffff_ffff_ffff != 0 {
        return Err("invalid five-component binary scale".into());
    }
    let shift = ((scale.to_bits() >> 52) & 2047) as i32 - 1023;
    for v in values {
        v.exponent = v
            .exponent
            .checked_add(shift)
            .ok_or("five-component exponent overflow")?;
    }
    Ok(())
}
fn native(
    v: &ExperimentalFiveQrValue,
) -> Result<physics::hair::HairResponseCoefficient, &'static str> {
    if !(-1022..=1023).contains(&v.exponent) {
        return Err("five-component recovery exponent range");
    }
    physics::hair::HairResponseCoefficient::from_expansion_terms(
        v.words.map(|w| f64::from(f32::from_bits(w))),
        [1., 2f64.powi(v.exponent)],
    )
}
fn dispatch<T>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    program: &voxy_render::ComputeProgram,
    input: &[u8],
    steps: usize,
    groups: [u32; 3],
    decode: impl FnOnce(&[u8]) -> T,
) -> Result<T, Box<dyn std::error::Error>> {
    let job = program.create_job(device, input)?;
    dispatch_job(device, queue, &job, steps, groups, decode)
}

fn dispatch_job<T>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    job: &voxy_render::ComputeJob,
    steps: usize,
    groups: [u32; 3],
    decode: impl FnOnce(&[u8]) -> T,
) -> Result<T, Box<dyn std::error::Error>> {
    let mut encoder = device.create_command_encoder(&Default::default());
    job.encode_repeated_steps(&mut encoder, groups, u32::try_from(steps)?)?;
    let snapshot = job.encode_snapshot(&mut encoder)?;
    let submission = queue.submit([encoder.finish()]);
    let mut read = snapshot.begin_read();
    device.poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    })?;
    Ok(read
        .try_read_with(decode)?
        .ok_or("missing five-component GPU readback")?)
}
