use crate::renderer::Renderer;

pub trait Scene {
    fn render(&mut self, renderer: &Renderer) -> Result<(), wgpu::SurfaceError>;
}
