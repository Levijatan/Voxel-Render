use gfx::camera;
use winit::{dpi::PhysicalPosition, event::{ElementState, KeyboardInput, MouseButton, WindowEvent}};

pub struct State {
    mouse_pressed: bool,
    camera_controller: camera::Controller,
    last_mouse_pos: PhysicalPosition<f64>,
}

impl State {
    pub fn new() -> Self {
        Self {
            mouse_pressed: false,
            camera_controller: camera::Controller::new(4.0, 0.4),
            last_mouse_pos: (0.0, 0.0).into(),
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        virtual_keycode: Some(key),
                        state,
                        ..
                    },
                ..
            } => self.camera_controller.process_keyboard(*key, *state),
            WindowEvent::MouseWheel { delta, .. } => {
                self.camera_controller.process_scroll(delta);
                true
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                self.mouse_pressed = *state == ElementState::Pressed;
                true
            }
            WindowEvent::CursorMoved { position, .. } => {
                let mouse_dx = position.x - self.last_mouse_pos.x;
                let mouse_dy = position.y - self.last_mouse_pos.y;
                self.last_mouse_pos = *position;
                if self.mouse_pressed {
                    self.camera_controller.process_mouse(mouse_dx, mouse_dy);
                }
                true
            }
            _ => false,
        }
    }

    pub fn update(&mut self, camera: &mut camera::Camera, dt: std::time::Duration) {
        self.camera_controller.update_camera(camera, dt);
    }
}
