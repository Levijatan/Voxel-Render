use anyhow::{anyhow, ensure, Result};

#[optick_attr::profile]
pub fn voxel_to_chunk_pos(voxel_pos: &glm::Vec3) -> glm::Vec3 {
    let size = crate::consts::CHUNK_SIZE_F32;
    let x = (voxel_pos.x / size).floor();
    let y = (voxel_pos.y / size).floor();
    let z = (voxel_pos.z / size).floor();
    glm::vec3(x, y, z)
}

#[optick_attr::profile]
pub fn calc_idx(x: usize, y: usize, z: usize) -> Result<usize> {
    let size = crate::consts::CHUNK_SIZE_USIZE;
    let out = (y * size * size) + (z * size) + x;
    ensure!(
        out < size * size * size,
        "Cannot use larger x:{}, y:{}, z:{} than CHUNK_SIZE:{}",
        x,
        y,
        z,
        size
    );
    Ok(out)
}

#[optick_attr::profile]
pub fn calc_idx_pos(pos: &glm::Vec3) -> Result<usize> {
    let x: usize = pos.x as usize;
    let y: usize = pos.y as usize;
    let z: usize = pos.z as usize;
    let out = calc_idx(x, y, z)?;
    Ok(out)
}

#[optick_attr::profile]
pub fn idx_to_pos(idx: usize) -> glm::Vec3 {
    let size = crate::consts::CHUNK_SIZE_USIZE;
    let i = idx;
    let x = i % size;
    let z = (i % (size * size)) / size;
    let y = i / (size * size);
    glm::vec3(x as f32, y as f32, z as f32)
}

#[repr(usize)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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

#[optick_attr::profile]
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

#[optick_attr::profile]
pub fn normals_i32(dir: &Direction) -> glm::TVec3<i32> {
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

#[optick_attr::profile]
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

pub fn go_left(dir: &Direction) -> Result<Direction> {
    use Direction::*;

    match dir {
        North => Ok(West),
        West => Ok(South),
        South => Ok(East),
        East => Ok(North),
        _ => Err(anyhow!("No left in 3 dimensions")),
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
    fn test_calc_idx() -> Result<()> {
        let idx = calc_idx(15, 1, 0)?;
        assert_eq!(idx, 241);
        Ok(())
    }

    #[test]
    fn test_idx_to_pos() -> Result<()> {
        let idx = calc_idx(15, 1, 0)?;
        let pos = idx_to_pos(idx);
        assert_eq!(pos.x, 15.0);
        assert_eq!(pos.y, 1.0);
        assert_eq!(pos.z, 0.0);
        Ok(())
    }

    #[test]
    fn test_normals_i32() {
        let expected_pos = glm::vec3(1, 0, 0);
        let mut pos = glm::vec3(0, 0, 0);
        pos += normals_i32(&Direction::East);
        assert_eq!(pos, expected_pos);
    }
}
