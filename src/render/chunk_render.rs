use crate::geom::voxel_to_chunk_pos;
use crate::geom::ChunkKey;
use crate::SharedState;

use gl::types::*;

use glm::Vec3;

use std::cmp::Ordering;
use std::mem;
use std::os::raw::c_void;
use std::ptr;
use std::sync::mpsc::Receiver;

#[derive(Copy, Clone, Debug)]
struct ChunkData {
    key: ChunkKey,
    rendered: bool,
    amount: i32,
    vbo: u32,
    priority: i32,
}

impl Ord for ChunkData {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .priority
            .cmp(&self.priority)
            .then_with(|| self.amount.cmp(&other.amount))
    }
}

impl PartialOrd for ChunkData {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ChunkData {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority || self.amount == other.amount || self.vbo == other.vbo
    }
}

impl Eq for ChunkData {}

pub struct ChunkRender {
    pub vao: u32,
    queue: Vec<ChunkData>,
    vbo_stack: Vec<u32>,
    next_free_vbo: usize,
    cnt_vbos: usize,
    state: SharedState,
    chunk_update_rx: Receiver<ChunkKey>,
}

impl ChunkRender {
    pub unsafe fn new(state: &SharedState, chunk_update_rx: Receiver<ChunkKey>) -> Self {
        let mut vao = 0 as u32;

        gl::GenVertexArrays(1, &mut vao);
        gl::BindVertexArray(vao);

        gl::EnableVertexAttribArray(0);
        gl::BindVertexArray(0);

        ChunkRender {
            vao,
            queue: Vec::new(),
            vbo_stack: Vec::new(),
            next_free_vbo: 0,
            cnt_vbos: 0,
            state: state.clone(),
            chunk_update_rx,
        }
    }

    pub fn process(&mut self) {
        if *self.state.clear_render.read().unwrap() {
            self.queue.clear();
            self.next_free_vbo = 0;
            let mut clear_render = self.state.clear_render.write().unwrap();
            *clear_render = false;
        }
        for d in self.chunk_update_rx.try_iter() {
            if self.next_free_vbo == self.cnt_vbos {
                unsafe {
                    let mut vbo = 0;
                    gl::GenBuffers(1, &mut vbo);
                    self.vbo_stack.push(vbo);
                    self.cnt_vbos += 1;
                }
            }
            println!("Rendering: {:?}", d);
            self.queue.push(ChunkData {
                key: d,
                rendered: false,
                amount: 0,
                vbo: self.vbo_stack[self.next_free_vbo],
                priority: -1,
            });
            self.next_free_vbo += 1;
        }
        unsafe {
            self.process_queue();
        }
    }

    unsafe fn process_queue(&mut self) {
        let world_id = *self.state.active_world.read().unwrap();
        let world_reg = self.state.world_registry.read().unwrap();
        let active_world = world_reg.world(&world_id);
        let half_size = active_world.pc.chunk_size() as f32 / 2.0;
        let half_size_vec = Vec3::new(half_size, half_size, half_size);
        let cam = self.state.cam.read().unwrap();
        for i in 0..self.queue.len() {
            let mut cd = self.queue.get_mut(i).unwrap();

            let cam_in_chunk = voxel_to_chunk_pos(&cam.pos, active_world.chunk_size());
            let chunk_pos = &active_world.pc.chunk_pos(&cd.key);
            let distance = glm::distance(&cam_in_chunk, &chunk_pos);
            // println!(
            //     "distance: {}, cam pos: {}, chunk pos: {}",
            //     distance, cam_in_chunk, chunk_pos
            // );
            cd.priority = distance as i32;

            // println!(
            //     "Rendering Chunk with {} priority, max priority: {}",
            //     cd.priority, self.max_render_radius
            // );

            if !cd.rendered {
                let d = active_world.pc.chunk_render(&cd.key);
                cd.amount = d.len() as i32;
                if cd.amount > 0 {
                    cd.rendered = true;
                    gl::BindBuffer(gl::ARRAY_BUFFER, cd.vbo);
                    gl::BufferData(
                        gl::ARRAY_BUFFER,
                        (cd.amount as usize * mem::size_of::<GLfloat>()) as GLsizeiptr,
                        &d[0] as *const f32 as *const c_void,
                        gl::DYNAMIC_DRAW,
                    );
                    gl::BindBuffer(gl::ARRAY_BUFFER, 0);
                }
            }

            let chunk_world_pos = active_world.pc.chunk_world_pos_min(&cd.key) + half_size_vec;
            if cam.cube_in_view(chunk_world_pos, active_world.chunk_size() as f32) {
                if cd.amount > 0 {
                    gl::BindBuffer(gl::ARRAY_BUFFER, cd.vbo);
                    let count = cd.amount;
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
        }
    }
}
