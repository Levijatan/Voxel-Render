use anyhow::{anyhow, Result};
use cached::proc_macro::cached;

#[optick_attr::profile]
#[cached]
pub fn calc_idx(x: usize, y: usize, z: usize) -> usize {
    let size = crate::consts::CHUNK_SIZE_USIZE;
    let out = (y * size * size) + (z * size) + x;
    assert!(
        out < size * size * size,
        "Cannot use larger x:{}, y:{}, z:{} than CHUNK_SIZE:{}",
        x,
        y,
        z,
        size
    );
    out
}

#[optick_attr::profile]
pub fn calc_idx_pos(pos: &glm::Vec3) -> usize {
    let x: usize = pos.x as usize;
    let y: usize = pos.y as usize;
    let z: usize = pos.z as usize;
    calc_idx(x, y, z)
}

#[optick_attr::profile]
#[cached]
pub fn idx_to_pos(idx: usize) -> glm::Vec3 {
    let size = crate::consts::CHUNK_SIZE_USIZE;
    let i = idx;
    let x = i % size;
    let z = (i % (size * size)) / size;
    let y = i / (size * size);
    glm::vec3(x as f32, y as f32, z as f32)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    East,
    West,
    Up,
    Down,
    North,
    South,
}

impl From<Direction> for usize {
    fn from(value: Direction) -> Self {
        use Direction::{East, West, Up, Down, North, South};
        match value {
            East => 0,
            West => 1,
            Up => 2,
            Down => 3,
            North => 4,
            South => 5,
        }
    }
}

impl From<Direction> for u8 {
    fn from(value: Direction) -> Self {
        use Direction::{East, West, Up, Down, North, South};
        match value {
            East => 0b0000_0001,
            West => 0b0000_0010,
            Up => 0b0000_0100,
            Down => 0b0000_1000,
            North => 0b0001_0000,
            South => 0b0010_0000,
        }
    }
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
pub fn normals_f32(dir: Direction) -> glm::Vec3 {
    use Direction::{East, West, Up, Down, North, South};
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
pub fn normals_i32(dir: Direction) -> glm::TVec3<i32> {
    use Direction::{East, West, Up, Down, North, South};
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
pub fn reverse_direction(dir: Direction) -> Direction {
    use Direction::{East, West, Up, Down, North, South};
    match dir {
        East => West,
        West => East,
        Up => Down,
        Down => Up,
        North => South,
        South => North,
    }
}

pub fn go_left(dir: Direction) -> Result<Direction> {
    use Direction::{East, West, North, South};

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
    fn test_calc_idx() {
        let idx = calc_idx(1, 0, 15);
        assert_eq!(idx, 241);
    }

    #[test]
    fn test_idx_to_pos() {
        let idx = calc_idx(15, 1, 0);
        let pos = idx_to_pos(idx);
        assert!((pos.x - 15.0).abs() < f32::EPSILON);
        assert!((pos.y - 1.0).abs() < f32::EPSILON);
        assert!(pos.z == 0.0);
    }

    #[test]
    fn test_normals_i32() {
        let expected_pos = glm::vec3(1, 0, 0);
        let mut pos = glm::vec3(0, 0, 0);
        pos += normals_i32(Direction::East);
        assert_eq!(pos, expected_pos);
    }
}
