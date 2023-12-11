use cgmath::{Vector3, Zero};
use winit::event::{ElementState, KeyboardInput, VirtualKeyCode, WindowEvent};

#[derive(Debug)]
pub struct Camera {
    pub origin: Vector3<f32>,
    pub forward: Vector3<f32>,
    pub right: Vector3<f32>,
    pub up: Vector3<f32>,
    pub focal_length: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraBuffer {
    origin: [f32; 3],
    focal_length: f32,
    forward: [f32; 3],
    _padding: u32,
    right: [f32; 3],
    _padding2: u32,
    up: [f32; 3],
    _padding3: u32,
}

impl From<&Camera> for CameraBuffer {
    fn from(camera: &Camera) -> Self {
        Self {
            origin: camera.origin.into(),
            focal_length: camera.focal_length,
            forward: camera.forward.into(),
            _padding: 0,
            right: camera.right.into(),
            _padding2: 0,
            up: camera.up.into(),
            _padding3: 0,
        }
    }
}

#[derive(Debug)]
pub struct CameraController {
    is_pressing_forward: bool,
    is_pressing_backward: bool,
    is_pressing_left: bool,
    is_pressing_right: bool,
    pub speed: f32,
}

impl CameraController {
    pub fn new() -> Self {
        Self {
            is_pressing_forward: false,
            is_pressing_backward: false,
            is_pressing_left: false,
            is_pressing_right: false,
            speed: 0.8,
        }
    }

    pub fn input(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state,
                        virtual_keycode: Some(keycode),
                        ..
                    },
                ..
            } => match keycode {
                VirtualKeyCode::W => {
                    self.is_pressing_forward = *state == ElementState::Pressed;
                }
                VirtualKeyCode::S => {
                    self.is_pressing_backward = *state == ElementState::Pressed;
                }
                VirtualKeyCode::A => {
                    self.is_pressing_left = *state == ElementState::Pressed;
                }
                VirtualKeyCode::D => {
                    self.is_pressing_right = *state == ElementState::Pressed;
                }
                _ => {}
            },
            _ => {}
        }
    }

    pub fn update_camera(&self, camera: &mut Camera, delta_time: f32) {
        let forward = if self.is_pressing_forward {
            camera.forward
        } else if self.is_pressing_backward {
            -camera.forward
        } else {
            Vector3::zero()
        };

        let right = if self.is_pressing_right {
            camera.right
        } else if self.is_pressing_left {
            -camera.right
        } else {
            Vector3::zero()
        };

        camera.origin += (forward + right) * self.speed * delta_time;
    }
}
