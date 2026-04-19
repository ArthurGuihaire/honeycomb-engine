use std::num::NonZeroU64;
use wgpu::{COPY_BUFFER_ALIGNMENT, QueueWriteBufferView};

use crate::GpuContext;
use std::cmp::max;
use std::sync::Arc;

const INITIAL_BUFFER_SIZE: u64 = 256;

pub struct GpuBuffer {
    pub buffer: wgpu::Buffer,
    pub bytes_used: u64,
    last_index: u16,
    gpu: Arc<GpuContext>,
}

impl GpuBuffer {
    pub fn new(gpu: Arc<GpuContext>, usage: wgpu::BufferUsages) -> Self {
        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: INITIAL_BUFFER_SIZE,
            usage: usage | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        Self {
            buffer,
            bytes_used: 0,
            last_index: 0,
            gpu,
        }
    }
    fn reallocate_buffer(&mut self, target_size: u64) {
        let new_size = max(target_size, (self.buffer.size() + 1).next_power_of_two());
        let buffer_usage = self.buffer.usage();
        let new_buffer = self.gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: new_size,
            usage: buffer_usage,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        //alignment rules yay, need to round up to 4 bytes
        encoder.copy_buffer_to_buffer(
            &self.buffer,
            0,
            &new_buffer,
            0,
            self.bytes_used.next_multiple_of(4),
        );
        let command_buffer = encoder.finish();
        self.gpu.queue.submit(std::iter::once(command_buffer));
        self.buffer = new_buffer;
        //self.gpu.queue.write_buffer(&self.buffer, 0, data);
    }
    //enforce u16 since dealing with single byte alignment is 10x worse
    pub fn append(&mut self, data_u16: &[u16]) {
        //TODO: this fix sucks, wgpu needs 4 byte alignment but this uses 2-byte indices
        let data: &[u8] = bytemuck::cast_slice(data_u16);
        let alignment_start = self.bytes_used % COPY_BUFFER_ALIGNMENT;
        let alignment_end = (data.len() as u64 + alignment_start) % COPY_BUFFER_ALIGNMENT;
        let write_size = data.len() as u64 + alignment_start + alignment_end;

        let new_target_size = write_size + self.bytes_used;
        if new_target_size < self.buffer.size() {
            self.reallocate_buffer(new_target_size);
        }
        let mut temp_buffer = self
            .gpu
            .queue
            .write_buffer_with(
                &self.buffer,
                self.bytes_used - alignment_start,
                NonZeroU64::new(write_size).unwrap(),
            )
            .unwrap();

        //this shit is so bad
        if alignment_start == 0 {
            temp_buffer.slice(0..data.len()).copy_from_slice(data);
        } else {
            let temp_slice = self.last_index.to_ne_bytes(); //get first 2 bytes from last misaligned write
            temp_buffer.slice(0..2).copy_from_slice(&temp_slice);
            temp_buffer.slice(2..).copy_from_slice(data); //rest of the buffer can be copied from data
        }

        if alignment_end != 0 {
            //yay more shenanigans
            const RANDOM_BUFFER: [u8; 2] = [67, 67];
            let misaligned_index = alignment_start as usize + data.len();
            temp_buffer
                .slice(misaligned_index..misaligned_index + 2)
                .copy_from_slice(&RANDOM_BUFFER);
            //also we need to save last 2 bytes since next write is also guaranteed to be misaligned
            self.last_index = data_u16[data_u16.len() - 1]; //u16 actually convenient for this
        }
        self.bytes_used += data.len() as u64;
        //We just leave buffer alone and it gets written to gpu... eventually???
        //I mean clearly it works
    }
    pub fn update_aligned(&mut self, offset: u32, new_data: &[u32]) {
        let data_u8: &[u8] = bytemuck::cast_slice(new_data);
        self.gpu
            .queue
            .write_buffer(&self.buffer, offset as u64, data_u8);
    }
}
