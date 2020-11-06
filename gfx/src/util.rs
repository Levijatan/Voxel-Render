#[rustfmt::skip]
pub fn opengl_to_wgpu_matrix() -> glm::Mat4 {
    glm::Mat4::new(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 0.5, 0.0,
        0.0, 0.0, 0.5, 1.0,
    )
}
