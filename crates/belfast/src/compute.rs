use crate::{BelfastError, BelfastResult, BindGroup, Device};

pub struct ComputeOptions<'a> {
    pub label: &'a str,
    pub layout: Option<&'a wgpu::PipelineLayout>,
    pub entry_point: &'a str,
}

impl<'a> ComputeOptions<'a> {
    pub fn new(label: &'a str) -> Self {
        Self {
            label,
            layout: None,
            entry_point: "cs_main",
        }
    }
}

pub struct Compute {
    pipeline: wgpu::ComputePipeline,
    device: Device,
}

impl Compute {
    pub fn new(device: &Device, shader_code: &str, options: ComputeOptions<'_>) -> Self {
        let module = device
            .gpu()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(&format!("{}Shader", options.label)),
                source: wgpu::ShaderSource::Wgsl(shader_code.into()),
            });
        let pipeline = device
            .gpu()
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(&format!("{}Pipeline", options.label)),
                layout: options.layout,
                module: &module,
                entry_point: Some(options.entry_point),
                compilation_options: Default::default(),
                cache: None,
            });

        Self {
            pipeline,
            device: device.clone(),
        }
    }

    pub fn get_bind_group_layout(&self, index: u32) -> wgpu::BindGroupLayout {
        self.pipeline.get_bind_group_layout(index)
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn validate_for_dispatch(&self, device: &Device) -> BelfastResult<()> {
        if !self.device.is_same(device) {
            return Err(BelfastError::ComputeDeviceMismatch);
        }
        Ok(())
    }

    pub fn dispatch<'a>(&'a self, pass: &mut wgpu::ComputePass<'a>, workgroups: [u32; 3]) {
        pass.set_pipeline(&self.pipeline);
        pass.dispatch_workgroups(workgroups[0], workgroups[1], workgroups[2]);
    }

    pub fn run(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        bind_group: Option<&BindGroup>,
        workgroups: [u32; 3],
        label: Option<&str>,
    ) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label,
            timestamp_writes: None,
        });
        if let Some(bind_group) = bind_group {
            bind_group.bind_compute(&mut pass, 0);
        }
        self.dispatch(&mut pass, workgroups);
    }
}
