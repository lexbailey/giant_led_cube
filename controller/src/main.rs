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

mod gl_abstractions;
use gl_abstractions as gla;
use gla::{UniformMat4, UniformVec3};

use plain_authentic_commands::{AuthState};

shader_struct!{
    CubeShader
    ,r#"
        #version 330 core
        layout (location = 0) in vec4 aPos;
        uniform mat4 u_face_transform;
        uniform mat4 u_offset;
        uniform mat4 u_transform;
        void main()
        {
            gl_Position = aPos  * u_offset * u_face_transform * u_transform;
        }
        "#
    ,r#"
        #version 330 core
        out vec4 FragColor;
        uniform vec3 u_color;
        void main()
        {
           // Set the fragment color to the color passed from the vertex shader
           FragColor = vec4(u_color, 1.0);
        }
        "#
    ,{
        u_face_transform: UniformMat4,
        u_offset: UniformMat4,
        u_transform: UniformMat4,
        u_color: UniformVec3,
    }
}

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
    ,offset: affine::Transform<f32>
    ,offset_subface: affine::Transform<f32>
    ,faces: Vec<affine::Transform<f32>>
    ,subfaces: Vec<affine::Transform<f32>>
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

    let cube_shader = CubeShader::new();

    let mut gfx_objs = unsafe{ 

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

        RenderData{
            shader: cube_shader
            ,cube_verts: cube_verts
            ,offset: offset
            ,offset_subface: offset_subface
            ,faces: face_transforms
            ,subfaces: subface_transforms
            ,window: gl_window
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
        }
    }

    fn draw(state: &mut DataModel, gfx: &mut RenderData){
        unsafe {
            gl::ClearColor(state.d, 0.58, 0.92, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            gfx.shader.use_();
            gl::BindVertexArray(gfx.cube_verts);
            let transform = affine::Transform::<GLfloat>::rotate_xyz((3.14*2.0)/16.0, (3.14*2.0)*(state.r as f32), 0.0);
            gfx.shader.u_transform.set(&transform.data);

            for i in 0..5{
                gfx.shader.u_offset.set(&gfx.offset.data);
                gfx.shader.u_color.set(0.0,0.0,0.0);
                gfx.shader.u_face_transform.set(&gfx.faces[i].data);
                gl::DrawArrays(gl::TRIANGLE_FAN, 0, 4);
                gfx.shader.u_offset.set(&gfx.offset_subface.data);
                let f = &state.cube.faces[i];
                for j in 0..9{
                    let col = f.subfaces[j].color;
                    //println!("{:?}", col);
                    match col {
                        cube::Colors::Red => gfx.shader.u_color.set(1.0,0.0,0.0),
                        cube::Colors::Green => gfx.shader.u_color.set(0.0,1.0,0.0),
                        cube::Colors::Orange => gfx.shader.u_color.set(1.0,0.6,0.2),
                        cube::Colors::Blue => gfx.shader.u_color.set(0.0,0.0,1.0),
                        cube::Colors::White => gfx.shader.u_color.set(1.0,1.0,1.0),
                        cube::Colors::Yellow => gfx.shader.u_color.set(1.0,1.0,0.0),
                    }
                    let sft = &gfx.subfaces[j];
                    gfx.shader.u_face_transform.set(&((&gfx.faces[i] * sft).data));
                    gl::DrawArrays(gl::TRIANGLE_FAN, 0, 4);
                }
            }
            gfx.shader.u_color.set(1.0,1.0,1.0);
            gfx.shader.u_offset.set(&gfx.offset.data);
            for i in 0..5{
                gfx.shader.u_face_transform.set(&gfx.faces[i].data);
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
