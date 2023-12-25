use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use wgpu::{CommandEncoder, Device, Queue, SurfaceTexture, TextureFormat};
use winit::{event::Event, window::Window};

pub struct Ui {
    platform: Platform,
    render_pass: RenderPass,
}

impl Ui {
    pub fn new(window: &Window, device: &Device, surface_format: TextureFormat) -> Self {
        let window_size = window.inner_size();
        let platform = Platform::new(PlatformDescriptor {
            physical_width: window_size.width,
            physical_height: window_size.height,
            scale_factor: window.scale_factor(),
            ..Default::default()
        });
        let render_pass = RenderPass::new(device, surface_format, 1);

        Self {
            platform,
            render_pass,
        }
    }

    pub fn render(
        &mut self,
        encoder: &mut CommandEncoder,
        output: &SurfaceTexture,
        window: &Window,
        device: &Device,
        queue: &Queue,
    ) {
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let platform = &mut self.platform;

        let full_output = platform.end_frame(Some(window));
        let paint_jobs = platform.context().tessellate(full_output.shapes);

        let screen_descriptor = ScreenDescriptor {
            physical_width: output.texture.width(),
            physical_height: output.texture.height(),
            scale_factor: window.scale_factor() as f32,
        };

        self.render_pass
            .add_textures(device, queue, &full_output.textures_delta)
            .expect("error adding textures");
        self.render_pass
            .update_buffers(device, queue, &paint_jobs, &screen_descriptor);
        self.render_pass
            .execute(encoder, &view, &paint_jobs, &screen_descriptor, None)
            .unwrap();
        self.render_pass
            .remove_textures(full_output.textures_delta)
            .expect("error removing textures");
    }

    pub fn begin_new_frame(&mut self, time: f64) {
        self.platform.update_time(time);
        self.platform.begin_frame();
    }

    pub fn platform_mut(&mut self) -> &mut Platform {
        &mut self.platform
    }

    pub fn handle_event(&mut self, event: &Event<()>) {
        self.platform.handle_event(event);
    }

    pub fn contains_mouse(&self) -> bool {
        self.platform.context().is_pointer_over_area()
    }
}
