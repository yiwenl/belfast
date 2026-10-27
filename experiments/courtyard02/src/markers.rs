use std::collections::HashSet;

use belfast::{BallHelper, BallHelperOptions, BindGroup, Buffer, BufferUsage, Device};
use serde::Deserialize;

#[derive(Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Quality {
    Likely,
    Review,
}

#[derive(Deserialize)]
pub struct FlowerEmitter {
    pub id: u32,
    pub position: [f32; 3],
    pub quality: Quality,
}

#[derive(Deserialize)]
pub struct EmitterFile {
    schema_version: u32,
    coordinate_system: String,
    count: usize,
    pub marker_radius: f32,
    pub emitters: Vec<FlowerEmitter>,
}

pub fn parse_emitters(bytes: &[u8]) -> Result<EmitterFile, String> {
    let file: EmitterFile =
        serde_json::from_slice(bytes).map_err(|e| format!("Invalid flower emitter JSON: {e}"))?;
    if file.schema_version != 1 || file.coordinate_system != "belfast-world-y-up" {
        return Err("Flower emitters must use schema 1 and Belfast world coordinates".into());
    }
    if file.emitters.is_empty() || file.emitters.len() != file.count {
        return Err("Flower emitter count is empty or does not match the records".into());
    }
    if !file.marker_radius.is_finite() || file.marker_radius <= 0. {
        return Err("Flower marker radius must be positive and finite".into());
    }
    let mut ids = HashSet::new();
    for emitter in &file.emitters {
        if !emitter.position.iter().all(|v| v.is_finite()) || !ids.insert(emitter.id) {
            return Err("Flower emitters need finite positions and unique IDs".into());
        }
    }
    Ok(file)
}

pub struct FlowerMarkers {
    /// Scene-space spawn sources shared by the marker overlay and petal simulation.
    pub emitters: Vec<FlowerEmitter>,
    balls: Vec<BallHelper>,
    camera: Buffer,
    bind_group: BindGroup,
}

impl FlowerMarkers {
    pub fn new(device: &Device, bytes: &[u8]) -> Result<Self, String> {
        let file = parse_emitters(bytes)?;
        let layout = device
            .gpu()
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("FlowerMarkerCameraLayout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let pipeline_layout =
            device
                .gpu()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("FlowerMarkerLayout"),
                    bind_group_layouts: &[Some(&layout)],
                    immediate_size: 0,
                });
        let camera = Buffer::create(device, 64, BufferUsage::uniform(), "FlowerMarkerCamera");
        let bind_group = BindGroup::create(
            device,
            &layout,
            &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.gpu().as_entire_binding(),
            }],
            "FlowerMarkerCameraBindGroup",
        );
        let balls = file
            .emitters
            .iter()
            .map(|emitter| {
                BallHelper::new(
                    device,
                    BallHelperOptions {
                        label: "FlowerMarker",
                        position: emitter.position,
                        radius: file.marker_radius,
                        color: match emitter.quality {
                            Quality::Likely => [0.0, 0.9, 1.0, 1.0],
                            Quality::Review => [1.0, 0.35, 0.03, 1.0],
                        },
                        ..BallHelperOptions::new(device.format(), &pipeline_layout)
                    },
                )
                .map_err(|e| e.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            emitters: file.emitters,
            balls,
            camera,
            bind_group,
        })
    }

    pub fn update_camera(&self, device: &Device, matrix: &[f32; 16]) {
        self.camera.write(device, matrix, 0);
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        // Match Blender's In Front markers so candidates remain visible for inspection.
        for ball in &self.balls {
            ball.draw(pass, &self.bind_group);
        }
    }
}
