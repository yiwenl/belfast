use belfast::{BelfastError, Device, Texture, TextureOptions};

#[test]
fn rejects_zero_sized_rgba_textures() {
    let Some(device) = create_optional_device() else {
        return;
    };

    let result = Texture::from_rgba8(&device, 0, 4, &[], TextureOptions::default());

    assert!(matches!(
        result,
        Err(BelfastError::InvalidTextureDimensions {
            width: 0,
            height: 4
        })
    ));
}

#[test]
fn rejects_incorrect_rgba_byte_length() {
    let Some(device) = create_optional_device() else {
        return;
    };

    let result = Texture::from_rgba8(&device, 2, 2, &[255; 15], TextureOptions::default());

    assert!(matches!(
        result,
        Err(BelfastError::InvalidTextureDataLength {
            expected: 16,
            actual: 15
        })
    ));
}

#[test]
fn uploads_rgba_texture_and_exposes_metadata() {
    let Some(device) = create_optional_device() else {
        return;
    };

    let texture = Texture::from_rgba8(
        &device,
        2,
        1,
        &[255, 0, 0, 255, 0, 255, 0, 255],
        TextureOptions::default(),
    )
    .expect("valid texture");

    assert_eq!(texture.width(), 2);
    assert_eq!(texture.height(), 1);
    assert_eq!(texture.format(), wgpu::TextureFormat::Rgba8UnormSrgb);
}

fn create_optional_device() -> Option<Device> {
    match pollster::block_on(Device::create_headless()) {
        Ok(device) => Some(device),
        Err(BelfastError::AdapterUnavailable) => {
            eprintln!("skipping GPU-backed Texture test: adapter unavailable");
            None
        }
        Err(error) => panic!("failed to create headless device: {error}"),
    }
}
