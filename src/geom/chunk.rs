use cgmath::{EuclideanSpace, Point3, Vector3};

use super::Voxel;
use super::NORMALS;

pub struct Chunk {
    pub size: f32,
    pub tot_size: f32,
    pub v: Vec<Voxel>,
    pub pos: Point3<f32>,
    pub render: bool,
    pub world_pos_min: Point3<f32>,
    pub world_pos_max: Point3<f32>,
}

impl Chunk {
    pub fn new(size: f32, x: f32, y: f32, z: f32) -> Chunk {
        let rx = x * size;
        let ry = y * size;
        let rz = z * size;

        let world_pos_min = Point3::new(rx, ry, rz);

        let mut c = Chunk {
            tot_size: (size * size * size),
            size,
            v: Vec::new(),
            pos: Point3::new(x, y, z),
            render: true,
            world_pos_min,
            world_pos_max: world_pos_min + Vector3::new(size as f32, size as f32, size as f32),
        };

        for z in 0..size as i32 {
            for y in 0..size as i32 {
                for x in 0..size as i32 {
                    c.v.push(Voxel::new(
                        true,
                        rx + x as f32,
                        ry + y as f32,
                        rz + z as f32,
                    ));
                }
            }
        }

        return c;
    }

    pub fn in_chunk(&self, pos: Point3<f32>) -> bool {
        return self.world_pos_min.x <= pos.x
            && pos.x < self.world_pos_max.x
            && self.world_pos_min.y <= pos.y
            && pos.y < self.world_pos_max.y
            && self.world_pos_min.z <= pos.z
            && pos.z < self.world_pos_max.z;
    }

    pub fn render(&self) -> Vec<&Voxel> {
        let mut out = Vec::new();
        for i in 0..self.tot_size as i32 {
            let v = &self.v[i as usize];
            if v.render {
                out.push(v);
            }
        }

        return out;
    }

    pub fn update_voxel(&mut self, idx: usize, render: bool) {
        //println!("{}", render);
        self.v[idx].update(render);
    }

    pub fn set_voxel(&mut self, v: Voxel) {
        let in_chunk_pos = self.in_chunk_pos(v.pos);
        let idx = self.calc_idx_point(in_chunk_pos);
        self.set_voxel_idx(idx as usize, v);
    }

    pub fn set_voxel_idx(&mut self, idx: usize, v: Voxel) {
        self.v[idx] = v;
    }

    pub fn chunk_normal_from_voxel_pos(&self, pos: Point3<f32>) -> Vector3<f32> {
        if pos.x >= self.world_pos_max.x {
            return NORMALS[0];
        } else if pos.x < self.world_pos_min.x {
            return NORMALS[1];
        } else if pos.y >= self.world_pos_max.y {
            return NORMALS[2];
        } else if pos.y < self.world_pos_min.y {
            return NORMALS[3];
        } else if pos.z >= self.world_pos_max.z {
            return NORMALS[4];
        } else if pos.z < self.world_pos_min.z {
            return NORMALS[5];
        } else {
            return Vector3::new(0.0, 0.0, 0.0);
        }
    }

    pub fn calc_idx_vect(&self, vect: Vector3<f32>) -> f32 {
        return self.calc_idx(vect.x, vect.y, vect.z);
    }

    pub fn calc_idx_point(&self, point: Point3<f32>) -> f32 {
        return self.calc_idx(point.x, point.y, point.z);
    }

    pub fn calc_idx(&self, x: f32, y: f32, z: f32) -> f32 {
        return (self.size as f32 * self.size as f32 * z) + (self.size as f32 * y) + x;
    }

    pub fn in_chunk_pos(&self, voxel_pos: Point3<f32>) -> Point3<f32> {
        return Point3::from_vec(voxel_pos - self.world_pos_min);
    }
}
