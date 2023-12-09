use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{CursorGrabMode, WindowBuilder},
};

use crate::app::App;
mod app;

const WINDOW_WIDTH: u32 = 1920;
const WINDOW_HEIGHT: u32 = 1080;

pub async fn run() {
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
        .build(&event_loop)
        .unwrap();

    window.set_cursor_grab(CursorGrabMode::Locked).unwrap();
    window
        .set_cursor_position(winit::dpi::PhysicalPosition::new(
            WINDOW_WIDTH as f64 / 2.0,
            WINDOW_HEIGHT as f64 / 2.0,
        ))
        .unwrap();

    let mut app = App::new(window).await;

    event_loop.run(move |event, _, control_flow| match event {
        Event::RedrawRequested(window_id) if window_id == app.window().id() => {
            app.update();
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
        Event::MainEventsCleared => {
            app.window().request_redraw();
        }

        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == app.window().id() => {
            if !app.input(event) {
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
        }
        _ => {}
    });
}
