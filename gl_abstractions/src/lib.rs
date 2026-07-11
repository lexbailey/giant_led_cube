use gl::types::*;
use std::marker::PhantomData;
use {
    std::mem,
    std::ffi::c_void,
};


pub trait ShaderPipeline{
    fn use_(&self);
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
define_uniform!(UniformSamplerBuffer; a:i32; Uniform1i; a as GLint);

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
        }

        impl gl_abstractions::ShaderPipeline for $t {
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

pub trait HasGLEnumValue{
    fn get_enum() -> GLenum;
}

pub struct GLTypeEnum<T>{
    _phantom: PhantomData<T>,
}

macro_rules! map_gl_enum_type {
    ($typename:ty, $value:expr) => {
        impl HasGLEnumValue for GLTypeEnum<$typename>{ fn get_enum() -> GLenum{ $value } }
    }
}

map_gl_enum_type!(GLbyte, gl::BYTE);
map_gl_enum_type!(GLubyte, gl::UNSIGNED_BYTE);
map_gl_enum_type!(GLshort, gl::SHORT);
map_gl_enum_type!(GLushort, gl::UNSIGNED_SHORT);
map_gl_enum_type!(GLint, gl::INT);
map_gl_enum_type!(GLuint, gl::UNSIGNED_INT);
map_gl_enum_type!(GLfloat, gl::FLOAT);
map_gl_enum_type!(GLdouble, gl::DOUBLE);
// TODO I'm sure I missed a lot of useful ones

pub struct BufferObject<T>{
    gl_id: GLuint,
    data: Vec<T>,
}

impl<T> BufferObject<T>{
    pub fn new(data: Vec<T>, target: GLenum, usage: GLenum) -> Self{
        let mut gl_id = 0;
        unsafe {
            gl::GenBuffers(1, &mut gl_id);
            gl::BindBuffer(target, gl_id);
            gl::BufferData(target, (data.len() * mem::size_of::<T>())as GLsizeiptr, &data[0] as *const T as *const c_void, usage);
        }
        Self{
            gl_id,
            data,
        }
    }

    pub fn bind(&self, target: GLenum){
        unsafe {gl::BindBuffer(target, self.gl_id);}
    }
}

pub struct VertexArrayObject{
    gl_id: GLuint,
    //buffers: Vec<Rc<BufferObject<T>>>,
}

impl VertexArrayObject{
    pub fn new() -> Self{
        let mut gl_id = 0;
        unsafe {gl::GenVertexArrays(1, &mut gl_id);}
        Self{
            gl_id,
        }
    }

    pub fn add_buffer<T>(&mut self, buffer: &BufferObject<T>, index: GLuint, size: GLint, normalized: GLboolean, pointer: *const GLvoid) where GLTypeEnum<T>: HasGLEnumValue{
        self.bind();
        buffer.bind(gl::ARRAY_BUFFER);
        unsafe{
            gl::EnableVertexAttribArray(index);
            gl::VertexAttribPointer(index, size, GLTypeEnum::<T>::get_enum(), normalized, (mem::size_of::<T>() * size as usize) as GLsizei, pointer);
        }
    }

    pub fn add_buffer_i<T>(&mut self, buffer: &BufferObject<T>, index: GLuint, size: GLint, pointer: *const GLvoid) where GLTypeEnum<T>: HasGLEnumValue{
        self.bind();
        buffer.bind(gl::ARRAY_BUFFER);
        unsafe{
            gl::EnableVertexAttribArray(index);
            gl::VertexAttribIPointer(index, size, GLTypeEnum::<T>::get_enum(), (mem::size_of::<T>() * size as usize) as GLsizei, pointer);
        }
    }

    pub fn bind(&self){
        unsafe {gl::BindVertexArray(self.gl_id);}
    }
}

fn calculate_surface_normal(tri: &[f32]) -> [f32;3]{
    let u = [
        tri[(1*3)+0] - tri[(0*3)+0],
        tri[(1*3)+1] - tri[(0*3)+1],
        tri[(1*3)+2] - tri[(0*3)+2],
    ];
    let v = [
        tri[(2*3)+0] - tri[(0*3)+0],
        tri[(2*3)+1] - tri[(0*3)+1],
        tri[(2*3)+2] - tri[(0*3)+2],
    ];
	[
        (u[1]*v[2]) - (u[2]*v[1]),
        (u[2]*v[0]) - (u[0]*v[2]),
        (u[0]*v[1]) - (u[1]*v[0]),
    ]

}

fn normalise(a: [f32;3]) -> [f32;3]{
    let sqlen = (a[0]*a[0]) + (a[1]*a[1]) + (a[2]*a[2]);
    let len = sqlen.sqrt();
    [a[0]/len, a[1]/len, a[2]/len]
}

pub struct TriangleMesh{
    vao: VertexArrayObject,
    vbuf: BufferObject<f32>,
    _nbuf: BufferObject<f32>,
    _cbuf: BufferObject<i32>,
    num_triangles: usize,
    wire_vao: Option<VertexArrayObject>,
}

impl TriangleMesh{
    fn mesh_to_vec(points: &Vec<Vec<f32>>, triangles: &Vec<Vec<usize>>, delta: (f32,f32,f32)) -> Vec<f32>{
        let num_triangles = triangles.len();
        let mut out_mesh = Vec::with_capacity(num_triangles*3);
        for tri in triangles{
            if tri.len() != 3{
                panic!("Error: a triangle in the file is not of length 3");
            }
            for p_id in tri{
                let point = &points[*p_id];
                if point.len() != 3{
                    panic!("Error: a point in the file does not have 3 coordinates.");
                }
                let x = point[0]+delta.0;
                let y = point[1]+delta.1;
                let z = point[2]+delta.2;
                out_mesh.push(x);
                out_mesh.push(y);
                out_mesh.push(z);
            }
        }
        out_mesh
    }

    pub fn new(points: &Vec<Vec<f32>>, triangles: &Vec<Vec<usize>>, normals: Option<&Vec<Vec<f32>>>, colours: Option<&Vec<i32>>, offset: f32) -> Self{
        let mut vao = VertexArrayObject::new();

        let mut tx = 0.0;
        let mut ty = 0.0;
        let mut tz = 0.0;
        for p in points{
            tx += p[0];
            ty += p[1];
            tz += p[2];
        }
        let np = points.len() as f32;
        tx /= np;
        ty /= np;
        tz /= np;
        let centre = (tx,ty,tz);
        let new_centre = (tx*offset, ty*offset, tz*offset);
        let dx = new_centre.0 - centre.0;
        let dy = new_centre.1 - centre.1;
        let dz = new_centre.2 - centre.2;
        let delta = (dx,dy,dz);

        let num_triangles = triangles.len();
        let verts = Self::mesh_to_vec(points, triangles, delta);

        let mut norms = Vec::with_capacity(num_triangles*3);
        let mut cols = Vec::with_capacity(num_triangles*3);
        for i in 0..num_triangles{
            if let Some(normals) = normals{
                norms.extend_from_slice(&normals[i]);
                norms.extend_from_slice(&normals[i]);
                norms.extend_from_slice(&normals[i]);
            }
            else{
                let a = normalise(calculate_surface_normal(&verts[i*9..(i+1)*9]));
                norms.extend_from_slice(&a);
                norms.extend_from_slice(&a);
                norms.extend_from_slice(&a);
            }
            if let Some(colours) = colours{
                cols.push(colours[i]);
                cols.push(colours[i]);
                cols.push(colours[i]);
            }
            else{
                cols.push(-1);
                cols.push(-1);
                cols.push(-1);
            }
        }

        let vbuf = BufferObject::new(verts, gl::ARRAY_BUFFER, gl::STATIC_DRAW);
        let nbuf = BufferObject::new(norms, gl::ARRAY_BUFFER, gl::STATIC_DRAW);
        let cbuf = BufferObject::new(cols, gl::ARRAY_BUFFER, gl::STATIC_DRAW);
        vao.add_buffer(&vbuf, 0, 3, gl::FALSE, std::ptr::null());
        vao.add_buffer(&nbuf, 1, 3, gl::FALSE, std::ptr::null());
        vao.add_buffer_i(&cbuf, 2, 1, std::ptr::null());
        Self{
            vao,
            vbuf,
            _nbuf: nbuf,
            _cbuf: cbuf,
            wire_vao: None,
            num_triangles,
        }
    }

    pub fn draw<S>(&self, shader: &S) where S: ShaderPipeline{
        shader.use_();
        self.vao.bind();
        unsafe{
            gl::DrawArrays(gl::TRIANGLES, 0, self.num_triangles as i32 * 3);
        }
    }

    pub fn generate_wireframe(&mut self){
        if self.wire_vao.is_some(){
            return;
        }
        let mut line_array = Vec::with_capacity(self.num_triangles * 6);
        for i in 0..self.num_triangles{
            let a = &self.vbuf.data[i*9..(i+1)*9];
            let p0 = &a[0..3];
            let p1 = &a[3..6];
            let p2 = &a[6..9];
            line_array.extend_from_slice(p0);
            line_array.extend_from_slice(p1);

            line_array.extend_from_slice(p1);
            line_array.extend_from_slice(p2);

            line_array.extend_from_slice(p2);
            line_array.extend_from_slice(p0);
        }

        let mut wire_cube = VertexArrayObject::new();
        let wire_verts = BufferObject::new(line_array, gl::ARRAY_BUFFER, gl::STATIC_DRAW);
        wire_cube.add_buffer(&wire_verts, 0, 3, gl::FALSE, std::ptr::null());
        self.wire_vao = Some(wire_cube);
    }

    pub fn draw_wireframe<S>(&self, shader: &S) where S: ShaderPipeline{
        if let Some(wvao) = &self.wire_vao{
            shader.use_();
            wvao.bind();
            unsafe{
                gl::DrawArrays(gl::LINES, 0, self.num_triangles as i32 * 6);
            }
        }
    }

}


