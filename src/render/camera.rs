use cgmath::{perspective, Deg, InnerSpace, Matrix4, Point3, Vector3};
use std::f32;
use std::f32::consts::PI;

pub struct Camera {
    pub pos: Point3<f32>,
    pub front: Vector3<f32>,
    pub up: Vector3<f32>,
    pub fov: f32,
    pub near_plane: f32,
    pub far_plane: f32,
    pub aspect_ratio: f32,
    speed_const: f32,
    speed: f32,
    frustum: Frustum,
    delta_time: f64,
    last_frame: f64,
}

struct Frustum {
    sphere_factor_x: f32,
    sphere_factor_y: f32,
    tang: f32,
    x: Vector3<f32>,
    y: Vector3<f32>,
    z: Vector3<f32>,
}

#[derive(PartialEq)]
enum FrustumPos {
    INSIDE,
    OUTSIDE,
    INTERSECTS,
}

impl Frustum {
    pub fn new(
        fov: f32,
        aspect_ratio: f32,
        cam_pos: Point3<f32>,
        cam_target: Point3<f32>,
        cam_dir: Vector3<f32>,
    ) -> Frustum {
        let angle = fov * (PI / 360.0);
        let tang = angle.tan();
        let anglex = (tang * aspect_ratio).atan();
        let z = (cam_pos - cam_target).normalize();
        let x = cam_dir.cross(z).normalize();
        Frustum {
            tang,
            sphere_factor_y: 1.0 / angle.cos(),
            sphere_factor_x: 1.0 / anglex.cos(),
            x,
            y: z.cross(x),
            z,
        }
    }

    pub fn update(&mut self, cam_pos: Point3<f32>, cam_target: Point3<f32>, cam_dir: Vector3<f32>) {
        self.z = (cam_pos - cam_target).normalize();
        self.x = cam_dir.cross(self.z).normalize();
        self.y = self.z.cross(self.x);
    }

    #[allow(dead_code)]
    pub fn point(
        &self,
        p: Point3<f32>,
        cam_pos: Point3<f32>,
        far_plane: f32,
        near_plane: f32,
        ratio: f32,
    ) -> FrustumPos {
        let v = p - cam_pos;

        let pcz = v.dot(-self.z);
        if pcz > far_plane || pcz < near_plane {
            return FrustumPos::OUTSIDE;
        }

        let pcy = v.dot(self.y);
        let mut aux = pcz * self.tang;
        if pcy > aux || pcy < -aux {
            return FrustumPos::OUTSIDE;
        }

        let pcx = v.dot(self.x);
        aux *= ratio;
        if pcx > aux || pcx < -aux {
            return FrustumPos::OUTSIDE;
        }

        FrustumPos::INSIDE
    }

    pub fn sphere(
        &self,
        center: Point3<f32>,
        radius: f32,
        cam_pos: Point3<f32>,
        far_plane: f32,
        near_plane: f32,
        ratio: f32,
    ) -> FrustumPos {
        let v = center - cam_pos;

        let az = v.dot(-self.z);
        if az > far_plane + radius || az < near_plane - radius {
            return FrustumPos::OUTSIDE;
        }

        let ax = v.dot(self.x);
        let zz1 = az * self.tang * ratio;
        let d1 = self.sphere_factor_x * radius;
        if ax > zz1 + d1 || az < -zz1 - d1 {
            return FrustumPos::OUTSIDE;
        }

        let ay = v.dot(self.y);
        let zz2 = az * self.tang;
        let d2 = self.sphere_factor_y * radius;
        if ay > zz2 + d2 || ay < -zz2 - d2 {
            return FrustumPos::OUTSIDE;
        }

        if az > far_plane - radius || az < near_plane + radius {
            FrustumPos::INTERSECTS
        } else if ay > zz2 - d2 || ay < -zz2 + d2 {
            FrustumPos::INTERSECTS
        } else if ax > zz1 - d1 || ax < -zz1 + d1 {
            FrustumPos::INTERSECTS
        } else {
            FrustumPos::INSIDE
        }
    }

    pub fn cube(
        &self,
        center: Point3<f32>,
        size: f32,
        cam_pos: Point3<f32>,
        far_plane: f32,
        near_plane: f32,
        ratio: f32,
    ) -> FrustumPos {
        let sphere_radius = (size / 2.0) * 1.732051;
        return self.sphere(center, sphere_radius, cam_pos, far_plane, near_plane, ratio);
    }
}

impl Camera {
    pub fn new(
        pos: Point3<f32>,
        up: Vector3<f32>,
        speed: f32,
        fov: f32,
        near_plane: f32,
        far_plane: f32,
        aspect_ratio: f32,
    ) -> Camera {
        let front = Vector3::new(0.0, 0.0, -1.0);
        return Camera {
            pos,
            front,
            up,
            speed_const: speed,
            fov,
            near_plane,
            far_plane,
            aspect_ratio,
            frustum: Frustum::new(fov, aspect_ratio, pos, pos + front, up),
            delta_time: 0.0,
            last_frame: 0.0,
            speed: 0.0,
        };
    }

    pub fn update(&mut self, time: f64) {
        let current_frame = time;
        self.delta_time = current_frame - self.last_frame;
        self.last_frame = current_frame;
        self.speed = self.speed_const * self.delta_time as f32;
    }

    pub fn view(&self) -> Matrix4<f32> {
        Matrix4::look_at(self.pos, self.pos + self.front, self.up)
    }

    pub fn projection(&self) -> Matrix4<f32> {
        return perspective(
            Deg(self.fov),
            self.aspect_ratio,
            self.near_plane,
            self.far_plane,
        );
    }

    pub fn rotate(&mut self, radius: f32, angle: f32) {
        let cam_x = angle.sin() * radius;
        let cam_z = angle.cos() * radius;
        self.pos.x = cam_x;
        self.pos.z = cam_z;
        self.update_frustum();
    }

    pub fn update_frustum(&mut self) {
        self.frustum
            .update(self.pos, self.pos + self.front, self.up);
    }

    pub fn move_forward(cam: &mut Camera) {
        cam.pos += cam.front * cam.speed;
        cam.update_frustum();
    }

    pub fn move_back(cam: &mut Camera) {
        cam.pos -= cam.front * cam.speed;
        cam.update_frustum();
    }

    pub fn move_left(cam: &mut Camera) {
        cam.pos -= cam.front.cross(cam.up).normalize() * cam.speed;
        cam.update_frustum();
    }

    pub fn move_right(cam: &mut Camera) {
        cam.pos += cam.front.cross(cam.up).normalize() * cam.speed;
        cam.update_frustum();
    }

    #[allow(dead_code)]
    pub fn point_in_view(&self, p: Point3<f32>) -> bool {
        self.frustum.point(
            p,
            self.pos,
            self.far_plane,
            self.near_plane,
            self.aspect_ratio,
        ) == FrustumPos::INSIDE
    }

    #[allow(dead_code)]
    pub fn sphere_in_view(&self, center: Point3<f32>, radius: f32) -> bool {
        self.frustum.sphere(
            center,
            radius,
            self.pos,
            self.far_plane,
            self.near_plane,
            self.aspect_ratio,
        ) != FrustumPos::OUTSIDE
    }

    #[allow(dead_code)]
    pub fn cube_in_view(&self, center: Point3<f32>, size: f32) -> bool {
        self.frustum.cube(
            center,
            size,
            self.pos,
            self.far_plane,
            self.near_plane,
            self.aspect_ratio,
        ) != FrustumPos::OUTSIDE
    }
}
