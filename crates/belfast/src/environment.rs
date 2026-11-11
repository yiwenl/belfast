use crate::{BelfastError, BelfastResult};
use glam::Vec3;

/// Equirectangular RGB image. `v = 0` is +Y.
#[derive(Clone, Debug)]
pub struct EnvironmentImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<f32>,
}

/// Shared image-based light built from a Radiance `.hdr` file.
///
/// The sun disk is removed from `irradiance` and returned separately in `sun_color`,
/// so a renderer can shadow the sun without darkening the rest of the sky.
#[derive(Clone, Debug)]
pub struct EnvironmentMap {
    pub sun_direction: [f32; 3],
    /// Lambert contribution of the sun disk, E/π, before the caller scales it.
    pub sun_color: [f32; 3],
    pub irradiance: EnvironmentImage,
    pub specular: EnvironmentImage,
}

impl EnvironmentMap {
    pub fn from_radiance_hdr(bytes: &[u8]) -> BelfastResult<Self> {
        let (width, height, pixels) = decode_radiance(bytes)?;
        let (sun_direction, sun_color) = sun(width, height, &pixels);
        // shortcut: one cosine irradiance map and one Phong specular map, both 64×32.
        // Build a GGX mip chain if a surface needs a sharp sun reflection.
        let source = downsample(&pixels, width, height, 128, 64);
        let mut irradiance = convolve(&source, 128, 64, 64, 32, true, sun_direction);
        let mut specular = convolve(&source, 128, 64, 64, 32, false, sun_direction);
        let mut sun_color = sun_color;
        // Normalize the complete upward illumination, not the sky left after removing the sun.
        let sun_luminance = Vec3::from(sun_color).dot(Vec3::new(0.2126, 0.7152, 0.0722));
        let upward = top_row_luminance(&irradiance) + sun_luminance * sun_direction[1].max(0.0);
        if upward > 1e-4 {
            let scale = 0.85 / upward;
            scale_image(&mut irradiance, scale);
            scale_image(&mut specular, scale);
            for channel in &mut sun_color {
                *channel *= scale;
            }
        }
        Ok(Self {
            sun_direction,
            sun_color,
            irradiance,
            specular,
        })
    }

    pub fn sun_yaw_pitch_degrees(&self) -> [f32; 2] {
        let direction = Vec3::from(self.sun_direction);
        [
            direction.x.atan2(direction.z).to_degrees(),
            direction.y.clamp(-1.0, 1.0).asin().to_degrees(),
        ]
    }
}

/// Columns of the rotation that maps `from` onto `to`.
pub fn rotation_from_to(from: [f32; 3], to: [f32; 3]) -> [[f32; 3]; 3] {
    let from = Vec3::from(from);
    let to = Vec3::from(to);
    if from.length_squared() < 1e-8 || to.length_squared() < 1e-8 {
        return [Vec3::X.into(), Vec3::Y.into(), Vec3::Z.into()];
    }
    let from = from.normalize();
    let to = to.normalize();
    let cosine = from.dot(to).clamp(-1.0, 1.0);
    let cross = from.cross(to);
    let (axis, sine) = if cross.length_squared() < 1e-10 {
        if cosine > 0.0 {
            return [Vec3::X.into(), Vec3::Y.into(), Vec3::Z.into()];
        }
        let mut axis = from.cross(Vec3::Y);
        if axis.length_squared() < 1e-6 {
            axis = from.cross(Vec3::X);
        }
        (axis.normalize(), 0.0)
    } else {
        let sine = cross.length();
        (cross / sine, sine)
    };
    let rotate = |vector: Vec3| {
        vector * cosine + axis.cross(vector) * sine + axis * axis.dot(vector) * (1.0 - cosine)
    };
    [
        rotate(Vec3::X).into(),
        rotate(Vec3::Y).into(),
        rotate(Vec3::Z).into(),
    ]
}

fn top_row_luminance(image: &EnvironmentImage) -> f32 {
    let width = image.width as usize;
    let mut luminance = 0.0;
    for x in 0..width {
        let index = x * 4;
        luminance += image.rgba[index] * 0.2126
            + image.rgba[index + 1] * 0.7152
            + image.rgba[index + 2] * 0.0722;
    }
    luminance / width as f32
}

fn scale_image(image: &mut EnvironmentImage, scale: f32) {
    for (index, value) in image.rgba.iter_mut().enumerate() {
        if index % 4 != 3 {
            *value *= scale;
        }
    }
}

fn direction(u: f32, v: f32) -> Vec3 {
    let phi = (u - 0.5) * std::f32::consts::TAU;
    let theta = v * std::f32::consts::PI;
    Vec3::new(
        theta.sin() * phi.cos(),
        theta.cos(),
        theta.sin() * phi.sin(),
    )
}

fn sun(width: u32, height: u32, pixels: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
    let mut brightest = 0.0_f32;
    let mut index = 0_usize;
    for (i, pixel) in pixels.iter().enumerate() {
        let luminance = pixel[0] * 0.2126 + pixel[1] * 0.7152 + pixel[2] * 0.0722;
        if luminance > brightest {
            brightest = luminance;
            index = i;
        }
    }
    if brightest <= 0.0 {
        return ([0.0, 1.0, 0.0], [0.0; 3]);
    }
    let x = (index % width as usize) as f32;
    let y = (index / width as usize) as f32;
    let peak = direction((x + 0.5) / width as f32, (y + 0.5) / height as f32);
    integrate_sun(width, height, pixels, peak, 8.0_f32.to_radians().cos())
}

fn integrate_sun(
    width: u32,
    height: u32,
    pixels: &[[f32; 3]],
    peak: Vec3,
    limit: f32,
) -> ([f32; 3], [f32; 3]) {
    let (d_theta, d_phi) = texel_solid_angle(width, height);
    let mut energy = Vec3::ZERO;
    let mut color = Vec3::ZERO;
    for y in 0..height {
        let v = (y as f32 + 0.5) / height as f32;
        let theta = v * std::f32::consts::PI;
        let omega = theta.sin() * d_theta * d_phi;
        for x in 0..width {
            let sample = direction((x as f32 + 0.5) / width as f32, v);
            if sample.dot(peak) < limit {
                continue;
            }
            let pixel = Vec3::from(pixels[(y * width + x) as usize]);
            energy += sample * (pixel.dot(Vec3::new(0.2126, 0.7152, 0.0722)) * omega);
            color += pixel * sample.dot(peak).max(0.0) * omega;
        }
    }
    let direction = if energy.length_squared() > 1e-8 {
        energy.normalize()
    } else {
        peak.normalize()
    };
    (direction.into(), (color / std::f32::consts::PI).into())
}

fn texel_solid_angle(width: u32, height: u32) -> (f32, f32) {
    (
        std::f32::consts::PI / height as f32,
        std::f32::consts::TAU / width as f32,
    )
}

fn downsample(pixels: &[[f32; 3]], width: u32, height: u32, dw: u32, dh: u32) -> Vec<[f32; 3]> {
    let mut sum = vec![[0.0_f32; 3]; (dw * dh) as usize];
    let mut count = vec![0_u32; sum.len()];
    for y in 0..height {
        let oy = y * dh / height;
        for x in 0..width {
            let ox = x * dw / width;
            let index = (oy * dw + ox) as usize;
            let pixel = pixels[(y * width + x) as usize];
            for channel in 0..3 {
                sum[index][channel] += pixel[channel];
            }
            count[index] += 1;
        }
    }
    for (pixel, samples) in sum.iter_mut().zip(count) {
        let samples = samples.max(1) as f32;
        for channel in pixel.iter_mut() {
            *channel /= samples;
        }
    }
    sum
}

fn convolve(
    source: &[[f32; 3]],
    source_width: u32,
    source_height: u32,
    width: u32,
    height: u32,
    irradiance: bool,
    sun_direction: [f32; 3],
) -> EnvironmentImage {
    let sun = Vec3::from(sun_direction).normalize_or_zero();
    let sun_limit = 8.0_f32.to_radians().cos();
    let (d_theta, d_phi) = texel_solid_angle(source_width, source_height);
    let mut samples = Vec::with_capacity((source_width * source_height) as usize);
    for y in 0..source_height {
        let v = (y as f32 + 0.5) / source_height as f32;
        let theta = v * std::f32::consts::PI;
        let omega = theta.sin() * d_theta * d_phi;
        for x in 0..source_width {
            let direction = direction((x as f32 + 0.5) / source_width as f32, v);
            let pixel = Vec3::from(source[(y * source_width + x) as usize]);
            samples.push((direction, pixel, omega));
        }
    }
    let mut rgba = vec![0.0_f32; (width * height * 4) as usize];
    for y in 0..height {
        for x in 0..width {
            let axis = direction(
                (x as f32 + 0.5) / width as f32,
                (y as f32 + 0.5) / height as f32,
            );
            let mut color = Vec3::ZERO;
            let mut weight = 0.0_f32;
            for (sample, radiance, omega) in &samples {
                let cosine = axis.dot(*sample);
                if irradiance {
                    if cosine <= 0.0 || (sun.length_squared() > 0.0 && sample.dot(sun) > sun_limit)
                    {
                        continue;
                    }
                    let solid = cosine * omega;
                    color += *radiance * solid;
                    weight += solid;
                } else if cosine > 0.0 {
                    let lobe = cosine.powi(16) * omega;
                    color += *radiance * lobe;
                    weight += lobe;
                }
            }
            let value = if irradiance {
                color / std::f32::consts::PI
            } else if weight > 0.0 {
                (color / weight).clamp(Vec3::ZERO, Vec3::splat(8.0))
            } else {
                Vec3::ZERO
            };
            let index = ((y * width + x) * 4) as usize;
            rgba[index] = value.x;
            rgba[index + 1] = value.y;
            rgba[index + 2] = value.z;
            rgba[index + 3] = 1.0;
        }
    }
    EnvironmentImage {
        width,
        height,
        rgba,
    }
}

fn decode_radiance(bytes: &[u8]) -> BelfastResult<(u32, u32, Vec<[f32; 3]>)> {
    let header_end = bytes
        .windows(2)
        .position(|pair| pair == b"\n\n")
        .ok_or(BelfastError::InvalidRadianceHdr("missing header"))?;
    let resolution = bytes
        .get(header_end + 2..)
        .and_then(|rest| rest.iter().position(|byte| *byte == b'\n'))
        .ok_or(BelfastError::InvalidRadianceHdr("missing resolution"))?;
    let line = std::str::from_utf8(&bytes[header_end + 2..header_end + 2 + resolution])
        .map_err(|_| BelfastError::InvalidRadianceHdr("resolution is not text"))?;
    let (height, width, bottom_up) = parse_resolution(line)?;
    let mut cursor = header_end + 2 + resolution + 1;
    let mut pixels = vec![[0.0_f32; 3]; (width * height) as usize];
    for y in 0..height {
        let scanline = read_scanline(bytes, &mut cursor, width)?;
        for x in 0..width as usize {
            pixels[(y * width) as usize + x] = rgbe(scanline[x]);
        }
    }
    if bottom_up {
        for y in 0..height as usize / 2 {
            let other = height as usize - 1 - y;
            for x in 0..width as usize {
                pixels.swap(y * width as usize + x, other * width as usize + x);
            }
        }
    }
    Ok((width, height, pixels))
}

fn parse_resolution(line: &str) -> BelfastResult<(u32, u32, bool)> {
    let token: Vec<&str> = line.split_whitespace().collect();
    if token.len() != 4 {
        return Err(BelfastError::InvalidRadianceHdr("unsupported resolution"));
    }
    let (y_axis, y_text, x_axis, x_text) = (token[0], token[1], token[2], token[3]);
    let height: u32 = y_text
        .parse()
        .map_err(|_| BelfastError::InvalidRadianceHdr("invalid height"))?;
    let width: u32 = x_text
        .parse()
        .map_err(|_| BelfastError::InvalidRadianceHdr("invalid width"))?;
    if width == 0 || height == 0 || x_axis != "+X" || (y_axis != "-Y" && y_axis != "+Y") {
        return Err(BelfastError::InvalidRadianceHdr("unsupported resolution"));
    }
    Ok((height, width, y_axis == "+Y"))
}

fn read_scanline(bytes: &[u8], cursor: &mut usize, width: u32) -> BelfastResult<Vec<[u8; 4]>> {
    if *cursor + 4 <= bytes.len()
        && bytes[*cursor] == 2
        && bytes[*cursor + 1] == 2
        && bytes[*cursor + 2] < 128
    {
        let encoded = ((bytes[*cursor + 2] as u32) << 8) | bytes[*cursor + 3] as u32;
        if encoded != width {
            return Err(BelfastError::InvalidRadianceHdr("scanline width mismatch"));
        }
        *cursor += 4;
        let mut channels = Vec::with_capacity(4);
        for _ in 0..4 {
            channels.push(read_rle_channel(bytes, cursor, width as usize)?);
        }
        return Ok((0..width as usize)
            .map(|x| {
                [
                    channels[0][x],
                    channels[1][x],
                    channels[2][x],
                    channels[3][x],
                ]
            })
            .collect());
    }
    let end = cursor
        .checked_add(width as usize * 4)
        .filter(|end| *end <= bytes.len())
        .ok_or(BelfastError::InvalidRadianceHdr("truncated scanline"))?;
    let raw = bytes[*cursor..end].as_chunks::<4>().0.to_vec();
    *cursor = end;
    Ok(raw)
}

fn read_rle_channel(bytes: &[u8], cursor: &mut usize, width: usize) -> BelfastResult<Vec<u8>> {
    let mut row = Vec::with_capacity(width);
    while row.len() < width {
        let count = *bytes
            .get(*cursor)
            .ok_or(BelfastError::InvalidRadianceHdr("truncated channel"))?
            as usize;
        *cursor += 1;
        if count > 128 {
            let run = count - 128;
            let value = *bytes
                .get(*cursor)
                .ok_or(BelfastError::InvalidRadianceHdr("truncated run"))?;
            *cursor += 1;
            if row.len() + run > width {
                return Err(BelfastError::InvalidRadianceHdr("run exceeds width"));
            }
            row.extend(std::iter::repeat_n(value, run));
        } else if count == 0 {
            return Err(BelfastError::InvalidRadianceHdr("empty run"));
        } else {
            let end = cursor
                .checked_add(count)
                .filter(|end| *end <= bytes.len())
                .ok_or(BelfastError::InvalidRadianceHdr("truncated literals"))?;
            if row.len() + count > width {
                return Err(BelfastError::InvalidRadianceHdr("literals exceed width"));
            }
            row.extend_from_slice(&bytes[*cursor..end]);
            *cursor = end;
        }
    }
    Ok(row)
}

fn rgbe(pixel: [u8; 4]) -> [f32; 3] {
    if pixel[3] == 0 {
        return [0.0; 3];
    }
    // 2^(exponent - 128) / 256 = 2^(exponent - 136)
    let scale = 2.0_f32.powi(i32::from(pixel[3]) - 136);
    [
        f32::from(pixel[0]) * scale,
        f32::from(pixel[1]) * scale,
        f32::from(pixel[2]) * scale,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_carries_one_direction_onto_another() {
        let from = [0.2_f32, 0.9, -0.3];
        let to = [-0.4_f32, 0.2, 0.8];
        let columns = rotation_from_to(from, to);
        let from = Vec3::from(from).normalize();
        let to = Vec3::from(to).normalize();
        let rotated = Vec3::from(columns[0]) * from.x
            + Vec3::from(columns[1]) * from.y
            + Vec3::from(columns[2]) * from.z;
        assert!((rotated - to).length() < 1e-5);
    }

    #[test]
    fn rle_scanline_decodes_a_repeated_color() {
        let mut bytes = b"#?RADIANCE\n\n-Y 1 +X 8\n\x02\x02\x00\x08".to_vec();
        for value in [10_u8, 20, 30, 136] {
            bytes.extend_from_slice(&[128 + 8, value]);
        }
        let map = EnvironmentMap::from_radiance_hdr(&bytes).unwrap();
        assert!(map.sun_color.iter().all(|channel| channel.is_finite()));
        assert!(map.sun_direction[1].abs() <= 1.0);
    }

    #[test]
    fn raw_hdr_sun_matches_the_bright_texel() {
        let bytes = b"#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y 2 +X 1\n\
            \xff\xff\xff\x88\x00\x00\x00\x00";
        let map = EnvironmentMap::from_radiance_hdr(bytes).unwrap();
        assert!(map.sun_direction[0] > 0.5);
        assert!(map.sun_direction[1] > 0.5);
        assert!(map.sun_color[0] > 0.0);
        assert_eq!(map.irradiance.width, 64);
    }

    #[test]
    fn golden_gate_hdr_sun_is_above_the_horizon() {
        let bytes = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../experiments/mountain_fold/public/golden_gate_hills_2k.hdr"
        ))
        .expect("golden gate hdr");
        let map = EnvironmentMap::from_radiance_hdr(&bytes).unwrap();
        assert!(map.sun_direction[1] > 0.5, "{:?}", map.sun_direction);
        assert!(map.sun_color[0] > 0.5);
    }
}
