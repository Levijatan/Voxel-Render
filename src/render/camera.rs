use super::consts::opengl_to_wgpu_matrix;

pub struct Frustum {
    sphere_factor_x: f32,
    sphere_factor_y: f32,
    tang: f32,
    x: glm::Vec3,
    y: glm::Vec3,
    z: glm::Vec3,
}

#[derive(PartialEq)]
pub enum FrustumPos {
    Inside,
    Outside,
    Intersects,
}

impl Frustum {
    pub fn new(p: &Projection, cam: &Camera) -> Frustum {
        let cam_target =
            cam.pos + glm::vec3(cam.yaw.cos(), cam.pitch.sin(), cam.yaw.sin()).normalize();
        let cam_dir = glm::vec3(0.0, 1.0, 0.0);
        let angle = p.fovy;
        let tang = angle.tan();
        let anglex = (tang * p.aspect).atan();
        let z = glm::normalize(&(cam.pos - cam_target));
        let x = glm::normalize(&cam_dir.cross(&z));
        Frustum {
            tang,
            sphere_factor_y: 1.0 / angle.cos(),
            sphere_factor_x: 1.0 / anglex.cos(),
            x,
            y: z.cross(&x),
            z,
        }
    }

    pub fn update(&mut self, cam: &Camera) {
        let cam_target =
            cam.pos + glm::vec3(cam.yaw.cos(), cam.pitch.sin(), cam.yaw.sin()).normalize();
        let cam_dir = glm::vec3(0.0, 1.0, 0.0);
        self.z = glm::normalize(&(cam.pos - cam_target));
        self.x = glm::normalize(&cam_dir.cross(&self.z));
        self.y = self.z.cross(&self.x);
    }

    #[allow(dead_code)]
    pub fn point(&self, p: &glm::Vec3, cam_pos: &glm::Vec3, proj: &Projection) -> FrustumPos {
        let v = p - cam_pos;

        let pcz = v.dot(&(-self.z));
        if pcz > proj.zfar || pcz < proj.znear {
            return FrustumPos::Outside;
        }

        let pcy = v.dot(&self.y);
        let mut aux = pcz * self.tang;
        if pcy > aux || pcy < -aux {
            return FrustumPos::Outside;
        }

        let pcx = v.dot(&self.x);
        aux *= proj.aspect;
        if pcx > aux || pcx < -aux {
            return FrustumPos::Outside;
        }

        FrustumPos::Inside
    }

    pub fn sphere(
        &self,
        center: &glm::Vec3,
        radius: f32,
        cam_pos: &glm::Vec3,
        proj: &Projection,
    ) -> FrustumPos {
        let v = center - cam_pos;

        let az = v.dot(&(-self.z));
        if az > proj.zfar + radius || az < proj.znear - radius {
            return FrustumPos::Outside;
        }

        let ax = v.dot(&self.x);
        let zz1 = az * self.tang * proj.aspect;
        let d1 = self.sphere_factor_x * radius;
        if ax > zz1 + d1 || az < -zz1 - d1 {
            return FrustumPos::Outside;
        }

        let ay = v.dot(&self.y);
        let zz2 = az * self.tang;
        let d2 = self.sphere_factor_y * radius;
        if ay > zz2 + d2 || ay < -zz2 - d2 {
            return FrustumPos::Outside;
        }

        if az > proj.zfar - radius || az < proj.znear + radius {
            FrustumPos::Intersects
        } else if ay > zz2 - d2 || ay < -zz2 + d2 {
            FrustumPos::Intersects
        } else if ax > zz1 - d1 || ax < -zz1 + d1 {
            FrustumPos::Intersects
        } else {
            FrustumPos::Inside
        }
    }

    pub fn cube(
        &self,
        center: &glm::Vec3,
        size: f32,
        cam_pos: &glm::Vec3,
        proj: &Projection,
    ) -> FrustumPos {
        let sphere_radius = (size / 2.0) * 1.732_051;
        self.sphere(center, sphere_radius, cam_pos, proj)
    }
}

pub struct Camera {
    pub pos: glm::Vec3,
    yaw: f32,
    pitch: f32,
}

impl Camera {
    pub fn new(pos: glm::Vec3, yaw: f32, pitch: f32) -> Camera {
        Camera {
            pos,
            yaw: yaw.to_radians(),
            pitch: pitch.to_radians(),
        }
    }

    pub fn calc_matrix(&self) -> glm::Mat4 {
        glm::look_at(
            &self.pos,
            &(self.pos + glm::vec3(self.yaw.cos(), self.pitch.sin(), self.yaw.sin()).normalize()),
            &glm::vec3(0.0, 1.0, 0.0),
        )
    }
}

pub struct Projection {
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Projection {
    pub fn new(width: u32, height: u32, fovy: f32, znear: f32, zfar: f32) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy: fovy.to_radians(),
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn calc_matrix(&self) -> glm::Mat4 {
        opengl_to_wgpu_matrix() * glm::perspective(self.aspect, self.fovy, self.znear, self.zfar)
    }
}

pub struct Controller {
    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,
    amount_up: f32,
    amount_down: f32,
    rotate_horizontal: f32,
    rotate_vertical: f32,
    scroll: f32,
    speed: f32,
    sensitivity: f32,
}

impl Controller {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            amount_left: 0.0,
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            scroll: 0.0,
            speed,
            sensitivity,
        }
    }

    pub fn process_keyboard(
        &mut self,
        key: winit::event::VirtualKeyCode,
        state: winit::event::ElementState,
    ) -> bool {
        use winit::event::ElementState;
        use winit::event::VirtualKeyCode;

        let amount = if state == ElementState::Pressed {
            1.0
        } else {
            0.0
        };
        match key {
            VirtualKeyCode::W | VirtualKeyCode::Up => {
                self.amount_forward = amount;
                true
            }
            VirtualKeyCode::S | VirtualKeyCode::Down => {
                self.amount_backward = amount;
                true
            }
            VirtualKeyCode::A | VirtualKeyCode::Left => {
                self.amount_left = amount;
                true
            }
            VirtualKeyCode::D | VirtualKeyCode::Right => {
                self.amount_right = amount;
                true
            }
            VirtualKeyCode::Space => {
                self.amount_up = amount;
                true
            }
            VirtualKeyCode::LShift => {
                self.amount_down = amount;
                true
            }
            _ => false,
        }
    }

    pub fn process_scroll(&mut self, delta: &winit::event::MouseScrollDelta) {
        self.scroll = match delta {
            // I'm assuming a line is about 100 pixels
            winit::event::MouseScrollDelta::LineDelta(_, scroll) => scroll * 100.0,
            winit::event::MouseScrollDelta::PixelDelta(winit::dpi::LogicalPosition {
                y: scroll,
                ..
            }) => *scroll as f32,
        };
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.rotate_horizontal = mouse_dx as f32;
        self.rotate_vertical = mouse_dy as f32;
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: std::time::Duration) {
        use std::f32::consts::FRAC_PI_2;

        let dt = dt.as_secs_f32();

        let (yaw_sin, yaw_cos) = camera.yaw.sin_cos();
        let forward = glm::vec3(yaw_cos, 0.0, yaw_sin).normalize();
        let right = glm::vec3(-yaw_sin, 0.0, yaw_cos).normalize();
        camera.pos += forward * (self.amount_forward - self.amount_backward) * self.speed * dt;
        camera.pos += right * (self.amount_right - self.amount_left) * self.speed * dt;

        let (pitch_sin, pitch_cos) = camera.pitch.sin_cos();
        let scrollward = glm::vec3(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
        camera.pos += scrollward * self.scroll * self.speed * self.sensitivity * dt;
        self.scroll = 0.0;

        camera.pos.y += (self.amount_up - self.amount_down) * self.speed * dt;

        camera.yaw += self.rotate_horizontal * self.sensitivity * dt;
        camera.pitch += -self.rotate_vertical * self.sensitivity * dt;

        self.rotate_horizontal = 0.0;
        self.rotate_vertical = 0.0;

        if camera.pitch < -FRAC_PI_2 {
            camera.pitch = -FRAC_PI_2;
        } else {
            if camera.pitch > FRAC_PI_2 {
                camera.pitch = FRAC_PI_2;
            }
        }
    }
}
