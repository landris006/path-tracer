#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};

use crate::app::App;
mod app;
mod camera;
mod scene;
mod texture;

const WINDOW_WIDTH: u32 = 1920;
const WINDOW_HEIGHT: u32 = 1080;

pub enum CustomEvent {
    RequestRedraw,
}
struct ExampleRepaintSignal(std::sync::Mutex<winit::event_loop::EventLoopProxy<CustomEvent>>);
impl epi::backend::RepaintSignal for ExampleRepaintSignal {
    fn request_repaint(&self) {
        self.0
            .lock()
            .unwrap()
            .send_event(CustomEvent::RequestRedraw)
            .ok();
    }
}

pub async fn run() {
    env_logger::init();

    let event_loop = EventLoopBuilder::<CustomEvent>::with_user_event().build();
    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
        .with_title("Raytracer")
        .build(&event_loop)
        .unwrap();

    let mut app = App::new(window).await;

    event_loop.run(move |event, _, control_flow| {
        app.ui_input(&event);

        match event {
            Event::RedrawRequested(window_id) if window_id == app.window().id() => {
                app.update();
                app.setup_ui();

                match app.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => {
                        eprintln!("Lost surface, resizing");
                        app.resize(app.window_size());
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        eprintln!("Out of memory, exiting");
                        *control_flow = ControlFlow::Exit;
                    }
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            Event::MainEventsCleared | Event::UserEvent(CustomEvent::RequestRedraw) => {
                app.window().request_redraw();
            }

            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == app.window().id() => {
                app.input(event);

                match event {
                    WindowEvent::Resized(physical_size) => {
                        app.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        app.resize(**new_inner_size);
                    }
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    _ => {}
                }
            }
            _ => {}
        }
    });
}
