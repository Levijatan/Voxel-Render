#![warn(
    clippy::all,
    clippy::restriction,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo
)]

extern crate glfw;
use self::glfw::{Action, Context, Key};

extern crate gl;

use cgmath::{Point3, SquareMatrix, Vector2, Vector3};
use std::ffi::CString;
use std::sync::mpsc::Receiver;

mod consts;
mod geom;
mod input;
mod render;
mod shader;
mod voxel_registry;

use input::KeyState;
use render::Camera;
use render::ChunkRender;
use shader::Shader;
use voxel_registry::VoxelReg;

const SCREEN_WIDTH: u32 = 2560;
const SCREEN_HEIGHT: u32 = 1440;
const GL_MAJOR_VERSION: u32 = 4;
const GL_MINOR_VERSION: u32 = 4;
const WINDOW_NAME: &'static str = "Voxel Renderer";

const VOXEL_SIZE: f32 = 1.0;
const CHUNK_SIZE: u32 = 16;

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

    //GL init
    gl::load_with(|symbol| window.get_proc_address(symbol) as *const _);

    let program = Shader::new("src/shaders/raybox.vert", "src/shaders/colored.frag");

    //Setings init
    let screen_size = Vector2::<f32>::new(SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32);
    let mut cam = Camera::new(
        Point3 {
            x: 0.0,
            y: 5.0,
            z: -1.0,
        },
        Vector3 {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
        20.0,
        70.0,
        0.1,
        100.0,
        SCREEN_WIDTH as f32 / SCREEN_HEIGHT as f32,
    );
    let mut voxreg = VoxelReg::new();
    voxreg.register_voxel_type(consts::OPAQUE_VOXEL, false);
    voxreg.register_voxel_type(consts::TRANSPARENT_VOXEL, true);

    //Camera Movement
    let mut keys = KeyState::new();

    keys.add_state(Key::W, Camera::move_forward);
    keys.add_state(Key::A, Camera::move_left);
    keys.add_state(Key::S, Camera::move_back);
    keys.add_state(Key::D, Camera::move_right);
    keys.add_state(Key::Space, Camera::move_up);
    keys.add_state(Key::LeftShift, Camera::move_down);

    //World Gen
    let mut pc = geom::PointCloud::new(CHUNK_SIZE as f32);

    pc.create_cube(
        Point3::new(-256.0, 0.0, -256.0),
        Point3::new(256.0, 1.0, 256.0),
        &voxreg,
    );
    pc.update(&voxreg);
    let render_data = pc.render();

    //Render setup
    let mut cr;
    unsafe {
        cr = ChunkRender::new(32.0);
        program.use_program();
        gl::ClearColor(1.0, 1.0, 1.0, 1.0);
        gl::Enable(gl::DEPTH_TEST);
        gl::Enable(gl::PROGRAM_POINT_SIZE);
    }

    for rd in render_data {
        cr.add_to_queue(rd);
    }

    while !window.should_close() {
        //Events
        process_events(&mut window, &events, &mut keys);
        cam.update(glfw.get_time());
        keys.process_all_states(&mut cam);

        //Render
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            gl::BindVertexArray(cr.vao);

            let mv = cam.view();
            let p = cam.projection();
            let mvp = p * mv;
            let inv_p = p.invert().unwrap();
            let inv_mv = mv.invert().unwrap();

            program.set_float(&CString::new("voxelSize").unwrap(), VOXEL_SIZE);
            program.set_mat4(&CString::new("mvp").unwrap(), &mvp);
            program.set_mat4(&CString::new("invP").unwrap(), &inv_p);
            program.set_mat4(&CString::new("invMv").unwrap(), &inv_mv);
            program.set_vector2(&CString::new("screenSize").unwrap(), &screen_size);

            cr.process_queue(&cam);
        }
        window.swap_buffers();
        glfw.poll_events();
    }
}

fn process_events(
    window: &mut glfw::Window,
    events: &Receiver<(f64, glfw::WindowEvent)>,
    ks: &mut KeyState,
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

            glfw::WindowEvent::Key(Key::W, _, Action::Press, _) => {
                ks.set_state(Key::W, true);
            }
            glfw::WindowEvent::Key(Key::W, _, Action::Release, _) => {
                ks.set_state(Key::W, false);
            }
            glfw::WindowEvent::Key(Key::A, _, Action::Press, _) => {
                ks.set_state(Key::A, true);
            }
            glfw::WindowEvent::Key(Key::A, _, Action::Release, _) => {
                ks.set_state(Key::A, false);
            }
            glfw::WindowEvent::Key(Key::S, _, Action::Press, _) => {
                ks.set_state(Key::S, true);
            }
            glfw::WindowEvent::Key(Key::S, _, Action::Release, _) => {
                ks.set_state(Key::S, false);
            }
            glfw::WindowEvent::Key(Key::D, _, Action::Press, _) => {
                ks.set_state(Key::D, true);
            }
            glfw::WindowEvent::Key(Key::D, _, Action::Release, _) => {
                ks.set_state(Key::D, false);
            }

            glfw::WindowEvent::Key(Key::Space, _, Action::Press, _) => {
                ks.set_state(Key::Space, true);
            }
            glfw::WindowEvent::Key(Key::Space, _, Action::Release, _) => {
                ks.set_state(Key::Space, false);
            }

            glfw::WindowEvent::Key(Key::LeftShift, _, Action::Press, _) => {
                ks.set_state(Key::LeftShift, true);
            }
            glfw::WindowEvent::Key(Key::LeftShift, _, Action::Release, _) => {
                ks.set_state(Key::LeftShift, false);
            }

            _ => {}
        }
    }
}
