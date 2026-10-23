use glam::{Mat4, Vec3, Vec4};

pub const SORT_TRANSLATION: f32 = 0.02;
pub const SORT_ANGLE: f32 = 0.5 * std::f32::consts::PI / 180.0;

#[derive(Clone, Copy, Debug)]
pub struct SortCamera {
    pub view: Mat4,
    pub projection: Mat4,
    pub viewport: [u32; 2],
}

impl SortCamera {
    pub fn new(view: Mat4, projection: Mat4, viewport: [u32; 2]) -> Self {
        Self {
            view,
            projection,
            viewport,
        }
    }
    pub fn encode(&self) -> Vec<f32> {
        let mut data = self.view.to_cols_array().to_vec();
        data.extend(self.projection.to_cols_array());
        data.extend(self.viewport.map(|n| n as f32));
        data
    }
    pub fn decode(data: &[f32]) -> Result<Self, String> {
        if data.len() != 34
            || !data.iter().all(|v| v.is_finite())
            || data[32] <= 0.
            || data[33] <= 0.
        {
            return Err("Expected finite view, projection and viewport for sorting".into());
        }
        Ok(Self::new(
            Mat4::from_cols_slice(&data[..16]),
            Mat4::from_cols_slice(&data[16..32]),
            [data[32] as u32, data[33] as u32],
        ))
    }
    fn eye(&self) -> Vec3 {
        self.view.inverse().w_axis.truncate()
    }
    fn forward(&self) -> Vec3 {
        -self.view.row(2).truncate()
    }
}

#[derive(Default)]
pub struct SortGate {
    last: Option<SortCamera>,
}

impl SortGate {
    pub fn needs_sort(&self, next: &SortCamera) -> bool {
        self.last.is_none_or(|last| {
            last.viewport != next.viewport
                || last.projection != next.projection
                || last.eye().distance_squared(next.eye()) >= SORT_TRANSLATION.powi(2)
                || last.forward().dot(next.forward()) <= SORT_ANGLE.cos()
        })
    }
    pub fn accept(&mut self, camera: SortCamera) {
        self.last = Some(camera);
    }
}

#[derive(Clone, Copy, Default)]
struct KeyIndex {
    key: u32,
    index: u32,
}

pub struct SplatSorter {
    positions: Vec<[f32; 4]>,
    keys: Vec<KeyIndex>,
    scratch: Vec<KeyIndex>,
    counts: Vec<usize>,
    indices: Vec<u32>,
    frozen_order: Vec<u32>,
    visible_mask: Vec<bool>,
    frozen: bool,
    frozen_ready: bool,
    brush_enabled: bool,
    sort_count: u32,
    did_sort: bool,
}

impl SplatSorter {
    pub fn new(positions: Vec<[f32; 4]>) -> Self {
        Self {
            positions,
            keys: Vec::new(),
            scratch: Vec::new(),
            counts: vec![0; 65536],
            indices: Vec::new(),
            frozen_order: Vec::new(),
            visible_mask: Vec::new(),
            frozen: false,
            frozen_ready: false,
            brush_enabled: false,
            sort_count: 0,
            did_sort: false,
        }
    }

    /// Each activation captures a fresh whole-scene order on the next update.
    pub fn set_frozen(&mut self, frozen: bool) {
        self.frozen = frozen;
        self.frozen_ready = false;
        self.frozen_order.clear();
    }

    pub fn set_brush_enabled(&mut self, enabled: bool) {
        if self.brush_enabled != enabled {
            self.brush_enabled = enabled;
            self.frozen_ready = false;
            self.frozen_order.clear();
        }
    }

    pub fn sort_count(&self) -> u32 {
        self.sort_count
    }
    pub fn did_sort(&self) -> bool {
        self.did_sort
    }

    pub fn sort(&mut self, camera: &SortCamera) -> &[u32] {
        self.did_sort = false;
        // A guard band reduces edge popping while an asynchronous sort is pending.
        let matrix = camera.projection * camera.view;
        let planes = [
            matrix.row(3) * 1.2 + matrix.row(0),
            matrix.row(3) * 1.2 - matrix.row(0),
            matrix.row(3) * 1.2 + matrix.row(1),
            matrix.row(3) * 1.2 - matrix.row(1),
            matrix.row(2),
            matrix.row(3) - matrix.row(2),
        ]
        .map(|p| p / p.truncate().length());
        // Match the doubled footprint in splat.wgsl, including occlusion passes.
        let footprint_scale = if self.brush_enabled { 2.0 } else { 1.0 };
        let visible = |&[x, y, z, radius]: &[f32; 4]| {
            let p = Vec4::new(x, y, z, 1.);
            !planes
                .iter()
                .any(|plane| plane.dot(p) < -radius * footprint_scale - SORT_TRANSLATION * 2.)
        };
        if !self.frozen || !self.frozen_ready {
            self.keys.clear();
            let depth_row = -camera.view.row(2);
            for (index, position) in self.positions.iter().enumerate() {
                if self.brush_enabled {
                    // Keep one stable member per pair, independent of view and brush texture.
                    let pair_hash = (index as u32 / 2).wrapping_mul(0x9e37_79b9);
                    let selected = (pair_hash ^ (pair_hash >> 16)) & 1;
                    if index as u32 & 1 != selected {
                        continue;
                    }
                }
                // Frozen order must include offscreen / behind-camera splats too.
                if !self.frozen && !visible(position) {
                    continue;
                }
                let depth = depth_row.dot(Vec4::new(position[0], position[1], position[2], 1.));
                if !depth.is_finite() {
                    continue;
                }
                // Monotonic float key supports signed depths; invert for far-to-near.
                let bits = depth.to_bits();
                let ascending = if bits & 0x8000_0000 != 0 {
                    !bits
                } else {
                    bits ^ 0x8000_0000
                };
                self.keys.push(KeyIndex {
                    key: !ascending,
                    index: index as u32,
                });
            }
            self.sort_keys();
            if self.frozen {
                self.frozen_order
                    .extend(self.keys.iter().map(|item| item.index));
                self.frozen_ready = true;
            }
        }
        self.indices.clear();
        if self.frozen {
            // Cull contiguous position data, then compact in the captured order.
            // This avoids random reads of the much larger position array.
            self.visible_mask.resize(self.positions.len(), false);
            for (mask, position) in self.visible_mask.iter_mut().zip(&self.positions) {
                *mask = visible(position);
            }
            self.indices.extend(
                self.frozen_order
                    .iter()
                    .copied()
                    .filter(|&i| self.visible_mask[i as usize]),
            );
        } else {
            self.indices.extend(self.keys.iter().map(|item| item.index));
        }
        &self.indices
    }

    fn sort_keys(&mut self) {
        self.scratch.resize(self.keys.len(), KeyIndex::default());
        for shift in [0, 16] {
            self.counts.fill(0);
            for item in &self.keys {
                self.counts[((item.key >> shift) & 65535) as usize] += 1;
            }
            let mut start = 0;
            for count in &mut self.counts {
                let n = *count;
                *count = start;
                start += n;
            }
            for &item in &self.keys {
                let bucket = ((item.key >> shift) & 65535) as usize;
                self.scratch[self.counts[bucket]] = item;
                self.counts[bucket] += 1;
            }
            std::mem::swap(&mut self.keys, &mut self.scratch);
        }
        self.did_sort = true;
        self.sort_count += 1;
    }
}
