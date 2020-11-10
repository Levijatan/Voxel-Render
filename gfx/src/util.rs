
/// Returns a `nalgebra_glm::Mat4` containing the values to go from a left handed NDC coordinate system to a right handed one
#[rustfmt::skip]
#[allow(clippy::must_use_candidate)]
pub fn opengl_to_wgpu_matrix() -> glm::Mat4 {
    glm::Mat4::new(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 0.5, 0.0,
        0.0, 0.0, 0.5, 1.0,
    )
}
