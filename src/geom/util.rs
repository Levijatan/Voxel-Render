use anyhow::{anyhow, Result};
use building_blocks::prelude::{PointN, Point3};

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
pub fn normals_f32(dir: Direction) -> glm::TVec3<f32> {
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
pub fn normals_i32(dir: Direction) -> Point3<i32> {
    use Direction::{East, West, Up, Down, North, South};
    match dir {
        East => PointN([1, 0, 0]),
        West => PointN([-1, 0, 0]),
        Up => PointN([0, 1, 0]),
        Down => PointN([0, -1, 0]),
        North => PointN([0, 0, 1]),
        South => PointN([0, 0, -1]),
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
