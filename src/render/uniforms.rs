use super::camera;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Uniforms {
    view_position: glm::Vec4,
    view_proj: glm::Mat4,
}

impl Uniforms {
    fn new() -> Self {
        Self {
            view_position: glm::Vec4::new(0.0, 0.0, 0.0, 0.0),
            view_proj: glm::identity(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &camera::Camera, projection: &camera::Projection) {
        self.view_position = camera.pos.to_homogeneous();
        self.view_proj = projection.calc_matrix() * camera.calc_matrix();
    }
}

unsafe impl bytemuck::Pod for Uniforms {}
unsafe impl bytemuck::Zeroable for Uniforms {}

pub struct State {
    pub uniforms: Uniforms,
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl State {
    pub fn new(
        cam: &camera::Camera,
        proj: &camera::Projection,
        device: &wgpu::Device,
    ) -> (Self, wgpu::BindGroupLayout) {
        use wgpu::util::DeviceExt as _;
        let mut uniforms = Uniforms::new();
        uniforms.update_view_proj(cam, proj);

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                ty: wgpu::BindingType::UniformBuffer {
                    dynamic: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("uniform_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(buffer.slice(..)),
            }],
            label: Some("uniform_bind_group"),
        });

        (
            Self {
                uniforms,
                buffer,
                bind_group,
            },
            bind_group_layout,
        )
    }

    pub fn update(
        &mut self,
        cam: &camera::Camera,
        proj: &camera::Projection,
        queue: &mut wgpu::Queue,
    ) {
        self.uniforms.update_view_proj(cam, proj);
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniforms]));
    }
}
