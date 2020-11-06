use super::camera;
use super::chunk;
use super::light;
use super::texture;
use super::uniforms;
use std::convert::TryInto;

#[derive(Clone, Debug, PartialEq)]
pub struct Instance {
    pub position: glm::Vec3,
    pub rotation: glm::Quat,
}

impl Instance {
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
        layout: Some(layout),
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



pub async fn new(window: &winit::window::Window, resource: &mut legion::Resources, chunk_size: u32, render_radius: u32) {
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
        .await.unwrap();

    let sc_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo,
    };

    let swap_chain = device.create_swap_chain(&surface, &sc_desc);

    let projection = camera::Projection::new(sc_desc.width as f32, sc_desc.height as f32, 45.0, 0.1, 1000.0);

    let camera = camera::Camera::new(
        glm::vec3(0.0, 5.0, 10.0),
        -90.0,
        -20.0,
        projection,
    );

    let (uniforms_state, uniform_bind_group_layout) = uniforms::State::new(&camera, &device);

    let (light_state, light_bind_group_layout) =
        light::State::new(&device, &sc_desc, &uniform_bind_group_layout);

    let chunk_state = chunk::State::new(
        &device,
        &sc_desc,
        &light_bind_group_layout,
        &uniform_bind_group_layout,
        &queue,
        chunk_size,
        render_radius
    ).unwrap();

    let depth_texture =
        texture::Texture::create_depth_texture(&device, &sc_desc, "depth_texture");

    resource.insert(surface);
    resource.insert(device);
    resource.insert(queue);
    resource.insert(sc_desc);
    resource.insert(swap_chain);
    resource.insert(camera);
    resource.insert(light_state);
    resource.insert(chunk_state);
    resource.insert(uniforms_state);
    resource.insert(depth_texture);
    resource.insert(size);
}

pub fn resize(new_size: winit::dpi::PhysicalSize<u32>, resource: &mut legion::Resources) {
    resource.insert(new_size);
    let depth_texture;
    let swap_chain;
    {
        let mut sc_desc = resource.get_mut::<wgpu::SwapChainDescriptor>().unwrap();
        sc_desc.width = new_size.width;
        sc_desc.height = new_size.height;
        let mut camera = resource.get_mut::<camera::Camera>().unwrap();
        camera
            .projection
            .resize(new_size.width, new_size.height);
        let device = resource.get::<wgpu::Device>().unwrap();
        depth_texture = super::texture::Texture::create_depth_texture(
            &device,
            &sc_desc,
            "depth_texure",
        );
        let surface = resource.get::<wgpu::Surface>().unwrap();
        swap_chain = device.create_swap_chain(&surface, &sc_desc);
    }
    resource.insert(depth_texture);
    resource.insert(swap_chain);
}

pub fn set_instance_buffer(queue: &wgpu::Queue, chunk_state: &chunk::State, instances: &[Instance], offset: u64) {
    let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
    let raw_instance_size: u64 = std::mem::size_of::<InstanceRaw>()
        .try_into()
        .unwrap();
    let rl_offset = offset * raw_instance_size;
    queue.write_buffer(
        &chunk_state.buffer,
        rl_offset,
        bytemuck::cast_slice(&instance_data),
    );
}
