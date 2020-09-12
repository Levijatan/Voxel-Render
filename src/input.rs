use crate::Camera;
use std::collections::HashMap;

extern crate glfw;
use self::glfw::Key;

pub struct KeyState<'a> {
    key_state: HashMap<Key, State<'a>>,
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

    pub fn add_state(&mut self, k: Key, action: impl FnMut(&mut Camera) + 'a) {
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

    pub fn set_state(&mut self, k: Key, held: bool) {
        self.key_state.get_mut(&k).unwrap().held = held;
    }
}
