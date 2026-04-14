use bytemuck::{Pod, Zeroable};
use glam::Vec2;
use image::{self, ImageReader, codecs::hdr::SIGNATURE, imageops::FilterType::Triangle};
use std::sync::Arc;
use wgpu::{naga::keywords::wgsl::RESERVED, util::RenderEncoder};

use crate::{GpuContext, Vertex, buffer::GpuBuffer};

pub struct Renderable {
    pub vertex_offset: u32,
    pub index_offset: u32,
    pub num_indices: u32,
    pub transformations: Vec<GPUTransform>, //each instance gets one transformation
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

pub struct ColoredObject {
    pub start_index: u32,
    pub num_indices: u32,
}

pub struct DynamicObject {
    pub start_index: u32,
    pub num_indices: u32,
    pub transformation_matrix: Vec2,
}

pub struct Material {
    pub bind_group: wgpu::BindGroup,
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,

    renderables: Vec<Renderable>,
    vertex_buffer: GpuBuffer,
    index_buffer: GpuBuffer,
    instance_buffer: GpuBuffer,
    next_vertex_offset: u32,
}

impl Material {
    pub fn new(
        image_path: &str,
        gpu: &Arc<GpuContext>,
        bind_group_layout: &wgpu::BindGroupLayout,
        diffuse_sampler: &wgpu::Sampler,
    ) -> Result<Self, image::ImageError> {
        let rgb_image = image::ImageReader::open(image_path)?.decode()?.to_rgba8();
        let dimensions = rgb_image.dimensions();
        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let diffuse_texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(image_path),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfoBase {
                texture: &diffuse_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgb_image,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(4 * dimensions.1),
            },
            texture_size,
        );

        let diffuse_texture_view =
            diffuse_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let diffuse_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(diffuse_sampler),
                },
            ],
        });

        Ok(Self {
            bind_group: diffuse_bind_group,
            texture: diffuse_texture,
            view: diffuse_texture_view,

            renderables: Vec::new(),
            vertex_buffer: GpuBuffer::new(gpu.clone(), wgpu::BufferUsages::VERTEX),
            index_buffer: GpuBuffer::new(gpu.clone(), wgpu::BufferUsages::INDEX),
            instance_buffer: GpuBuffer::new(gpu.clone(), wgpu::BufferUsages::VERTEX),
            next_vertex_offset: 0,
        })
    }

    pub fn create_renderable(&mut self, mesh: &[Vertex], indices: &[u16]) -> &Renderable {
        let new_renderable = Renderable {
            vertex_offset: self.index_buffer.bytes_used as u32 / size_of::<Vertex>() as u32,
            index_offset: self.vertex_buffer.bytes_used as u32 / size_of::<u16>() as u32,
            num_indices: indices.len() as u32,
            transformations: Vec::new(),
        };
        self.vertex_buffer.append(bytemuck::cast_slice(mesh));
        self.index_buffer.append(indices);
        self.renderables.push(new_renderable);
        self.renderables.last().unwrap()
    }

    pub fn add_instance(&mut self, transform: &glam::Affine2, renderable_id: u32) {
        let renderable = &mut self.renderables[renderable_id as usize];
        let size = renderable.transformations.len();
        renderable
            .transformations
            .push(GPUTransform::from(transform));
        self.instance_buffer.append(bytemuck::cast_slice(
            &renderable.transformations[size..size + 1],
        ));
    }

    pub fn render(&self, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_bind_group(0, Some(&self.bind_group), &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.buffer.slice(..));
        render_pass.set_index_buffer(
            self.index_buffer.buffer.slice(..),
            wgpu::IndexFormat::Uint16,
        );
        for renderable in &self.renderables {
            render_pass.draw_indexed(
                renderable.index_offset..(renderable.index_offset + renderable.num_indices),
                renderable.vertex_offset as i32,
                0..renderable.transformations.len() as u32,
            );
        }
    }
}
