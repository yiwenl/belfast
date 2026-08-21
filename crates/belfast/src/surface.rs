#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceColorChoice {
    pub format: wgpu::TextureFormat,
    pub color_space: wgpu::SurfaceColorSpace,
    pub hdr: bool,
}

const HDR_PREFERENCES: &[(wgpu::TextureFormat, wgpu::SurfaceColorSpace)] = &[
    (
        wgpu::TextureFormat::Rgba16Float,
        wgpu::SurfaceColorSpace::ExtendedSrgbLinear,
    ),
    (
        wgpu::TextureFormat::Rgba16Float,
        wgpu::SurfaceColorSpace::ExtendedSrgb,
    ),
];

pub fn pick_surface_color(caps: &wgpu::SurfaceCapabilities, want_hdr: bool) -> SurfaceColorChoice {
    if want_hdr {
        for &(format, color_space) in HDR_PREFERENCES {
            if let Some(flag) = color_space.to_color_spaces() {
                if caps.color_spaces(format).contains(flag) {
                    return SurfaceColorChoice {
                        format,
                        color_space,
                        hdr: true,
                    };
                }
            }
        }
    }
    sdr_choice(&caps.formats)
}

fn sdr_choice(formats: &[wgpu::TextureFormat]) -> SurfaceColorChoice {
    let format = formats
        .iter()
        .copied()
        .find(wgpu::TextureFormat::is_srgb)
        .or_else(|| formats.first().copied())
        .unwrap_or(wgpu::TextureFormat::Bgra8UnormSrgb);
    SurfaceColorChoice {
        format,
        color_space: wgpu::SurfaceColorSpace::Auto,
        hdr: false,
    }
}

#[cfg(test)]
mod tests {
    use super::{pick_surface_color, SurfaceColorChoice};
    use wgpu::{
        SurfaceCapabilities, SurfaceColorSpace, SurfaceColorSpaces, SurfaceFormatCapabilities,
        TextureFormat,
    };

    fn caps(
        formats: Vec<TextureFormat>,
        format_capabilities: Vec<SurfaceFormatCapabilities>,
    ) -> SurfaceCapabilities {
        SurfaceCapabilities {
            formats,
            format_capabilities,
            ..Default::default()
        }
    }

    fn format_caps(
        format: TextureFormat,
        color_spaces: SurfaceColorSpaces,
    ) -> SurfaceFormatCapabilities {
        SurfaceFormatCapabilities {
            format,
            color_spaces,
        }
    }

    #[test]
    fn sdr_prefers_first_srgb_format_and_auto() {
        let caps = caps(
            vec![TextureFormat::Rgba8Unorm, TextureFormat::Bgra8UnormSrgb],
            vec![
                format_caps(TextureFormat::Rgba8Unorm, SurfaceColorSpaces::SRGB),
                format_caps(TextureFormat::Bgra8UnormSrgb, SurfaceColorSpaces::SRGB),
            ],
        );
        assert_eq!(
            pick_surface_color(&caps, false),
            SurfaceColorChoice {
                format: TextureFormat::Bgra8UnormSrgb,
                color_space: SurfaceColorSpace::Auto,
                hdr: false,
            }
        );
    }

    #[test]
    fn hdr_prefers_linear_scrgb_when_available() {
        let caps = caps(
            vec![TextureFormat::Bgra8UnormSrgb, TextureFormat::Rgba16Float],
            vec![
                format_caps(TextureFormat::Bgra8UnormSrgb, SurfaceColorSpaces::SRGB),
                format_caps(
                    TextureFormat::Rgba16Float,
                    SurfaceColorSpaces::EXTENDED_SRGB_LINEAR | SurfaceColorSpaces::EXTENDED_SRGB,
                ),
            ],
        );
        assert_eq!(
            pick_surface_color(&caps, true),
            SurfaceColorChoice {
                format: TextureFormat::Rgba16Float,
                color_space: SurfaceColorSpace::ExtendedSrgbLinear,
                hdr: true,
            }
        );
    }

    #[test]
    fn hdr_falls_back_to_encoded_extended_srgb() {
        let caps = caps(
            vec![TextureFormat::Rgba8UnormSrgb, TextureFormat::Rgba16Float],
            vec![
                format_caps(TextureFormat::Rgba8UnormSrgb, SurfaceColorSpaces::SRGB),
                format_caps(
                    TextureFormat::Rgba16Float,
                    SurfaceColorSpaces::EXTENDED_SRGB,
                ),
            ],
        );
        assert_eq!(
            pick_surface_color(&caps, true),
            SurfaceColorChoice {
                format: TextureFormat::Rgba16Float,
                color_space: SurfaceColorSpace::ExtendedSrgb,
                hdr: true,
            }
        );
    }

    #[test]
    fn hdr_falls_back_to_sdr_when_no_hdr_space() {
        let caps = caps(
            vec![TextureFormat::Bgra8UnormSrgb],
            vec![format_caps(
                TextureFormat::Bgra8UnormSrgb,
                SurfaceColorSpaces::SRGB,
            )],
        );
        assert_eq!(
            pick_surface_color(&caps, true),
            SurfaceColorChoice {
                format: TextureFormat::Bgra8UnormSrgb,
                color_space: SurfaceColorSpace::Auto,
                hdr: false,
            }
        );
    }

    #[test]
    fn hdr_can_pick_float_format_only_listed_in_format_capabilities() {
        let caps = caps(
            vec![TextureFormat::Bgra8UnormSrgb],
            vec![
                format_caps(TextureFormat::Bgra8UnormSrgb, SurfaceColorSpaces::SRGB),
                format_caps(
                    TextureFormat::Rgba16Float,
                    SurfaceColorSpaces::EXTENDED_SRGB_LINEAR,
                ),
            ],
        );
        assert_eq!(
            pick_surface_color(&caps, true),
            SurfaceColorChoice {
                format: TextureFormat::Rgba16Float,
                color_space: SurfaceColorSpace::ExtendedSrgbLinear,
                hdr: true,
            }
        );
    }
}
