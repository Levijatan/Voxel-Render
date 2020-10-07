use glm::Vec3;

use flamer::flame;

use crate::consts::CHUNK_SIZE;

#[flame("geom::util")]
pub fn voxel_to_chunk_pos(voxel_pos: &Vec3) -> Vec3 {
    let size = CHUNK_SIZE as f32;
    let x = (voxel_pos.x / size).floor();
    let y = (voxel_pos.y / size).floor();
    let z = (voxel_pos.z / size).floor();
    return Vec3::new(x, y, z);
}

#[flame("geom::util")]
pub fn calc_idx(x: usize, y: usize, z: usize) -> usize {
    let out = (z * CHUNK_SIZE * CHUNK_SIZE) + (x * CHUNK_SIZE) + y;
    if out >= CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE {
        panic!(
            "Cannot use larger x:{}, y:{} ,z:{} than size:{}",
            x, y, z, CHUNK_SIZE
        );
    }
    out
}

pub fn calc_idx_pos(pos: &Vec3) -> usize {
    calc_idx(pos.x as usize, pos.y as usize, pos.z as usize)
}

#[flame("geom::util")]
pub fn idx_to_pos(idx: usize) -> Vec3 {
    let i = idx;
    let y = i % CHUNK_SIZE;
    let x = (i % (CHUNK_SIZE * CHUNK_SIZE)) / CHUNK_SIZE;
    let z = i / (CHUNK_SIZE * CHUNK_SIZE);
    Vec3::new(x as f32, y as f32, z as f32)
}

#[flame("geom::util")]
pub fn normals(i: i32) -> Vec3 {
    match i {
        0 => Vec3::new(1.0, 0.0, 0.0),
        1 => Vec3::new(-1.0, 0.0, 0.0),
        2 => Vec3::new(0.0, 1.0, 0.0),
        3 => Vec3::new(0.0, -1.0, 0.0),
        4 => Vec3::new(0.0, 0.0, 1.0),
        5 => Vec3::new(0.0, 0.0, -1.0),
        _ => panic!("Not valid use"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voxel_to_chunk_pos() {
        let pos = Vec3::new(17.0, 0.0, 14.0);
        let chunk_pos = voxel_to_chunk_pos(&pos);
        assert_eq!(chunk_pos, Vec3::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn test_calc_idx() {
        let idx = calc_idx(15, 1, 0);
        assert_eq!(idx, 241);
    }

    #[test]
    fn test_idx_to_pos() {
        let idx = calc_idx(15, 1, 0);
        let pos = idx_to_pos(idx);
        assert_eq!(pos.x, 15.0);
        assert_eq!(pos.y, 1.0);
        assert_eq!(pos.z, 0.0);
    }
}
