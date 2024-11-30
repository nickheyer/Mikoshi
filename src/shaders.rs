use gl::types::*;
use std::ffi::CString;
use std::fs;
use std::ptr;
use std::str;

pub struct ShaderProgram {
    pub id: GLuint,
}

impl ShaderProgram {
    pub fn new(vertex_path: &str, fragment_path: &str) -> Result<Self, String> {
        let vertex_code = fs::read_to_string(vertex_path)
            .map_err(|e| format!("Failed to read vertex shader file: {}", e))?;
        let fragment_code = fs::read_to_string(fragment_path)
            .map_err(|e| format!("Failed to read fragment shader file: {}", e))?;

        let vertex_shader = Self::compile_shader(&vertex_code, gl::VERTEX_SHADER)?;
        let fragment_shader = Self::compile_shader(&fragment_code, gl::FRAGMENT_SHADER)?;
        let program_id = Self::link_program(vertex_shader, fragment_shader)?;

        unsafe {
            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(fragment_shader);
        }

        Ok(Self { id: program_id })
    }

    fn compile_shader(source_code: &str, shader_type: GLenum) -> Result<GLuint, String> {
        let shader;
        unsafe {
            shader = gl::CreateShader(shader_type);
            let c_str_code = CString::new(source_code).map_err(|e| format!("Failed to create CString: {}", e))?;
            gl::ShaderSource(shader, 1, &c_str_code.as_ptr(), ptr::null());
            gl::CompileShader(shader);

            let mut success = gl::FALSE as GLint;
            gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
            if success != gl::TRUE as GLint {
                let mut len = 0;
                gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
                let mut buffer = Vec::with_capacity(len as usize);
                buffer.set_len((len as usize) - 1);
                gl::GetShaderInfoLog(shader, len, ptr::null_mut(), buffer.as_mut_ptr() as *mut GLchar);
                return Err(format!(
                    "Shader compilation failed: {}",
                    str::from_utf8(&buffer).unwrap_or("Unknown error")
                ));
            }
        }
        Ok(shader)
    }

    fn link_program(vertex_shader: GLuint, fragment_shader: GLuint) -> Result<GLuint, String> {
        let program;
        unsafe {
            program = gl::CreateProgram();
            gl::AttachShader(program, vertex_shader);
            gl::AttachShader(program, fragment_shader);
            gl::LinkProgram(program);

            let mut success = gl::FALSE as GLint;
            gl::GetProgramiv(program, gl::LINK_STATUS, &mut success);
            if success != gl::TRUE as GLint {
                let mut len = 0;
                gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
                let mut buffer = Vec::with_capacity(len as usize);
                buffer.set_len((len as usize) - 1);
                gl::GetProgramInfoLog(program, len, ptr::null_mut(), buffer.as_mut_ptr() as *mut GLchar);
                return Err(format!(
                    "Program linking failed: {}",
                    str::from_utf8(&buffer).unwrap_or("Unknown error")
                ));
            }
        }
        Ok(program)
    }

    pub fn set(&self) {
        unsafe {
            gl::UseProgram(self.id);
        }
    }

    pub fn set_uniform_f32(&self, name: &str, value: f32) {
        unsafe {
            let c_name = CString::new(name).unwrap();
            let location = gl::GetUniformLocation(self.id, c_name.as_ptr());
            if location != -1 {
                gl::Uniform1f(location, value);
            }
        }
    }

    pub fn set_uniform_vec2(&self, name: &str, x: f32, y: f32) {
        unsafe {
            let c_name = CString::new(name).unwrap();
            let location = gl::GetUniformLocation(self.id, c_name.as_ptr());
            if location != -1 {
                gl::Uniform2f(location, x, y);
            }
        }
    }
}

pub struct Quad {
    vao: GLuint,
    _vbo: GLuint,
    _ebo: GLuint,
}

impl Quad {
    fn new() -> Self {
        let vertices: [f32; 16] = [
            -1.0,  1.0,    0.0, 0.0,
            -1.0, -1.0,    0.0, 1.0,
             1.0, -1.0,    1.0, 1.0,
             1.0,  1.0,    1.0, 0.0
        ];
        let indices = [0u32, 1, 2, 0, 2, 3];

        let (vao, vbo, ebo) = unsafe {
            let mut vao = 0;
            let mut vbo = 0;
            let mut ebo = 0;
            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);
            gl::GenBuffers(1, &mut ebo);

            gl::BindVertexArray(vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * std::mem::size_of::<f32>()) as GLsizeiptr,
                vertices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );

            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (indices.len() * std::mem::size_of::<u32>()) as GLsizeiptr,
                indices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );

            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE,
                (4 * std::mem::size_of::<f32>()) as GLint,
                std::ptr::null());
            gl::EnableVertexAttribArray(0);

            gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE,
                (4 * std::mem::size_of::<f32>()) as GLint,
                (2 * std::mem::size_of::<f32>()) as *const _);
            gl::EnableVertexAttribArray(1);

            (vao, vbo, ebo)
        };

        Self {
            vao,
            _vbo: vbo,
            _ebo: ebo,
        }
    }

    pub fn draw(&self) {
        unsafe {
            gl::BindVertexArray(self.vao);
            gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, std::ptr::null());
        }
    }
}

impl Drop for Quad {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteVertexArrays(1, &self.vao);
            gl::DeleteBuffers(1, &self._vbo);
            gl::DeleteBuffers(1, &self._ebo);
        }
    }
}

pub fn create_screen_quad() -> Quad {
    Quad::new()
}
