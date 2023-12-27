use std::time::Instant;

use cgmath::{InnerSpace, Vector2, Vector3, Zero};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, KeyboardInput, VirtualKeyCode, WindowEvent},
    window::{CursorGrabMode, Window},
};

#[derive(Debug)]
pub struct Camera {
    pub origin: Vector3<f32>,
    pub forward: Vector3<f32>,
    pub right: Vector3<f32>,
    pub up: Vector3<f32>,
    pub focal_length: f32,
    pub vfov: f32,
    last_move_time: Instant,
}

#[derive(Debug)]
pub struct Ray {
    pub origin: Vector3<f32>,
    pub direction: Vector3<f32>,
}

impl Ray {
    pub fn at(&self, t: f32) -> Vector3<f32> {
        self.origin + self.direction * t
    }
}

impl Camera {
    pub fn new() -> Self {
        Self {
            origin: Vector3::new(0.0, 0.0, 0.0),
            forward: Vector3::new(0.0, 0.0, -1.0),
            right: Vector3::new(1.0, 0.0, 0.0),
            up: Vector3::new(0.0, 1.0, 0.0),
            focal_length: 1.0,
            vfov: 75.0,
            last_move_time: Instant::now(),
        }
    }

    pub fn moved_recently(&self) -> bool {
        self.last_move_time.elapsed().as_secs_f32() < 0.2
    }

    pub fn screen_pos_to_ray(
        &self,
        position: PhysicalPosition<f64>,
        screen_size: PhysicalSize<u32>,
    ) -> Ray {
        let aspect_ratio = screen_size.width as f32 / screen_size.height as f32;
        let fov_adjustment = (self.vfov.to_radians() / 2.0).tan();
        let screen_x = (((position.x as f32 / screen_size.width as f32) * 2.0 - 1.0)
            * fov_adjustment
            * aspect_ratio)
            * self.focal_length;
        let screen_y = (1.0 - (position.y as f32 / screen_size.height as f32) * 2.0)
            * fov_adjustment
            * self.focal_length;

        let direction = self.forward + self.right * screen_x + self.up * screen_y;
        Ray {
            origin: self.origin,
            direction: direction.normalize(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraBuffer {
    origin: [f32; 3],
    focal_length: f32,
    forward: [f32; 3],
    vfov: f32,
    right: [f32; 3],
    _padding1: u32,
    up: [f32; 3],
    _padding2: u32,
}

impl From<&Camera> for CameraBuffer {
    fn from(camera: &Camera) -> Self {
        Self {
            origin: camera.origin.into(),
            focal_length: camera.focal_length,
            forward: camera.forward.into(),
            vfov: camera.vfov,
            right: camera.right.into(),
            _padding1: 0,
            up: camera.up.into(),
            _padding2: 0,
        }
    }
}

#[derive(Debug)]
pub struct CameraController {
    is_right_mouse_button_pressed: bool,
    is_pressing_forward: bool,
    is_pressing_backward: bool,
    is_pressing_left: bool,
    is_pressing_right: bool,
    is_pressing_up: bool,
    is_pressing_down: bool,
    yaw: f32,
    pitch: f32,
    prev_cursor_pos: Option<Vector2<f32>>,
    pub speed: f32,
}

impl CameraController {
    pub fn new() -> Self {
        Self {
            is_right_mouse_button_pressed: false,
            is_pressing_forward: false,
            is_pressing_backward: false,
            is_pressing_left: false,
            is_pressing_right: false,
            is_pressing_up: false,
            is_pressing_down: false,
            prev_cursor_pos: None,
            yaw: 0.0,
            pitch: 0.0,
            speed: 3.0,
        }
    }

    pub fn input(&mut self, event: &WindowEvent, window: &mut Window) {
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state,
                        virtual_keycode: Some(keycode),
                        ..
                    },
                ..
            } if self.is_right_mouse_button_pressed => match keycode {
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
                VirtualKeyCode::Space => {
                    self.is_pressing_up = *state == ElementState::Pressed;
                }
                VirtualKeyCode::LShift => {
                    self.is_pressing_down = *state == ElementState::Pressed;
                }
                _ => {}
            },
            WindowEvent::MouseInput {
                state,
                button: winit::event::MouseButton::Right,
                ..
            } => {
                self.is_right_mouse_button_pressed = *state == ElementState::Pressed;

                if *state == ElementState::Released {
                    self.is_pressing_forward = false;
                    self.is_pressing_backward = false;
                    self.is_pressing_left = false;
                    self.is_pressing_right = false;
                    self.is_pressing_up = false;
                    self.is_pressing_down = false;

                    window.set_cursor_grab(CursorGrabMode::None).unwrap();
                    window.set_cursor_visible(true);
                } else {
                    window.set_cursor_grab(CursorGrabMode::Confined).unwrap();
                    window.set_cursor_visible(false);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let (x, y) = (position.x as f32, position.y as f32);
                let (prev_x, prev_y) = if let Some(prev_cursor_pos) = self.prev_cursor_pos {
                    (prev_cursor_pos.x, prev_cursor_pos.y)
                } else {
                    (x, y)
                };

                let (x_offset, y_offset) = (x - prev_x, prev_y - y);
                self.prev_cursor_pos = Some(Vector2::new(x, y));

                if !self.is_right_mouse_button_pressed {
                    return;
                }

                self.yaw += x_offset * 0.1;
                self.pitch += y_offset * 0.1;

                if self.pitch > 89.0 {
                    self.pitch = 89.0;
                } else if self.pitch < -89.0 {
                    self.pitch = -89.0;
                }
            }
            _ => {}
        }
    }

    pub fn update_camera(&self, camera: &mut Camera, delta_time: f32) {
        let new_forward = Vector3::new(
            self.yaw.to_radians().cos() * self.pitch.to_radians().cos(),
            self.pitch.to_radians().sin(),
            self.yaw.to_radians().sin() * self.pitch.to_radians().cos(),
        );

        if camera.forward.dot(new_forward) < 0.999999 {
            camera.last_move_time = Instant::now();
        }
        camera.forward = new_forward.normalize();
        camera.right = camera.forward.cross(Vector3::unit_y()).normalize();
        camera.up = camera.right.cross(camera.forward).normalize();

        let forward = if self.is_pressing_forward && !self.is_pressing_backward {
            camera.forward
        } else if self.is_pressing_backward && !self.is_pressing_forward {
            -camera.forward
        } else {
            Vector3::zero()
        };

        let right = if self.is_pressing_right && !self.is_pressing_left {
            camera.right
        } else if self.is_pressing_left && !self.is_pressing_right {
            -camera.right
        } else {
            Vector3::zero()
        };

        let up = if self.is_pressing_up && !self.is_pressing_down {
            Vector3::unit_y()
        } else if self.is_pressing_down && !self.is_pressing_up {
            -Vector3::unit_y()
        } else {
            Vector3::zero()
        };

        let new_origin = camera.origin + (forward + right + up) * self.speed * delta_time;
        if new_origin.ne(&camera.origin) {
            camera.last_move_time = Instant::now();
        }
        camera.origin += (forward + right + up) * self.speed * delta_time;
    }
}
