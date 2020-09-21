use glm::Vec3;

pub fn voxel_to_chunk_pos(voxel_pos: &Vec3, chunk_size: usize) -> Vec3 {
    let size = chunk_size as f32;
    let x = (voxel_pos.x / size).floor();
    let y = (voxel_pos.y / size).floor();
    let z = (voxel_pos.z / size).floor();
    return Vec3::new(x, y, z);
}

pub fn calc_idx(x: usize, y: usize, z: usize, size: usize) -> usize {
    let out = (z * size * size) + (x * size) + y;
    if out >= size * size * size {
        panic!("Cannot use larger x,y,z than size");
    }
    out
}

pub fn idx_to_pos(idx: usize, size: usize) -> Vec3 {
    let i = idx;
    let y = i % size;
    let x = (i % (size * size)) / size;
    let z = i / (size * size);
    Vec3::new(x as f32, y as f32, z as f32)
}

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
        let chunk_pos = voxel_to_chunk_pos(&pos, 16);
        assert_eq!(chunk_pos, Vec3::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn test_calc_idx() {
        let idx = calc_idx(15, 1, 0, 16);
        assert_eq!(idx, 241);
    }

    #[test]
    fn test_idx_to_pos() {
        let idx = calc_idx(15, 1, 0, 16);
        let pos = idx_to_pos(idx, 16);
        assert_eq!(pos.x, 15.0);
        assert_eq!(pos.y, 1.0);
        assert_eq!(pos.z, 0.0);
    }
}
