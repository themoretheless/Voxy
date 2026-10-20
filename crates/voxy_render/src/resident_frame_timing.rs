//! One bounded asynchronous timestamp slot for the actual resident voxel pass.
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
    pending: bool,
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
                count: 2,
            }),
            resolve: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("resident timestamp resolve"),
                size: 256,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            readback: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("resident timestamp readback"),
                size: 16,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            ready: Arc::new(AtomicU8::new(0)),
            pending: false,
        })
    }
    pub fn writes(&self) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        (!self.pending).then_some(wgpu::RenderPassTimestampWrites {
            query_set: &self.queries,
            beginning_of_pass_write_index: Some(0),
            end_of_pass_write_index: Some(1),
        })
    }
    pub fn resolve(&self, encoder: &mut wgpu::CommandEncoder) {
        if !self.pending {
            encoder.resolve_query_set(&self.queries, 0..2, &self.resolve, 0);
            encoder.copy_buffer_to_buffer(&self.resolve, 0, &self.readback, 0, 16);
        }
    }
    pub fn map(&mut self) {
        if self.pending {
            return;
        }
        self.pending = true;
        let ready = self.ready.clone();
        self.readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                ready.store(if result.is_ok() { 1 } else { 2 }, Ordering::Release)
            });
    }
    pub fn collect(&mut self, period: f32) -> Option<f64> {
        match self.ready.swap(0, Ordering::AcqRel) {
            1 => {}
            2 => {
                self.pending = false;
                return None;
            }
            _ => return None,
        }
        let result = self
            .readback
            .slice(..)
            .get_mapped_range()
            .ok()
            .and_then(|bytes| {
                let begin = u64::from_le_bytes(bytes[0..8].try_into().ok()?);
                let end = u64::from_le_bytes(bytes[8..16].try_into().ok()?);
                (period.is_finite() && period > 0. && end > begin)
                    .then(|| (end - begin) as f64 * f64::from(period) / 1e6)
            });
        self.readback.unmap();
        self.pending = false;
        result
    }
}
