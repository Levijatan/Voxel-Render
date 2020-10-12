#[allow(dead_code)]
pub fn voxel_to_chunk_pos(voxel_pos: &glm::Vec3) -> glm::Vec3 {
    let size = crate::consts::CHUNK_SIZE_F32;
    let x = (voxel_pos.x / size).floor();
    let y = (voxel_pos.y / size).floor();
    let z = (voxel_pos.z / size).floor();
    glm::vec3(x, y, z)
}

pub fn calc_idx(x: usize, y: usize, z: usize) -> usize {
    let size = crate::consts::CHUNK_SIZE_USIZE;
    let out = (z * size * size) + (x * size) + y;
    if out >= size * size * size {
        panic!(
            "Cannot use larger x:{}, y:{} ,z:{} than size:{}",
            x, y, z, size
        );
    }
    out
}

pub fn calc_idx_pos(pos: &glm::Vec3) -> usize {
    calc_idx(pos.x as usize, pos.y as usize, pos.z as usize)
}

pub fn idx_to_pos(idx: usize) -> glm::Vec3 {
    let size = crate::consts::CHUNK_SIZE_USIZE;
    let i = idx;
    let y = i % size;
    let x = (i % (size * size)) / size;
    let z = i / (size * size);
    glm::vec3(x as f32, y as f32, z as f32)
}

#[derive(Clone, Debug, PartialEq)]
pub enum Direction {
    East,
    West,
    Up,
    Down,
    North,
    South,
}

pub const ALL_DIRECTIONS: [Direction; 6] = [
    Direction::East,
    Direction::West,
    Direction::Up,
    Direction::Down,
    Direction::North,
    Direction::South,
];

pub fn normals_f32(dir: &Direction) -> glm::Vec3 {
    use Direction::*;
    match dir {
        East => glm::vec3(1.0, 0.0, 0.0),
        West => glm::vec3(-1.0, 0.0, 0.0),
        Up => glm::vec3(0.0, 1.0, 0.0),
        Down => glm::vec3(0.0, -1.0, 0.0),
        North => glm::vec3(0.0, 0.0, 1.0),
        South => glm::vec3(0.0, 0.0, -1.0),
    }
}

pub fn normals_i64(dir: &Direction) -> glm::TVec3<i64> {
    use Direction::*;
    match dir {
        East => glm::vec3(1, 0, 0),
        West => glm::vec3(-1, 0, 0),
        Up => glm::vec3(0, 1, 0),
        Down => glm::vec3(0, -1, 0),
        North => glm::vec3(0, 0, 1),
        South => glm::vec3(0, 0, -1),
    }
}

pub fn reverse_direction(dir: &Direction) -> Direction {
    use Direction::*;
    match dir {
        East => West,
        West => East,
        Up => Down,
        Down => Up,
        North => South,
        South => North,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voxel_to_chunk_pos() {
        let pos = glm::vec3(17.0, 0.0, 14.0);
        let chunk_pos = voxel_to_chunk_pos(&pos);
        assert_eq!(chunk_pos, glm::vec3(1.0, 0.0, 0.0));
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
