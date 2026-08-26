use bytemuck::{Pod, Zeroable};
use glam::{Mat2, Vec2};

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

    pub fn move_absolute(&mut self, position: Vec2) {
        self.translation = position.to_array();
    }

    pub fn move_relative(&mut self, offset: Vec2) {
        self.translation[0] += offset.x;
        self.translation[1] += offset.y;
    }

    pub fn reset_transform(&mut self) {
        self.col0 = [1.0, 0.0];
        self.col1 = [0.0, 1.0];
    }

    pub fn apply_transform(&mut self, transform: &Mat2) {
        let t_row0 = transform.row(0);
        let t_row1 = transform.row(1);
        self.col0[0] = self.col0[0] * t_row0[0] + self.col0[1] * t_row0[1];
        self.col0[1] = self.col0[0] * t_row1[0] + self.col0[1] * t_row1[1];
        self.col1[0] = self.col1[0] * t_row0[0] + self.col1[1] * t_row0[1];
        self.col1[1] = self.col1[0] * t_row1[0] + self.col1[1] * t_row1[1];
    }
}

impl From<&glam::Affine2> for GPUTransform {
    fn from(src: &glam::Affine2) -> Self {
        let mat = src.matrix2;
        let t = src.translation;
        Self {
            col0: mat.col(0).into(),
            col1: mat.col(1).into(),
            translation: t.into(),
        }
    }
}
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub color: [f32; 3],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ], //attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct TextureVertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
}
impl TextureVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TextureVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}
