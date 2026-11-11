//! One bounded asynchronous timestamp slot for resident voxel and vegetation passes.
use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};
#[derive(Debug)]
pub(super) struct Timing {
    queries: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    readback: wgpu::Buffer,
    ready: Arc<AtomicU8>,
    pending_passes: usize,
    scheduled: u64,
    valid: u64,
    rejected: u64,
    map_failures: u64,
}
impl Timing {
    pub fn new(device: &wgpu::Device) -> Option<Self> {
        if !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            return None;
        }
        Some(Self {
            queries: device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("resident voxel timestamps"),
                ty: wgpu::QueryType::Timestamp,
                count: 8,
            }),
            resolve: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("resident timestamp resolve"),
                size: 256,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            readback: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("resident timestamp readback"),
                size: 64,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            ready: Arc::new(AtomicU8::new(0)),
            pending_passes: 0,
            scheduled: 0,
            valid: 0,
            rejected: 0,
            map_failures: 0,
        })
    }
    pub fn status(&self) -> [u64; 5] {
        [
            self.scheduled,
            self.valid,
            self.rejected,
            self.map_failures,
            u64::from(self.pending_passes != 0),
        ]
    }

    pub fn writes(&self, pass: u32) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        (self.pending_passes == 0).then_some(wgpu::RenderPassTimestampWrites {
            query_set: &self.queries,
            beginning_of_pass_write_index: Some(2 * pass),
            end_of_pass_write_index: Some(2 * pass + 1),
        })
    }
    pub fn resolve(&self, encoder: &mut wgpu::CommandEncoder, passes: usize) {
        if self.pending_passes == 0 {
            encoder.resolve_query_set(&self.queries, 0..2 * passes as u32, &self.resolve, 0);
            encoder.copy_buffer_to_buffer(&self.resolve, 0, &self.readback, 0, 16 * passes as u64);
        }
    }
    pub fn map(&mut self, passes: usize) {
        if self.pending_passes != 0 {
            return;
        }
        self.pending_passes = passes;
        self.scheduled += 1;
        let ready = self.ready.clone();
        self.readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                ready.store(if result.is_ok() { 1 } else { 2 }, Ordering::Release);
            });
    }
    pub fn collect(&mut self, period: f32) -> [Option<f64>; 4] {
        let mut result = [None; 4];
        match self.ready.swap(0, Ordering::AcqRel) {
            1 => {}
            2 => {
                self.map_failures += 1;
                self.pending_passes = 0;
                return result;
            }
            _ => return result,
        }
        if let Ok(bytes) = self.readback.slice(..).get_mapped_range() {
            for (pair, output) in bytes
                .chunks_exact(16)
                .zip(&mut result[..self.pending_passes])
            {
                let begin = u64::from_le_bytes(pair[..8].try_into().unwrap());
                let end = u64::from_le_bytes(pair[8..].try_into().unwrap());
                *output = (period.is_finite() && period > 0. && end > begin)
                    .then(|| (end - begin) as f64 * f64::from(period) / 1e6);
            }
        }
        self.readback.unmap();
        self.pending_passes = 0;
        if result.iter().any(Option::is_some) {
            self.valid += 1;
        } else {
            self.rejected += 1;
        }
        result
    }
}
