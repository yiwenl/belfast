use belfast::{
    BindGroup, Buffer, BufferUsage, Device, Draw, DrawOptions, Mesh, MeshIndexFormat,
    VertexAttributeDescriptor, VertexBufferBinding,
};
use glam::{Mat4, Quat, Vec3};
use serde::Deserialize;

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

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
}

impl Petal {
    pub fn new(device: &Device, bytes: &[u8]) -> Result<Self, String> {
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
        let camera = Buffer::create(device, 144, BufferUsage::uniform(), "PetalCamera");
        let binding = BindGroup::create(
            device,
            &draw.get_bind_group_layout(0),
            &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.gpu().as_entire_binding(),
            }],
            "PetalBinding",
        );
        Ok(Self {
            mesh,
            draw,
            camera,
            binding,
            srgb: device.format().is_srgb(),
        })
    }

    pub fn update(&self, device: &Device, view_projection: &[f32; 16], scale: f32) {
        // Static foreground sample; the reusable mesh keeps its base at the origin.
        let model = Mat4::from_scale_rotation_translation(
            Vec3::splat(scale),
            Quat::from_rotation_y(-0.55) * Quat::from_rotation_x(0.2) * Quat::from_rotation_z(-0.3),
            Vec3::new(-1.19, 0.05, -0.93),
        );
        let mut values = view_projection.to_vec();
        values.extend(model.to_cols_array());
        values.extend([u32::from(self.srgb) as f32, 0.0, 0.0, 0.0]);
        self.camera.write(device, &values, 0);
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        self.binding.bind(pass, 0);
        self.draw.draw(pass, &self.mesh, 1);
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
