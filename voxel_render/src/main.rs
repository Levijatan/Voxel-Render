#![warn(clippy::all)]

extern crate nalgebra_glm as glm;

use anyhow::Result;
use futures::executor::block_on;
use legion::{Read, Write, Resources, Schedule, World, WorldOptions, storage, component, IntoQuery};
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};

use gfx::chunk::Draw as _;
use gfx::light::Draw as _;

mod consts;
use engine::{input, clock, ticket};
use geom::{voxel, chunk};

fn main() -> Result<()> {
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

    let input = input::State::new();

    let mut chunk_group = storage::GroupDef::new();
    chunk_group.add(storage::ComponentTypeId::of::<geom::chunk::Position>());

    let world_options = WorldOptions {
        groups: vec![chunk_group],
    };

    let mut ecs = World::new(world_options);
    let mut resources = Resources::default();

    let mut voxreg = geom::voxel::Registry::new();
    voxreg.register_voxel_type(voxel::OPAQUE_VOXEL, false);
    voxreg.register_voxel_type(voxel::TRANSPARENT_VOXEL, true);

    let mut world_type_reg = geom::world::TypeRegistry::<gfx::chunk::BufferOffset>::new();
    let world_type = world_type_reg.register_world_type(Box::new(geom::world::FlatWorldType {}));

    let (active_world,) = geom::world::Map::<gfx::chunk::BufferOffset>::create(world_type);
    ecs.push((active_world, geom::world::Active {}));

    let clock = clock::Clock::new();

    block_on(gfx::state::new(&window, &mut resources, chunk::CHUNK_SIZE_U32, consts::RENDER_RADIUS as u32));

    let chunk_renderer = gfx::chunk::Renderer::new(consts::RENDER_RADIUS as u32);

    resources.insert(voxreg);
    resources.insert(world_type_reg);
    resources.insert(clock);
    resources.insert(chunk_renderer);
    resources.insert(input);

    let mut schedule_builder = Schedule::builder();
    ticket::Ticket::systems(&mut schedule_builder, consts::RENDER_RADIUS);
    update_every_tick_system(&mut schedule_builder);
    render_system(&mut schedule_builder);
    update_system(&mut schedule_builder);
    

    let mut schedule = schedule_builder.build();

    event_loop.run(move |event, _, control_flow| {
        use winit::event_loop::ControlFlow;
        *control_flow = ControlFlow::Poll;
        match event {
            Event::MainEventsCleared => window.request_redraw(),
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                let mut input = resources.get_mut::<input::State>().unwrap();
                if !input.input(event) {
                    match event {
                        WindowEvent::CloseRequested => {
                            *control_flow = ControlFlow::Exit;
                        }
                        WindowEvent::KeyboardInput { input, .. } => match input {
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            } => {
                                *control_flow = ControlFlow::Exit;
                            }
                            _ => {}
                        },
                        WindowEvent::Resized(physical_size) => {
                            drop(input);
                            gfx::state::resize(*physical_size, &mut resources);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            drop(input);
                            gfx::state::resize(**new_inner_size, &mut resources);
                        }
                        _ => {}
                    }
                }
            }
            Event::RedrawRequested(_) => {

                schedule.execute(&mut ecs, &mut resources);

            }
            _ => {}
        }
    });
}

fn update_every_tick_system(schedule_builder: &mut legion::systems::Builder) {
    schedule_builder.add_system(legion::SystemBuilder::new("UpdateEveryTickSystem")
        .write_resource::<clock::Clock>()
        .build(|_, _, clock, _| {
            if clock.cur_tick() > clock.last_tick() {
                clock.tick_done();
            }
        }),
    );
}

fn update_system(schedule_builder: &mut legion::systems::Builder) {
    schedule_builder.add_system(legion::SystemBuilder::new("UpdateSystem")
        .write_resource::<clock::Clock>()
        .write_resource::<gfx::uniforms::State>()
        .write_resource::<gfx::light::State>()
        .write_resource::<input::State>()
        .write_resource::<wgpu::Queue>()
        .write_resource::<gfx::camera::Camera>()
        .build(|_, _, (clock, uniform, light, input, queue, camera), _| {
            clock.tick();
            let dt = clock.delta();
            uniform.update(camera, queue);
            light.update(queue, &dt);
            input.update(camera, dt);
        }),
    );
}

fn render_system(schedule_builder: &mut legion::systems::Builder) {
    schedule_builder.add_thread_local(legion::SystemBuilder::new("RenderSystem")
        .with_query(<(Read<geom::world::Id>, Read<ticket::Ticket>)>::query())
        .with_query(<(geom::world::Id, Write<geom::world::Map<gfx::chunk::BufferOffset>>)>::query().filter(component::<geom::world::Active>()))
        .read_resource::<wgpu::Device>()
        .read_resource::<gfx::chunk::State>()
        .read_resource::<gfx::light::State>()
        .read_resource::<gfx::uniforms::State>()
        .read_resource::<gfx::camera::Camera>()
        .read_resource::<gfx::texture::Texture>()
        .write_resource::<wgpu::SwapChain>()
        .write_resource::<wgpu::Queue>()
        .build(|_, ecs, (device, chunk_state, light_state, uniforms_state, camera, depth_texture, swap_chain, queue), (ticket_query, world_query)| {
            let frame = swap_chain.get_current_frame().expect("Timeout getting texture").output;
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
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
                            attachment: &depth_texture.view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: true,
                            }),
                            stencil_ops: None,
                        },
                    ),
                });

                render_pass.set_pipeline(&light_state.render_pipeline);
                render_pass.draw_light_model(
                    &chunk_state.voxel_model,
                    &uniforms_state.bind_group,
                    &light_state.bind_group,
                );

                render_pass.set_pipeline(&chunk_state.render_pipeline);

                let (mut world_ecs, ticket_ecs) = ecs.split_for_query(world_query); 
                world_query.for_each_mut(&mut world_ecs, |(world_id, map)| {
                    ticket_query.for_each(&ticket_ecs, |(ticket_world_id, ticket)| {
                        if world_id == ticket_world_id {
                            for key in map.chunk_map.key_iter(&ticket.extent()) {
                                if let Some(chunk) = map.chunk_map.get_mut_chunk(key) {
                                    if chunk.metadata.is_visible() {
                                        if let Some(offset) = chunk.metadata.render_offset() {
                                            if camera.cube_in_view(&geom::chunk::calc_center_point(key), geom::chunk::calc_radius()) {
                                                render_pass.draw_chunk(
                                                    &chunk_state.voxel_model,
                                                    offset..(chunk.metadata.render_amount as u32 + offset),
                                                    &chunk_state.bind_group,
                                                    &uniforms_state.bind_group,
                                                    &light_state.bind_group,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    })
                });
            }
            queue.submit(std::iter::once(encoder.finish()));
        }),
    );
}