use gl::types::*;

#[derive(Default)]
pub struct UniformMat4{
    id: i32
}

#[derive(Default)]
pub struct UniformVec2{
    id: i32
}

#[derive(Default)]
pub struct UniformVec3{
    id: i32
}

#[derive(Default)]
pub struct UniformVec4{
    id: i32
}

#[derive(Default)]
pub struct UniformSampler2D{
    id: i32
}

impl UniformMat4{
    pub fn set(&self, data: &[f32;16]){
        unsafe{
            gl::UniformMatrix4fv(self.id, 1, gl::FALSE, &data[0] as *const GLfloat);
        }
    }
}

impl UniformVec4 {
    pub fn set(&self, r:f32, g:f32, b:f32, a: f32){
        unsafe{
            gl::Uniform4f(self.id, r, g, b, a);
        }
    }
}

impl UniformVec3 {
    pub fn set(&self, r:f32, g:f32, b:f32){
        unsafe{
            gl::Uniform3f(self.id, r, g, b);
        }
    }
}

impl UniformVec2 {
    pub fn set(&self, r:f32, g:f32){
        unsafe{
            gl::Uniform2f(self.id, r, g);
        }
    }
}

impl UniformSampler2D {
    pub fn set(&self, a:i32){
        unsafe{
            gl::Uniform1i(self.id, a);
        }
    }
}

macro_rules! uni_from {
    ($u:ident) => {
        impl From<i32> for $u{
            fn from(id:i32) -> $u {
                $u{id:id}
            }
        }
    }
}

uni_from!(UniformMat4);
uni_from!(UniformVec2);
uni_from!(UniformVec3);
uni_from!(UniformVec4);
uni_from!(UniformSampler2D);

#[macro_export]
macro_rules! impl_shader{
    ($t:ty, $vs:expr, $fs:expr $(,$field:ident:$kind:ident)*) => {
        impl $t{
            fn init(&mut self) -> Result<(),String> {
                unsafe {
                    // Setup shader compilation checks
                    const LOG_MAX_LEN: usize = 512;
                    let mut success = i32::from(gl::FALSE);
                    let mut info_log = Vec::with_capacity(LOG_MAX_LEN);
                    let mut log_len = 0i32;

                    // Vertex shader
                    let vertex_shader = gl::CreateShader(gl::VERTEX_SHADER);
                    let c_str_vert = CString::new($vs.as_bytes()).unwrap();
                    gl::ShaderSource(vertex_shader, 1, &c_str_vert.as_ptr(), ptr::null());
                    gl::CompileShader(vertex_shader);

                    // Check for shader compilation errors
                    gl::GetShaderiv(vertex_shader, gl::COMPILE_STATUS, &mut success);
                    if success != i32::from(gl::TRUE) {
                        gl::GetShaderInfoLog(
                            vertex_shader,
                            LOG_MAX_LEN as i32,
                            (&mut log_len) as *mut GLsizei,
                            info_log.as_mut_ptr() as *mut GLchar,
                        );
                        info_log.set_len(log_len as usize);
                        return Err(format!(
                            "Error in vertex shader compilation\n{}",
                            String::from_utf8_lossy(&info_log[0..(log_len as usize)])
                        ));
                    }

                    // Fragment shader
                    let fragment_shader = gl::CreateShader(gl::FRAGMENT_SHADER);
                    let c_str_frag = CString::new($fs.as_bytes()).unwrap();
                    gl::ShaderSource(fragment_shader, 1, &c_str_frag.as_ptr(), ptr::null());
                    gl::CompileShader(fragment_shader);

                    // Check for shader compilation errors
                    gl::GetShaderiv(fragment_shader, gl::COMPILE_STATUS, &mut success);
                    if success != i32::from(gl::TRUE) {
                        gl::GetShaderInfoLog(
                            fragment_shader,
                            LOG_MAX_LEN as i32,
                            (&mut log_len) as *mut GLsizei,
                            info_log.as_mut_ptr() as *mut GLchar,
                        );
                        info_log.set_len(log_len as usize);
                        return Err(format!(
                            "Error in fragment shader compilation\n{}",
                            String::from_utf8_lossy(&info_log[0..(log_len as usize)])
                        ));
                    }

                    // Link Shaders
                    let shader_program = gl::CreateProgram();
                    gl::AttachShader(shader_program, vertex_shader);
                    gl::AttachShader(shader_program, fragment_shader);
                    gl::LinkProgram(shader_program);

                    // Check for linking errors
                    gl::GetProgramiv(shader_program, gl::LINK_STATUS, &mut success);
                    if success != i32::from(gl::TRUE) {
                        gl::GetProgramInfoLog(
                            shader_program,
                            LOG_MAX_LEN as i32,
                            (&mut log_len) as *mut GLsizei,
                            info_log.as_mut_ptr() as *mut GLchar,
                        );
                        info_log.set_len(log_len as usize);
                        return Err(format!(
                            "Error in shader linking step\n{}",
                            String::from_utf8_lossy(&info_log[0..(log_len as usize)])
                        ));
                    }
                    gl::DeleteShader(vertex_shader);
                    gl::DeleteShader(fragment_shader);

                    self.shader_id = shader_program;

                    Ok(())
                }
            }

            fn new() -> $t {
                let mut shader:$t = Default::default();
                match shader.init() {
                    Err(msg) => panic!("Error when compiling shader: {}", msg)
                    ,_ => ()
                };
                unsafe {
                    gl::UseProgram(shader.shader_id);
                    $(
                        let uniform = gl::GetUniformLocation(shader.shader_id, std::ffi::CString::new(stringify!($field)).unwrap().into_raw() as *const GLchar);
                        shader.$field = $kind::from(uniform);
                    )*
                }
                shader
            }

            fn use_(&self) {
                unsafe {gl::UseProgram(self.shader_id);}
            }
        }
    }
}

#[macro_export]
macro_rules! shader_struct {
    ($shader_name:ident,  $vs:expr, $fs:expr, {$($name:ident:$type:ident,)*}) => {
        #[derive(Default)]
        struct $shader_name{
            shader_id: u32,
            $($name:$type,)*
        }
        
        impl_shader!(
           $shader_name 
            ,$vs
            ,$fs
            $(,$name:$type)*
        );
    }
}

