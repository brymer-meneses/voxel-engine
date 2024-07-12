use crate::renderer::Renderer;

pub trait Scene {
    fn render(&mut self, renderer: &Renderer) -> Result<(), wgpu::SurfaceError>;
    fn tick(&mut self, renderer: &Renderer);
    fn initialize_buffers(&mut self, renderer: &Renderer);
}
