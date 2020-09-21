use crate::geom::voxel_to_chunk_pos;
use crate::geom::ChunkKey;
use crate::geom::PointCloud;

use super::Camera;

use gl::types::*;

use glm::Vec3;

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::mem;
use std::os::raw::c_void;
use std::ptr;

#[derive(Copy, Clone)]
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
    queue: BinaryHeap<ChunkData>,
    pub max_render_radius: f32,
    vbo_stack: Vec<u32>,
}

impl ChunkRender {
    pub unsafe fn new(render_radius: f32) -> Self {
        let mut vao = 0 as u32;

        gl::GenVertexArrays(1, &mut vao);
        gl::BindVertexArray(vao);

        gl::EnableVertexAttribArray(0);
        gl::BindVertexArray(0);

        let mut vbo_stack = Vec::new();
        let render_diam = render_radius as i64 * 2;
        for _ in 0..(render_diam * render_diam * render_diam) {
            let mut vbo = 0;
            gl::GenBuffers(1, &mut vbo);
            vbo_stack.push(vbo);
        }

        let max_render_radius =
            ((render_radius * render_radius) + (render_radius * render_radius)).sqrt();

        ChunkRender {
            vao,
            queue: BinaryHeap::new(),
            max_render_radius,
            vbo_stack,
        }
    }

    pub fn add_to_queue(&mut self, key: ChunkKey, pc: &mut PointCloud) {
        if !pc.chunk_in_queue(&key) {
            pc.chunk_set_in_queue(&key, true);

            self.queue.push(ChunkData {
                key,
                rendered: false,
                amount: 0,
                vbo: self.vbo_stack.pop().unwrap(),
                priority: -1,
            })
        }
    }

    pub fn remove_from_queue(&mut self, vbo: u32, key: ChunkKey, pc: &mut PointCloud) {
        pc.chunk_set_in_queue(&key, false);
        self.vbo_stack.push(vbo);
    }

    pub unsafe fn process_queue(&mut self, cam: &Camera, pc: &mut PointCloud) {
        let mut done = BinaryHeap::new();
        let half_size = pc.chunk_size() as f32 / 2.0;
        let half_size_vec = Vec3::new(half_size, half_size, half_size);
        while !self.queue.is_empty() {
            let mut cd = self.queue.pop().unwrap();

            let cam_in_chunk = voxel_to_chunk_pos(&cam.pos, pc.chunk_size()) + half_size_vec;
            cd.priority = glm::distance(&cam_in_chunk, &pc.chunk_pos(&cd.key)) as i32;
            /*
            println!(
                "Rendering Chunk with {} priority, max priority: {}",
                cd.priority, self.max_render_radius
            );
            */
            if !cd.rendered || pc.chunk_rerender(&cd.key) {
                pc.chunk_set_rerender(&cd.key, false);
                let d = pc.render_chunk(&cd.key);
                cd.rendered = true;
                cd.amount = d.len() as i32;
                if cd.amount > 0 {
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

            let chunk_world_pos = pc.chunk_world_pos_min(&cd.key) + half_size_vec;
            if cam.cube_in_view(chunk_world_pos, pc.chunk_size() as f32) {
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
            if cd.priority as f32 <= self.max_render_radius {
                done.push(cd);
            } else {
                self.remove_from_queue(cd.vbo, cd.key, pc);
            }
        }
        self.queue = done;
    }
}
