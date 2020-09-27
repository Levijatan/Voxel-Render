#![warn(
    clippy::all,
    clippy::restriction,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo
)]
extern crate dashmap;
extern crate gl;
extern crate glfw;
extern crate image;
extern crate nalgebra_glm as glm;

extern crate flame;

extern crate flamer;

use flame as f;
use flamer::flame;

use glfw::{Action, Context, Key};
use glm::{Vec2, Vec3};

use std::ffi::CString;
use std::fs::File;
use std::sync::mpsc::Receiver;
use std::sync::{mpsc, Arc, RwLock};

mod chunk_gen;
mod chunk_updater;
mod consts;
mod geom;
mod input;
mod render;
mod shader;
mod texture;
mod voxel_registry;
mod world;

use chunk_gen::ChunkGen;
use chunk_updater::ChunkTicket;
use chunk_updater::ChunkUpdater;
use geom::ChunkKey;
use input::CursorState;
use input::KeyState;
use render::Camera;
use render::ChunkRender;
use shader::Shader;
use texture::generate_texture;
use voxel_registry::Material;
use voxel_registry::VoxelReg;
use world::FlatWorldType;
use world::World;
use world::WorldRegistry;
use world::WorldTypeRegistry;

const SCREEN_WIDTH: u32 = 2560;
const SCREEN_HEIGHT: u32 = 1440;
const GL_MAJOR_VERSION: u32 = 4;
const GL_MINOR_VERSION: u32 = 4;
const WINDOW_NAME: &'static str = "Voxel Renderer";

const VOXEL_SIZE: f32 = 1.0;
const CHUNK_SIZE: usize = 16;

#[derive(Clone)]
pub struct SharedState {
    voxel_registry: Arc<VoxelReg>,
    world_type_registry: Arc<WorldTypeRegistry>,
    world_registry: Arc<WorldRegistry>,
    tick: Arc<RwLock<u32>>,
    active_world: Arc<RwLock<u64>>,
    cam_chunk_pos: Arc<RwLock<Vec3>>,
    clear_render: Arc<RwLock<bool>>,
    chunk_size: Arc<usize>,
}

#[flame]
fn main() {
    //GLFW init
    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();
    glfw.window_hint(glfw::WindowHint::ContextVersion(
        GL_MAJOR_VERSION,
        GL_MINOR_VERSION,
    ));
    glfw.window_hint(glfw::WindowHint::OpenGlProfile(
        glfw::OpenGlProfileHint::Core,
    ));

    let (mut window, events) = glfw
        .create_window(
            SCREEN_WIDTH,
            SCREEN_HEIGHT,
            WINDOW_NAME,
            glfw::WindowMode::Windowed,
        )
        .expect("Failed to create GLFW window");

    window.make_current();
    window.set_key_polling(true);
    window.set_framebuffer_size_polling(true);
    window.set_cursor_mode(glfw::CursorMode::Disabled);
    window.set_cursor_pos_polling(true);

    //GL init
    gl::load_with(|symbol| window.get_proc_address(symbol) as *const _);

    let program = Shader::new("src/shaders/raybox.vert", "src/shaders/colored.frag");

    //Setings init
    let screen_size = Vec2::new(SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32);
    let mut cam = Camera::new(
        glm::vec3(0.0, 5.0, 0.0),
        glm::vec3(0.0, 1.0, 0.0),
        20.0,
        70.0,
        0.001,
        1000.0,
        SCREEN_WIDTH as f32 / SCREEN_HEIGHT as f32,
    );
    let mut voxreg = VoxelReg::new();
    voxreg.register_voxel_type(
        consts::OPAQUE_VOXEL,
        false,
        Material {
            ambient: Vec3::new(1.0, 1.0, 1.0),
            diffuse: Vec3::new(0.8, 0.8, 0.8),
            specular: Vec3::new(0.5, 0.8, 0.1),
            shininess: 0.1,
        },
    );
    voxreg.register_voxel_type(
        consts::TRANSPARENT_VOXEL,
        true,
        Material {
            ambient: Vec3::new(0.0, 0.0, 0.0),
            diffuse: Vec3::new(0.0, 0.0, 0.0),
            specular: Vec3::new(0.0, 0.0, 0.0),
            shininess: 0.0,
        },
    );

    let mut world_type_reg = WorldTypeRegistry::new();
    world_type_reg.register_world_type(Box::new(FlatWorldType {
        chunk_size: CHUNK_SIZE,
    }));

    let mut world_reg = WorldRegistry::new();
    let active_world = world_reg.new_world(World::new(true, CHUNK_SIZE, 1));

    let shared_state = SharedState {
        voxel_registry: Arc::new(voxreg),
        world_type_registry: Arc::new(world_type_reg),
        world_registry: Arc::new(world_reg),
        tick: Arc::new(RwLock::new(1)),
        cam_chunk_pos: Arc::new(RwLock::new(cam.chunk_pos(CHUNK_SIZE))),
        active_world: Arc::new(RwLock::new(active_world)),
        clear_render: Arc::new(RwLock::new(true)),
        chunk_size: Arc::new(CHUNK_SIZE),
    };

    //Camera Movement
    let mut keys = KeyState::new();
    let mut cursor = CursorState::new(SCREEN_WIDTH as f32 / 2.0, SCREEN_HEIGHT as f32 / 2.0, 10.0);

    keys.add_state(Key::W, Camera::move_forward);
    keys.add_state(Key::A, Camera::move_left);
    keys.add_state(Key::S, Camera::move_back);
    keys.add_state(Key::D, Camera::move_right);
    keys.add_state(Key::Space, Camera::move_up);
    keys.add_state(Key::LeftShift, Camera::move_down);

    //World Gen
    let (tx_chunk_gen, rx_chunk_gen) = mpsc::channel();
    let (tx_chunk_ticket, rx_chunk_ticket) = mpsc::channel();
    let (tx_render, rx_render) = mpsc::channel();
    ChunkGen::init(shared_state.clone(), rx_chunk_gen);
    ChunkUpdater::init(
        rx_chunk_ticket,
        tx_render,
        tx_chunk_gen,
        shared_state.clone(),
    );

    //Render setup
    let mut renderer: ChunkRender;

    unsafe {
        renderer = ChunkRender::new(&shared_state, rx_render);
        program.use_program();
        gl::ClearColor(1.0, 1.0, 1.0, 1.0);
        gl::Enable(gl::DEPTH_TEST);
        gl::Enable(gl::PROGRAM_POINT_SIZE);
    }

    generate_texture("src/texture/T_UV_Map.jpg".to_string());

    let ticks_per_second = 20.0;
    let tick_step = 1.0 / ticks_per_second;

    let mut last_time = 0.0;
    let mut last_ticket_tick = 0;

    while !window.should_close() {
        {
            let cur_time = glfw.get_time();
            cam.update(glfw.get_time());

            if last_time + tick_step <= cur_time {
                println!(
                    "cur time: {}, last time {}, step size {}, current step {}",
                    cur_time,
                    last_time,
                    tick_step,
                    cur_time - last_time
                );
                let mut tick = shared_state.tick.write().unwrap();
                *tick = tick.wrapping_add(1);

                let mut cam_chunk_pos = shared_state.cam_chunk_pos.write().unwrap();
                *cam_chunk_pos = cam.chunk_pos(CHUNK_SIZE);
                last_time = cur_time;
            }

            if *shared_state.tick.read().unwrap() >= last_ticket_tick + 20 {
                let cam_chunk_pos = cam.chunk_pos(CHUNK_SIZE);
                let key = ChunkKey::new(cam_chunk_pos);
                tx_chunk_ticket
                    .send(ChunkTicket::new(
                        key,
                        5,
                        20,
                        *shared_state.active_world.read().unwrap(),
                    ))
                    .unwrap();
                last_ticket_tick = *shared_state.tick.read().unwrap();
            }

            //Events
            process_events(&mut window, &events, &mut keys, &mut cursor, &mut cam);
            keys.process_all_states(&mut cam);

            //Render
            unsafe {
                gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
                gl::BindVertexArray(renderer.vao);

                let mv = cam.view();
                let p = cam.projection();
                let mvp = p * mv;
                let inv_p = glm::inverse(&p);
                let inv_mv = glm::inverse(&mv);

                program.set_float(&CString::new("voxelSize").unwrap(), VOXEL_SIZE);
                program.set_mat4(&CString::new("mvp").unwrap(), &mvp);
                program.set_mat4(&CString::new("invP").unwrap(), &inv_p);
                program.set_mat4(&CString::new("invMv").unwrap(), &inv_mv);
                program.set_vec2(&CString::new("screenSize").unwrap(), &screen_size);
                renderer.process(&cam);
            }
        }
        window.swap_buffers();
        glfw.poll_events();
    }
    f::dump_html(File::create("flamegraph.html").unwrap()).unwrap();
}

#[flame]
fn process_events(
    window: &mut glfw::Window,
    events: &Receiver<(f64, glfw::WindowEvent)>,
    ks: &mut KeyState,
    cs: &mut CursorState,
    cam: &mut Camera,
) {
    for (_, event) in glfw::flush_messages(events) {
        match event {
            glfw::WindowEvent::FramebufferSize(width, height) => {
                // make sure the viewport matches the new window dimensions; note that width and
                // height will be significantly larger than specified on retina displays.
                unsafe { gl::Viewport(0, 0, width, height) }
            }
            glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                window.set_should_close(true)
            }

            glfw::WindowEvent::Key(_, _, _, _) => {
                if let glfw::WindowEvent::Key(key, _, action, _) = event {
                    if action == Action::Press {
                        ks.set_state(key, true);
                    } else if action == Action::Release {
                        ks.set_state(key, false)
                    }
                }
            }

            glfw::WindowEvent::CursorPos(_, _) => {
                if let glfw::WindowEvent::CursorPos(x, y) = event {
                    cs.process(x as f32, y as f32, cam);
                }
            }

            _ => {}
        }
    }
}
