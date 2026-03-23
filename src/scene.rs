use std::cmp::max;
use std::sync::Arc;

use wgpu::include_spirv_raw;

use crate::{
    GpuContext,
    buffer::GpuBuffer,
    object::{ColoredObject, TexturedObject},
    vertex::Vertex,
};

const INITIAL_BUFFER_SIZE: u64 = 256;

fn reallocate_buffer(gpu: &GpuContext, buffer: &mut wgpu::Buffer, data: &[u8]) {
    let new_size = max(
        data.len().next_power_of_two() as u64,
        (buffer.size() + 1).next_power_of_two(),
    );
    let buffer_usage = buffer.usage();
    *buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: new_size,
        usage: buffer_usage,
        mapped_at_creation: false,
    });
    gpu.queue.write_buffer(buffer, 0, data);
}

pub struct Scene {
    //for colored vertices only
    pub static_vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub transformation_matrices: Vec<glam::Mat4>,

    pub textures: Vec<wgpu::Texture>,

    pub vertex_buffer: GpuBuffer,
    pub index_buffer: GpuBuffer,

    gpu: Arc<GpuContext>,
}

impl Scene {
    pub fn new(gpu: Arc<GpuContext>) -> Self {
        Self {
            static_vertices: Vec::new(),
            indices: Vec::new(),
            transformation_matrices: Vec::new(),
            textures: Vec::new(),
            vertex_buffer: GpuBuffer::new(gpu.clone(), wgpu::BufferUsages::VERTEX),
            index_buffer: GpuBuffer::new(gpu.clone(), wgpu::BufferUsages::INDEX),
            gpu,
        }
    }

    pub fn add_object(&mut self, new_vertices: &[Vertex], new_indices: &[u16]) -> ColoredObject {
        let start_index = self.indices.len() as u32;
        self.static_vertices.extend_from_slice(new_vertices);
        self.vertex_buffer
            .append(bytemuck::cast_slice(new_vertices));

        self.indices.extend_from_slice(new_indices);
        self.index_buffer.append(bytemuck::cast_slice(new_indices));
        ColoredObject {
            start_index,
            num_indices: new_indices.len() as u16,
            transformation_index: 0,
        }
    }
}
