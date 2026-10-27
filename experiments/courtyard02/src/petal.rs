use crate::markers::FlowerEmitter;
use belfast::{
    BindGroup, Buffer, BufferUsage, Device, Draw, DrawOptions, Mesh, MeshIndexFormat,
    VertexAttributeDescriptor, VertexBufferBinding,
};
use glam::{Quat, Vec3};
use serde::Deserialize;

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub const MIN_PETALS: u32 = 10_000;
pub const MAX_PETALS: u32 = 100_000;
const PARTICLE_SEED: u32 = 0x6d61_676e;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct PetalInstance {
    position: [f32; 4],
    rotation: [f32; 4], // Unit quaternion, xyzw.
}

#[derive(Deserialize)]
struct PetalMesh {
    version: u32,
    // Position, normal, UV, linear RGBA, exported from the same Blender mesh as the GLB.
    vertices: Vec<[f32; 12]>,
    indices: Vec<u32>,
}

pub struct Petal {
    mesh: Mesh,
    draw: Draw,
    camera: Buffer,
    binding: BindGroup,
    srgb: bool,
    instances: Buffer,
    initialized_count: u32,
    random_state: u32,
}

impl Petal {
    pub fn new(
        device: &Device,
        bytes: &[u8],
        emitters: &[FlowerEmitter],
        count: u32,
    ) -> Result<Self, String> {
        let data: PetalMesh = serde_json::from_slice(bytes)
            .map_err(|error| format!("Invalid petal mesh: {error}"))?;
        let limit = device.gpu().limits().max_buffer_size;
        if data.version != 1
            || data.vertices.is_empty()
            || data.indices.is_empty()
            || !data.indices.len().is_multiple_of(3)
            || data.vertices.len() as u64 * 48 > limit
            || data.indices.len() as u64 * 4 > limit
            || data.vertices.iter().any(|v| {
                !v.iter().all(|n| n.is_finite())
                    || Vec3::new(v[3], v[4], v[5]).length_squared() < 1e-10
            })
            || data
                .indices
                .iter()
                .any(|&i| i as usize >= data.vertices.len())
        {
            return Err("Petal mesh has invalid geometry or exceeds GPU buffer limits".into());
        }
        let vertices = Buffer::from_data(
            device,
            &data.vertices,
            BufferUsage::vertex(),
            "PetalVertices",
        );
        let indices =
            Buffer::from_data(device, &data.indices, BufferUsage::index(), "PetalIndices");
        let mut mesh = Mesh::new(data.vertices.len() as u32)
            .and_then(|mesh| {
                mesh.add_vertex_buffer(VertexBufferBinding {
                    buffer: vertices,
                    array_stride: 48,
                    attributes: [
                        (wgpu::VertexFormat::Float32x3, 0),
                        (wgpu::VertexFormat::Float32x3, 12),
                        (wgpu::VertexFormat::Float32x2, 24),
                        (wgpu::VertexFormat::Float32x4, 32),
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(location, (format, offset))| VertexAttributeDescriptor {
                        shader_location: location as u32,
                        format,
                        offset,
                    })
                    .collect(),
                    slot: Some(0),
                    step_mode: None,
                })
            })
            .map_err(|error| error.to_string())?;
        mesh.set_index_buffer(indices, data.indices.len() as u32, MeshIndexFormat::Uint32)
            .map_err(|error| error.to_string())?;
        let mut options = DrawOptions::new("WhitePetal", device.format());
        options.primitive.cull_mode = None;
        options.depth_stencil = Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: Default::default(),
            bias: Default::default(),
        });
        let draw = Draw::new(device, include_str!("petal.wgsl"), &mesh, options);
        let camera = Buffer::create(device, 80, BufferUsage::uniform(), "PetalCamera");
        let capacity = u64::from(MAX_PETALS) * std::mem::size_of::<PetalInstance>() as u64;
        if capacity > limit
            || capacity > u64::from(device.gpu().limits().max_storage_buffer_binding_size)
        {
            return Err("Petal instances exceed GPU storage limits".into());
        }
        let instances = Buffer::create(device, capacity, BufferUsage::storage(), "PetalInstances");
        let binding = BindGroup::create(
            device,
            &draw.get_bind_group_layout(0),
            &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera.gpu().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: instances.gpu().as_entire_binding(),
                },
            ],
            "PetalBinding",
        );
        let mut petals = Self {
            mesh,
            draw,
            camera,
            binding,
            srgb: device.format().is_srgb(),
            instances,
            initialized_count: 0,
            random_state: PARTICLE_SEED,
        };
        petals.ensure_instances(device, emitters, count)?;
        Ok(petals)
    }

    pub fn ensure_instances(
        &mut self,
        device: &Device,
        emitters: &[FlowerEmitter],
        count: u32,
    ) -> Result<(), String> {
        if !(MIN_PETALS..=MAX_PETALS).contains(&count)
            || emitters.is_empty()
            || emitters
                .iter()
                .any(|e| !e.position.iter().all(|v| v.is_finite()))
        {
            return Err(
                "Petals require finite flower positions and a count between 10,000 and 100,000"
                    .into(),
            );
        }
        if count <= self.initialized_count {
            return Ok(());
        }
        // Continue the seeded stream so changing the visible count never reshuffles existing petals.
        let mut random = || {
            self.random_state ^= self.random_state << 13;
            self.random_state ^= self.random_state >> 17;
            self.random_state ^= self.random_state << 5;
            (self.random_state >> 8) as f32 / 16_777_216.0
        };
        let mut particles = Vec::with_capacity((count - self.initialized_count) as usize);
        for index in self.initialized_count..count {
            let source = Vec3::from_array(emitters[index as usize % emitters.len()].position);
            let jitter = Vec3::new(random() - 0.5, random() - 0.5, random() - 0.5) * 0.02;
            // Uniform unit quaternion: independent orientation around arbitrary axes.
            let u = random();
            let a = random() * std::f32::consts::TAU;
            let b = random() * std::f32::consts::TAU;
            let low = (1.0 - u).sqrt();
            let high = u.sqrt();
            let rotation =
                Quat::from_xyzw(low * a.sin(), low * a.cos(), high * b.sin(), high * b.cos())
                    .normalize();
            particles.push(PetalInstance {
                position: (source + jitter).extend(1.0).to_array(),
                rotation: rotation.to_array(),
            });
        }
        self.instances.write(
            device,
            &particles,
            u64::from(self.initialized_count) * std::mem::size_of::<PetalInstance>() as u64,
        );
        self.initialized_count = count;
        Ok(())
    }

    pub fn reset_instances(
        &mut self,
        device: &Device,
        emitters: &[FlowerEmitter],
        count: u32,
    ) -> Result<(), String> {
        self.initialized_count = 0;
        self.random_state = PARTICLE_SEED;
        self.ensure_instances(device, emitters, count)
    }

    pub fn update(&self, device: &Device, view_projection: &[f32; 16], scale: f32) {
        let mut values = view_projection.to_vec();
        values.extend([u32::from(self.srgb) as f32, scale, 0.0, 0.0]);
        self.camera.write(device, &values, 0);
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, count: u32) {
        self.binding.bind(pass, 0);
        self.draw.draw(pass, &self.mesh, count);
    }
}

pub fn depth_view(device: &Device, viewport: [u32; 2]) -> wgpu::TextureView {
    device
        .gpu()
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("CourtyardDepth"),
            size: wgpu::Extent3d {
                width: viewport[0].max(1),
                height: viewport[1].max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&Default::default())
}
