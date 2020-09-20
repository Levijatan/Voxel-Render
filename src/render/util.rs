use glm::Vec3;

pub fn min_max_norm(val: f32, min: f32, max: f32) -> f32 {
    (val - min) / (max - min)
}

pub fn min_max_norm_range(val: f32, min: f32, max: f32, range_min: f32, range_max: f32) -> f32 {
    let n = min_max_norm(val, min, max);
    n * (range_max - range_min) + range_min
}

pub fn min_max_norm_range_point(
    val: Vec3,
    min: f32,
    max: f32,
    range_min: f32,
    range_max: f32,
) -> Vec3 {
    let x = min_max_norm_range(val.x, min, max, range_min, range_max);
    let y = min_max_norm_range(val.y, min, max, range_min, range_max);
    let z = min_max_norm_range(val.z, min, max, range_min, range_max);
    Vec3::new(x, y, z)
}

pub fn min_max_norm_range_vector(
    val: Vec3,
    min: f32,
    max: f32,
    range_min: f32,
    range_max: f32,
) -> Vec3 {
    let x = min_max_norm_range(val.x, min, max, range_min, range_max);
    let y = min_max_norm_range(val.y, min, max, range_min, range_max);
    let z = min_max_norm_range(val.z, min, max, range_min, range_max);
    Vec3::new(x, y, z)
}
