use crate::GPUTransform;
use image;
use std::sync::Arc;

use crate::{GpuContext, buffer::GpuBuffer, vertex::TextureVertex};

pub struct Mesh {
    pub vertex_offset: u32,
    pub index_offset: u32,
    pub num_indices: u32,
    pub transformations: Vec<GPUTransform>, //each instance gets one transformation
}

pub struct ColoredObject {
    pub start_index: u32,
    pub num_indices: u32,
}

pub struct Material {
    pub bind_group: wgpu::BindGroup,
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,

    meshes: Vec<Mesh>,
    vertex_buffer: GpuBuffer,
    index_buffer: GpuBuffer,
    instance_buffer: GpuBuffer,
}

impl Material {
    pub(crate) fn new(
        image_path: &str,
        gpu: &Arc<GpuContext>,
        bind_group_layout: &wgpu::BindGroupLayout,
        diffuse_sampler: &wgpu::Sampler,
    ) -> Result<Material, image::ImageError> {
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
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
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

            meshes: Vec::new(),
            vertex_buffer: GpuBuffer::new(gpu.clone(), wgpu::BufferUsages::VERTEX),
            index_buffer: GpuBuffer::new(gpu.clone(), wgpu::BufferUsages::INDEX),
            instance_buffer: GpuBuffer::new(gpu.clone(), wgpu::BufferUsages::VERTEX),
        })
    }

    pub fn create_renderable(&mut self, mesh: &[TextureVertex], indices: &[u16]) -> usize {
        let new_renderable = Mesh {
            vertex_offset: self.index_buffer.bytes_used as u32 / size_of::<TextureVertex>() as u32,
            index_offset: self.vertex_buffer.bytes_used as u32 / size_of::<u16>() as u32,
            num_indices: indices.len() as u32,
            transformations: Vec::new(),
        };
        self.vertex_buffer.append(bytemuck::cast_slice(mesh));
        self.index_buffer.append(indices);
        self.meshes.push(new_renderable);
        self.meshes.len() - 1
    }

    pub fn add_instance(&mut self, transform: &glam::Affine2, renderable: usize) {
        let renderable = &mut self.meshes[renderable];
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
        for renderable in &self.meshes {
            render_pass.draw_indexed(
                renderable.index_offset..(renderable.index_offset + renderable.num_indices),
                renderable.vertex_offset as i32,
                0..renderable.transformations.len() as u32,
            );
        }
    }
}
