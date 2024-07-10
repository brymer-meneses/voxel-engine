#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
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
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

use std::mem::MaybeUninit;

pub struct Geometry {
    pub points: Vec<Vertex>,
    pub indices: Vec<u16>,

    pub index_buffer: MaybeUninit<wgpu::Buffer>,
    pub vertex_buffer: MaybeUninit<wgpu::Buffer>,
}

impl Geometry {
    pub fn new(points: Vec<Vertex>, indices: Vec<u16>) -> Self {
        return Self {
            points,
            indices,
            index_buffer: MaybeUninit::uninit(),
            vertex_buffer: MaybeUninit::uninit(),
        };
    }

    pub fn initialize_buffers(&mut self, device: &wgpu::Device) {
        use wgpu::util::DeviceExt;

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertex buffer"),
            contents: bytemuck::cast_slice(&self.points),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("index buffer"),
            contents: bytemuck::cast_slice(&self.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        self.index_buffer.write(index_buffer);
        self.vertex_buffer.write(vertex_buffer);
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        unsafe {
            render_pass.set_vertex_buffer(0, self.vertex_buffer.assume_init_ref().slice(..));
            render_pass.set_index_buffer(
                self.index_buffer.assume_init_ref().slice(..),
                wgpu::IndexFormat::Uint16,
            );
        }

        render_pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);
    }
}
