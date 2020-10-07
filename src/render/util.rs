use crate::consts::*;

pub fn in_render_radius(pos: &crate::geom::chunk::Position) -> bool {
    let min_x = -RENDER_RADIUS;
    let max_x = RENDER_RADIUS;
    let min_y = -RENDER_RADIUS;
    let max_y = RENDER_RADIUS;
    let min_z = -RENDER_RADIUS;
    let max_z = RENDER_RADIUS;
    pos.x > min_x
        && pos.x < max_x
        && pos.y > min_y
        && pos.y < max_y
        && pos.z > min_z
        && pos.z < max_z
}
