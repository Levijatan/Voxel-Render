#![warn(
    clippy::all,
    clippy::restriction,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo
)]

extern crate nalgebra_glm as glm;

use flame as f;
use flamer::flame;
use legion::*;

use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use futures::executor::block_on;

use std::fs::File;

mod geom;
//mod input;
mod consts;
mod render;
mod voxel_registry;

#[flame]
fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let title = env!("CARGO_PKG_NAME");
    let window = WindowBuilder::new()
        .with_title(title)
        .build(&event_loop)
        .unwrap();

    let mut ecs = World::default();
    let mut resources = Resources::default();
    let mut schedule_builder = Schedule::builder();

    let mut voxreg = voxel_registry::VoxelReg::new();
    voxreg.register_voxel_type(consts::OPAQUE_VOXEL, false);
    voxreg.register_voxel_type(consts::TRANSPARENT_VOXEL, true);

    let mut world_type_reg = geom::world::WorldTypeRegistry::new();
    let world_type = world_type_reg.register_world_type(Box::new(geom::world::FlatWorldType {}));

    let active_world = geom::world::World {
        chunk_map: dashmap::DashMap::new(),
        world_type,
        active: true,
    };

    let cmd = systems::CommandBuffer::new(&ecs);

    let world_id = ecs.push((active_world,));

    resources.insert(voxreg);
    resources.insert(world_type_reg);

    geom::world::generate_chunks_system(&mut schedule_builder);
    schedule_builder.add_system(geom::chunk::update_transparency_system());
    schedule_builder.add_system(geom::chunk::update_chunk_render_system());
    geom::chunk::should_render_system(&mut schedule_builder);
    geom::chunk::should_not_render(&mut schedule_builder);

    let mut schedule = schedule_builder.build();

    let mut last_render_time = std::time::Instant::now();
    let mut state = block_on(render::State::new(&window));
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::MainEventsCleared => window.request_redraw(),
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                if !state.input(event) {
                    match event {
                        WindowEvent::CloseRequested => {
                            *control_flow = ControlFlow::Exit;
                            f::dump_html(File::create("flamegraph.html").unwrap()).unwrap();
                        }
                        WindowEvent::KeyboardInput { input, .. } => match input {
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            } => {
                                *control_flow = ControlFlow::Exit;
                                f::dump_html(File::create("flamegraph.html").unwrap()).unwrap();
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
                use consts::RENDER_RADIUS;
                schedule.execute(&mut ecs, &mut resources);
                let now = std::time::Instant::now();
                let dt = now - last_render_time;
                last_render_time = now;
                state.update(dt);
                let mut query = <(&geom::chunk::RenderData, &geom::chunk::Render)>::query();
                let mut instances: Vec<render::Instance> = Vec::new();
                for (ren, _) in query.iter(&ecs) {
                    instances.append(&mut ren.render.clone());
                }
                state.set_instance_buffer(instances);
                state.render();
            }
            _ => {}
        }
    });
}
