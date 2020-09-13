use cgmath::Point3;

pub fn check_start_stop(start: Point3<f32>, stop: Point3<f32>) -> (Point3<f32>, Point3<f32>) {
    let mut out_start = Point3::new(0.0, 0.0, 0.0);
    let mut out_stop = Point3::new(0.0, 0.0, 0.0);

    if start.x < stop.x {
        out_start.x = start.x;
        out_stop.x = stop.x;
    } else {
        out_start.x = stop.x;
        out_stop.x = start.x;
    }

    if start.y < stop.y {
        out_start.y = start.y;
        out_stop.y = stop.y;
    } else {
        out_start.y = stop.y;
        out_stop.y = start.y;
    }

    if start.z < stop.z {
        out_start.z = start.z;
        out_stop.z = stop.z;
    } else {
        out_start.z = stop.z;
        out_stop.z = start.z;
    }

    return (out_start, out_stop);
}

pub fn check_start_stop_to_i32(
    start: Point3<f32>,
    stop: Point3<f32>,
) -> (Point3<i32>, Point3<i32>) {
    let (t_start, t_stop) = check_start_stop(start, stop);
    return (pos_f32_to_i32(t_start), pos_f32_to_i32(t_stop));
}

pub fn pos_f32_to_i32(pos: Point3<f32>) -> Point3<i32> {
    return Point3::new(pos.x as i32, pos.y as i32, pos.z as i32);
}

pub fn voxel_to_chunk_pos(voxel_pos: Point3<f32>, chunk_size: f32) -> Point3<f32> {
    let x = (voxel_pos.x / chunk_size).floor();
    let y = (voxel_pos.y / chunk_size).floor();
    let z = (voxel_pos.z / chunk_size).floor();
    return Point3::new(x, y, z);
}

pub fn idx_to_pos(idx: usize, size: f32) -> Point3<f32> {
    let i = idx as f32;
    let x = i % size;
    let y = (i % (size * size)) / size;
    let z = i / (size * size);
    Point3::new(x, y, z)
}
