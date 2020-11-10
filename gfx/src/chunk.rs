use super::model;
use super::texture;
use super::state;
use super::buffer;
use state::Instance;
use model::Vertex;

use anyhow::Result;
use thiserror::Error;
use std::convert::TryInto;
use std::ops::Range;
use std::collections::{VecDeque, HashMap};
use std::sync::{atomic::{AtomicU32, Ordering}, Arc, Mutex, RwLock};


pub fn render_area_u32(render_radius: u32) -> u32 {
    let render_diameter = (render_radius * 2) + 1;
    render_diameter * render_diameter * render_diameter
}

fn render_area_u64(render_radius: u32) -> Result<u64> {
    let render_area = render_area_u32(render_radius).try_into()?;
    Ok(render_area)
}

pub fn buffer_offset_u32(size: u32) -> u32 {
    let tot_chunk_size = size * size * size;
    tot_chunk_size * 3
}

fn buffer_offset_u64(size: u32) -> Result<u64> {
    let offset_u32 = buffer_offset_u32(size);
    let offset: u64 = offset_u32.try_into()?;
    Ok(offset)
}

type QueueEntityId = u32;
pub type QueueChunkId = u32;

struct QueueEntity {
    offset: u32,
    total_amount: u32,
    chunk_meta: VecDeque<(QueueChunkId, Vec<Instance>)>,
}

#[derive(Error, Debug)]
enum RenderQueueError {
    #[error("There is not a QueueEntity in RenderQueue.render that has enough free space to facilitate chunk")]
    NoFreeSlotsLargeEnough,
}

pub enum RenderQueueCommand{
    Insert(QueueChunkId, Vec<Instance>),
    Update(QueueChunkId, Vec<Instance>),
}

struct QueueChunk {
    id: QueueChunkId,
    in_entity: QueueEntityId,
}

impl PartialEq for QueueChunk {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

pub struct RenderQueue {
    render: VecDeque<QueueEntityId>,
    process: Arc<Mutex<Vec<RenderQueueCommand>>>,
    queue_entities: HashMap<QueueEntityId, QueueEntity>,
    chunks: Arc<RwLock<Vec<QueueChunk>>>,
    next_chunk_id: AtomicU32,
    next_entity_id: AtomicU32,
    next_free: usize,
    max_each_entity: u32,
}

impl RenderQueue {
    pub fn new(offset_controller: &mut buffer::OffsetControllerU32) -> Self {
        let next_entity_id = AtomicU32::new(0);
        let mut render = VecDeque::new();
        let mut queue_entities = HashMap::new();
        let queue_entity = QueueEntity {
            offset: offset_controller.fetch_offset().unwrap(),
            total_amount: 0,
            chunk_meta: VecDeque::new(),
        };
        let queue_entity_id = next_entity_id.fetch_add(1, Ordering::Relaxed);
        queue_entities.insert(queue_entity_id, queue_entity);
        render.push_back(queue_entity_id);
        
        Self{
            render,
            process: Arc::new(Mutex::new(Vec::new())),
            queue_entities,
            chunks: Arc::new(RwLock::new(Vec::new())),
            next_chunk_id: AtomicU32::new(0),
            next_entity_id,
            next_free: 0,
            max_each_entity: offset_controller.multiplier(),
        }
    }

    pub fn add_chunk(&self, instances: Vec<Instance>) -> QueueChunkId {
        let out = self.fetch_chunk_id();
        let mut process = self.process.lock().unwrap();
        process.push(RenderQueueCommand::Insert(out, instances));
        out
    }

    pub fn in_queue(&self, value: QueueChunkId) -> bool {
        self.chunks.read().unwrap().iter().any(|chunk| chunk.id == value)
    }

    fn fetch_chunk_id(&self) -> QueueChunkId {
        self.next_chunk_id.fetch_add(1, Ordering::Relaxed)
    }

    fn fetch_entity_id(&self) -> QueueEntityId {
        self.next_entity_id.fetch_add(1, Ordering::Relaxed)
    }

    fn process_queue(&mut self, offset_controller: &mut buffer::OffsetControllerU32, gpu_queue: &wgpu::Queue, chunk_state: &State) {

        let mut process: Vec<RenderQueueCommand>;
        {
            process = self.process.lock().unwrap().drain(..).collect();
        }
        for command in process.drain(..) {
            match command {
                RenderQueueCommand::Insert(chunk_id, instances) => self.insert_chunk(chunk_id, instances, offset_controller, gpu_queue, chunk_state),
                RenderQueueCommand::Update(chunk_id, instances) => {
                    if let Some(entity_id) = self.find_entity_id(chunk_id) {
                        if let Some(entity) = self.queue_entities.get_mut(&entity_id) {
                            if entity.chunk_meta.len() == 1 {
                                state::set_instance_buffer(gpu_queue, chunk_state, &instances, entity.offset.into());
                                entity.total_amount = instances.len() as u32;
                                let (_, ins) = &mut entity.chunk_meta[0];
                                *ins = instances;
                            }
                            let idx;
                            for i in 0..entity.chunk_meta.len() {
                                let (c_id, _) = &entity.chunk_meta[i];
                                if *c_id == chunk_id {
                                    idx = i;
                                    break;
                                }
                            }
                        }
                    } else {
                        self.insert_chunk(chunk_id, instances, offset_controller, gpu_queue, chunk_state);
                    }
                },
            }
        }

        for idx in self.next_free..self.render.len() {
            if let Some(queue_entity) = self.render.get(idx) {
                if self.queue_entities.get(queue_entity).unwrap().total_amount < self.max_each_entity {
                    self.next_free = idx;
                    break;
                }
            }
        }
    }

    fn find_entity_id(&self, value: QueueChunkId) -> Option<QueueEntityId> {
        if let Ok(chunks) = self.chunks.read() {
            let chunk = chunks.iter().find(|chunk| chunk.id == value)?;
            Some(chunk.in_entity)
        } else {
            None
        }
    }

    fn insert_chunk(&mut self, chunk_id: QueueChunkId, instances: Vec<Instance>, offset_controller: &mut buffer::OffsetControllerU32, gpu_queue: &wgpu::Queue, chunk_state: &State) {
        let queue_entity_id = match self.insert_into_existing_queue_entities(chunk_id, instances.clone(), gpu_queue, chunk_state) {
            Err(RenderQueueError::NoFreeSlotsLargeEnough) => self.insert_into_new_queue_entity(chunk_id, instances, offset_controller, gpu_queue, chunk_state),
            Ok(id) => id,
        };
        let mut chunks = self.chunks.write().unwrap();
        if self.in_queue(chunk_id) {
            chunks.iter_mut().for_each(|chunk| {
                if chunk.id == chunk_id {
                    chunk.in_entity = queue_entity_id;
                }
            });
        } else {
            chunks.push(QueueChunk{id: chunk_id, in_entity: queue_entity_id});
        }
    }

    fn insert_into_existing_queue_entities(&mut self, chunk_id: QueueChunkId, instances: Vec<Instance>, gpu_queue: &wgpu::Queue, chunk_state: &State) -> Result<QueueEntityId, RenderQueueError> {
        let amount = instances.len() as u32;
        for idx in self.next_free..self.render.len() {
            if let Some(queue_entity_id) = self.render.get(idx) {
                let mut queue_entity = self.queue_entities.get_mut(queue_entity_id).unwrap();
                if self.max_each_entity - queue_entity.total_amount >= amount {
                    Self::load_data_to_buffer(queue_entity, &instances, gpu_queue, chunk_state);
                    queue_entity.total_amount += amount;
                    queue_entity.chunk_meta.push_back((chunk_id, instances));
                    return Ok(*queue_entity_id)
                }
            }
        }
        Err(RenderQueueError::NoFreeSlotsLargeEnough)
    }

    fn insert_into_new_queue_entity(&mut self, chunk_id: QueueChunkId, instances: Vec<Instance>, offset_controller: &mut buffer::OffsetControllerU32, gpu_queue: &wgpu::Queue, chunk_state: &State) -> QueueEntityId {
        let queue_entity_id = self.fetch_entity_id();
        let mut chunk_meta = VecDeque::new();
        let amount = instances.len() as u32;
        let mut queue_entity = QueueEntity{
            offset: offset_controller.fetch_offset().unwrap(),
            total_amount: 0,
            chunk_meta,
        };
        Self::load_data_to_buffer(&queue_entity, &instances, gpu_queue, chunk_state);
        queue_entity.chunk_meta.push_back((chunk_id, instances));
        queue_entity.total_amount += amount;
        self.queue_entities.insert(queue_entity_id, queue_entity);
        self.render.push_back(queue_entity_id);
        queue_entity_id
    }

    fn load_data_to_buffer(queue_entity: &QueueEntity, instances: &[Instance], gpu_queue: &wgpu::Queue, chunk_state: &State) {
        state::set_instance_buffer(gpu_queue, chunk_state, instances, (queue_entity.offset + queue_entity.total_amount).into());
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
