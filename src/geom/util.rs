use glm::Vec3;

pub fn check_start_stop(start: Vec3, stop: Vec3) -> (Vec3, Vec3) {
    let mut out_start = Vec3::new(0.0, 0.0, 0.0);
    let mut out_stop = Vec3::new(0.0, 0.0, 0.0);

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

pub fn voxel_to_chunk_pos(voxel_pos: Vec3, chunk_size: usize) -> Vec3 {
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
