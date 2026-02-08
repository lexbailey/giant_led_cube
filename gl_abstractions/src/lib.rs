use gl::types::*;

macro_rules! uni_from {
    ($u:ident) => {
        impl From<i32> for $u{
            fn from(id:i32) -> $u {
                $u{id:id}
            }
        }
    }
}

macro_rules! define_uniform{
    ($name:ident; $($aname:ident:$atype:ty),* ; $setfun:ident ; $($aexpr:expr),*) => {
        #[derive(Default)]
        pub struct $name{ id: i32 }
        impl $name{
            pub fn set(&self, $($aname:$atype),*){
                unsafe{ gl::$setfun(self.id, $($aexpr),*); }
            }
        }
        uni_from!($name);
    }
}

define_uniform!(Uniform1F; a:f32;                      Uniform1f; a as GLfloat);
define_uniform!(Uniform2F; a:f32, b:f32;               Uniform2f; a as GLfloat, b as GLfloat);
define_uniform!(Uniform3F; a:f32, b:f32, c:f32;        Uniform3f; a as GLfloat, b as GLfloat, c as GLfloat);
define_uniform!(Uniform4F; a:f32, b:f32, c:f32, d:f32; Uniform4f; a as GLfloat, b as GLfloat, c as GLfloat, d as GLfloat);

define_uniform!(Uniform1I; a:i32;                      Uniform1i; a as GLint);
define_uniform!(Uniform2I; a:i32, b:i32;               Uniform2i; a as GLint, b as GLint);
define_uniform!(Uniform3I; a:i32, b:i32, c:i32;        Uniform3i; a as GLint, b as GLint, c as GLint);
define_uniform!(Uniform4I; a:i32, b:i32, c:i32, d:i32; Uniform4i; a as GLint, b as GLint, c as GLint, d as GLint);

define_uniform!(Uniform1UI; a:u32;                      Uniform1ui; a as GLuint);
define_uniform!(Uniform2UI; a:u32, b:u32;               Uniform2ui; a as GLuint, b as GLuint);
define_uniform!(Uniform3UI; a:u32, b:u32, c:u32;        Uniform3ui; a as GLuint, b as GLuint, c as GLuint);
define_uniform!(Uniform4UI; a:u32, b:u32, c:u32, d:u32; Uniform4ui; a as GLuint, b as GLuint, c as GLuint, d as GLuint);

define_uniform!(Uniform1FV; data: &[f32]; Uniform1fv; data.len().try_into().unwrap(), &data[0] as *const GLfloat);
define_uniform!(Uniform2FV; data: &[f32]; Uniform2fv; data.len().try_into().unwrap(), &data[0] as *const GLfloat);
define_uniform!(Uniform3FV; data: &[f32]; Uniform3fv; data.len().try_into().unwrap(), &data[0] as *const GLfloat);
define_uniform!(Uniform4FV; data: &[f32]; Uniform4fv; data.len().try_into().unwrap(), &data[0] as *const GLfloat);

define_uniform!(Uniform1IV; data: &[i32]; Uniform1iv; data.len().try_into().unwrap(), &data[0] as *const GLint);
define_uniform!(Uniform2IV; data: &[i32]; Uniform2iv; data.len().try_into().unwrap(), &data[0] as *const GLint);
define_uniform!(Uniform3IV; data: &[i32]; Uniform3iv; data.len().try_into().unwrap(), &data[0] as *const GLint);
define_uniform!(Uniform4IV; data: &[i32]; Uniform4iv; data.len().try_into().unwrap(), &data[0] as *const GLint);

define_uniform!(Uniform1UIV; data: &[u32]; Uniform1uiv; data.len().try_into().unwrap(), &data[0] as *const GLuint);
define_uniform!(Uniform2UIV; data: &[u32]; Uniform2uiv; data.len().try_into().unwrap(), &data[0] as *const GLuint);
define_uniform!(Uniform3UIV; data: &[u32]; Uniform3uiv; data.len().try_into().unwrap(), &data[0] as *const GLuint);
define_uniform!(Uniform4UIV; data: &[u32]; Uniform4uiv; data.len().try_into().unwrap(), &data[0] as *const GLuint);

// These next 3 not provided by opengl api, but easy enough to provide here
define_uniform!(UniformMat2F; transpose: bool, data: &[f32;4];  UniformMatrix2fv; 1, if transpose {gl::TRUE} else {gl::FALSE}, &data[0] as *const GLfloat );
define_uniform!(UniformMat3F; transpose: bool, data: &[f32;9];  UniformMatrix3fv; 1, if transpose {gl::TRUE} else {gl::FALSE}, &data[0] as *const GLfloat );
define_uniform!(UniformMat4F; transpose: bool, data: &[f32;16]; UniformMatrix4fv; 1, if transpose {gl::TRUE} else {gl::FALSE}, &data[0] as *const GLfloat );

define_uniform!(UniformMat2FV; count: i32, transpose: bool, data: &[f32;4];  UniformMatrix2fv; count, if transpose {gl::TRUE} else {gl::FALSE}, &data[0] as *const GLfloat );
define_uniform!(UniformMat3FV; count: i32, transpose: bool, data: &[f32;9];  UniformMatrix3fv; count, if transpose {gl::TRUE} else {gl::FALSE}, &data[0] as *const GLfloat );
define_uniform!(UniformMat4FV; count: i32, transpose: bool, data: &[f32;16]; UniformMatrix4fv; count, if transpose {gl::TRUE} else {gl::FALSE}, &data[0] as *const GLfloat );

// non-square matricies without the vector api are also not provided by opengl, but also easy to provide
define_uniform!(UniformMat2x3F; transpose: bool, data: &[f32;6];   UniformMatrix2x3fv; 1, if transpose {gl::TRUE} else {gl::FALSE}, &data[0] as *const GLfloat );
define_uniform!(UniformMat3x2F; transpose: bool, data: &[f32;6];   UniformMatrix3x2fv; 1, if transpose {gl::TRUE} else {gl::FALSE}, &data[0] as *const GLfloat );
define_uniform!(UniformMat2x4F; transpose: bool, data: &[f32;8];   UniformMatrix2x4fv; 1, if transpose {gl::TRUE} else {gl::FALSE}, &data[0] as *const GLfloat );
define_uniform!(UniformMat4x2F; transpose: bool, data: &[f32;8];   UniformMatrix4x2fv; 1, if transpose {gl::TRUE} else {gl::FALSE}, &data[0] as *const GLfloat );
define_uniform!(UniformMat3x4F; transpose: bool, data: &[f32;12];  UniformMatrix3x4fv; 1, if transpose {gl::TRUE} else {gl::FALSE}, &data[0] as *const GLfloat );
define_uniform!(UniformMat4x3F; transpose: bool, data: &[f32;12];  UniformMatrix4x3fv; 1, if transpose {gl::TRUE} else {gl::FALSE}, &data[0] as *const GLfloat );

define_uniform!(UniformMat2x3FV; count: i32, transpose: bool, data: &[f32;6];   UniformMatrix2x3fv; count, if transpose {gl::TRUE} else {gl::FALSE}, &data[0] as *const GLfloat );
define_uniform!(UniformMat3x2FV; count: i32, transpose: bool, data: &[f32;6];   UniformMatrix3x2fv; count, if transpose {gl::TRUE} else {gl::FALSE}, &data[0] as *const GLfloat );
define_uniform!(UniformMat2x4FV; count: i32, transpose: bool, data: &[f32;8];   UniformMatrix2x4fv; count, if transpose {gl::TRUE} else {gl::FALSE}, &data[0] as *const GLfloat );
define_uniform!(UniformMat4x2FV; count: i32, transpose: bool, data: &[f32;8];   UniformMatrix4x2fv; count, if transpose {gl::TRUE} else {gl::FALSE}, &data[0] as *const GLfloat );
define_uniform!(UniformMat3x4FV; count: i32, transpose: bool, data: &[f32;12];  UniformMatrix3x4fv; count, if transpose {gl::TRUE} else {gl::FALSE}, &data[0] as *const GLfloat );
define_uniform!(UniformMat4x3FV; count: i32, transpose: bool, data: &[f32;12];  UniformMatrix4x3fv; count, if transpose {gl::TRUE} else {gl::FALSE}, &data[0] as *const GLfloat );

// Alternative names for some types
define_uniform!(UniformSampler2D; a:i32; Uniform1i; a as GLint);
define_uniform!(UniformSampler2DV; data: &[i32]; Uniform1iv; data.len().try_into().unwrap(), &data[0] as *const GLint);

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
                    let c_str_vert = std::ffi::CString::new($vs.as_bytes()).unwrap();

                    let name = format!("{}.vert.glsl", stringify!(ty));
                    let c_str_name = std::ffi::CString::new(name.as_bytes()).unwrap();
                    gl::ObjectLabel(gl::SHADER, vertex_shader, name.as_bytes().len() as i32, c_str_name.as_ptr());

                    gl::ShaderSource(vertex_shader, 1, &c_str_vert.as_ptr(), std::ptr::null());
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
                    let c_str_frag = std::ffi::CString::new($fs.as_bytes()).unwrap();

                    let name = format!("{}.frag.glsl", stringify!(ty));
                    let c_str_name = std::ffi::CString::new(name.as_bytes()).unwrap();
                    gl::ObjectLabel(gl::SHADER, fragment_shader, name.as_bytes().len() as i32, c_str_name.as_ptr());

                    gl::ShaderSource(fragment_shader, 1, &c_str_frag.as_ptr(), std::ptr::null());
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

