#![warn(
    clippy::all,
    clippy::restriction,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo
)]

extern crate nalgebra_glm as glm;

use anyhow::Result;
use futures::executor::block_on;
use legion::*;
use log::info;
use std::collections::HashMap;
use winit::event::*;

use render::chunk::Draw as _;
use render::light::Draw as _;

mod geom;
//mod input;
mod consts;
mod input;
mod render;
mod voxel_registry;

pub struct Clock {
    cur_tick: u64,
    last_tick: std::time::Instant,
    last_render: std::time::Instant,
    delta: std::time::Duration,
}

impl Clock {
    #[optick_attr::profile]
    pub fn tick(&mut self) -> u64 {
        let now = std::time::Instant::now();

        self.delta = now - self.last_render;
        self.last_render = now;
        let step = now - self.last_tick;
        if step.as_secs_f32() >= consts::TICK_STEP {
            self.last_tick = now;
            self.cur_tick += 1;
            info!(
                "Tick: {} at tps: {}",
                self.cur_tick,
                1.0 / step.as_secs_f32()
            );
        }
        self.cur_tick
    }

    pub fn cur_tick(&self) -> u64 {
        self.cur_tick
    }

    pub fn delta(&self) -> std::time::Duration {
        self.delta
    }
}

fn main() -> Result<()> {
    optick::start_capture();
    env_logger::init();
    let event_loop = winit::event_loop::EventLoop::new();
    let title = env!("CARGO_PKG_NAME");
    let window = winit::window::WindowBuilder::new()
        .with_title(title)
        .with_inner_size(winit::dpi::PhysicalSize {
            width: consts::SCREEN_WIDTH,
            height: consts::SCREEN_HEIGHT,
        })
        .build(&event_loop)?;

    let mut input = input::State::new();

    let mut chunk_group = storage::GroupDef::new();
    chunk_group.add(storage::ComponentTypeId::of::<geom::chunk::Position>());
    chunk_group.add(storage::ComponentTypeId::of::<geom::chunk::Data>());

    let world_options = WorldOptions {
        groups: vec![chunk_group],
    };

    let mut ecs = World::new(world_options);
    let mut resources = Resources::default();
    let mut schedule_builder_every_frame = Schedule::builder();
    let mut schedule_builder_every_tick = Schedule::builder();

    let mut voxreg = voxel_registry::VoxelReg::new();
    voxreg.register_voxel_type(consts::OPAQUE_VOXEL, false);
    voxreg.register_voxel_type(consts::TRANSPARENT_VOXEL, true);

    let mut world_type_reg = geom::world::TypeRegistry::new();
    let world_type = world_type_reg.register_world_type(Box::new(geom::world::FlatWorldType {}));

    let active_world = geom::world::World {
        chunk_map: HashMap::new(),
        world_type,
    };

    ecs.push((active_world, geom::world::Active {}));

    resources.insert(voxreg);
    resources.insert(world_type_reg);

    render::chunk::remove_system(&mut schedule_builder_every_frame);
    render::chunk::add_system(&mut schedule_builder_every_frame);

    geom::world::generate_chunks_system(&mut schedule_builder_every_tick);
    geom::ticket::update_tickets_system(&mut schedule_builder_every_tick);
    geom::ticket::add_ticket_system(&mut schedule_builder_every_tick);
    geom::ticket::propagate_tickets_system(&mut schedule_builder_every_tick);
    geom::chunk::update_transparent(&mut schedule_builder_every_tick);
    geom::chunk::update_voxel_render_system(&mut schedule_builder_every_tick);
    geom::chunk::culling(&mut schedule_builder_every_tick);

    let mut schedule_every_frame = schedule_builder_every_frame.build();
    let mut schedule_every_tick = schedule_builder_every_tick.build();

    let mut last_tick = 0;
    let clock = Clock {
        cur_tick: last_tick,
        last_tick: std::time::Instant::now(),
        last_render: std::time::Instant::now(),
        delta: std::time::Duration::default(),
    };

    let state = block_on(render::state::State::new(&window))?;

    let mut frustum = render::camera::Frustum::new(&state.projection, &state.camera);
    let chunk_renderer = render::chunk::Renderer::new();

    resources.insert(clock);
    resources.insert(chunk_renderer);
    resources.insert(state);

    event_loop.run(move |event, _, control_flow| {
        use winit::event_loop::ControlFlow;
        *control_flow = ControlFlow::Poll;
        match event {
            Event::MainEventsCleared => window.request_redraw(),
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                if !input.input(event) {
                    match event {
                        WindowEvent::CloseRequested => {
                            optick::stop_capture("Voxel_Render");
                            *control_flow = ControlFlow::Exit;
                        }
                        WindowEvent::KeyboardInput { input, .. } => match input {
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            } => {
                                optick::stop_capture("Voxel_Render");
                                *control_flow = ControlFlow::Exit;
                            }
                            _ => {}
                        },
                        WindowEvent::Resized(physiical_size) => {
                            let mut state = resources.get_mut::<render::state::State>().unwrap();
                            state.resize(*physiical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            let mut state = resources.get_mut::<render::state::State>().unwrap();
                            state.resize(**new_inner_size);
                        }
                        _ => {}
                    }
                }
            }
            Event::RedrawRequested(_) => {
                optick::next_frame();

                let tick: u64;
                let dt: std::time::Duration;
                let frame: wgpu::SwapChainTexture;

                {
                    let mut clock = resources.get_mut::<Clock>().unwrap();
                    tick = clock.tick();
                    dt = clock.delta();

                    let mut state = resources.get_mut::<render::state::State>().unwrap();

                    state.update(dt);
                    input.update(&mut state.camera, dt);
                    frustum.update(&state.camera);
                    frame = state
                        .swap_chain
                        .get_current_frame()
                        .expect("Timeout getting texture")
                        .output;
                }

                if tick > last_tick {
                    schedule_every_tick.execute(&mut ecs, &mut resources);
                    last_tick = tick;
                }
                schedule_every_frame.execute(&mut ecs, &mut resources);

                let state = resources.get::<render::state::State>().unwrap();
                let mut encoder = state.create_command_encoder();

                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                            attachment: &frame.view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.1,
                                    g: 0.2,
                                    b: 0.3,
                                    a: 1.0,
                                }),
                                store: true,
                            },
                        }],
                        depth_stencil_attachment: Some(
                            wgpu::RenderPassDepthStencilAttachmentDescriptor {
                                attachment: &state.depth_texture.view,
                                depth_ops: Some(wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(1.0),
                                    store: true,
                                }),
                                stencil_ops: None,
                            },
                        ),
                    });

                    render_pass.set_pipeline(&state.light_state.render_pipeline);
                    render_pass.draw_light_model(
                        &state.chunk_state.voxel_model,
                        &state.uniforms_state.bind_group,
                        &state.light_state.bind_group,
                    );

                    let mut query = <(
                        Read<geom::chunk::Position>,
                        Read<geom::chunk::Visible>,
                        Read<geom::ticket::Ticket>,
                        Read<render::chunk::Component>,
                    )>::query();

                    render_pass.set_pipeline(&state.chunk_state.render_pipeline);

                    let chunk_size = consts::CHUNK_SIZE_F32 * consts::VOXEL_SIZE;
                    let half_size = chunk_size / 2.0;
                    for (pos, _, _, ren) in query.iter(&ecs) {
                        let voxel_pos: glm::Vec3 = (pos.get_f32_pos() * chunk_size)
                            - glm::vec3(half_size, half_size, half_size);
                        assert!(voxel_pos.x != std::f32::NAN);
                        assert!(voxel_pos.y != std::f32::NAN);
                        assert!(voxel_pos.z != std::f32::NAN);
                        if frustum.cube(&voxel_pos, half_size, &state.camera.pos, &state.projection)
                            != crate::render::camera::FrustumPos::Outside
                        {
                            render_pass.draw_chunk(
                                &state.chunk_state.voxel_model,
                                ren.offset..(ren.amount + ren.offset),
                                &state.chunk_state.bind_group,
                                &state.uniforms_state.bind_group,
                                &state.light_state.bind_group,
                                0,
                            );
                        }
                    }
                }
                state.queue.submit(std::iter::once(encoder.finish()));
            }
            _ => {}
        }
    });
}
