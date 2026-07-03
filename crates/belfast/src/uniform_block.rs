use std::collections::BTreeMap;

use crate::{BelfastError, BelfastResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UniformFieldType {
    F32,
    U32,
    Vec2F,
    Vec3F,
    Vec4F,
    Mat4x4F,
}

#[derive(Clone, Copy, Debug)]
struct TypeSpec {
    alignment: usize,
    storage_byte_size: usize,
    value_float_count: usize,
    name: &'static str,
}

#[derive(Clone, Debug)]
struct UniformFieldMeta {
    field_type: UniformFieldType,
    float_offset: usize,
    value_float_count: usize,
}

#[derive(Clone, Debug)]
pub struct UniformBlock {
    byte_size: usize,
    float_count: usize,
    data: Vec<f32>,
    uint_data: Vec<u32>,
    fields: BTreeMap<String, UniformFieldMeta>,
}

impl UniformBlock {
    pub fn create<const N: usize>(schema: [(&str, UniformFieldType); N]) -> BelfastResult<Self> {
        let mut byte_offset = 0;
        let mut fields = BTreeMap::new();

        for (name, field_type) in schema {
            let spec = type_spec(field_type);
            byte_offset = align_to(byte_offset, spec.alignment);
            fields.insert(
                name.to_string(),
                UniformFieldMeta {
                    field_type,
                    float_offset: byte_offset / 4,
                    value_float_count: spec.value_float_count,
                },
            );
            byte_offset += spec.storage_byte_size;
        }

        let float_count = byte_offset / 4;
        Ok(Self {
            byte_size: byte_offset,
            float_count,
            data: vec![0.0; float_count],
            uint_data: vec![0; float_count],
            fields,
        })
    }

    pub fn byte_size(&self) -> usize {
        self.byte_size
    }

    pub fn float_count(&self) -> usize {
        self.float_count
    }

    pub fn f32_data(&self) -> &[f32] {
        &self.data
    }

    pub fn u32_data(&self) -> &[u32] {
        &self.uint_data
    }

    pub fn bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.data)
    }

    pub fn get_offset(&self, name: &str) -> BelfastResult<usize> {
        Ok(self.field(name)?.float_offset)
    }

    pub fn set_f32(&mut self, name: &str, value: f32) -> BelfastResult<&mut Self> {
        let field = self.field(name)?.clone();
        if field.field_type != UniformFieldType::F32 {
            return Err(type_mismatch(name, type_spec(field.field_type).name, "f32"));
        }
        self.data[field.float_offset] = value;
        self.uint_data[field.float_offset] = value.to_bits();
        Ok(self)
    }

    pub fn set_u32(&mut self, name: &str, value: u32) -> BelfastResult<&mut Self> {
        let field = self.field(name)?.clone();
        if field.field_type != UniformFieldType::U32 {
            return Err(type_mismatch(name, type_spec(field.field_type).name, "u32"));
        }
        self.uint_data[field.float_offset] = value;
        self.data[field.float_offset] = f32::from_bits(value);
        Ok(self)
    }

    pub fn set_f32_slice(&mut self, name: &str, value: &[f32]) -> BelfastResult<&mut Self> {
        let field = self.field(name)?.clone();
        if field.field_type == UniformFieldType::U32 {
            return Err(type_mismatch(name, "u32", "float slice"));
        }
        if field.field_type == UniformFieldType::F32 {
            if value.is_empty() {
                return Err(value_too_short(name, 1, 0));
            }
            return self.set_f32(name, value[0]);
        }
        if value.len() < field.value_float_count {
            return Err(value_too_short(name, field.value_float_count, value.len()));
        }

        let offset = field.float_offset;
        for (index, item) in value.iter().take(field.value_float_count).enumerate() {
            self.data[offset + index] = *item;
            self.uint_data[offset + index] = item.to_bits();
        }
        if field.field_type == UniformFieldType::Vec3F {
            self.data[offset + 3] = 0.0;
            self.uint_data[offset + 3] = 0;
        }
        Ok(self)
    }

    fn field(&self, name: &str) -> BelfastResult<&UniformFieldMeta> {
        self.fields
            .get(name)
            .ok_or_else(|| BelfastError::UnknownUniformField(name.to_string()))
    }
}

fn align_to(value: usize, alignment: usize) -> usize {
    let remainder = value % alignment;
    if remainder == 0 {
        value
    } else {
        value + alignment - remainder
    }
}

fn type_spec(field_type: UniformFieldType) -> TypeSpec {
    match field_type {
        UniformFieldType::F32 => TypeSpec {
            alignment: 4,
            storage_byte_size: 4,
            value_float_count: 1,
            name: "f32",
        },
        UniformFieldType::U32 => TypeSpec {
            alignment: 4,
            storage_byte_size: 4,
            value_float_count: 1,
            name: "u32",
        },
        UniformFieldType::Vec2F => TypeSpec {
            alignment: 8,
            storage_byte_size: 8,
            value_float_count: 2,
            name: "vec2f",
        },
        UniformFieldType::Vec3F => TypeSpec {
            alignment: 16,
            storage_byte_size: 16,
            value_float_count: 3,
            name: "vec3f",
        },
        UniformFieldType::Vec4F => TypeSpec {
            alignment: 16,
            storage_byte_size: 16,
            value_float_count: 4,
            name: "vec4f",
        },
        UniformFieldType::Mat4x4F => TypeSpec {
            alignment: 16,
            storage_byte_size: 64,
            value_float_count: 16,
            name: "mat4x4f",
        },
    }
}

fn type_mismatch(name: &str, expected: &'static str, actual: &'static str) -> BelfastError {
    BelfastError::UniformTypeMismatch {
        name: name.to_string(),
        expected,
        actual,
    }
}

fn value_too_short(name: &str, expected: usize, actual: usize) -> BelfastError {
    BelfastError::UniformValueTooShort {
        name: name.to_string(),
        expected,
        actual,
    }
}
