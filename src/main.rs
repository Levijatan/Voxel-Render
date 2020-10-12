#![warn(
    clippy::all,
    clippy::restriction,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo
)]

extern crate nalgebra_glm as glm;

use futures::executor::block_on;
use legion::*;
use winit::event::*;

mod geom;
//mod input;
mod consts;
mod render;
mod voxel_registry;

pub struct Clock {
    cur_tick: u64,
    last_tick: std::time::Instant,
}

impl Clock {
    pub fn tick(&mut self, now: std::time::Instant) -> u64 {
        let step: std::time::Duration = now - self.last_tick;
        if step.as_secs_f32() >= consts::TICK_STEP {
            self.last_tick = now;
            self.cur_tick += 1;
            println!("Tick: {:?}", self.cur_tick);
        }
        self.cur_tick
    }

    pub fn cur_tick(&self) -> u64 {
        self.cur_tick
    }
}

fn main() {
    env_logger::init();
    let event_loop = winit::event_loop::EventLoop::new();
    let title = env!("CARGO_PKG_NAME");
    let window = winit::window::WindowBuilder::new()
        .with_title(title)
        .with_inner_size(winit::dpi::PhysicalSize {
            width: consts::SCREEN_WIDTH,
            height: consts::SCREEN_HEIGHT,
        })
        .build(&event_loop)
        .unwrap();

    let mut ecs = World::default();
    let mut resources = Resources::default();
    let mut schedule_builder_every_frame = Schedule::builder();
    let mut schedule_builder_every_tick = Schedule::builder();

    let mut voxreg = voxel_registry::VoxelReg::new();
    voxreg.register_voxel_type(consts::OPAQUE_VOXEL, false);
    voxreg.register_voxel_type(consts::TRANSPARENT_VOXEL, true);

    let mut world_type_reg = geom::world::TypeRegistry::new();
    let world_type = world_type_reg.register_world_type(Box::new(geom::world::FlatWorldType {}));

    let active_world = geom::world::World {
        chunk_map: dashmap::DashMap::new(),
        world_type,
    };

    ecs.push((active_world, geom::world::Active {}));

    resources.insert(voxreg);
    resources.insert(world_type_reg);

    geom::chunk::update_transparent(&mut schedule_builder_every_frame);
    geom::chunk::update_voxel_render_system(&mut schedule_builder_every_frame);
    schedule_builder_every_frame.add_system(geom::chunk::frustum_cull_system());
    geom::chunk::culling(&mut schedule_builder_every_frame);

    geom::ticket::propagate_tickets_system(&mut schedule_builder_every_tick);
    schedule_builder_every_tick.flush();
    geom::world::generate_chunks_system(&mut schedule_builder_every_tick);
    geom::ticket::update_tickets_system(&mut schedule_builder_every_tick);
    geom::ticket::add_ticket_system(&mut schedule_builder_every_tick);

    let mut schedule_every_frame = schedule_builder_every_frame.build();
    let mut schedule_every_tick = schedule_builder_every_tick.build();

    let mut last_render_time = std::time::Instant::now();
    let mut last_tick = 0;
    let clock = Clock {
        cur_tick: last_tick,
        last_tick: last_render_time,
    };

    let state = block_on(render::state::State::new(&window));

    let frustum = render::camera::Frustum::new(&state.projection, &state.camera);
    resources.insert(state);
    resources.insert(frustum);
    resources.insert(clock);

    event_loop.run(move |event, _, control_flow| {
        use winit::event_loop::ControlFlow;
        *control_flow = ControlFlow::Poll;
        match event {
            Event::MainEventsCleared => window.request_redraw(),
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                let mut state = resources.get_mut::<render::state::State>().unwrap();
                if !state.input(event) {
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
                        WindowEvent::Resized(physiical_size) => {
                            state.resize(*physiical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            state.resize(**new_inner_size);
                        }
                        _ => {}
                    }
                }
            }
            Event::RedrawRequested(_) => {
                let tick: u64;
                {
                    let mut clock = resources.get_mut::<Clock>().unwrap();
                    let mut state = resources.get_mut::<render::state::State>().unwrap();
                    let mut frustum = resources.get_mut::<render::camera::Frustum>().unwrap();
                    let now = std::time::Instant::now();
                    let dt = now - last_render_time;
                    tick = clock.tick(now);
                    last_render_time = now;
                    state.update(dt);
                    frustum.update(&state.camera);
                    let mut query = <(
                        &geom::chunk::RenderData,
                        &geom::chunk::Render,
                        &geom::chunk::Visible,
                        &geom::ticket::Ticket,
                    )>::query();
                    let mut instances: Vec<render::state::Instance> = Vec::new();
                    for (ren, rend, _, _) in query.iter(&ecs) {
                        if !rend.culled {
                            instances.append(&mut ren.render.clone());
                        }
                    }
                    state.set_instance_buffer(instances);
                    state.render();
                }
                if tick > last_tick {
                    schedule_every_tick.execute(&mut ecs, &mut resources);
                    last_tick = tick;
                }
                schedule_every_frame.execute(&mut ecs, &mut resources);
            }
            _ => {}
        }
    });
}
