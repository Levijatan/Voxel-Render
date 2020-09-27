use super::Camera;
use crate::geom::ChunkKey;
use crate::SharedState;

use gl::types::*;

use flamer::flame;
use glm::Vec3;

use std::collections::HashMap;
use std::mem;
use std::os::raw::c_void;
use std::ptr;
use std::sync::mpsc::Receiver;

#[derive(Copy, Clone, Debug)]
struct ChunkData {
    rendered: bool,
    amount: i32,
    vbo: u32,
}

impl ChunkData {
    #[flame("ChunkData")]
    unsafe fn load_data(&mut self, data: &Vec<f32>) {
        self.amount = data.len() as i32;
        if self.amount > 0 {
            self.rendered = true;
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (self.amount as usize * mem::size_of::<GLfloat>()) as GLsizeiptr,
                &data[0] as *const f32 as *const c_void,
                gl::DYNAMIC_DRAW,
            );
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        }
    }

    #[flame("ChunkData")]
    unsafe fn draw(&self) {
        gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
        let count = self.amount;
        gl::VertexAttribPointer(
            0,
            3,
            gl::FLOAT,
            gl::FALSE,
            3 * mem::size_of::<GLfloat>() as i32,
            ptr::null(),
        );
        gl::DrawArrays(gl::POINTS, 0, count / 3);
        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
    }
}

pub struct ChunkRender {
    pub vao: u32,
    queue: Vec<ChunkKey>,
    old_queue: Vec<ChunkKey>,
    render_map: HashMap<ChunkKey, ChunkData>,
    vbo_stack: Vec<u32>,
    state: SharedState,
    chunk_update_rx: Receiver<ChunkKey>,
    last_clear_render: bool,
}

impl ChunkRender {
    #[flame("ChunkRender")]
    pub unsafe fn new(state: &SharedState, chunk_update_rx: Receiver<ChunkKey>) -> Self {
        let mut vao = 0 as u32;

        gl::GenVertexArrays(1, &mut vao);
        gl::BindVertexArray(vao);

        gl::EnableVertexAttribArray(0);
        gl::BindVertexArray(0);

        ChunkRender {
            vao,
            queue: Vec::new(),
            old_queue: Vec::new(),
            render_map: HashMap::new(),
            vbo_stack: Vec::new(),
            state: state.clone(),
            chunk_update_rx,
            last_clear_render: false,
        }
    }

    #[flame("ChunkRender")]
    fn clear_queue(&mut self, clear_render: bool) {
        if clear_render && !self.last_clear_render {
            self.old_queue = self.queue.clone();
            self.queue = Vec::new();
        }
    }

    #[flame("ChunkRender")]
    fn insert_keys(&mut self) {
        for key in self.chunk_update_rx.try_iter() {
            println!("Rendering: {:?}", key);
            self.render_map.entry(key).or_insert({
                if self.vbo_stack.is_empty() {
                    unsafe {
                        let mut vbo = 0;
                        gl::GenBuffers(1, &mut vbo);
                        self.vbo_stack.push(vbo);
                    }
                }
                ChunkData {
                    rendered: false,
                    amount: 0,
                    vbo: self.vbo_stack.pop().unwrap(),
                }
            });
            if !self.queue.contains(&key) {
                self.queue.push(key);
            }
        }
    }

    #[flame("ChunkRender")]
    pub fn process(&mut self, cam: &Camera) {
        let clear_render = *self.state.clear_render.read().unwrap();

        self.clear_queue(clear_render);

        self.insert_keys();

        self.clear_old(clear_render);

        unsafe {
            self.process_queue(cam);
        }

        self.last_clear_render = clear_render;
    }

    #[flame("ChunkRender")]
    fn clear_old(&mut self, clear_render: bool) {
        if !clear_render && self.last_clear_render {
            for i in 0..self.old_queue.len() {
                let key = self.old_queue[i];
                if !self.queue.contains(&key) {
                    self.vbo_stack.push(self.render_map[&key].vbo);
                    self.render_map.remove(&key);
                }
            }
        }
    }

    #[flame("ChunkRender")]
    fn chunk_render_data(&mut self, key: &ChunkKey) -> Vec<f32> {
        let world_id = *self.state.active_world.read().unwrap();
        let world_reg = self.state.world_registry.read().unwrap();
        let active_world = world_reg.world(&world_id);
        active_world.pc.chunk_render(&key).clone()
    }

    #[flame("ChunkRender")]
    unsafe fn load_data(&mut self, entry: usize) -> bool {
        let key = self.queue[entry];
        let d = self.chunk_render_data(&key);
        let cd = self.render_map.get_mut(&key).unwrap();

        if d.len() > 0 {
            cd.load_data(&d);
        } else {
            self.queue.remove(entry);
            self.vbo_stack.push(self.render_map[&key].vbo);
            self.render_map.remove(&key);
            return false;
        }

        true
    }

    #[flame("ChunkRender")]
    unsafe fn process_queue_entry(&mut self, entry: usize, cam: &Camera) -> bool {
        let chunk_size = *self.state.chunk_size as i32;
        let half_size = chunk_size as f32 / 2.0;
        let half_size_vec = Vec3::new(half_size, half_size, half_size);

        let key = self.queue[entry];

        if !self.render_map[&key].rendered {
            if !self.load_data(entry) {
                return false;
            }
        }

        let cd = self.render_map.get_mut(&key).unwrap();

        let chunk_world_pos = Vec3::new(
            (key.x * chunk_size) as f32,
            (key.y * chunk_size) as f32,
            (key.z * chunk_size) as f32,
        ) + half_size_vec;
        if cam.cube_in_view(chunk_world_pos, chunk_size as f32) {
            cd.draw();
        }

        true
    }

    #[flame("ChunkRender")]
    unsafe fn process_queue(&mut self, cam: &Camera) {
        let mut i = 0;
        while i < self.queue.len() {
            if self.process_queue_entry(i, cam) {
                i += 1;
            }
        }
    }
}
