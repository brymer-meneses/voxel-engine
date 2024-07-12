mod geometry;
mod renderer;
mod scene;

use core::time;
use std::{
    mem::MaybeUninit,
    time::{Duration, Instant},
};

use log::{log, Level};

use wgpu::{
    util::{DeviceExt, RenderEncoder},
    BufferUsages,
};
use winit::{
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::WindowBuilder,
};

use geometry::{Geometry, Vertex};
use renderer::Renderer;
use scene::Scene;

fn main() {
    env_logger::init();

    #[rustfmt::skip]
    let geometries = vec![Geometry::new(
        vec![
            Vertex { position: [-0.5, -0.5, -0.3], color: [1.0, 1.0, 1.0] },
            Vertex { position: [0.5, -0.5, -0.3], color: [1.0, 1.0, 1.0] },
            Vertex { position: [0.5, 0.5, -0.3], color: [1.0, 1.0, 1.0] },
            Vertex { position: [-0.5, 0.5, -0.3], color: [1.0, 1.0, 1.0] },
            Vertex { position: [0.0, 0.0, 0.5], color: [0.5, 0.5, 0.5] },
        ],
        vec![
             0, 1, 2, 
             0, 2, 3, 

             0, 1, 4,
             1, 2, 4,
             2, 3, 4,
             3, 0, 4,
        ],
    )];

    let scene = TestScene::new(geometries);

    pollster::block_on(run(scene));
}

pub async fn run(mut scene: impl Scene) {
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut renderer = Renderer::new(&window).await;
    scene.initialize_buffers(&renderer);

    let result = event_loop.run(move |event, control_flow| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == renderer.window.id() => match event {
            WindowEvent::CloseRequested => control_flow.exit(),

            WindowEvent::Resized(physical_size) => {
                renderer.resize(*physical_size);
            }

            WindowEvent::KeyboardInput { event, .. } => match event {
                KeyEvent {
                    state: ElementState::Pressed,
                    physical_key: PhysicalKey::Code(KeyCode::Escape),
                    ..
                } => control_flow.exit(),
                _ => {}
            },

            WindowEvent::RedrawRequested if window_id == renderer.window.id() => {
                scene.tick(&renderer);

                match scene.render(&renderer) {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => renderer.resize(renderer.size),
                    Err(wgpu::SurfaceError::OutOfMemory) => control_flow.exit(),
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            _ => {}
        },

        Event::AboutToWait => {
            renderer.window.request_redraw();
        }
        _ => {}
    });

    if let Err(err) = result {
        log!(Level::Error, "Something went wrong {err}");
    }
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
struct State {
    time: f32,
}

struct TestScene {
    geometries: Vec<Geometry>,
    time: Instant,
    state: State,
    state_buffer: Option<wgpu::Buffer>,
}

impl TestScene {
    fn new(geometries: Vec<Geometry>) -> Self {
        let time = Instant::now();

        Self {
            geometries,
            time,
            state: State {
                time: time.elapsed().as_secs_f32(),
            },
            state_buffer: None,
        }
    }
}

impl Scene for TestScene {
    fn initialize_buffers(&mut self, renderer: &Renderer) {
        // initialize buffers
        for geometry in &mut self.geometries {
            geometry.initialize_buffers(&renderer.device);
        }

        let state_buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: std::mem::size_of::<State>() as u64,
            usage: wgpu::BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });

        self.state_buffer = Some(state_buffer);
    }

    fn tick(&mut self, renderer: &Renderer) {
        let elapsed = self.time.elapsed();

        self.state.time = elapsed.as_secs_f32();

        if let Some(ref buffer) = self.state_buffer {
            renderer
                .queue
                .write_buffer(buffer, 0, bytemuck::cast_slice(&[self.state]));
        }
    }

    fn render(&mut self, renderer: &Renderer) -> Result<(), wgpu::SurfaceError> {
        let shader = renderer
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
            });

        let state_bind_group_layout =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let state_bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &state_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.state_buffer.as_ref().unwrap().as_entire_binding(),
                }],
                label: None,
            });

        let render_pipeline_layout =
            renderer
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &[&state_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let render_pipeline =
            renderer
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Render Pipeline"),
                    layout: Some(&render_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: "vs_main",
                        buffers: &[Vertex::desc()],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: "fs_main",
                        targets: &[Some(wgpu::ColorTargetState {
                            format: renderer.config.format,
                            blend: Some(wgpu::BlendState::REPLACE),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: None,
                        polygon_mode: wgpu::PolygonMode::Fill,
                        unclipped_depth: false,
                        conservative: false,
                    },
                    depth_stencil: None,
                    multiview: None,
                    multisample: wgpu::MultisampleState::default(),
                });

        let output = renderer.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&render_pipeline);
            render_pass.set_bind_group(0, &state_bind_group, &[]);

            for geometry in &self.geometries {
                geometry.render(&mut render_pass);
            }
        }

        renderer.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
