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
        if self.gpu_response_transport && !self.compact_response_readback {
            return Err("GPU response transport requires compact readback".into());
        }
        let limits=self.device.limits();
        // Ordered GPU copies preserve RHS/status bits on devices that cannot
        // bind both transport buffers or dispatch the gather shader.
        let shader_transport=self.gpu_response_transport
            && limits.max_storage_buffers_per_shader_stage>=2
            && limits.max_compute_workgroup_size_x>=64
            && limits.max_compute_invocations_per_workgroup>=64
            && inputs.iter().all(|input| {
                let size=input.compact_output_size();
                input.bytes().len() as u64<=limits.max_storage_buffer_binding_size
                    && size<=limits.max_storage_buffer_binding_size
                    && (size/4).div_ceil(64)<=limits.max_compute_workgroups_per_dimension as u64
            });
        if shader_transport {
            if self.rhs_transfer_program.is_none() {
                self.rhs_transfer_program = Some(pollster::block_on(
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
        // Use actual staging sizes and the shared pool's current capacity.
        // An admission error after earlier copies were encoded would abandon
        // those copies in quarantine, consuming budget on every retry.
        let pool = voxy_render::ComputeReadbackPool::for_device(&self.device);
        let mut output = Vec::with_capacity(inputs.len());
        let mut start = 0;
        while start < inputs.len() {
            let capacity = pool.available_capacity();
            let mut bytes = 0u64;
            let mut end = start;
            while end < inputs.len() && end - start < capacity.max_buffers.min(8) {
                let size = if self.compact_response_readback {
                    inputs[end].compact_output_size()
                } else {
                    inputs[end].bytes().len() as u64
                };
                if size > capacity.max_bytes - bytes { break; }
                bytes += size;
                end += 1;
            }
            if end == start {
                return Err(format!("hair response staging capacity exhausted before encoding: {capacity:?}").into());
            }
            let job = self.resident.as_ref().unwrap();
            let mut encoder = self.device.create_command_encoder(&Default::default());
            let mut snapshots = Vec::new();
            let mut uploads = Vec::new();
            for wave in start..end {
                let compact = if shader_transport {
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
                        self.rhs_transfer_program.as_ref().unwrap().encode(
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
                if !shader_transport && (reuse || wave != 0) {
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
                    self.rhs_transfer_program.as_ref().unwrap().encode(
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
            let submission=self.queue.submit([encoder.finish()]);
            self.response_submissions += 1;
            let mut reads = snapshots
                .into_iter()
                .map(|snapshot| snapshot.begin_read())
                .collect::<Vec<_>>();
            self.device.poll(wgpu::PollType::Wait {submission_index:Some(submission),timeout:None})?;
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
            start = end;
        }
        self.response_dispatches += inputs.len();
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires real GPU; response waves with constrained shared staging"]
    fn gpu_response_waves_respect_small_readback_budgets_and_reuse_factors() {
        for compact in [false, true] {
            for byte_limited in [false, true] {
                let instance = voxy_render::GraphicsOptions::default().create_instance();
                let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
                let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
                let curve = (0..=8).map(|i| [0., i as f64 * 0.01, 0.]).collect();
                let mut system = physics::hair::HairRod::new(curve, Default::default()).unwrap().linear_system(1./240.).unwrap();
                for value in &mut system.matrix { *value *= (1./240.)*(1./240.); }
                system.rhs.fill(0.);
                let loads: Vec<Vec<f64>> = (0..11).map(|i| {
                    let mut load = system.rhs.clone();
                    load[6 + (i%7)*6] = (i+1) as f64 * 1e-7;
                    load
                }).collect();
                let rows = [BandedSystem { matrix: &system.matrix, rhs: &loads[0] }];
                let first = BandedSolveInput::new_with_underflow_tolerance(&rows, system.active.clone(), 1e-40).unwrap();
                let mut inputs = Vec::new();
                for load in &loads { inputs.push(first.with_rhs(&[load.as_slice()], 1e-40).unwrap()); }
                let size = if compact { first.compact_output_size() } else { first.bytes().len() as u64 };
                let pool = voxy_render::ComputeReadbackPool::configure(&device, voxy_render::ComputeReadbackLimits {
                    max_bytes: size * if byte_limited {2} else {8},
                    max_buffers: if byte_limited {8} else {2},
                }).unwrap();
                let mut gpu = pollster::block_on(GpuHairLinearSolver::new(&device,&queue)).unwrap();
                gpu.compact_response_readback = compact;
                if compact && byte_limited {
                    assert!(first.bytes().len() as u64 + first.compact_output_size() > pool.limits().max_bytes);
                    gpu.reference_audit = true;
                    assert!(gpu.dispatch_input(&first,None).is_err());
                    assert_eq!(pool.stats().quarantined_buffers,0);
                    assert_eq!(pool.available_capacity(),pool.limits());
                    gpu.reference_audit = false;
                    gpu.dispatch_input(&first,None).unwrap();
                }
                // Exercise both ordered-copy and gather-shader transports.
                for transport in [false, true].into_iter().filter(|&t| compact || !t) {
                    gpu.gpu_response_transport = transport;
                    let mut expected = Vec::new();
                    for input in &inputs { expected.push(gpu.dispatch_response_waves(std::slice::from_ref(input),false).unwrap().pop().unwrap()); }
                    for reuse in [false,true] {
                        let before = gpu.response_submissions;
                        let actual = gpu.dispatch_response_waves(&inputs,reuse).unwrap();
                        assert_eq!(actual,expected,"wave arithmetic/order changed: compact={compact} transport={transport} reuse={reuse}");
                        assert_eq!(gpu.response_submissions-before,6);
                        for (load, response) in loads.iter().zip(&actual) {
                            system.validate_load_correction(&response[0],load).unwrap();
                        }
                        assert_eq!(pool.stats().quarantined_buffers,0);
                        assert_eq!(pool.available_capacity(),pool.limits());
                    }
                }
                eprintln!("GPU BOUNDED RESPONSE compact={compact} byte_limited={byte_limited} loads=11 original_residuals=true exact_serial=true factor_reuse=true quarantine=0");
            }
        }
    }
}
