//! Standard, uncompressed 3DGS PLY. Coordinates become Y-up via (x, -y, -z).
use glam::{Mat3, Quat, Vec3};

const FIELDS: [&str; 14] = [
    "x", "y", "z", "f_dc_0", "f_dc_1", "f_dc_2", "opacity", "scale_0", "scale_1", "scale_2",
    "rot_0", "rot_1", "rot_2", "rot_3",
];
pub const ALPHA_CUTOFF: f32 = 1.0 / 255.0;

#[derive(Debug)]
pub struct GaussianCloud {
    pub source_count: u32,
    /// XYZ and covariance normalization (largest scale squared).
    pub positions: Vec<[f32; 4]>,
    /// Six normalized covariance half-floats, then RGBA8.
    pub attributes: Vec<[u32; 4]>,
    /// XYZ and conservative three-sigma radius, transferred to the Rust sort worker.
    pub sort_positions: Vec<[f32; 4]>,
}

impl GaussianCloud {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let header_limit = bytes.len().min(64 * 1024);
        let mut end = 0;
        for line in bytes[..header_limit].split_inclusive(|&b| b == b'\n') {
            end += line.len();
            if line == b"end_header\n" || line == b"end_header\r\n" {
                break;
            }
        }
        let header = std::str::from_utf8(&bytes[..end]).map_err(|_| "Invalid PLY header")?;
        if !header.ends_with("end_header\n") && !header.ends_with("end_header\r\n") {
            return Err("Missing PLY end_header (64 KiB header limit)".into());
        }
        let mut lines = header.lines();
        if lines.next() != Some("ply") || lines.next() != Some("format binary_little_endian 1.0") {
            return Err("Expected binary_little_endian 1.0 PLY".into());
        }
        let mut count = None;
        let mut in_vertex = false;
        let mut saw_element = false;
        let mut stride = 0usize;
        let mut offsets = [None; 14];
        for line in lines {
            let parts: Vec<_> = line.split_whitespace().collect();
            match parts.as_slice() {
                ["element", name, number] => {
                    if *name == "vertex" {
                        if saw_element {
                            return Err(
                                "PLY vertex must be the first and only vertex element".into()
                            );
                        }
                        count = Some(number.parse::<u32>().map_err(|_| "Invalid vertex count")?);
                        in_vertex = true;
                    } else {
                        in_vertex = false;
                    }
                    saw_element = true;
                }
                ["property", ty, name] if in_vertex => {
                    let size = match *ty {
                        "char" | "uchar" | "int8" | "uint8" => 1,
                        "short" | "ushort" | "int16" | "uint16" => 2,
                        "float" | "float32" | "int" | "uint" | "int32" | "uint32" => 4,
                        "double" | "float64" => 8,
                        _ => return Err(format!("Unsupported PLY property type: {ty}")),
                    };
                    if let Some(index) = FIELDS.iter().position(|field| field == name) {
                        if !matches!(*ty, "float" | "float32") || offsets[index].is_some() {
                            return Err(format!("Expected one float property for {name}"));
                        }
                        offsets[index] = Some(stride);
                    }
                    if name.starts_with("f_rest_") {
                        return Err("This renderer supports SH DC only; higher-order f_rest coefficients are not supported".into());
                    }
                    stride += size;
                }
                ["property", ..] if in_vertex => {
                    return Err("Unsupported vertex property (lists are not supported)".into())
                }
                _ => {}
            }
        }
        let count = count.filter(|&n| n > 0).ok_or("PLY contains no vertices")?;
        let offsets: Vec<usize> = offsets
            .into_iter()
            .enumerate()
            .map(|(i, v)| v.ok_or_else(|| format!("Missing Gaussian property {}", FIELDS[i])))
            .collect::<Result<_, _>>()?;
        let body_len = (count as usize)
            .checked_mul(stride)
            .ok_or("PLY vertex data too large")?;
        if bytes.len().saturating_sub(end) < body_len {
            return Err("PLY vertex data is truncated".into());
        }
        // Keep peak allocations proportional to input, including rejected records.
        let mut cloud = Self {
            source_count: count,
            positions: Vec::with_capacity(count as usize),
            attributes: Vec::with_capacity(count as usize),
            sort_positions: Vec::with_capacity(count as usize),
        };
        let flip = Mat3::from_diagonal(Vec3::new(1., -1., -1.));
        for (index, record) in bytes[end..end + body_len].chunks_exact(stride).enumerate() {
            let v: [f32; 14] = std::array::from_fn(|i| {
                f32::from_le_bytes(record[offsets[i]..offsets[i] + 4].try_into().unwrap())
            });
            if !v.iter().all(|v| v.is_finite()) {
                return Err(format!("Non-finite Gaussian attribute at vertex {index}"));
            }
            let alpha = 1.0 / (1.0 + (-v[6]).exp());
            if alpha < ALPHA_CUTOFF {
                continue;
            }
            let scale = Vec3::new(v[7].exp(), v[8].exp(), v[9].exp());
            let max_scale = scale.max_element();
            let variance = max_scale * max_scale;
            if !variance.is_finite() || variance <= 0.0 {
                return Err(format!("Invalid Gaussian scale at vertex {index}"));
            }
            let quat = Quat::from_xyzw(v[11], v[12], v[13], v[10]);
            if !quat.length_squared().is_finite() || quat.length_squared() < 1e-12 {
                return Err(format!("Invalid Gaussian quaternion at vertex {index}"));
            }
            let rotation = flip * Mat3::from_quat(quat.normalize());
            let normalized_scale = scale / max_scale;
            let basis = rotation * Mat3::from_diagonal(normalized_scale);
            let covariance = basis * basis.transpose();
            let position = [v[0], -v[1], -v[2]];
            let color: [u8; 3] = std::array::from_fn(|i| {
                ((0.5 + 0.282_094_8 * v[3 + i]).clamp(0., 1.) * 255.).round() as u8
            });
            cloud
                .positions
                .push([position[0], position[1], position[2], variance]);
            cloud
                .sort_positions
                .push([position[0], position[1], position[2], 3.0 * max_scale]);
            cloud.attributes.push([
                pack_half(covariance.x_axis.x, covariance.y_axis.x),
                pack_half(covariance.z_axis.x, covariance.y_axis.y),
                pack_half(covariance.z_axis.y, covariance.z_axis.z),
                u32::from_le_bytes([color[0], color[1], color[2], (alpha * 255.).round() as u8]),
            ]);
        }
        if cloud.positions.is_empty() {
            return Err("PLY contains no visible Gaussians".into());
        }
        Ok(cloud)
    }
}

fn pack_half(a: f32, b: f32) -> u32 {
    u32::from(half::f16::from_f32(a).to_bits())
        | (u32::from(half::f16::from_f32(b).to_bits()) << 16)
}
