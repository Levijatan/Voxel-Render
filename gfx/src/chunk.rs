use super::model;
use super::texture;
use anyhow::{ensure, Result};
use model::Vertex;
use std::convert::TryInto;
use std::ops::Range;

pub type BufferOffset = u32;

fn render_area_u32(render_radius: u32) -> u32 {
    let render_diameter = (render_radius * 2) + 1;
    render_diameter * render_diameter * render_diameter
}

fn render_area_u64(render_radius: u32) -> Result<u64> {
    let render_area = render_area_u32(render_radius).try_into()?;
    Ok(render_area)
}

fn buffer_offset_u32(size: u32) -> u32 {
    let tot_chunk_size = size * size * size;
    tot_chunk_size * 3
}

fn buffer_offset_u64(size: u32) -> Result<u64> {
    let offset_u32 = buffer_offset_u32(size);
    let offset: u64 = offset_u32.try_into()?;
    Ok(offset)
}

#[derive(Debug, Clone)]
pub struct Renderer {
    free_offsets: Vec<BufferOffset>,
    buffer_offset_multiplier: u32,
}

impl Renderer {
    pub fn new(size: u32) -> Self {
        let free_offsets = (0..render_area_u32(size)).collect();
        let buffer_offset_multiplier = buffer_offset_u32(size);

        Self {
            free_offsets,
            buffer_offset_multiplier,
        }
    }

    pub fn fetch_offset(&mut self) -> Option<BufferOffset> {
        let offset = self.free_offsets.pop()?;
        Some(offset * self.buffer_offset_multiplier)
    }

    pub fn return_offset(&mut self, off: BufferOffset) -> Result<()> {
        let offset = off / self.buffer_offset_multiplier;
        ensure!(
            !self.free_offsets.contains(&offset),
            "There cannot exist more than one of an offset"
        );
        self.free_offsets.push(offset);
        Ok(())
    }
}

pub struct State {
    pub voxel_model: model::Model,
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub render_pipeline: wgpu::RenderPipeline,
}

impl State {
    pub fn new(
        device: &wgpu::Device,
        sc_desc: &wgpu::SwapChainDescriptor,
        light_bind_group_layout: &wgpu::BindGroupLayout,
        uniform_bind_group_layout: &wgpu::BindGroupLayout,
        queue: &wgpu::Queue,
        size: u32,
        render_radius: u32,
    ) -> Result<Self> {
        let raw_instance_size: u64 = std::mem::size_of::<super::state::InstanceRaw>().try_into()?;
        let instance_buffer_size = render_area_u64(render_radius)? * buffer_offset_u64(size)? * raw_instance_size;

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instace Buffer"),
            size: instance_buffer_size,
            usage: wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::SampledTexture {
                            multisampled: false,
                            dimension: wgpu::TextureViewDimension::D2,
                            component_type: wgpu::TextureComponentType::Uint,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler { comparison: false },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::SampledTexture {
                            multisampled: false,
                            component_type: wgpu::TextureComponentType::Float,
                            dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler { comparison: false },
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStage::VERTEX,
                ty: wgpu::BindingType::StorageBuffer {
                    dynamic: false,
                    readonly: true,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("chunk_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(buffer.slice(..)),
            }],
            label: Some("chunk_bind_group"),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    uniform_bind_group_layout,
                    &bind_group_layout,
                    light_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let render_pipeline = super::state::create_render_pipeline(
            device,
            &render_pipeline_layout,
            sc_desc.format,
            Some(texture::Texture::DEPTH_FORMAT),
            &[model::MVertex::desc()],
            wgpu::include_spirv!("./shaders/shader.vert.spv"),
            wgpu::include_spirv!("./shaders/shader.frag.spv"),
        );

        let res_dir = std::path::Path::new(env!("OUT_DIR")).join("res");
        let voxel_model = model::Model::load(
            device,
            queue,
            &texture_bind_group_layout,
            res_dir.join("cube.obj"),
        )?;

        Ok(Self {
            voxel_model,
            buffer,
            bind_group,
            render_pipeline,
        })
    }
}


pub trait Draw<'a, 'b>
where
    'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b model::Mesh,
        material: &'b model::Material,
        chunk: &'b wgpu::BindGroup,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    );
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b model::Mesh,
        material: &'b model::Material,
        instances: Range<u32>,
        chunk: &'b wgpu::BindGroup,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    );

    fn draw_model(
        &mut self,
        model: &'b model::Model,
        chunk: &'b wgpu::BindGroup,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    );
    fn draw_model_instanced(
        &mut self,
        model: &'b model::Model,
        instances: Range<u32>,
        chunk: &'b wgpu::BindGroup,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    );
    fn draw_model_instanced_with_material(
        &mut self,
        model: &'b model::Model,
        material: &'b model::Material,
        instances: Range<u32>,
        chunk: &'b wgpu::BindGroup,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    );
    fn draw_chunk(
        &mut self,
        model: &'b model::Model,
        instances: Range<u32>,
        chunk: &'b wgpu::BindGroup,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    );
}

impl<'a, 'b> Draw<'a, 'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b model::Mesh,
        material: &'b model::Material,
        chunk: &'b wgpu::BindGroup,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        self.draw_mesh_instanced(mesh, material, 0..1, chunk, uniforms, light);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b model::Mesh,
        material: &'b model::Material,
        instances: Range<u32>,
        chunk: &'b wgpu::BindGroup,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..));
        self.set_bind_group(0, &material.bind_group, &[]);
        self.set_bind_group(1, uniforms, &[]);
        self.set_bind_group(2, chunk, &[]);
        self.set_bind_group(3, light, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

    fn draw_model(
        &mut self,
        model: &'b model::Model,
        chunk: &'b wgpu::BindGroup,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        self.draw_model_instanced(model, 0..1, chunk, uniforms, light);
    }

    fn draw_model_instanced(
        &mut self,
        model: &'b model::Model,
        instances: Range<u32>,
        chunk: &'b wgpu::BindGroup,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            let material = &model.materials[mesh.material];
            self.draw_mesh_instanced(
                mesh,
                material,
                instances.clone(),
                chunk,
                uniforms,
                light,
            );
        }
    }

    fn draw_model_instanced_with_material(
        &mut self,
        model: &'b model::Model,
        material: &'b model::Material,
        instances: Range<u32>,
        chunk: &'b wgpu::BindGroup,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            self.draw_mesh_instanced(
                mesh,
                material,
                instances.clone(),
                chunk,
                uniforms,
                light,
            );
        }
    }

    fn draw_chunk(
        &mut self,
        model: &'b model::Model,
        instances: Range<u32>,
        chunk: &'b wgpu::BindGroup,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        self.draw_model_instanced(model, instances, chunk, uniforms, light);
    }
}