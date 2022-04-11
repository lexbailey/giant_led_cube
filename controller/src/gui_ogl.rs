extern crate gl;
extern crate glutin;

mod affine;
use gl::types::*;
use std::mem;
use std::ptr;
use std::os::raw::c_void;
use std::ffi::CString;
use std::time::{Instant,Duration};
use std::cell::RefCell;

mod gl_abstractions;
use gl_abstractions as gla;
use gla::{UniformMat4, UniformVec3};

pub mod client;
use client::{start_client, ToGUI, FromGUI, ClientState};

use cube_model as cube;
use cube::Cube;

use std::sync::{Arc,Mutex};


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
    // TODO move d, r, diff, and frames into the renderdata for the opengl version
    d: f32
    ,r: f32
    ,diff: f32
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
    ,events_loop: RefCell<Option<glutin::event_loop::EventLoop<()>>>
}


fn init_render_data() -> RenderData{
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

    let gfx_objs = unsafe{ 

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

        RenderData{
            shader: cube_shader
            ,cube_verts: cube_verts
            ,offset: offset
            ,offset_subface: offset_subface
            ,faces: face_transforms
            ,subfaces: subface_transforms
            ,window: gl_window
            ,events_loop: RefCell::new(Some(events_loop))
        }
    };
    gfx_objs
}

fn ui_loop(mut gfx: RenderData, state: Arc<Mutex<ClientState>>){

    let mut data = DataModel{
        d:0.0
        ,r:0.0
        ,diff: 0.0
        ,frames:0
    };

    fn update(data: &mut DataModel){
        data.d += data.diff;
        data.r += data.diff.abs()/2.0;
        data.r %= 1.0;
        data.frames += 1;
        if data.d >= 1.0 {data.diff = -0.01;}
        if data.d <= 0.0 {data.diff = 0.01;}
    }

    fn draw(data: &mut DataModel, gfx: &mut RenderData, state: &Arc<Mutex<ClientState>>){
        let state = state.lock().unwrap();
        unsafe {
            gl::ClearColor(data.d, 0.58, 0.92, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            gfx.shader.use_();
            gl::BindVertexArray(gfx.cube_verts);
            let transform = affine::Transform::<GLfloat>::rotate_xyz((3.14*2.0)/16.0, (3.14*2.0)*(data.r as f32), 0.0);
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
                        cube::Colors::Blank => gfx.shader.u_color.set(0.0,0.0,0.0),
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

    }

    let target_fps = 30.0;
    let frame_dur_ms: f32 = 1000.0/target_fps;

    let mut last_frame_start = Instant::now();

    let frame_duration = Duration::from_millis(frame_dur_ms.floor() as u64);

    use glutin::event::{Event, WindowEvent};
    use glutin::event_loop::ControlFlow;
    let events_loop = gfx.events_loop.take();
    events_loop.unwrap().run(move |event, _win_target, cf|
        match event {
            //glutin::event::Event::WindowEvent{ event: glutin::event::WindowEvent::CloseRequested,..} => std::process::exit(0)
            Event::WindowEvent{ event: WindowEvent::CloseRequested,..} => *cf = ControlFlow::Exit
            ,Event::WindowEvent{ event: WindowEvent::Resized(newsize),..} => gfx.window.resize(newsize)
            ,Event::RedrawRequested(_win) => {
                draw(&mut data, &mut gfx, &state);
            }
            ,Event::RedrawEventsCleared => {
                let start = Instant::now();
                update(&mut data);
                draw(&mut data, &mut gfx, &state);
                *cf = ControlFlow::WaitUntil(last_frame_start+frame_duration); last_frame_start = start;
            }
            ,_ => ()
        }
    );

}

fn main() {
    let gfx = init_render_data();

    let (state, sender, _c_receiver, _client) = start_client();

    let secret = b"secret".to_vec(); // TODO load from file
    let addr = "localhost:9876".to_string(); // TODO load from tile

    use client::FromGUI::*;
    sender.send(Connect(secret, addr));
    sender.send(SetState(Cube::new()));
    ui_loop(gfx, state);
}
