use std::sync::{Arc, Mutex};

use belfast::Device;

// Three compute passes, optional forest depth, forest color, firefly glow.
const QUERY_COUNT: u32 = 12;
const STRIDE: u64 = wgpu::QUERY_RESOLVE_BUFFER_ALIGNMENT;
type Readback = Arc<Mutex<Option<Result<Vec<f64>, String>>>>;

/// Resolve into a different aligned region each frame; map only after the run.
/// Nothing is allocated, mapped or read back inside the measured frame loop.
pub struct GpuTiming {
    pub queries: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    readback: wgpu::Buffer,
    capacity: u32,
    depth_frames: Vec<bool>,
    result: Readback,
    finished: bool,
}

impl GpuTiming {
    pub fn new(device: &Device, frames: u32) -> Self {
        let buffer = |label, usage| {
            device.gpu().create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: u64::from(frames) * STRIDE,
                usage,
                mapped_at_creation: false,
            })
        };
        Self {
            queries: device.gpu().create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("BenchmarkTimestamps"),
                ty: wgpu::QueryType::Timestamp,
                count: QUERY_COUNT,
            }),
            resolve: buffer(
                "BenchmarkResolve",
                wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            ),
            readback: buffer(
                "BenchmarkReadback",
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            ),
            capacity: frames,
            depth_frames: Vec::with_capacity(frames as usize),
            result: Arc::new(Mutex::new(None)),
            finished: false,
        }
    }

    pub fn recording(&self) -> bool {
        !self.finished && self.depth_frames.len() < self.capacity as usize
    }

    pub fn record(&mut self, encoder: &mut wgpu::CommandEncoder, depth: bool) {
        if !self.recording() {
            return;
        }
        encoder.resolve_query_set(
            &self.queries,
            0..QUERY_COUNT,
            &self.resolve,
            self.depth_frames.len() as u64 * STRIDE,
        );
        self.depth_frames.push(depth);
    }

    pub fn finish(&mut self, device: &Device) {
        if self.finished {
            return;
        }
        self.finished = true;
        if self.depth_frames.len() != self.capacity as usize {
            *self.result.lock().unwrap() = Some(Err("Incomplete GPU measurement".into()));
            return;
        }
        let size = u64::from(self.capacity) * STRIDE;
        let mut encoder = device.gpu().create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(&self.resolve, 0, &self.readback, 0, size);
        device.queue().submit([encoder.finish()]);
        let buffer = self.readback.clone();
        let result = self.result.clone();
        let depth_frames = self.depth_frames.clone();
        let period_ms = f64::from(device.queue().get_timestamp_period()) / 1_000_000.;
        self.readback
            .slice(..size)
            .map_async(wgpu::MapMode::Read, move |status| {
                let values = status.map_err(|error| error.to_string()).and_then(|()| {
                    let bytes = buffer
                        .slice(..size)
                        .get_mapped_range()
                        .map_err(|error| error.to_string())?;
                    let mut values = Vec::with_capacity(depth_frames.len() * 4);
                    for (frame, &has_depth) in depth_frames.iter().enumerate() {
                        let offset = frame * STRIDE as usize;
                        let query = |index: usize| {
                            let start = offset + index * 8;
                            u64::from_le_bytes(bytes[start..start + 8].try_into().unwrap())
                        };
                        let elapsed = |start, end| {
                            query(end)
                                .checked_sub(query(start))
                                .map(|ticks| ticks as f64 * period_ms)
                                .ok_or_else(|| "Invalid GPU timestamp ordering".to_string())
                        };
                        let compute = elapsed(0, 1)? + elapsed(2, 3)? + elapsed(4, 5)?;
                        let forest = elapsed(8, 9)? + if has_depth { elapsed(6, 7)? } else { 0. };
                        // Span includes gaps between passes, but excludes query resolve,
                        // readback, presentation and CPU work.
                        values.extend([elapsed(0, 11)?, compute, forest, elapsed(10, 11)?]);
                    }
                    Ok(values)
                });
                buffer.unmap();
                *result.lock().unwrap() = Some(values);
            });
    }

    pub fn take(&mut self) -> Result<Option<Vec<f64>>, String> {
        self.result.lock().unwrap().take().transpose()
    }
}

pub fn render_timestamps(
    queries: Option<&wgpu::QuerySet>,
    start: u32,
) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
    queries.map(|query_set| wgpu::RenderPassTimestampWrites {
        query_set,
        beginning_of_pass_write_index: Some(start),
        end_of_pass_write_index: Some(start + 1),
    })
}
