use crate::object::Material;
use std::sync::Arc;

use wgpu::util::{BufferInitDescriptor, DeviceExt};

use crate::{
    GpuContext,
    buffer::GpuBuffer,
    object::{ColoredObject, GPUTransform, Renderable},
    vertex::Vertex,
};

pub struct Scene {
    //for colored vertices only
    pub static_vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    static_vb: GpuBuffer,
    static_ib: GpuBuffer,
    static_transform_buffer: wgpu::Buffer,

    pub materials: Vec<Material>,

    pub textures: Vec<wgpu::Texture>,

    gpu: Arc<GpuContext>,
}

impl Scene {
    pub fn new(gpu: Arc<GpuContext>) -> Self {
        let ret = Self {
            static_vertices: Vec::new(),
            indices: Vec::new(),
            textures: Vec::new(),
            static_vb: GpuBuffer::new(gpu.clone(), wgpu::BufferUsages::VERTEX),
            static_ib: GpuBuffer::new(gpu.clone(), wgpu::BufferUsages::INDEX),
            static_transform_buffer: gpu.device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&[GPUTransform::from(&glam::Affine2::IDENTITY)]),
                usage: wgpu::BufferUsages::VERTEX,
            }),
            materials: Vec::new(),
            gpu,
        };
        ret
    }

    pub fn add_static_object(
        &mut self,
        new_vertices: &[Vertex],
        new_indices: &[u16],
    ) -> ColoredObject {
        let start_index = self.indices.len() as u32;
        self.static_vertices.extend_from_slice(new_vertices);
        self.static_vb.append(bytemuck::cast_slice(new_vertices));

        self.indices.extend_from_slice(new_indices);
        self.static_ib.append(bytemuck::cast_slice(new_indices));
        ColoredObject {
            start_index,
            num_indices: new_indices.len() as u32,
        }
    }

    pub fn render(&self, render_pass: &mut wgpu::RenderPass) {
        //draw static geometry first
        render_pass.set_vertex_buffer(0, self.static_vb.buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.static_transform_buffer.slice(..));
        render_pass.set_index_buffer(self.static_ib.buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..(self.static_ib.bytes_used / 2) as u32, 0, 0..1);

        //draw each renderable
    }
}
