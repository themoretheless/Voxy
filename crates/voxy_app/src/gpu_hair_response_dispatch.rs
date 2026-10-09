//! Ordered RHS waves share resident factors and one submission/readback wait.
use super::*;
use wgpu::util::DeviceExt;
impl GpuHairLinearSolver {
    pub(super) fn dispatch_response_waves(
        &mut self,
        inputs: &[BandedSolveInput],
        reuse: bool,
    ) -> Result<Vec<Vec<Vec<f64>>>, Box<dyn std::error::Error>> {
        let Some(first) = inputs.first() else {
            return Ok(Vec::new());
        };
        if self.gpu_response_transport {
            if !self.compact_response_readback {
                return Err("GPU response transport requires compact readback".into());
            }
            if self.response_transfer_program.is_none() {
                self.response_transfer_program = Some(pollster::block_on(
                    voxy_render::BandedTransferProgram::new(&self.device),
                )?);
            }
        }
        // Validate immutable matrix/layout equivalence before any queue mutation.
        let updates = inputs
            .iter()
            .map(|input| input.factored_rhs_updates(first))
            .collect::<Result<Vec<_>, _>>()?;
        if reuse {
            if self
                .resident
                .as_ref()
                .is_none_or(|job| job.buffer().size() != first.bytes().len() as u64)
            {
                return Err("missing resident response wave factors".into());
            }
        } else if self
            .resident
            .as_ref()
            .is_none_or(|job| job.buffer().size() != first.bytes().len() as u64)
        {
            self.resident = Some(self.program.create_job(&self.device, first.bytes())?);
        } else {
            self.queue
                .write_buffer(self.resident.as_ref().unwrap().buffer(), 0, first.bytes());
        }
        // Keep snapshots below the device pool's default 64 buffers / 64 MiB.
        // No load is dropped: larger batches continue in ordered chunks.
        let chunk_size = (32 * 1024 * 1024 / first.bytes().len().max(1)).clamp(1, 8);
        let mut output = Vec::with_capacity(inputs.len());
        for start in (0..inputs.len()).step_by(chunk_size) {
            let end = (start + chunk_size).min(inputs.len());
            let job = self.resident.as_ref().unwrap();
            let mut encoder = self.device.create_command_encoder(&Default::default());
            let mut snapshots = Vec::new();
            let mut uploads = Vec::new();
            for wave in start..end {
                let compact = if self.gpu_response_transport {
                    let mut payload = inputs[wave].bytes()[..16].to_vec();
                    for (_, bytes) in &updates[wave] {
                        payload.extend_from_slice(bytes);
                    }
                    let buffer =
                        self.device
                            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some("hair GPU RHS transport"),
                                contents: &payload,
                                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                            });
                    if reuse || wave != 0 {
                        self.response_transfer_program.as_ref().unwrap().encode(
                            &mut encoder,
                            job.buffer(),
                            &buffer,
                            &inputs[wave],
                            true,
                        )?;
                        self.response_transfer_dispatches += 1;
                    }
                    Some(buffer)
                } else {
                    None
                };
                if !self.gpu_response_transport && (reuse || wave != 0) {
                    let mut payload = Vec::new();
                    let mut copies = Vec::new();
                    for (target, bytes) in &updates[wave] {
                        copies.push((payload.len() as u64, *target, bytes.len() as u64));
                        payload.extend_from_slice(bytes);
                    }
                    let upload =
                        self.device
                            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some("hair response RHS wave"),
                                contents: &payload,
                                usage: wgpu::BufferUsages::COPY_SRC,
                            });
                    for (source, target, size) in copies {
                        encoder.copy_buffer_to_buffer(&upload, source, job.buffer(), target, size);
                    }
                    uploads.push(upload);
                }
                job.encode_step(&mut encoder, inputs[wave].dispatch())?;
                snapshots.push(if let Some(compact) = compact.as_ref() {
                    self.response_transfer_program.as_ref().unwrap().encode(
                        &mut encoder,
                        job.buffer(),
                        compact,
                        &inputs[wave],
                        false,
                    )?;
                    self.response_transfer_dispatches += 1;
                    voxy_render::ComputeDispatch::copy_buffer(
                        &self.device,
                        &mut encoder,
                        compact,
                        0,
                        inputs[wave].compact_output_size(),
                    )?
                } else if self.compact_response_readback {
                    voxy_render::ComputeDispatch::gather_buffer(
                        &self.device,
                        &mut encoder,
                        job.buffer(),
                        &inputs[wave].compact_output_ranges(),
                        inputs[wave].compact_output_size(),
                    )?
                } else {
                    job.encode_snapshot(&mut encoder)?
                });
                if let Some(compact) = compact {
                    uploads.push(compact);
                }
            }
            self.queue.submit([encoder.finish()]);
            self.response_submissions += 1;
            let mut reads = snapshots
                .into_iter()
                .map(|snapshot| snapshot.begin_read())
                .collect::<Vec<_>>();
            self.device.poll(wgpu::PollType::wait_indefinitely())?;
            for (input, read) in inputs[start..end].iter().zip(&mut reads) {
                let bytes = read
                    .try_read()?
                    .ok_or("hair response wave readback pending")?;
                output.push(if self.compact_response_readback {
                    input.decode_compact_checked(&bytes, 1e-8)?
                } else {
                    input.decode_checked(&bytes, 1e-8)?
                });
            }
            drop(uploads);
        }
        self.response_dispatches += inputs.len();
        Ok(output)
    }
}
