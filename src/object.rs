use bytemuck::{Pod, Zeroable};
use glam::Vec2;

use crate::Vertex;

pub struct Renderable {
    pub vertex_offset: u32,
    pub index_offset: u32,
    pub num_indices: u32,
    pub transformations: Vec<glam::Affine2>,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GPUTransform {
    col0: [f32; 2],
    col1: [f32; 2],
    translation: [f32; 2],
}

impl GPUTransform {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 2,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: size_of::<Vec2>() as u64,
                    shader_location: 3,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 2 * size_of::<Vec2>() as u64,
                    shader_location: 4,
                },
            ],
        }
    }
}

impl From<glam::Affine2> for GPUTransform {
    fn from(src: glam::Affine2) -> Self {
        let mat = src.matrix2;
        let t = src.translation;
        Self {
            col0: mat.col(0).into(),
            col1: mat.col(1).into(),
            translation: t.into(),
        }
    }
}

pub struct ColoredObject {
    pub start_index: u32,
    pub num_indices: u32,
}

pub struct DynamicObject {
    pub start_index: u32,
    pub num_indices: u32,
    pub transformation_matrix: Vec2,
}

pub struct TexturedObject<'a> {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub texture: &'a wgpu::Texture,
    pub transformation: glam::Mat4,
}
