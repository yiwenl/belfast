//! COLMAP binary PLY positions and sRGB vertex colors, in the original world frame.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Point {
    pub position: [f32; 3],
    pub color: [u8; 4],
}

#[derive(Debug)]
pub struct PointCloud {
    pub points: Vec<Point>,
}

impl PointCloud {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let mut end = 0;
        let mut found_end = false;
        for line in bytes[..bytes.len().min(65_536)].split_inclusive(|&b| b == b'\n') {
            end += line.len();
            if line == b"end_header\n" || line == b"end_header\r\n" {
                found_end = true;
                break;
            }
        }
        if !found_end {
            return Err("PLY header is missing end_header (64 KiB limit)".into());
        }
        let header = std::str::from_utf8(&bytes[..end]).map_err(|_| "Invalid PLY header")?;
        let mut lines = header.lines();
        if lines.next() != Some("ply") || lines.next() != Some("format binary_little_endian 1.0") {
            return Err("Expected binary_little_endian 1.0 PLY".into());
        }
        let fields = ["x", "y", "z", "red", "green", "blue"];
        let mut offsets = [None; 6];
        let mut stride = 0usize;
        let mut count = None;
        let mut in_vertex = false;
        let mut saw_element = false;
        for line in lines {
            let words: Vec<_> = line.split_whitespace().collect();
            match words.as_slice() {
                ["element", name, number] => {
                    in_vertex = *name == "vertex";
                    if in_vertex {
                        if saw_element {
                            return Err("PLY vertices must be the first element".into());
                        }
                        count = Some(
                            number
                                .parse::<u32>()
                                .map_err(|_| "Invalid PLY vertex count")?,
                        );
                    }
                    saw_element = true;
                }
                ["property", ty, name] if in_vertex => {
                    let size = match *ty {
                        "char" | "uchar" | "int8" | "uint8" => 1,
                        "short" | "ushort" | "int16" | "uint16" => 2,
                        "int" | "uint" | "float" | "int32" | "uint32" | "float32" => 4,
                        "double" | "float64" => 8,
                        _ => return Err(format!("Unsupported PLY property type: {ty}")),
                    };
                    if let Some(i) = fields.iter().position(|field| field == name) {
                        let valid = if i < 3 {
                            matches!(*ty, "float" | "float32")
                        } else {
                            matches!(*ty, "uchar" | "uint8")
                        };
                        if !valid || offsets[i].is_some() {
                            return Err(format!("Invalid or duplicate PLY property: {name}"));
                        }
                        offsets[i] = Some(stride);
                    }
                    stride = stride.checked_add(size).ok_or("PLY stride overflow")?;
                }
                ["property", ..] if in_vertex => {
                    return Err("PLY vertex lists are unsupported".into())
                }
                _ => {}
            }
        }
        let count = count.filter(|&n| n > 0).ok_or("PLY has no vertices")? as usize;
        let offsets = offsets
            .into_iter()
            .enumerate()
            .map(|(i, value)| value.ok_or_else(|| format!("Missing PLY property: {}", fields[i])))
            .collect::<Result<Vec<_>, _>>()?;
        let length = count
            .checked_mul(stride)
            .ok_or("PLY vertex data too large")?;
        if bytes.len() - end < length {
            return Err("PLY vertex data is truncated".into());
        }
        let mut points = Vec::new();
        points
            .try_reserve_exact(count)
            .map_err(|_| "Not enough memory for this point cloud")?;
        for record in bytes[end..end + length].chunks_exact(stride) {
            let position = std::array::from_fn(|i| {
                f32::from_le_bytes(record[offsets[i]..offsets[i] + 4].try_into().unwrap())
            });
            if !position.iter().all(|v| v.is_finite()) {
                return Err("PLY positions must be finite".into());
            }
            points.push(Point {
                position,
                color: [
                    record[offsets[3]],
                    record[offsets[4]],
                    record[offsets[5]],
                    255,
                ],
            });
        }
        Ok(Self { points })
    }
}
