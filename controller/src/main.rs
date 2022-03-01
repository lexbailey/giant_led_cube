extern crate gl;
extern crate glutin;

mod affine;
use cube_model as cube;

use gl::types::*;
use std::mem;
use std::ptr;
use std::str;
use std::os::raw::c_void;
use std::ffi::CString;
use std::time::{Instant,Duration};
use rand::Rng;
use std::process::Command;

macro_rules! impl_shader{
    ($t:ty, $vs:expr, $fs:expr $(,$field:ident), *) => {
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
                        return Err(format!(
                            "ERROR::SHADER::VERTEX::COMPILATION_FAILED\n{}",
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
                        return Err(format!(
                            "ERROR::SHADER::FRAGMENT::COMPILATION_FAILED\n{}",
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
                        return Err(format!(
                            "ERROR::SHADER::PROGRAM::COMPILATION_FAILED\n{}",
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
                        let uniform = gl::GetUniformLocation(shader_program, std::ffi::CString::new(stringify!($field)).unwrap().into_raw() as *const GLchar);
                        self.$field = uniform;
                    )*
                }
                shader
            }
        }
    }
}

const VERTEX_SHADER_SOURCE: &str = r#"
#version 330 core
layout (location = 0) in vec4 aPos;
uniform mat4 face_transform;
uniform mat4 offset;
uniform mat4 transform;
void main()
{
    gl_Position = aPos  * offset* face_transform * transform;
}
"#;

const FRAGMENT_SHADER_SOURCE: &str = r#"
#version 330 core
out vec4 FragColor;
uniform vec3 color;
void main()
{
   // Set the fragment color to the color passed from the vertex shader
   FragColor = vec4(color, 1.0);
}
"#;

#[derive(Default)]
struct CubeShader{
    shader_id: u32
    ,u_face_transform: u32
    ,u_offset: u32
    ,u_transform: u32
}

impl_shader!(CubeShader, VERTEX_SHADER_SOURCE, FRAGMENT_SHADER_SOURCE);

struct DataModel{
    d: f32
    ,r: f32
    ,diff: f32
    ,cube: cube::Cube
    ,frames: i32
}

use glutin::ContextWrapper;
use glutin::PossiblyCurrent;
struct RenderData{
    shader: CubeShader
    ,cube_verts: u32
    ,transform: i32
    ,offset: i32
    ,offset_mat: affine::Transform<f32>
    ,offset_subface_mat: affine::Transform<f32>
    ,face_transform: i32
    ,faces: Vec<affine::Transform<f32>>
    ,subfaces: Vec<affine::Transform<f32>>
    ,color: i32
    ,window: ContextWrapper<PossiblyCurrent, glutin::window::Window>
    ,tc: TermCols
}

fn tput (f:fn (&mut Command)-> &mut Command) -> String {
    String::from_utf8(f(&mut Command::new("tput")).output().expect("tput failed").stdout).unwrap()
}

struct TermCols{
    white:String
    ,red:String
    ,green:String
    ,yellow:String
    ,blue:String
    ,orange:String
    ,default:String
    ,fg_black:String
}

fn color_string(s: String, col: cube::Colors, tc: &TermCols) -> String {
    format!("{}{}{:03}{}", tc.fg_black, match col {
        cube::Colors::White => &tc.white
        ,cube::Colors::Red => &tc.red
        ,cube::Colors::Green => &tc.green
        ,cube::Colors::Yellow => &tc.yellow
        ,cube::Colors::Blue => &tc.blue
        ,cube::Colors::Orange => &tc.orange
    }, s, tc.default)
}

fn nb (f: &cube::Face, i:usize, tc: &TermCols) -> String{ color_string(i.to_string(), f.subfaces[i].color, &tc) }
fn bb (f: &cube::Face, i:usize, tc: &TermCols) -> String{ color_string("".to_string(), f.subfaces[i].color, &tc) }

fn main() {
    
    let events_loop = glutin::event_loop::EventLoop::new();
    let window = glutin::window::WindowBuilder::new()
        .with_title("Big cube")
        .with_inner_size(glutin::dpi::PhysicalSize::new(800,800));
    let context = glutin::ContextBuilder::new().with_vsync(true);
    let gl_window = unsafe {
        let win = context.build_windowed(window, &events_loop).unwrap().make_current().unwrap();
        gl::load_with(|s| win.get_proc_address(s) as *const _);
        win
    };

    let mut gfx_objs = unsafe{ 
    //let (shader_program, cube_verts, transform, color) = unsafe {
        // Setup shader compilation checks
        //let mut success = i32::from(gl::FALSE);
        //let mut info_log = Vec::with_capacity(512);
        //info_log.set_len(512 - 1); // -1 to skip trialing null character

        //// Vertex shader
        //let vertex_shader = gl::CreateShader(gl::VERTEX_SHADER);
        //let c_str_vert = CString::new(VERTEX_SHADER_SOURCE.as_bytes()).unwrap();
        //gl::ShaderSource(vertex_shader, 1, &c_str_vert.as_ptr(), ptr::null());
        //gl::CompileShader(vertex_shader);

        //// Check for shader compilation errors
        //gl::GetShaderiv(vertex_shader, gl::COMPILE_STATUS, &mut success);
        //if success != i32::from(gl::TRUE) {
        //    gl::GetShaderInfoLog(
        //        vertex_shader,
        //        512,
        //        ptr::null_mut(),
        //        info_log.as_mut_ptr() as *mut GLchar,
        //    );
        //    println!(
        //        "ERROR::SHADER::VERTEX::COMPILATION_FAILED\n{}",
        //        String::from_utf8_lossy(&info_log)
        //    );
        //}

        //// Fragment shader
        //let fragment_shader = gl::CreateShader(gl::FRAGMENT_SHADER);
        //let c_str_frag = CString::new(FRAGMENT_SHADER_SOURCE.as_bytes()).unwrap();
        //gl::ShaderSource(fragment_shader, 1, &c_str_frag.as_ptr(), ptr::null());
        //gl::CompileShader(fragment_shader);

        //// Check for shader compilation errors
        //gl::GetShaderiv(fragment_shader, gl::COMPILE_STATUS, &mut success);
        //if success != i32::from(gl::TRUE) {
        //    gl::GetShaderInfoLog(
        //        fragment_shader,
        //        512,
        //        ptr::null_mut(),
        //        info_log.as_mut_ptr() as *mut GLchar,
        //    );
        //    println!(
        //        "ERROR::SHADER::FRAGMENT::COMPILATION_FAILED\n{}",
        //        String::from_utf8_lossy(&info_log)
        //    );
        //}

        //// Link Shaders
        //let shader_program = gl::CreateProgram();
        //gl::AttachShader(shader_program, vertex_shader);
        //gl::AttachShader(shader_program, fragment_shader);
        //gl::LinkProgram(shader_program);

        //// Check for linking errors
        //gl::GetProgramiv(shader_program, gl::LINK_STATUS, &mut success);
        //if success != i32::from(gl::TRUE) {
        //    gl::GetProgramInfoLog(
        //        shader_program,
        //        512,
        //        ptr::null_mut(),
        //        info_log.as_mut_ptr() as *mut GLchar,
        //    );
        //    println!(
        //        "ERROR::SHADER::PROGRAM::COMPILATION_FAILED\n{}",
        //        String::from_utf8_lossy(&info_log)
        //    );
        //}
        //gl::DeleteShader(vertex_shader);
        //gl::DeleteShader(fragment_shader);

        let mut cube_shader = CubeShader::new();

        let offset = affine::Transform::<f32>::translate(0.0,0.0,-0.5);
        // a square
        let vertices: [f32;16] = [
            -0.5, -0.5, 0.0, 1.0
            ,-0.5, 0.5, 0.0, 1.0
            ,0.5, 0.5, 0.0, 1.0
            ,0.5, -0.5, 0.0, 1.0
        ];

        // Build the cube by transforming the square into five different orientations (bottom is missing)
        use std::f32::consts::TAU;

        // Top front left back right
        let face_transforms = vec![
            affine::Transform::<f32>::rotate_xyz(TAU/4.0,0.0,0.0),
            affine::Transform::<f32>::none(),
            affine::Transform::<f32>::rotate_xyz(0.0,(TAU/4.0)*1.0,0.0),
            affine::Transform::<f32>::rotate_xyz(0.0,(TAU/4.0)*2.0,0.0),
            affine::Transform::<f32>::rotate_xyz(0.0,(TAU/4.0)*3.0,0.0),
        ];

        let subface_transforms = vec![
            affine::Transform::<f32>::translate(-0.33, 0.33, 0.0),
            affine::Transform::<f32>::translate(0.0, 0.33, 0.0),
            affine::Transform::<f32>::translate(0.33, 0.33, 0.0),
            affine::Transform::<f32>::translate(-0.33, 0.0, 0.0),
            affine::Transform::<f32>::none(),
            affine::Transform::<f32>::translate(0.33, 0.0, 0.0),
            affine::Transform::<f32>::translate(-0.33, -0.33, 0.0),
            affine::Transform::<f32>::translate(0.0, -0.33, 0.0),
            affine::Transform::<f32>::translate(0.33, -0.33, 0.0),
        ];
        //let vertices: Vec<f32> = face_transforms.into_iter().map(|t|
        //    (0..4).map(|a|affine::Vec4::<f32>{data:[
        //        vertices[a*3]
        //        ,vertices[(a*3)+1]
        //        ,vertices[(a*3)+2]
        //        ,1.0 // Extra 1 for 4-vectors, required for transforms to work
        //    ]}.transform(&t).data).flatten().collect::<Vec<_>>()
        //).flatten().collect();

        let mut vbo = 0;
        let mut cube_verts = 0;
        gl::GenVertexArrays(1, &mut cube_verts);
        gl::GenBuffers(1, &mut vbo);

        gl::BindVertexArray(cube_verts);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (vertices.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
            &vertices[0] as *const f32 as *const c_void,
            gl::STATIC_DRAW,
        );

        gl::EnableVertexAttribArray(0);
        gl::VertexAttribPointer(
            0,
            4,
            gl::FLOAT,
            gl::FALSE,
            4 * mem::size_of::<GLfloat>() as GLsizei,
            ptr::null(),
        );

        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        gl::BindVertexArray(0);

        gl::UseProgram(cube_shader.shader_id);
        // TODO string not deallocated? not really a problem if not, but could fix it.
        let transform = gl::GetUniformLocation(cube_shader.shader_id, std::ffi::CString::new("transform").unwrap().into_raw() as *const GLchar);
        let offset_u = gl::GetUniformLocation(cube_shader.shader_id, std::ffi::CString::new("offset").unwrap().into_raw() as *const GLchar);
        let face_transform = gl::GetUniformLocation(cube_shader.shader_id, std::ffi::CString::new("face_transform").unwrap().into_raw() as *const GLchar);
        let color = gl::GetUniformLocation(cube_shader.shader_id, std::ffi::CString::new("color").unwrap().into_raw() as *const GLchar);

        gl::Enable(gl::DEPTH_TEST);
        let offset_subface = &((&affine::Transform::<f32>::scale(1.01,1.01,1.01)) * &offset) * (&affine::Transform::<f32>::scale(0.3,0.3,0.3));


        let tc = TermCols{
            white: tput(|t|t.arg("setab").arg("15"))
            ,red: tput(|t|t.arg("setab").arg("9"))
            ,green: tput(|t|t.arg("setab").arg("10"))
            ,yellow: tput(|t|t.arg("setab").arg("11"))
            ,blue: tput(|t|t.arg("setab").arg("12"))
            ,orange: tput(|t|t.arg("setab").arg("208"))
            ,default: tput(|t|t.arg("sgr0"))
            ,fg_black: tput(|t|t.arg("setaf").arg("0"))
        };

        //(shader_program, cube_verts, transform, color);
        RenderData{
            shader: cube_shader
            ,cube_verts: cube_verts
            ,transform: transform
            ,offset: offset_u
            ,offset_mat: offset
            ,offset_subface_mat: offset_subface
            ,face_transform: face_transform
            ,faces: face_transforms
            ,subfaces: subface_transforms
            ,window: gl_window
            ,color: color
            ,tc: tc
        }
    };

    let mut state = DataModel{
        d: 0.0
        ,r: 0.0
        ,diff: 0.01
        ,cube: cube::Cube::new()
        ,frames: 0
    };

    //state.cube.twist(cube::CENTER_LR, false);
    //state.cube.twist(cube::CENTER_BT, false);
    //state.cube.twist(cube::CENTER_LR, false);
    //state.cube.twist(cube::CENTER_BT, false);
    //state.cube.twist(cube::CENTER_LR, false);
    //state.cube.twist(cube::CENTER_BT, false);
    //state.cube.twist(cube::CENTER_LR, false);
    //state.cube.twist(cube::CENTER_BT, false);

    //state.cube.twist(cube::TOP, false);
    //state.cube.twist(cube::BOTTOM, false);
    //state.cube.twist(cube::BOTTOM, false);

    //state.cube.twist(cube::RIGHT, false);
    //state.cube.twist(cube::RIGHT, false);
    //state.cube.twist(cube::LEFT, false);
    //state.cube.twist(cube::LEFT, false);

    //state.cube.twist(cube::FRONT, false);
    //state.cube.twist(cube::FRONT, false);
    //state.cube.twist(cube::BACK, false);
    //state.cube.twist(cube::BACK, false);

    
    let target_fps = 30.0;
    let frame_dur_ms: f32 = 1000.0/target_fps;

    let mut last_frame_start = Instant::now();

    let frame_duration = Duration::from_millis(frame_dur_ms.floor() as u64);

    fn update(state: &mut DataModel){
        state.d += state.diff;
        state.r += state.diff.abs()/2.0;
        state.r %= 1.0;
        state.frames += 1;
        if state.d >= 1.0 {state.diff = -0.01;}
        if state.d <= 0.0 {state.diff = 0.01;}
        
        if state.frames % 30 == 0{
            let face = rand::thread_rng().gen_range(0..6);
            let dir: i32 = rand::thread_rng().gen_range(0..2);
            state.cube.twist(cube::Twist{face:face, reverse:dir==0});
            //state.cube.twist(0, false);
        }
    }

    fn draw(state: &mut DataModel, gfx: &mut RenderData){
        unsafe {
            gl::ClearColor(state.d, 0.58, 0.92, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            gl::UseProgram(gfx.shader.shader_id);
            gl::BindVertexArray(gfx.cube_verts);
            let transform = affine::Transform::<GLfloat>::rotate_xyz((3.14*2.0)/16.0, (3.14*2.0)*(state.r as f32), 0.0);
            gl::UniformMatrix4fv(gfx.transform, 1, gl::FALSE, &transform.data[0] as *const GLfloat);

            for i in 0..5{
                gl::UniformMatrix4fv(gfx.offset, 1, gl::FALSE, &gfx.offset_mat.data[0] as *const GLfloat);
                gl::Uniform3f(gfx.color, 0.0,0.0,0.0);
                gl::UniformMatrix4fv(gfx.face_transform, 1, gl::FALSE, &gfx.faces[i].data[0] as *const GLfloat);
                gl::DrawArrays(gl::TRIANGLE_FAN, 0, 4);
                gl::UniformMatrix4fv(gfx.offset, 1, gl::FALSE, &gfx.offset_subface_mat.data[0] as *const GLfloat);
                let f = &state.cube.faces[i];
                for j in 0..9{
                    let col = f.subfaces[j].color;
                    //println!("{:?}", col);
                    match col {
                        cube::Colors::Red => gl::Uniform3f(gfx.color, 1.0,0.0,0.0),
                        cube::Colors::Green => gl::Uniform3f(gfx.color, 0.0,1.0,0.0),
                        cube::Colors::Orange => gl::Uniform3f(gfx.color, 1.0,0.6,0.2),
                        cube::Colors::Blue => gl::Uniform3f(gfx.color, 0.0,0.0,1.0),
                        cube::Colors::White => gl::Uniform3f(gfx.color, 1.0,1.0,1.0),
                        cube::Colors::Yellow => gl::Uniform3f(gfx.color, 1.0,1.0,0.0),
                    }
                    let sft = &gfx.subfaces[j];
                    gl::UniformMatrix4fv(gfx.face_transform, 1, gl::FALSE, &((&gfx.faces[i] * sft).data[0]) as *const GLfloat);
                    gl::DrawArrays(gl::TRIANGLE_FAN, 0, 4);
                }
                //println!("fdsfds");
            }
            gl::Uniform3f(gfx.color, 1.0,1.0,1.0);
            gl::UniformMatrix4fv(gfx.offset, 1, gl::FALSE, &gfx.offset_mat.data[0] as *const GLfloat);
            for i in 0..5{
                gl::UniformMatrix4fv(gfx.face_transform, 1, gl::FALSE, &gfx.faces[i].data[0] as *const GLfloat);
                gl::DrawArrays(gl::LINE_LOOP, 0, 4);
            }
        }
        gfx.window.swap_buffers().unwrap();

        let ba = &state.cube.faces[cube::BACK];
        let l = &state.cube.faces[cube::LEFT];
        let t = &state.cube.faces[cube::TOP];
        let r = &state.cube.faces[cube::RIGHT];
        let bo = &state.cube.faces[cube::BOTTOM];
        let f = &state.cube.faces[cube::FRONT];

        let nb = |f,i|nb(f,i,&gfx.tc);
        let bb = |f,i|bb(f,i,&gfx.tc);

        let clear = tput(|f|f.arg("clear"));

        println!("{}              Back", clear);
        println!("              ┏━━━━━━━━━━━━━┓");
        println!("              ┃ {} {} {} ┃", nb(ba, 8), nb(ba, 7), nb(ba, 6));
        println!("              ┃ {} {} {} ┃", bb(ba, 8), bb(ba, 7), bb(ba, 6));
        println!("              ┃             ┃");
        println!("              ┃ {} {} {} ┃", nb(ba, 5), nb(ba, 4), nb(ba, 3));
        println!("              ┃ {} {} {} ┃", bb(ba, 5), bb(ba, 4), bb(ba, 3));
        println!("              ┃             ┃");
        println!("              ┃ {} {} {} ┃", nb(ba, 2), nb(ba, 1), nb(ba, 0));
        println!("Left          ┃ {} {} {} ┃    Right             Bottom", bb(ba, 2), bb(ba, 1), bb(ba, 0));
        println!("┏━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━━━━━━━━━┳━━━━━━━━━━━━━┓");
        println!("┃ {} {} {} ┃ {} {} {} ┃ {} {} {} ┃ {} {} {} ┃", nb(l,6), nb(l,3), nb(l,0),   nb(t,0), nb(t,1), nb(t,2),   nb(r,2), nb(r,5), nb(r,8),   nb(bo,0), nb(bo,1), nb(bo,2));
        println!("┃ {} {} {} ┃ {} {} {} ┃ {} {} {} ┃ {} {} {} ┃", bb(l,6), bb(l,3), bb(l,0),   bb(t,0), bb(t,1), bb(t,2),   bb(r,2), bb(r,5), bb(r,8),   bb(bo,0), bb(bo,1), bb(bo,2));
        println!("┃             ┃             ┃             ┃             ┃");
        println!("┃ {} {} {} ┃ {} {} {} ┃ {} {} {} ┃ {} {} {} ┃", nb(l,7), nb(l,4), nb(l,1),   nb(t,3), nb(t,4), nb(t,5),   nb(r,1), nb(r,4), nb(r,7),   nb(bo,3), nb(bo,4), nb(bo,5));
        println!("┃ {} {} {} ┃ {} {} {} ┃ {} {} {} ┃ {} {} {} ┃", bb(l,7), bb(l,4), bb(l,1),   bb(t,3), bb(t,4), bb(t,5),   bb(r,1), bb(r,4), bb(r,7),   bb(bo,3), bb(bo,4), bb(bo,5));
        println!("┃             ┃             ┃             ┃             ┃");
        println!("┃ {} {} {} ┃ {} {} {} ┃ {} {} {} ┃ {} {} {} ┃", nb(l,8), nb(l,5), nb(l,2),   nb(t,6), nb(t,7), nb(t,8),   nb(r,0), nb(r,3), nb(r,6),   nb(bo,6), nb(bo,7), nb(bo,8));
        println!("┃ {} {} {} ┃ {} {} {} ┃ {} {} {} ┃ {} {} {} ┃", bb(l,8), bb(l,5), bb(l,2),   bb(t,6), bb(t,7), bb(t,8),   bb(r,0), bb(r,3), bb(r,6),   bb(bo,6), bb(bo,7), bb(bo,8));
        println!("┗━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━━━━━━━━━┻━━━━━━━━━━━━━┛");
        println!("              ┃ {} {} {} ┃", nb(f, 0), nb(f, 1), nb(f, 2));
        println!("              ┃ {} {} {} ┃", bb(f, 0), bb(f, 1), bb(f, 2));
        println!("              ┃             ┃");
        println!("              ┃ {} {} {} ┃", nb(f, 3), nb(f, 4), nb(f, 5));
        println!("              ┃ {} {} {} ┃", bb(f, 3), bb(f, 4), bb(f, 5));
        println!("              ┃             ┃");
        println!("              ┃ {} {} {} ┃", nb(f, 6), nb(f, 7), nb(f, 8));
        println!("              ┃ {} {} {} ┃", bb(f, 6), bb(f, 7), bb(f, 8));
        println!("              ┗━━━━━━━━━━━━━┛");
        println!("              Front");
    }

    use glutin::event::{Event, WindowEvent};
    use glutin::event_loop::ControlFlow;
    events_loop.run(move |event, _win_target, cf|
        match event {
            //glutin::event::Event::WindowEvent{ event: glutin::event::WindowEvent::CloseRequested,..} => std::process::exit(0)
            Event::WindowEvent{ event: WindowEvent::CloseRequested,..} => *cf = ControlFlow::Exit
            ,Event::WindowEvent{ event: WindowEvent::Resized(newsize),..} => gfx_objs.window.resize(newsize)
            ,Event::RedrawRequested(_win) => {
                draw(&mut state, &mut gfx_objs);
            }
            ,Event::RedrawEventsCleared => {
                let start = Instant::now();
                update(&mut state);
                draw(&mut state, &mut gfx_objs);
                *cf = ControlFlow::WaitUntil(last_frame_start+frame_duration); last_frame_start = start;
            }
            ,_ => ()
        }
    );
}
