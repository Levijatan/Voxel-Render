use crate::Camera;
use std::collections::HashMap;

use winit::event::VirtualKeyCode;

pub struct KeyState<'a> {
    key_state: HashMap<VirtualKeyCode, State<'a>>,
}

pub struct State<'a> {
    held: bool,
    action: Box<dyn FnMut(&mut Camera) + 'a>,
}

impl<'a> State<'a> {
    fn process(&mut self, cam: &mut Camera) {
        if self.held {
            (self.action)(cam);
        }
    }
}

impl<'a> KeyState<'a> {
    pub fn new() -> Self {
        KeyState {
            key_state: HashMap::new(),
        }
    }

    pub fn add_state(&mut self, k: VirtualKeyCode, action: impl FnMut(&mut Camera) + 'a) {
        self.key_state.entry(k).or_insert(State {
            held: false,
            action: Box::new(action),
        });
    }

    pub fn process_all_states(&mut self, cam: &mut Camera) {
        for (_, state) in self.key_state.iter_mut() {
            state.process(cam);
        }
    }

    pub fn set_state(&mut self, k: VirtualKeyCode, held: bool) {
        if self.key_state.contains_key(&k) {
            self.key_state.get_mut(&k).unwrap().held = held;
        }
    }
}

pub struct CursorState {
    last_x: f32,
    last_y: f32,
    sensitivity: f32,
}

impl CursorState {
    pub fn new(x: f32, y: f32, sensitivity: f32) -> CursorState {
        CursorState {
            last_x: x,
            last_y: y,
            sensitivity,
        }
    }

    pub fn process(&mut self, x: f32, y: f32, cam: &mut Camera) {
        let x_offset = (x - self.last_x) * self.sensitivity * cam.delta_time as f32;
        let y_offset = (self.last_y - y) * self.sensitivity * cam.delta_time as f32;
        self.last_x = x;
        self.last_y = y;

        cam.rotate(x_offset, y_offset);
    }
}
