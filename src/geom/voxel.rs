use cgmath::Point3;

#[derive(Debug)]
pub struct Voxel {
    pub render: bool,
    pub transparent: bool,
    pub pos: Point3<f32>,
}

impl Voxel {
    pub fn new(transparent: bool, x: f32, y: f32, z: f32) -> Voxel {
        return Voxel {
            render: false,
            transparent,
            pos: Point3::new(x, y, z),
        };
    }

    pub fn update(&mut self, render: bool) {
        self.render = render;
    }
}
