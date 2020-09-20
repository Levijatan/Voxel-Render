use std::ffi::{CStr, CString};
use std::fs::File;
use std::io::Read;
use std::ptr;
use std::str;

use gl::types::*;

use glm::{Mat4, Vec2, Vec3};

pub struct Shader {
    pub id: u32,
}

impl Shader {
    pub fn new(vertex_path: &str, fragment_path: &str) -> Shader {
        let mut shader = Shader { id: 0 };

        let mut v_shader_file =
            File::open(vertex_path).unwrap_or_else(|_| panic!("Failed to open {}", vertex_path));
        let mut f_shader_file = File::open(fragment_path)
            .unwrap_or_else(|_| panic!("Failed to open {}", fragment_path));

        let mut vertex_code = String::new();
        let mut fragment_code = String::new();

        v_shader_file
            .read_to_string(&mut vertex_code)
            .expect("Failed to read vertex shader");
        f_shader_file
            .read_to_string(&mut fragment_code)
            .expect("Failed to read fragment shader");

        let v_shader_code = CString::new(vertex_code.as_bytes()).unwrap();
        let f_shader_code = CString::new(fragment_code.as_bytes()).unwrap();

        unsafe {
            let vertex = gl::CreateShader(gl::VERTEX_SHADER);
            gl::ShaderSource(vertex, 1, &v_shader_code.as_ptr(), ptr::null());
            gl::CompileShader(vertex);
            shader.check_compile_errors(vertex, "VERTEX");

            let fragment = gl::CreateShader(gl::FRAGMENT_SHADER);
            gl::ShaderSource(fragment, 1, &f_shader_code.as_ptr(), ptr::null());
            gl::CompileShader(fragment);
            shader.check_compile_errors(fragment, "FRAGMENT");

            let id = gl::CreateProgram();
            gl::AttachShader(id, vertex);
            gl::AttachShader(id, fragment);
            gl::LinkProgram(id);
            shader.check_compile_errors(id, "PROGRAM");

            gl::DeleteShader(vertex);
            gl::DeleteShader(fragment);
            shader.id = id;
        }

        shader
    }

    pub unsafe fn use_program(&self) {
        gl::UseProgram(self.id)
    }

    #[allow(dead_code)]
    pub unsafe fn set_bool(&self, name: &CStr, value: bool) {
        gl::Uniform1i(gl::GetUniformLocation(self.id, name.as_ptr()), value as i32);
    }

    #[allow(dead_code)]
    pub unsafe fn set_int(&self, name: &CStr, value: i32) {
        gl::Uniform1i(gl::GetUniformLocation(self.id, name.as_ptr()), value);
    }

    #[allow(dead_code)]
    pub unsafe fn set_float(&self, name: &CStr, value: f32) {
        gl::Uniform1f(gl::GetUniformLocation(self.id, name.as_ptr()), value);
    }

    #[allow(dead_code)]
    pub unsafe fn set_vec3(&self, name: &CStr, value: &Vec3) {
        gl::Uniform3fv(
            gl::GetUniformLocation(self.id, name.as_ptr()),
            1,
            value.as_ptr(),
        );
    }

    #[allow(dead_code)]
    pub unsafe fn set_vec2(&self, name: &CStr, value: &Vec2) {
        gl::Uniform2fv(
            gl::GetUniformLocation(self.id, name.as_ptr()),
            1,
            value.as_ptr(),
        );
    }

    #[allow(dead_code)]
    pub unsafe fn set_mat4(&self, name: &CStr, mat: &Mat4) {
        gl::UniformMatrix4fv(
            gl::GetUniformLocation(self.id, name.as_ptr()),
            1,
            gl::FALSE,
            mat.as_ptr(),
        );
    }

    unsafe fn check_compile_errors(&self, shader: u32, type_: &str) {
        let mut success = gl::FALSE as GLint;
        let mut info_log = Vec::with_capacity(1024);
        info_log.set_len(1024 - 1); // subtract 1 to skip the trailing null character
        if type_ != "PROGRAM" {
            gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
            if success != gl::TRUE as GLint {
                gl::GetShaderInfoLog(
                    shader,
                    1024,
                    ptr::null_mut(),
                    info_log.as_mut_ptr() as *mut GLchar,
                );
                println!(
                    "ERROR::SHADER_COMPILATION_ERROR of type: {}\n{}\n \
                         -- --------------------------------------------------- -- ",
                    type_,
                    str::from_utf8(&info_log).unwrap()
                );
            }
        } else {
            gl::GetProgramiv(shader, gl::LINK_STATUS, &mut success);
            if success != gl::TRUE as GLint {
                gl::GetProgramInfoLog(
                    shader,
                    1024,
                    ptr::null_mut(),
                    info_log.as_mut_ptr() as *mut GLchar,
                );
                println!(
                    "ERROR::PROGRAM_LINKING_ERROR of type: {}\n{}\n \
                         -- --------------------------------------------------- -- ",
                    type_,
                    str::from_utf8(&info_log).unwrap()
                );
            }
        }
    }
}
