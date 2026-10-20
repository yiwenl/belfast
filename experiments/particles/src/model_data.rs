//! Static, embedded GLB geometry. Node transforms are baked once at load time.
use glam::{Mat3, Mat4, Vec3};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

pub struct ImageData {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub struct PrimitiveData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub base_color: ImageData,
    pub normal_map: ImageData,
    pub base_factor: [f32; 4],
    pub normal_scale: f32,
    pub double_sided: bool,
}

pub struct ModelData {
    pub primitives: Vec<PrimitiveData>,
}

impl ModelData {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 12
            || &bytes[..4] != b"glTF"
            || u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize != bytes.len()
        {
            return Err("Expected a complete binary GLB file".into());
        }
        let gltf = gltf::Gltf::from_slice(bytes).map_err(|e| format!("Invalid GLB: {e}"))?;
        let blob = gltf.blob.as_deref().ok_or("GLB has no embedded buffer")?;
        if gltf.extensions_required().next().is_some() {
            return Err("Required glTF extensions are not supported".into());
        }
        for buffer in gltf.buffers() {
            if !matches!(buffer.source(), gltf::buffer::Source::Bin) || buffer.length() > blob.len()
            {
                return Err("Only complete embedded GLB buffers are supported".into());
            }
        }
        for view in gltf.views() {
            if view
                .offset()
                .checked_add(view.length())
                .is_none_or(|end| end > blob.len())
            {
                return Err("Truncated GLB buffer view".into());
            }
        }
        for accessor in gltf.accessors() {
            if accessor.sparse().is_some() {
                return Err("Sparse GLB accessors are not supported".into());
            }
            let view = accessor.view().ok_or("Missing GLB accessor buffer view")?;
            let stride = view.stride().unwrap_or(accessor.size());
            let end = accessor
                .count()
                .checked_sub(1)
                .and_then(|n| n.checked_mul(stride))
                .and_then(|n| n.checked_add(accessor.offset()))
                .and_then(|n| n.checked_add(accessor.size()));
            if stride < accessor.size() || end.is_none_or(|end| end > view.length()) {
                return Err("Invalid GLB accessor range".into());
            }
        }
        let scene = gltf
            .default_scene()
            .or_else(|| gltf.scenes().next())
            .ok_or("GLB has no scene")?;
        let mut data = Self {
            primitives: Vec::new(),
        };
        for node in scene.nodes() {
            data.read_node(node, Mat4::IDENTITY, blob, 0)?;
        }
        if data.primitives.is_empty() {
            return Err("GLB scene has no triangle meshes".into());
        }
        Ok(data)
    }

    fn read_node(
        &mut self,
        node: gltf::Node<'_>,
        parent: Mat4,
        blob: &[u8],
        depth: usize,
    ) -> Result<(), String> {
        if depth > 128 || node.skin().is_some() {
            return Err("GLB hierarchy is too deep or contains an unsupported skin".into());
        }
        let transform = parent * Mat4::from_cols_array_2d(&node.transform().matrix());
        let normals = Mat3::from_mat4(transform).inverse().transpose();
        if !transform.is_finite() || !normals.is_finite() {
            return Err("GLB contains a non-invertible node transform".into());
        }
        if let Some(mesh) = node.mesh() {
            for primitive in mesh.primitives() {
                let material = primitive.material();
                if primitive.mode() != gltf::mesh::Mode::Triangles
                    || primitive.morph_targets().next().is_some()
                    || material.alpha_mode() != gltf::material::AlphaMode::Opaque
                {
                    return Err("Only static, opaque GLB triangle meshes are supported".into());
                }
                validate_primitive_accessors(&primitive)?;
                let reader = primitive.reader(|_| Some(blob));
                let positions: Vec<_> = reader
                    .read_positions()
                    .ok_or("GLB mesh needs positions")?
                    .collect();
                let source_normals: Vec<_> = reader
                    .read_normals()
                    .ok_or("GLB mesh needs normals")?
                    .collect();
                let uv: Vec<_> = reader
                    .read_tex_coords(0)
                    .map(|v| v.into_f32().collect())
                    .unwrap_or_else(|| vec![[0., 0.]; positions.len()]);
                if positions.is_empty()
                    || source_normals.len() != positions.len()
                    || uv.len() != positions.len()
                {
                    return Err("GLB vertex attribute counts do not match".into());
                }
                let mut indices: Vec<_> = reader
                    .read_indices()
                    .map(|v| v.into_u32().collect())
                    .unwrap_or_else(|| (0..positions.len() as u32).collect());
                if indices.is_empty()
                    || !indices.len().is_multiple_of(3)
                    || indices.iter().any(|&i| i as usize >= positions.len())
                {
                    return Err("GLB contains invalid triangle indices".into());
                }
                // A reflected node reverses winding, even after baking its normal matrix.
                if transform.determinant() < 0. {
                    for triangle in indices.chunks_exact_mut(3) {
                        triangle.swap(1, 2);
                    }
                }
                let mut vertices = Vec::with_capacity(positions.len());
                for ((p, n), uv) in positions.into_iter().zip(source_normals).zip(uv) {
                    let position = transform.transform_point3(Vec3::from(p));
                    let normal = (normals * Vec3::from(n)).normalize_or_zero();
                    if !position.is_finite()
                        || normal.length_squared() < 0.5
                        || !uv.iter().all(|v| v.is_finite())
                    {
                        return Err("GLB contains invalid vertex attributes".into());
                    }
                    vertices.push(Vertex {
                        position: position.to_array(),
                        normal: normal.to_array(),
                        uv,
                    });
                }
                let pbr = material.pbr_metallic_roughness();
                let base_color = match pbr.base_color_texture() {
                    Some(info) => {
                        if info.tex_coord() != 0 || reader.read_tex_coords(0).is_none() {
                            return Err("Base color needs UV0".into());
                        }
                        decode_image(info.texture().source(), blob)?
                    }
                    None => solid_image([255; 4]),
                };
                let (normal_map, normal_scale) = match material.normal_texture() {
                    Some(info) => {
                        if info.tex_coord() != 0 || reader.read_tex_coords(0).is_none() {
                            return Err("Normal map needs UV0".into());
                        }
                        (decode_image(info.texture().source(), blob)?, info.scale())
                    }
                    None => (solid_image([128, 128, 255, 255]), 0.),
                };
                self.primitives.push(PrimitiveData {
                    vertices,
                    indices,
                    base_color,
                    normal_map,
                    normal_scale,
                    base_factor: pbr.base_color_factor(),
                    double_sided: material.double_sided(),
                });
            }
        }
        for child in node.children() {
            self.read_node(child, transform, blob, depth + 1)?;
        }
        Ok(())
    }

    pub fn bounds(&self) -> (Vec3, Vec3) {
        self.primitives.iter().flat_map(|p| &p.vertices).fold(
            (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
            |(min, max), v| {
                (
                    min.min(Vec3::from(v.position)),
                    max.max(Vec3::from(v.position)),
                )
            },
        )
    }

    /// Fit uniformly, placing the horizontal bounds center and lowest vertex on `base`.
    pub fn placement_transform(&self, base: Vec3, height: f32) -> Result<Mat4, String> {
        let (min, max) = self.bounds();
        if !base.is_finite()
            || !height.is_finite()
            || height <= 0.
            || !min.is_finite()
            || max.y - min.y <= 1e-6
        {
            return Err("Model placement needs finite bounds and a positive height".into());
        }
        let scale = height / (max.y - min.y);
        let origin = Vec3::new((min.x + max.x) * 0.5, min.y, (min.z + max.z) * 0.5);
        Ok(Mat4::from_scale_rotation_translation(
            Vec3::splat(scale),
            glam::Quat::IDENTITY,
            base - origin * scale,
        ))
    }

    pub fn place(&mut self, base: Vec3, height: f32) -> Result<(), String> {
        let transform = self.placement_transform(base, height)?;
        for vertex in self.primitives.iter_mut().flat_map(|p| &mut p.vertices) {
            vertex.position = transform
                .transform_point3(vertex.position.into())
                .to_array();
        }
        Ok(())
    }
}

fn solid_image(rgba: [u8; 4]) -> ImageData {
    ImageData {
        width: 1,
        height: 1,
        rgba: rgba.to_vec(),
    }
}

// gltf validates enum values, but its utility readers assume each semantic uses
// the correct component/dimension combination and can panic if it does not.
fn validate_primitive_accessors(primitive: &gltf::Primitive<'_>) -> Result<(), String> {
    use gltf::accessor::{DataType, Dimensions};
    for semantic in [gltf::Semantic::Positions, gltf::Semantic::Normals] {
        let accessor = primitive
            .get(&semantic)
            .ok_or("GLB mesh needs positions and normals")?;
        if accessor.data_type() != DataType::F32
            || accessor.dimensions() != Dimensions::Vec3
            || accessor.normalized()
        {
            return Err("GLB positions and normals must be unnormalized float VEC3".into());
        }
    }
    if let Some(accessor) = primitive.get(&gltf::Semantic::TexCoords(0)) {
        let valid_type = match accessor.data_type() {
            DataType::F32 => !accessor.normalized(),
            DataType::U8 | DataType::U16 => accessor.normalized(),
            _ => false,
        };
        if accessor.dimensions() != Dimensions::Vec2 || !valid_type {
            return Err(
                "GLB UV0 must be VEC2 with float or normalized unsigned byte/short components"
                    .into(),
            );
        }
    }
    if let Some(accessor) = primitive.indices() {
        if accessor.dimensions() != Dimensions::Scalar
            || accessor.normalized()
            || !matches!(
                accessor.data_type(),
                DataType::U8 | DataType::U16 | DataType::U32
            )
        {
            return Err("GLB indices must be unnormalized unsigned scalars".into());
        }
    }
    Ok(())
}

fn decode_image(image: gltf::Image<'_>, blob: &[u8]) -> Result<ImageData, String> {
    let gltf::image::Source::View { view, mime_type } = image.source() else {
        return Err("GLB textures must be embedded".into());
    };
    let format = match mime_type {
        "image/png" => image::ImageFormat::Png,
        "image/jpeg" => image::ImageFormat::Jpeg,
        _ => return Err("GLB texture must be PNG or JPEG".into()),
    };
    let mut reader = image::ImageReader::with_format(
        std::io::Cursor::new(&blob[view.offset()..view.offset() + view.length()]),
        format,
    );
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    let decoded = reader
        .decode()
        .map_err(|e| format!("GLB texture: {e}"))?
        .into_rgba8();
    Ok(ImageData {
        width: decoded.width(),
        height: decoded.height(),
        rgba: decoded.into_raw(),
    })
}
