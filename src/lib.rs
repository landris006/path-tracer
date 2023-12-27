#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use crate::app::App;
use winit::{dpi::LogicalSize, event_loop::EventLoopBuilder, window::WindowBuilder};

mod app;
mod camera;
mod renderer;
mod scene;
mod texture;
mod ui;
mod utils;

const WINDOW_WIDTH: u32 = 1920;
const WINDOW_HEIGHT: u32 = 1080;
const MAX_NUMBER_OF_SPHERES: u32 = 256;

pub async fn run() {
    env_logger::init();

    let event_loop = EventLoopBuilder::new().build();
    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
        .with_title("Raytracer")
        .build(&event_loop)
        .unwrap();

    App::new(window).await.run(event_loop);
}
