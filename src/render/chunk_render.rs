use crate::geom::voxel_to_chunk_pos;
use crate::geom::Chunk;

use super::Camera;

extern crate gl;
use self::gl::types::*;

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::mem;
use std::os::raw::c_void;
use std::ptr;

use cgmath::{MetricSpace, Vector3};

#[derive(Copy, Clone)]
struct ChunkData<'a> {
    c: &'a Chunk,
    rendered: bool,
    amount: i32,
    vbo: u32,
    priority: i32,
}

impl<'a> Ord for ChunkData<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .priority
            .cmp(&self.priority)
            .then_with(|| self.amount.cmp(&other.amount))
    }
}

impl<'a> PartialOrd for ChunkData<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> PartialEq for ChunkData<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority || self.amount == other.amount || self.vbo == other.vbo
    }
}

impl<'a> Eq for ChunkData<'a> {}

pub struct ChunkRender<'a> {
    pub vao: u32,
    queue: BinaryHeap<ChunkData<'a>>,
    max_render_radius: f32,
}

impl<'a> ChunkRender<'a> {
    pub unsafe fn new(render_radius: f32) -> Self {
        let mut vao = 0 as u32;

        gl::GenVertexArrays(1, &mut vao);
        gl::BindVertexArray(vao);

        gl::EnableVertexAttribArray(0);
        gl::BindVertexArray(0);

        let max_render_radius =
            ((render_radius * render_radius) + (render_radius * render_radius)).sqrt();

        ChunkRender {
            vao,
            queue: BinaryHeap::new(),
            max_render_radius,
        }
    }

    pub fn add_to_queue(&mut self, chunk: &'a Chunk) {
        let mut vbo = 0;
        unsafe {
            gl::GenBuffers(1, &mut vbo);
        }
        self.queue.push(ChunkData {
            c: chunk,
            rendered: false,
            amount: 0,
            vbo,
            priority: -1,
        })
    }

    pub fn remove_from_queue(&self, vbo: &mut u32) {
        unsafe {
            gl::DeleteBuffers(1, vbo);
        }
    }

    pub unsafe fn process_queue(&mut self, cam: &Camera) {
        let mut done = BinaryHeap::new();
        while !self.queue.is_empty() {
            let mut cd = self.queue.pop().unwrap();

            let cam_in_chunk = voxel_to_chunk_pos(cam.pos, cd.c.size)
                + Vector3::new(cd.c.size / 2.0, cd.c.size / 2.0, cd.c.size / 2.0);
            cd.priority = cam_in_chunk.distance(cd.c.pos) as i32;
            /*
            println!(
                "Rendering Chunk with {} priority, max priority: {}",
                cd.priority, self.max_render_radius
            );
            */

            if !cd.rendered {
                let d = cd.c.render();
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

            let chunk_world_pos = cd.c.world_pos_min
                + Vector3::new(cd.c.size / 2.0, cd.c.size / 2.0, cd.c.size / 2.0);
            if cam.cube_in_view(chunk_world_pos, cd.c.size) {
                if cd.amount > 0 {
                    gl::BindBuffer(gl::ARRAY_BUFFER, cd.vbo);
                    let count = cd.amount;
                    gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, 0, ptr::null());
                    gl::DrawArrays(gl::POINTS, 0, count / 3);
                    gl::BindBuffer(gl::ARRAY_BUFFER, 0);
                }
            }
            if cd.priority as f32 <= self.max_render_radius {
                done.push(cd);
            } else {
                self.remove_from_queue(&mut cd.vbo);
            }
        }
        self.queue = done;
    }
}
