use belfast::{BelfastError, Device, RenderPassOptions, RenderTarget, RenderTargetOptions};

#[test]
fn render_target_tracks_creator_device_after_resize() {
    let Some(device) = create_optional_device() else {
        return;
    };
    let mut target = RenderTarget::create(
        &device,
        RenderTargetOptions {
            width: 8,
            height: 8,
            ..Default::default()
        },
    );
    target.resize(16, 12);

    assert!(target.device().is_same(&device));
    assert_eq!((target.width(), target.height()), (16, 12));
}

#[test]
fn render_target_options_clamp_dimensions_and_default_format() {
    let options = RenderTargetOptions {
        width: 0,
        height: 0,
        ..Default::default()
    };

    assert_eq!(options.resolved_width(), 1);
    assert_eq!(options.resolved_height(), 1);
    assert_eq!(options.format, None);
    assert_eq!(options.sample_count, 1);
    assert!(!options.with_depth);
    assert_eq!(options.depth_format, wgpu::TextureFormat::Depth24Plus);
}

#[test]
fn render_pass_options_default_to_clear_and_store() {
    let options = RenderPassOptions::default();

    assert_eq!(
        options.clear_color,
        wgpu::Color {
            r: 0.05,
            g: 0.05,
            b: 0.08,
            a: 1.0,
        }
    );
    assert_eq!(options.load_op, wgpu::LoadOp::Clear(options.clear_color));
    assert_eq!(options.store_op, wgpu::StoreOp::Store);
}

#[test]
fn render_target_tracks_depth_metadata() {
    let options = RenderTargetOptions {
        width: 320,
        height: 180,
        format: Some(wgpu::TextureFormat::Rgba16Float),
        with_depth: true,
        depth_format: wgpu::TextureFormat::Depth32Float,
        ..Default::default()
    };

    assert_eq!(options.resolved_width(), 320);
    assert_eq!(options.resolved_height(), 180);
    assert_eq!(options.format, Some(wgpu::TextureFormat::Rgba16Float));
    assert!(options.with_depth);
    assert_eq!(options.depth_format, wgpu::TextureFormat::Depth32Float);
}

#[test]
fn render_target_create_and_resize_update_metadata() {
    let Some(device) = create_optional_device() else {
        return;
    };
    let mut target = RenderTarget::create(
        &device,
        RenderTargetOptions {
            width: 8,
            height: 4,
            with_depth: true,
            ..Default::default()
        },
    );

    assert_eq!(target.width(), 8);
    assert_eq!(target.height(), 4);
    assert_eq!(target.format(), device.format());
    assert_eq!(
        target.depth_format(),
        Some(wgpu::TextureFormat::Depth24Plus)
    );
    assert!(target.depth_view().is_some());

    target.resize(0, 16);

    assert_eq!(target.width(), 1);
    assert_eq!(target.height(), 16);
    assert!(target.depth_view().is_some());
}

#[test]
fn render_target_exposes_render_pass_target_views() {
    let Some(device) = create_optional_device() else {
        return;
    };
    let target = RenderTarget::create(
        &device,
        RenderTargetOptions {
            width: 2,
            height: 2,
            with_depth: true,
            ..Default::default()
        },
    );

    let pass_target = target.render_pass_target();

    assert!(std::ptr::addr_eq(
        pass_target.color_view,
        target.color_view()
    ));
    assert!(pass_target.depth_view.is_some());
}

fn create_optional_device() -> Option<Device> {
    match pollster::block_on(Device::create_headless()) {
        Ok(device) => Some(device),
        Err(BelfastError::AdapterUnavailable) => {
            eprintln!("skipping GPU-backed RenderTarget test: adapter unavailable");
            None
        }
        Err(error) => panic!("failed to create headless device: {error}"),
    }
}
