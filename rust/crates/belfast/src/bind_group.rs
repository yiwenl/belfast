use crate::Device;

pub struct BindGroup {
    gpu: wgpu::BindGroup,
}

impl BindGroup {
    pub fn create(
        device: &Device,
        layout: &wgpu::BindGroupLayout,
        entries: &[wgpu::BindGroupEntry<'_>],
        label: &str,
    ) -> Self {
        let gpu = device.gpu().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout,
            entries,
        });
        Self { gpu }
    }

    pub fn bind<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, index: u32) {
        pass.set_bind_group(index, &self.gpu, &[]);
    }

    pub fn gpu(&self) -> &wgpu::BindGroup {
        &self.gpu
    }
}
