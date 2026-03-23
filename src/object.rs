use crate::Vertex;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ColoredObject {
    pub start_index: u32,
    pub num_indices: u16,
    pub transformation_index: u16,
}

pub struct TexturedObject<'a> {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub texture: &'a wgpu::Texture,
    pub transformation: glam::Mat4,
}
