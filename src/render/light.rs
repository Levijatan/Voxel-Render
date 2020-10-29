use super::model;
use super::state;
use super::texture;

use std::ops::Range;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Light {
    pub position: glm::Vec3,
    _padding: u32,
    color: glm::Vec3,
}

unsafe impl bytemuck::Zeroable for Light {}
unsafe impl bytemuck::Pod for Light {}

pub struct State {
    pub light: Light,
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub render_pipeline: wgpu::RenderPipeline,
}

impl State {
    #[optick_attr::profile]
    pub fn new(
        device: &wgpu::Device,
        sc_desc: &wgpu::SwapChainDescriptor,
        uniform_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> (Self, wgpu::BindGroupLayout) {
        use crate::render::model::Vertex as _;
        use wgpu::util::DeviceExt as _;

        let light = Light {
            position: glm::Vec3::new(2.0, -8.0, 2.0),
            _padding: 0,
            color: glm::Vec3::new(1.0, 1.0, 1.0),
        };

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light VB"),
            contents: bytemuck::cast_slice(&[light]),
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
            label: None,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(buffer.slice(..)),
            }],
            label: None,
        });

        let render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Light Pipeline Layout"),
                bind_group_layouts: &[uniform_bind_group_layout, &bind_group_layout],
                push_constant_ranges: &[],
            });

            state::create_render_pipeline(
                device,
                &layout,
                sc_desc.format,
                Some(texture::Texture::DEPTH_FORMAT),
                &[model::MVertex::desc()],
                wgpu::include_spirv!("../shaders/light.vert.spv"),
                wgpu::include_spirv!("../shaders/light.frag.spv"),
            )
        };

        (
            Self {
                light,
                buffer,
                bind_group,
                render_pipeline,
            },
            bind_group_layout,
        )
    }

    #[optick_attr::profile]
    pub fn update(&mut self, queue: &mut wgpu::Queue, dt: &std::time::Duration) {
        let old_position = self.light.position;
        self.light.position = glm::quat_rotate_vec3(
            &glm::quat_angle_axis(5.0 * dt.as_secs_f32(), &glm::Vec3::new(0.0, 1.0, 0.0)),
            &old_position,
        );
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.light]));
    }
}

pub trait Draw<'a, 'b>
where
    'b: 'a,
{
    fn draw_light_mesh(
        &mut self,
        mesh: &'b model::Mesh,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    );

    fn draw_light_mesh_instaced(
        &mut self,
        mesh: &'b model::Mesh,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) where
        'b: 'a;

    fn draw_light_model(
        &mut self,
        model: &'b model::Model,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    );

    fn draw_light_model_instanced(
        &mut self,
        model: &'b model::Model,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    );
}

impl<'a, 'b> Draw<'a, 'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    #[optick_attr::profile]
    fn draw_light_mesh(
        &mut self,
        mesh: &'b model::Mesh,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        self.draw_light_mesh_instaced(mesh, 0..1, uniforms, light);
    }

    #[optick_attr::profile]
    fn draw_light_mesh_instaced(
        &mut self,
        mesh: &'b model::Mesh,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..));
        self.set_bind_group(0, uniforms, &[]);
        self.set_bind_group(1, light, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

    #[optick_attr::profile]
    fn draw_light_model(
        &mut self,
        model: &'b model::Model,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        self.draw_light_model_instanced(model, 0..1, uniforms, light);
    }

    #[optick_attr::profile]
    fn draw_light_model_instanced(
        &mut self,
        model: &'b model::Model,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            self.draw_light_mesh_instaced(mesh, instances.clone(), uniforms, light);
        }
    }
}
