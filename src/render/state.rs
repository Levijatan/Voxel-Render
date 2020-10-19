use super::camera;
use super::chunk;
use super::light;
use super::texture;
use super::uniforms;
use anyhow::Result;
use std::convert::TryInto;

#[derive(Clone, Debug, PartialEq)]
pub struct Instance {
    pub position: glm::Vec3,
    pub rotation: glm::Quat,
}

impl Instance {
    #[optick_attr::profile]
    pub fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: glm::translate(&glm::quat_to_mat4(&self.rotation), &self.position),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct InstanceRaw {
    model: glm::Mat4,
}

unsafe impl bytemuck::Pod for InstanceRaw {}
unsafe impl bytemuck::Zeroable for InstanceRaw {}

#[optick_attr::profile]
pub fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    vertex_descs: &[wgpu::VertexBufferDescriptor],
    vs_src: wgpu::ShaderModuleSource,
    fs_src: wgpu::ShaderModuleSource,
) -> wgpu::RenderPipeline {
    let vs_module = device.create_shader_module(vs_src);
    let fs_module = device.create_shader_module(fs_src);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&layout),
        vertex_stage: wgpu::ProgrammableStageDescriptor {
            module: &vs_module,
            entry_point: "main",
        },
        fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
            module: &fs_module,
            entry_point: "main",
        }),
        rasterization_state: Some(wgpu::RasterizationStateDescriptor {
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: wgpu::CullMode::Back,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
            clamp_depth: false,
        }),
        primitive_topology: wgpu::PrimitiveTopology::TriangleList,
        color_states: &[wgpu::ColorStateDescriptor {
            format: color_format,
            color_blend: wgpu::BlendDescriptor::REPLACE,
            alpha_blend: wgpu::BlendDescriptor::REPLACE,
            write_mask: wgpu::ColorWrite::ALL,
        }],
        depth_stencil_state: depth_format.map(|format| wgpu::DepthStencilStateDescriptor {
            format,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilStateDescriptor::default(),
        }),
        sample_count: 1,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
        vertex_state: wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint32,
            vertex_buffers: vertex_descs,
        },
    })
}

pub struct State {
    surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub sc_desc: wgpu::SwapChainDescriptor,
    pub swap_chain: wgpu::SwapChain,
    pub camera: camera::Camera,
    pub projection: camera::Projection,
    pub light_state: light::State,
    pub uniforms_state: uniforms::State,
    pub chunk_state: chunk::State,
    #[allow(dead_code)]
    pub depth_texture: texture::Texture,
    size: winit::dpi::PhysicalSize<u32>,
}

impl State {
    #[optick_attr::profile]
    pub async fn new(window: &winit::window::Window) -> Result<Self> {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    shader_validation: true,
                },
                None,
            )
            .await?;

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let camera = camera::Camera::new(glm::vec3(0.0, 5.0, 10.0), -90.0, -20.0);
        let projection = camera::Projection::new(
            sc_desc.width as f32,
            sc_desc.height as f32,
            45.0,
            0.1,
            100.0,
        );

        let (uniforms_state, uniform_bind_group_layout) =
            uniforms::State::new(&camera, &projection, &device);

        let (light_state, light_bind_group_layout) =
            light::State::new(&device, &sc_desc, &uniform_bind_group_layout);

        let chunk_state = chunk::State::new(
            &device,
            &sc_desc,
            &light_bind_group_layout,
            &uniform_bind_group_layout,
            &queue,
        )?;

        let depth_texture =
            texture::Texture::create_depth_texture(&device, &sc_desc, "depth_texture");

        Ok(Self {
            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            camera,
            projection,
            light_state,
            chunk_state,
            uniforms_state,
            depth_texture,
            size,
        })
    }

    #[optick_attr::profile]
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.projection.resize(new_size.width, new_size.height);
        self.depth_texture = super::texture::Texture::create_depth_texture(
            &self.device,
            &self.sc_desc,
            "depth_texure",
        );
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    #[optick_attr::profile]
    pub fn update(&mut self, dt: std::time::Duration) {
        self.uniforms_state
            .update(&self.camera, &self.projection, &mut self.queue);
        self.light_state.update(&mut self.queue, &dt);
    }

    #[optick_attr::profile]
    pub fn create_command_encoder(&self) -> wgpu::CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            })
    }

    #[optick_attr::profile]
    pub fn set_instance_buffer(&mut self, instances: &Vec<Instance>, offset: u64) {
        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let raw_instance_size: u64 = std::mem::size_of::<super::state::InstanceRaw>()
            .try_into()
            .unwrap();
        let rl_offset = offset * raw_instance_size;
        self.queue.write_buffer(
            &self.chunk_state.buffer,
            rl_offset,
            bytemuck::cast_slice(&instance_data),
        );
    }
}
