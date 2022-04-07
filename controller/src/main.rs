#[cfg(feature="opengl")]
extern crate gl;
#[cfg(feature="opengl")]
extern crate glutin;

mod affine;
use cube_model as cube;
use cube::{Output, OutputMap5Faces, Twist};

#[cfg(feature="opengl")]
use gl::types::*;
use std::mem;
use std::ptr;
use std::str;
use std::os::raw::c_void;
use std::ffi::CString;
use std::time::{Instant,Duration};
use rand::Rng;
use std::process::Command;
use std::cell::RefCell;
use std::io::{self,Read,Write,BufRead,BufReader};
use std::net::TcpStream;
use std::sync::mpsc::{channel,Sender};
use std::thread::{self,Thread,JoinHandle};
use std::collections::VecDeque;
use std::str::FromStr;
use std::collections::HashSet;

#[cfg(feature="opengl")]
mod gl_abstractions;
#[cfg(feature="opengl")]
use gl_abstractions as gla;
#[cfg(feature="opengl")]
use gla::{UniformMat4, UniformVec3};

use plain_authentic_commands::{MessageHandler, ParseStatus};

#[cfg(feature="opengl")]
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
    ,cube: cube::Cube
    ,frames: i32
}

trait Connector{
    type Stream;
    fn connect(addr: &str) -> std::io::Result<Self::Stream>;
}

struct Messenger<T: Read + Write, C: Connector>{
    handler: MessageHandler
    ,connector: C
    ,address: String
    ,stream: Option<T>
}

struct TcpConnector{
}

impl Connector for TcpConnector{
    type Stream = TcpStream;
    fn connect(addr: &str) -> std::io::Result<Self::Stream>{
       TcpStream::connect(addr)
    }
}

type TcpMessenger = Messenger<TcpStream, TcpConnector>;

enum Event {
    Response(Vec<u8>)
    ,Command(String, Vec<String>)
    ,DetectInputs()
}

fn handle_responses<T: Read>(stream: &mut T, events: Sender<Event>) {
    let mut reader = BufReader::new(stream);
    for line in reader.split(b'\n'){
        if let Ok(line) = line {
            events.send(Event::Response(line));
        }
        else {
            println!("Error: {:?}", line.err().unwrap());
        }
    }
}

impl<T: Read + Write> Messenger<T, TcpConnector>{
    fn new(secret: Vec<u8>, address: &str) -> Messenger<T, TcpConnector>{
        Messenger{
            handler: MessageHandler::signing_only(secret)
            ,connector: TcpConnector{}
            ,address: address.to_string()
            ,stream: None
        }
    }
}

impl<T: Read + Write, C: Connector<Stream=T>> Messenger<T, C>{
    fn connect(&mut self) -> std::io::Result<()>{
        let mut stream = C::connect(&self.address)?;
        stream.write_all(b"next_challenge:a#a\n")?;
        self.stream = Some(stream);
        Ok(())
    }

    fn get_stream(&mut self) -> std::io::Result<&mut Option<T>>{
        if self.stream.is_none(){
            self.connect()?
        }
        Ok(&mut self.stream)
    }

    fn send_command(&mut self, command: &str, args: &Vec<&str>) -> std::io::Result<()>{
        let message = self.handler.construct_message(command, args);
        let s = self.get_stream()?.as_mut().unwrap();
        s.write(message.as_bytes())?;
        Ok(())
    }
}

struct DetectState {
    twist: usize
    ,cur_sample: usize
    ,samples: [Option<u32>;5]
    ,map: [u32;18]
    ,complete: bool
    ,active: bool
}

enum DetectMessage {
    Nothing()
    ,TestState(String)
    ,Mapping(String)
}

impl DetectState{

    fn new() -> Self{
        DetectState{
            twist: 0
            ,cur_sample: 0
            ,samples: [None;5]
            ,map: [0;18]
            ,complete: false
            ,active: false
        }
    }

    fn activate(&mut self) {
        self.active = true;
    }

    fn detected_input_num(&mut self) -> Option<u32>{
        for s in self.samples{
            let mut n = 0;
            for  s2 in self.samples{
                if s2 == s {
                    n += 1;
                }
            }
            if n >= 3 {
                return s;
            }
        }
        None
    }

    fn reset_samples(&mut self) {
        for i in 0..5{
            self.samples[i] = None;
        }
    }

    fn ui(&mut self) -> String{
        let mut test_state = [b' ';54];
        let (red, green) = match self.twist {
            0 => (8,4*9)
            ,1 => (4*9,8)
            ,2 => ((4*9)+2,2)
            ,3 => (2,(4*9)+2)
            ,4 => (9+2,8)
            ,5 => (8,9+2)
            ,6 => (6,9)
            ,7 => (9,6)
            ,8 => (4*9,9+2)
            ,9 => (9+2,4*9)
            ,10 => (9+8,(4*9)+6)
            ,11 => ((4*9)+6,9+8)

            ,12 => (9+5,(4*9)+3)
            ,13 => ((4*9)+3,9+5)

            ,14 => (7,9+1)
            ,15 => (9+1,7)

            ,16 => (5,(4*9)+1)
            ,17 => ((4*9)+1,5)
            ,_ => (0,0)
        };
        test_state[red] = b'R';
        test_state[green] = b'G';
        println!("Push the switch between the red and green LEDs towards the green LED. Repeat several times to continue.");
        String::from_utf8_lossy(&test_state).to_string()
    }

    fn sample_input(&mut self, sample: u32) -> DetectMessage{
        use DetectMessage::*;
        if !self.active {
            return Nothing();
        }
        self.samples[self.cur_sample] = Some(sample);
        self.cur_sample = (self.cur_sample + 1) % 5;
        if let Some(input) = self.detected_input_num(){
            println!("Mapping input {} to twist number {}", input, self.twist);
            self.map[self.twist] = input;
            self.reset_samples();
            self.twist += 1;
            if self.twist > 17 {
                self.complete = true;
                let duplicates = self.map.iter().collect::<HashSet<_>>().len() != self.map.len();
                if duplicates {
                    println!("Some inputs were duplicated, this config is invalid, try again.");
                }
                else{
                    println!("TODO send new mapping");
                    // TODO generate mapping
                    let mut mapping = String::with_capacity(36);
                    for i in 0..18{
                        mapping.push_str(&format!("{:02}", self.map[i]));
                    }
                    return Mapping(mapping);
                }
                self.active = false;
                TestState("                                                      ".to_string())
            }
            else{
                TestState(self.ui())
            }
        }
        else{
            Nothing()
        }
    }
}

fn start_service_threads() -> io::Result<(JoinHandle<()>, JoinHandle<()>, Sender<Event>)>{

    let (sender, receiver) = channel();

    // Split the sender into two
    let CLI_sender = sender.clone();
    let service_sender = sender;

    let mut msg = TcpMessenger::new(b"secret".to_vec(), "localhost:9876");
    msg.connect()?;

    let mut reader = msg.stream.as_ref().unwrap().try_clone()?;

    let net_thread = thread::spawn(move||{
        handle_responses(&mut reader, service_sender);
    });
    
    let event_thread = thread::spawn(move||{
        let mut command_queue: VecDeque<(String, Vec<String>)> = VecDeque::new();
        let mut got_challenge = false;

        fn send_events(got_challenge: &mut bool, command_queue: &mut VecDeque<(String, Vec<String>)>, msg: &mut TcpMessenger){
            if *got_challenge {
                if let Some((command, args)) = command_queue.pop_front() {
                    *got_challenge = false;
                    //println!("Send command: {}, {:?}", command, args);
                    let args = args.iter().map(|a|a.as_ref()).collect();
                    msg.send_command(&command, &args);
                }
            }
        }

        let mut detect_state = DetectState::new();

        for event in receiver.iter(){
            match event {
                Event::Response(s) => {
                    match msg.handler.parse_response(&s) {
                        ParseStatus::Success(response, args) => {
                            match response.as_ref() {
                                "challenge" => {
                                    got_challenge = true;
                                }   
                                ,"input" => {
                                    println!("user applied input: {}", args[0]);
                                    if let Ok(input) = u32::from_str(&args[0]){
                                        use DetectMessage::*;
                                        match detect_state.sample_input(input){
                                            Nothing() => {
                                                // do nothing
                                            }
                                            ,TestState(test_state) => {
                                                command_queue.push_back(("set_state".to_string(), vec![test_state]));
                                            }
                                            ,Mapping(mapping) => {
                                                command_queue.push_back(("input_mapping".to_string(), vec![mapping]));
                                            }
                                        }
                                    }
                                    else {
                                        println!("Not a valid number: {}", args[0]);
                                    }
                                }
                                ,"twist" => {
                                    println!("Twist: {}", args[0]);
                                }
                                ,r=>{
                                    eprintln!("TODO handle response: {}", r);
                                }   
                            };  
                        }   
                        ,ParseStatus::BadClient() => {
                            eprintln!("Reply appears malformed");
                            return;
                        }
                        ,ParseStatus::Unauthorised() => {
                            eprintln!("Reply appears inauthentic");
                            return;
                        }
                    };
                }
                ,Event::Command(command, args) => {
                    command_queue.push_back((command, args));
                }
                ,Event::DetectInputs() => {
                    command_queue.push_back(("detect".to_string(), vec!["inputs".to_string()]));
                    detect_state = DetectState::new();
                    detect_state.activate();
                    let test_state = detect_state.ui();
                    command_queue.push_back(("set_state".to_string(), vec![test_state]));
                }
            }
            send_events(&mut got_challenge, &mut command_queue, &mut msg);
        }
    });
    Ok((net_thread, event_thread, CLI_sender))
}

#[cfg(feature="opengl")]
use glutin::ContextWrapper;
#[cfg(feature="opengl")]
use glutin::PossiblyCurrent;

#[cfg(feature="opengl")]
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

#[cfg(feature="cli")]
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

#[cfg(feature="cli")]
struct RenderData{
    tc: TermCols
}

#[cfg(feature="cli")]
fn tput (f:fn (&mut Command)-> &mut Command) -> String {
    String::from_utf8(f(&mut Command::new("tput")).output().expect("tput failed").stdout).unwrap()
}

#[cfg(feature="cli")]
fn color_string(s: String, col: cube::Colors, tc: &TermCols) -> String {
    format!("{}{}{:03}{}", tc.fg_black, match col {
        cube::Colors::White => &tc.white
        ,cube::Colors::Red => &tc.red
        ,cube::Colors::Green => &tc.green
        ,cube::Colors::Yellow => &tc.yellow
        ,cube::Colors::Blue => &tc.blue
        ,cube::Colors::Orange => &tc.orange
        ,cube::Colors::Blank => ""
    }, s, tc.default)
}


#[cfg(feature="opengl")]
fn init_render_data_opengl() -> RenderData{
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

fn init_render_data_cli() -> RenderData{
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
        tc: tc
    }
}

fn init_render_data() -> RenderData{
    #[cfg(feature="opengl")]
    {init_render_data_opengl()}
    #[cfg(feature="cli")]
    {init_render_data_cli()}
}

#[cfg(feature="opengl")]
fn ui_loop_opengl(mut gfx: RenderData, mut data: DataModel){
    fn update(data: &mut DataModel){
        data.d += data.diff;
        data.r += data.diff.abs()/2.0;
        data.r %= 1.0;
        data.frames += 1;
        if data.d >= 1.0 {data.diff = -0.01;}
        if data.d <= 0.0 {data.diff = 0.01;}
        
        if data.frames % 30 == 0{
            let face = rand::thread_rng().gen_range(0..6);
            let dir: i32 = rand::thread_rng().gen_range(0..2);
            data.cube.twist(cube::Twist{face:face, reverse:dir==0});
        }
    }

    fn draw(data: &mut DataModel, gfx: &mut RenderData){
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
                let f = &data.cube.faces[i];
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
                draw(&mut data, &mut gfx);
            }
            ,Event::RedrawEventsCleared => {
                let start = Instant::now();
                update(&mut data);
                draw(&mut data, &mut gfx);
                *cf = ControlFlow::WaitUntil(last_frame_start+frame_duration); last_frame_start = start;
            }
            ,_ => ()
        }
    );

}

fn ui_loop_cli(mut gfx: RenderData, mut data: DataModel){
    use std::io::{self, BufRead, StdinLock, Write};
    let stdin = io::stdin();
    let mut user_input = stdin.lock().lines();
    fn nb (f: &cube::Face, i:usize, tc: &TermCols) -> String{ color_string(i.to_string(), f.subfaces[i].color, &tc) }
    fn bb (f: &cube::Face, i:usize, tc: &TermCols) -> String{ color_string("".to_string(), f.subfaces[i].color, &tc) }
    fn draw(gfx: &RenderData, data: &DataModel){
        let ba = &data.cube.faces[cube::BACK];
        let l = &data.cube.faces[cube::LEFT];
        let t = &data.cube.faces[cube::TOP];
        let r = &data.cube.faces[cube::RIGHT];
        let bo = &data.cube.faces[cube::BOTTOM];
        let f = &data.cube.faces[cube::FRONT];

        let nb = |f,i|nb(f,i,&gfx.tc);
        let bb = |f,i|bb(f,i,&gfx.tc);

        //let clear = tput(|f|f.arg("clear"));

        //println!("{}              Back", clear);
        println!("              Back ({})", cube::BACK);
        println!("              ┏━━━━━━━━━━━━━┓");
        println!("              ┃ {} {} {} ┃", nb(ba, 8), nb(ba, 7), nb(ba, 6));
        println!("              ┃ {} {} {} ┃", bb(ba, 8), bb(ba, 7), bb(ba, 6));
        println!("              ┃             ┃");
        println!("              ┃ {} {} {} ┃", nb(ba, 5), nb(ba, 4), nb(ba, 3));
        println!("              ┃ {} {} {} ┃", bb(ba, 5), bb(ba, 4), bb(ba, 3));
        println!("              ┃             ┃");
        println!("              ┃ {} {} {} ┃", nb(ba, 2), nb(ba, 1), nb(ba, 0));
        println!("Left ({})      ┃ {} {} {} ┃    Right ({})      Bottom ({})", cube::LEFT, bb(ba, 2), bb(ba, 1), bb(ba, 0), cube::RIGHT, cube::BOTTOM);
        println!("┏━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━━━━━━━━━┳━━━━━━━━━━━━━┓");
        println!("┃ {} {} {} ┃ {} {} {} ┃ {} {} {} ┃ {} {} {} ┃", nb(l,6), nb(l,3), nb(l,0),   nb(t,0), nb(t,1), nb(t,2),   nb(r,2), nb(r,5), nb(r,8),   nb(bo,0), nb(bo,1), nb(bo,2));
        println!("┃ {} {} {} ┃ {} {} {} ┃ {} {} {} ┃ {} {} {} ┃", bb(l,6), bb(l,3), bb(l,0),   bb(t,0), bb(t,1), bb(t,2),   bb(r,2), bb(r,5), bb(r,8),   bb(bo,0), bb(bo,1), bb(bo,2));
        println!("┃             ┃    Top ({})  ┃             ┃             ┃", cube::TOP);
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
        println!("              Front ({})", cube::FRONT);
    }

    fn prompt(data: &mut DataModel, user_input: &mut std::io::Lines<StdinLock<'_>>) -> String{
        print!("Cube control> ");
        std::io::stdout().flush();
        match user_input.next(){
            None => {std::process::exit(0);}
            Some(result) => {match result{
                Err(e) => println!("Error: {:?}", e)
                ,Ok(line) => return line
            }}
        }
        std::process::exit(1);
    }

    let mut sender: Option<Sender<Event>> = None;
    let mut net_thread: Option<JoinHandle<()>> = None;
    let mut event_thread: Option<JoinHandle<()>> = None;


    fn send_event(sender: &Option<Sender<Event>>, ev: Event) {
        if sender.is_none(){
            println!("Not connected, please run `connect` command first.");
            return;
        }
        let sender = sender.as_ref().unwrap();
        let result = sender.send(ev);
        match result {
            Ok(_) => {}
            ,Err(e) => {println!("Failed to send command: {:?}", e);}
        }
    }

    fn send_command(sender: &Option<Sender<Event>>, command: &str, args: Vec<&str>){
        send_event(sender, Event::Command(command.to_string(), args.iter().map(|s|s.to_string()).collect()));
    }

    draw(&gfx, &data);

    let mut led_map: OutputMap5Faces = [Output{face:0,subface:0}; 45];

    let mut detecting_led: u32 = 0;
    
    fn detect_led_ui(led: u32, sender: &Option<Sender<Event>>){
        if sender.is_none(){
            println!("not connected, connect and then run 'detect again'");
            return;
        }
        let mut test_state = String::new();
        for i in 0..54{
            test_state.push(if i == led {'W'} else {' '});
        }
        send_command(sender, "set_state", vec![&test_state]);
        println!("LED {} is lit. Use 'map <F> <S>' to map this LED to face F and subface S", led);
    }
    
    loop {
        let command = prompt(&mut data, &mut user_input);
        match command.as_ref(){
            "connect" => {
                if sender.is_some(){
                    sender = None;
                    net_thread.unwrap().join();
                    net_thread = None;
                    event_thread.unwrap().join();
                    event_thread = None;
                }
                let result = start_service_threads();
                match result{
                    Err(e) => {
                        println!("Error connecting to remote: {:?}", e);
                    }
                    Ok((new_net, new_events, new_sender)) => {
                        sender = Some(new_sender);
                        net_thread = Some(new_net);
                        event_thread = Some(new_events);
                        println!("Connected");
                        send_command(&sender, "play", vec![]);
                    }
                }
            }
            ,"show" => {
                draw(&gfx, &data);
            }
            ,"solved" => {
                data.cube = cube_model::Cube::new();
                send_command(&sender, "set_state", vec![&data.cube.serialise()]);
                draw(&gfx, &data);
            }
            ,"detect leds" => {
                println!("Starting LED detect sequence...");
                println!("Use 'detect next' to move to next LED");
                send_command(&sender, "detect", vec!["leds"]);
                detecting_led = 0;
                detect_led_ui(detecting_led, &sender);
            }
            ,"detect inputs" => {
                println!("Starting input detect sequence...");
                println!("Use 'detect next' to move to next input");
                send_event(&sender, Event::DetectInputs());
            }
            ,"detect next" => {
                println!("Next item...");
                detecting_led += 1;
                detect_led_ui(detecting_led, &sender);
            }
            ,"detect done" => {
                println!("Done detecting, sending new config");
                send_command(&sender, "led_mapping", vec![&cube::serialise_output_map(&led_map)]);
                send_command(&sender, "play", vec![]);
            }
            ,"start" => {
                data.cube = cube_model::Cube::new();
                let mut last_twist = Twist::from_string("F").unwrap();
                let mut twist = Twist::from_string("F").unwrap();
                let mut rng = rand::rngs::OsRng;
                // A very naive scramble algorithm
                for i in 0..30{
                    while twist == last_twist{
                        twist = Twist{
                            face: rng.gen_range(0..6)
                            ,reverse: rng.gen_bool(0.5)
                        }
                    }
                    last_twist = twist;
                    data.cube.twist(twist);
                }
                send_command(&sender, "set_state", vec![&data.cube.serialise()]);
                send_command(&sender, "play", vec![]);
            }
            ,"" => {}
            ,cmd => {
                let mut parts = cmd.split(' ');
                let name = parts.next().unwrap();
                let args_str = &cmd[name.len()..cmd.len()];
                let args = parts.collect::<Vec<&str>>();
                match name.as_ref(){
                    "twist" => {
                        match data.cube.twists(args_str){
                            Err(msg) => {println!("Error: {}", msg)}
                            ,Ok(_) => {
                                send_command(&sender, "set_state", vec![&data.cube.serialise()]);
                                draw(&gfx, &data);
                                println!("Done twists.");
                            }
                        }
                    }
                    ,"map" => {
                        if args.len() != 2{
                            println!("map requires two parameters");
                        }
                        else{
                            if let Ok((f, s)) = (||{
                                Result::<(usize, usize), std::num::ParseIntError>::Ok((
                                    usize::from_str(args[0])?
                                    ,usize::from_str(args[1])?
                                ))
                            })() {
                                led_map[detecting_led as usize] = Output{face:f, subface:s};
                                println!("mapped led {} to (face, subface) = ({}, {})", detecting_led, f, s);
                                detecting_led += 1;
                                detect_led_ui(detecting_led, &sender);
                            }
                        }
                    }
                    ,_ => {println!("Unknown command: {}",cmd);}
                }
            }
        }
    }
}

fn ui_loop(mut gfx: RenderData, mut data: DataModel){
    #[cfg(feature="opengl")]
    ui_loop_opengl(gfx, data);
    #[cfg(feature="cli")]
    ui_loop_cli(gfx, data);
}

#[cfg(all(feature="cli", feature="opengl"))]
compile_error!("Cannot compile with both the opengl interface _and_ the cli interface. Choose only one.");

fn main() {
    
    let gfx = init_render_data();
    
    let mut data = DataModel{
        d: 0.0
        ,r: 0.0
        ,diff: 0.01
        ,cube: cube::Cube::new()
        ,frames: 0
    };
    
    ui_loop(gfx, data);
}
