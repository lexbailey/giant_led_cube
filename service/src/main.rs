#![allow(non_upper_case_globals)]
use std::net::TcpListener;
use std::thread;
use std::sync::mpsc::{channel,Sender,Receiver,SendError};
use std::io::{Write,Read,BufReader,BufRead};
use std::str::FromStr;
use std::fs::File;
use std::path::Path;
use std::marker::Send;
use clap::Parser as CLIParser;
use plain_authentic_commands::{MessageHandler, ParseStatus};
extern crate pest;
use serde::{Deserialize, Serialize};
use cube_model::{Cube, Twist};
use thiserror::Error;
use game_timer::TimerState;
use std::time::{Instant,Duration};
use serialport::SerialPort;
use rodio::{Decoder, OutputStream, source::Source, source::Buffered};
use rand::Rng;
use std::io::Cursor;

#[cfg(feature="output_mode_jumbotron")]
use {
    std::{mem,ptr},
    std::ffi::c_void,
    std::sync::{Arc,Mutex},
    gl::types::*,
    glfw::Context,
    gl_abstractions::*,
    affine::Transform,
};

#[derive(CLIParser, Debug)]
struct Args{
    #[clap()]
    config: String,
    #[clap()]
    device: String,
    /// TCP addr:port to listen on to serve the controller interface (example: --tcp localhost:9876)
    #[clap(long)]
    tcp: Option<String>,
    /// Name of a serial device to use to serve the controller interface (example: --serial /dev/ttyUSB0)
    #[clap(long)]
    serial: Option<String>,
    /// Enable jumbotron output mode
    #[cfg(feature="output_mode_jumbotron")]
    #[clap(long="jumbotron")]
    jumbotron: bool,
}

#[derive(Serialize, Deserialize, Clone)]
struct CubeConfig{
    led_map: String,
    input_map: String,
    secret: String,
    top_score: u128,
    facelet_px: Option<u32>,
    rotation_map: String,
}

enum DeviceEvent{
    Switch(i32)
    ,Solved()
    ,Twist(Twist)
}

enum StreamEvent{
    GUI(DeviceEvent)
    ,RecvLine(Vec<u8>)
    ,EOS()
    ,SyncTimers((String, String, String))
    ,ReportTime(Duration)
    ,CubeState(Cube)
    ,RecordState(u128)
}

enum ClientEvent{
    Connected(Sender<StreamEvent>),
    SetState(String),
    GetState(),
    StartDetectLED(),
    StartDetectSwitches(),
    UpdateLEDMap(String),
    UpdateInputMap(String),
    Play(),
    StartTimedGame(),
    CancelTimedGame(),
    SetBrightness(u8),
    EnableCalibrationView(),
    DisableCalibrationView(),
    RotateSubface(usize,usize),
    ApplyTwist(String),
}

enum Event{
    Client(ClientEvent)
    ,Device(DeviceEvent)
}

enum Sound{
    Twist()
    ,Win()
    ,NoMoreSounds()
}

#[derive(Error, Debug)]
enum EvStreamError {
    #[error("IO Error: {0}")]
    IO(#[from] std::io::Error)
    ,#[error("Sender Error: {0}")]
    Sender(#[from] std::sync::mpsc::SendError<Event>)
    ,#[error("Stream Sender Error: {0}")]
    StreamSender(#[from] std::sync::mpsc::SendError<StreamEvent>)
}



fn handle_stream<R: 'static + Read + Send + Sync, W: 'static + Write + Send + Sync>(read_stream: R, mut write_stream: W, sender: Sender<Event>, secret: Vec<u8>){
    let mut auth = MessageHandler::new(secret);
    let buffer = BufReader::new(read_stream);
    let (stream_sender, stream_receiver) = channel::<StreamEvent>();
    let gui_sender = stream_sender.clone();
    
    let stream_thread = thread::spawn(move||{
        match sender.send(Event::Client(ClientEvent::Connected(gui_sender))) {
            Err(e) => {println!("Error handling incoming connection: {:?}", e);}
            Ok(_) => {
                for event in stream_receiver.iter() {
                    use StreamEvent::*;
                    enum EvDone {Done, Loop}
                    use EvDone::*;
                    let r: Result<EvDone, EvStreamError> = (||{
                        match event{
                            EOS() => {
                                Ok(Done)
                            }
                            ,RecvLine(line) => {
                                match auth.parse_command(&line) {
                                    ParseStatus::Success(command, args) => {
                                        match command.as_ref() {
                                            "next_challenge" => {
                                                // Do nothing, command exists purely to cause a challenge to be sent
                                                // The next challenge is sent after each command anyway
                                            }
                                            ,"set_state" => {
                                                if args.len() >= 1{
                                                    println!("Set absolute cube state: {}", args[0]);
                                                    sender.send(Event::Client(ClientEvent::SetState(args[0].clone())))?;
                                                }
                                                else{
                                                    let msg = auth.construct_reply("wrong_arguments", &vec![&command]);
                                                    write_stream.write(msg.as_bytes())?;
                                                }
                                            }
                                            ,"get_state" => {
                                                sender.send(Event::Client(ClientEvent::GetState()))?;
                                            }
                                            ,"detect" => {
                                                if args.len() < 1{
                                                    let msg = auth.construct_reply("wrong_arguments", &vec![&command]);
                                                    write_stream.write(msg.as_bytes())?;
                                                }
                                                let subcommand = &args[0];
                                                match subcommand.as_ref() {
                                                    "leds" => { sender.send(Event::Client(ClientEvent::StartDetectLED()))?; }
                                                    "inputs" => { sender.send(Event::Client(ClientEvent::StartDetectSwitches()))?; }
                                                    ,_ => {
                                                        let msg = auth.construct_reply("unknown_subcommand", &vec![&command]);
                                                        write_stream.write(msg.as_bytes())?;
                                                    }
                                                }
                                            }
                                            ,"led_mapping" => {
                                                if args.len() != 1{
                                                    let msg = auth.construct_reply("wrong_arguments", &vec![&command]);
                                                    write_stream.write(msg.as_bytes())?;
                                                }
                                                let new_mapping = &args[0];
                                                sender.send(Event::Client(ClientEvent::UpdateLEDMap(new_mapping.clone())))?;
                                            }
                                            ,"input_mapping" => {
                                                if args.len() != 1{
                                                    let msg = auth.construct_reply("wrong_arguments", &vec![&command]);
                                                    write_stream.write(msg.as_bytes())?;
                                                }
                                                let new_mapping = &args[0];
                                                sender.send(Event::Client(ClientEvent::UpdateInputMap(new_mapping.clone())))?;

                                            }
                                            ,"play" => {
                                                sender.send(Event::Client(ClientEvent::Play()))?;
                                            }
                                            ,"timed_start" => {
                                                sender.send(Event::Client(ClientEvent::StartTimedGame()))?;
                                            }
                                            ,"cancel_timer" => {
                                                sender.send(Event::Client(ClientEvent::CancelTimedGame()))?;
                                            }
                                            ,"set_brightness" => {
                                                if args.len() != 1{
                                                    let msg = auth.construct_reply("wrong_arguments", &vec![&command]);
                                                    write_stream.write(msg.as_bytes())?;
                                                }
                                                let b = u8::from_str(&args[0]);
                                                match b {
                                                    Err(_) => {
                                                        let msg = auth.construct_reply("bad_argument", &vec![&command]);
                                                        write_stream.write(msg.as_bytes())?;
                                                    }
                                                    ,Ok(b) => {sender.send(Event::Client(ClientEvent::SetBrightness(b)))?;}
                                                }
                                            },
                                            "enable_calibration" => {
                                                sender.send(Event::Client(ClientEvent::EnableCalibrationView()))?;
                                            },
                                            "disable_calibration" => {
                                                sender.send(Event::Client(ClientEvent::DisableCalibrationView()))?;
                                            },
                                            "rotate_subface" => {
                                                if args.len() != 2{
                                                    let msg = auth.construct_reply("wrong_arguments", &vec![&command]);
                                                    write_stream.write(msg.as_bytes())?;
                                                }
                                                let f = usize::from_str(&args[0]);
                                                let sf = usize::from_str(&args[1]);
                                                match (f, sf) {
                                                    (Ok(f),Ok(sf)) => {sender.send(Event::Client(ClientEvent::RotateSubface(f,sf)))?;}
                                                    _ => {
                                                        let msg = auth.construct_reply("bad_argument", &vec![&command]);
                                                        write_stream.write(msg.as_bytes())?;
                                                    }
                                                }
                                            },
                                            _=>{
                                                let msg = auth.construct_reply("unknown_command", &vec![&command]);
                                                write_stream.write(msg.as_bytes())?;
                                            }
                                        };
                                        auth.step();
                                        let msg = auth.construct_reply("challenge", &vec![&auth.get_salt()]);
                                        write_stream.write(msg.as_bytes())?;
                                    }
                                    // Dont sign replies to messages that are not authorised. If we don't trust the source, we won't sign things for them
                                    ,ParseStatus::BadClient() => {write_stream.write(b"+malformed_command:a#a\n")?; return Ok(Done);}
                                    ,ParseStatus::Unauthorised() => {write_stream.write(b"+auth_fail:a#a\n")?; return Ok(Done);}
                                };
                                Ok(Loop)
                            }
                            ,GUI(e) => {
                                match e {
                                    DeviceEvent::Switch(i) => {
                                        let msg = auth.construct_reply("input", &vec![&format!("{}", i)]);
                                        write_stream.write(msg.as_bytes())?;
                                    }
                                    ,DeviceEvent::Twist(t) => {
                                        let msg = auth.construct_reply("twist", &vec![&format!("{}", t)]);
                                        write_stream.write(msg.as_bytes())?;
                                    }
                                    ,DeviceEvent::Solved() => {
                                        let msg = auth.construct_reply("solved", &vec![]);
                                        write_stream.write(msg.as_bytes())?;
                                    }
                                }
                                Ok(Loop)
                            }
                            ,SyncTimers((a,b,c)) => {
                                let msg = auth.construct_reply("timer_state", &vec![&a,&b,&c]);
                                write_stream.write(msg.as_bytes())?;
                                Ok(Loop)
                            }
                            ,ReportTime(dur) => {
                                let msg = auth.construct_reply("solve_time", &vec![&format!("{}", dur.as_millis())]);
                                write_stream.write(msg.as_bytes())?;
                                Ok(Loop)
                            }
                            ,CubeState(cube) => {
                                let msg = auth.construct_reply("cube_state", &vec![&cube.serialise()]);
                                write_stream.write(msg.as_bytes())?;
                                Ok(Loop)
                            }
                            ,RecordState(record) => {
                                let msg = auth.construct_reply("record_time", &vec![&format!("{}", record)]);
                                write_stream.write(msg.as_bytes())?;
                                Ok(Loop)
                            }
                        }
                    })();
                    match r {
                        Ok(Done) => {break;}
                        Err(e) => {println!("Error handling stream event: {:?}", e); break;}
                        Ok(Loop) => {}
                    }
                }
            }
        }
    });

    for line_result in buffer.split(b'\n'){
        match line_result {
            Ok(line) => {
                match stream_sender.send(StreamEvent::RecvLine(line)) {
                    Err(e) => {
                        println!("Internal error sedding event to event handler: {:?}", e);
                        break;
                    }
                    ,Ok(_) =>{}
                }}
            ,Err(e) => {
                println!("Unable to read from remote: {:?}", e);
                break;
            }
        }
    }
    let _ignored = stream_sender.send(StreamEvent::EOS());
    println!("Client stream ended, disconnected.");
    let _ignored = stream_thread.join();
}

fn persist_config(config: &CubeConfig, file: &str) {
    let p = Path::new(file);
    match File::create(p) {
        Err(e) => {println!("Unable to persist config to file '{}': {}", file, e);}
        ,Ok(f) => {
            match serde_json::to_writer_pretty(f, config){
                Err(e) => {println!("Unable to persist config to file '{}': {}", file, e);}
                ,Ok(_) => {}
            }
        }
    }
}

fn send_state_to_client(gui_sender: Option<&Sender<StreamEvent>>, cube: Cube, record: u128) -> Result<(), SendError<StreamEvent>>{
    if let Some(sender) = gui_sender {
        sender.send(StreamEvent::CubeState(cube))?;
        sender.send(StreamEvent::RecordState(record))?;
    }
    Ok(())
}

enum CubeDevice{
    PhysicalDevice{serial_port:Box<dyn SerialPort>},
    TestDevice{sequence:Vec<Twist>, buffer: Vec<u8>, next_twist: Instant},
    IdleDevice{},
}

impl Read for CubeDevice{
    fn read(&mut self, data: &mut [u8]) -> Result<usize, std::io::Error> {
        use CubeDevice::*;
        match self{
            TestDevice{sequence, buffer, next_twist, ..} => {
                if sequence.len() <= 0 {
                    // Generate a new sequence
                    for _ in 0..20{
                        sequence.push(Twist::from_string("F").unwrap());
                    }
                    for _ in 0..20{
                        sequence.push(Twist::get_random())
                    }
                    for i in 0..20{
                        sequence.push(sequence[19-i].inverse())
                    }
                    *next_twist = Instant::now() + Duration::from_secs(3);
                }
                while *next_twist < Instant::now() {
                    let next = sequence.remove(0);
                    *next_twist = *next_twist + Duration::from_millis(5000);
                    buffer.extend_from_slice(format!("*{};\n", next).as_bytes());
                }
                let mut mdata = data;
                let n = mdata.write(&buffer)?;
                let mut newbuf = Vec::new();
                newbuf.extend_from_slice(&buffer[n..]);
                *buffer = newbuf;
                Ok(n)
            },
            PhysicalDevice{serial_port} => {serial_port.read(data)},
            IdleDevice{..} => {Ok(0)},
        }
    }
}

impl Write for CubeDevice{
    fn write(&mut self, data: &[u8]) -> Result<usize, std::io::Error> {
        use CubeDevice::*;
        match self{
            TestDevice{..} | IdleDevice{..} => { Ok(data.len()) },
            PhysicalDevice{serial_port} => {serial_port.write(data)},
        }
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        use CubeDevice::*;
        match self{
            TestDevice{..} => {Ok(())}, // Does nothing on the test device
            IdleDevice{..} => {Ok(())},
            PhysicalDevice{serial_port} => {serial_port.flush()},
        }
    }
}

impl CubeDevice{
    fn from_device_name(name: &str) -> Result<CubeDevice, ()>{
        use CubeDevice::*;
        if name == "testdevice" {
            // testdevice is a special name that refers to a fake cube controller that does not display anything, but will repeatedly apply and unapply
            // random sequences for testing purposes
            Ok(TestDevice{ sequence: Vec::new(), buffer: Vec::new(), next_twist:Instant::now() })
        }
        else if name == "idledevice" {
            // testdevice is a special name that refers to a fake cube controller that does not display anything, but will repeatedly apply and unapply
            // random sequences for testing purposes
            Ok(IdleDevice{})
        }
        else{
            let result = serialport::new(name, 115200).timeout(Duration::from_secs(10)).open();
            match result{
                Ok(device) => Ok(PhysicalDevice{serial_port:device}),
                Err(_e) => Err(()) // TODO better error handling here
            }
        }
    }

    fn try_clone(&self) -> Result<CubeDevice, ()>{
        use CubeDevice::*;
        match self{
            TestDevice{sequence, buffer, next_twist} => Ok(TestDevice{sequence:sequence.clone(), buffer:buffer.clone(), next_twist: *next_twist}),
            IdleDevice{} => Ok(IdleDevice{}),
            PhysicalDevice{serial_port} => match serial_port.try_clone() {
                Ok(cloned) => Ok(PhysicalDevice{serial_port: cloned}),
                Err(_e) => Err(()), // TODO better error handling here
            },
        }
    }
}

fn device_thread_main(mut device: CubeDevice, dev_sender: Sender<Event>) {
    let mut switch_num: [u8;2] = [0,0];
    let mut num_pos = 0;
    let mut twist_id: [u8;2] = [0,0];
    let mut twist_pos = 0;
    #[derive(Debug)]
    enum Mode {Normal, ParseNum, ParseTwist, Debugmsg}
    use Mode::*;
    let mut mode = Normal;
    loop{
        let mut s = [0u8;50];
        let r = device.read(&mut s);
        match r {
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {}
            ,Err(e) => {println!("{:?}", e);break;}
            ,Ok(n) => {
                for i in 0..n{
                    let c = s[i];
                    let r: Result<(), SendError<Event>> = (||{
                        match (&mode, c){
                            (Normal, b'i') => {
                                // start of config mode switch report
                                num_pos = 0;
                                switch_num = [b' ',b' '];
                                mode = ParseNum;
                            }
                            ,(ParseNum, b';') => {
                                // end of config mode switch report
                                mode = Normal;
                                if let Ok(n) = i32::from_str(String::from_utf8_lossy(&switch_num).trim()){
                                    println!("Raw input: {}", n);
                                    dev_sender.send(Event::Device(DeviceEvent::Switch(n)))?;
                                }
                            }
                            ,(ParseNum, d) => {
                                if num_pos < 2{
                                    switch_num[num_pos] = d;
                                    num_pos += 1;
                                }
                                else{
                                    mode = Normal; // malformed, ignore
                                }
                            }
                            ,(Normal, b'#') => {
                                    dev_sender.send(Event::Device(DeviceEvent::Solved()))?;
                            }
                            ,(Normal, b'*') => {
                                twist_pos = 0;
                                twist_id = [b' ',b' '];
                                mode = ParseTwist;
                            }
                            ,(ParseTwist, b';') => {
                                // end of twist
                                mode = Normal;
                                if let Ok(t) = Twist::from_bytes(&twist_id){
                                    dev_sender.send(Event::Device(DeviceEvent::Twist(t)))?;
                                }
                            }
                            ,(ParseTwist, d) => {
                                if twist_pos < 2{
                                    twist_id[twist_pos] = d;
                                    twist_pos += 1;
                                }
                                else{
                                    mode = Normal; // malformed, ignore
                                }
                            }
                            ,(Normal, b'?') => {
                                mode = Debugmsg;
                            }
                            ,(Debugmsg, c) => {
                                if c == b';'{
                                    mode = Normal;
                                    println!("\n");
                                }
                                else{
                                     print!("{}", String::from_utf8_lossy(&[c]));
                                }
                            }
                            ,(Normal, _c) => {} //unknown char
                        }
                        Ok(())
                    })();
                    match r {
                        Ok(_) => {}
                        ,Err(e) => {println!("Unable to send device event, client disconnected? {:?}", e);}
                    }
                }
            }
        }
    }
}

fn sound_thread_main(sound_events: Receiver<Sound>) {
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sound_files: Vec<&[u8]> = include!("sounds.rs");
    let sounds: Vec<Buffered<_>> = (0..11).map(|n|{
        let file = BufReader::new(Cursor::new(sound_files[n]));
        Decoder::new(file).unwrap().buffered()
    }).collect();

    let win_sound = Decoder::new(BufReader::new(Cursor::new(include_bytes!("../../sounds/win.wav")))).unwrap().buffered();

    let mut rng = rand::thread_rng();
    for ev in sound_events.iter() {
        match ev {
            Sound::Twist() => {
                let n = rng.gen_range(0..11);
                // ignore sound errors, there's not much to do about them
                let _ignored = stream_handle.play_raw(sounds[n].clone().convert_samples());
            }
            ,Sound::Win() => {
                let _ignored = stream_handle.play_raw(win_sound.clone().convert_samples());
            }
            ,Sound::NoMoreSounds() => {
                break;
            }
        }
    }
}

#[cfg(feature="output_mode_jumbotron")]
shader_struct!{
    PreviewCube 
    ,r#"
        #version 330 core
        layout (location = 0) in vec4 aPos;
        layout (location = 1) in vec4 aNorm;
        layout (location = 2) in vec2 aUV;
        uniform mat4 u_screen_transform;
        uniform mat4 u_transform;
        out float light;
        out vec2 UV;
        void main() {
            gl_Position = aPos * u_transform * u_screen_transform;
            vec4 normal = normalize(aNorm * u_transform);
            light = dot(vec3(normal), vec3(1.0,0.0,0.0));
            UV = aUV;
        }
        "#
    ,r#"
        #version 330 core
        in float light;
        in vec2 UV;
        uniform sampler2D u_texture;
        uniform mat4 u_texture_transform;
        uniform vec3 u_base_cols[6];
        uniform int u_cur_face;
        uniform vec2 u_facelet_coords;
        uniform float u_border_size;
        out vec4 FragColor;
        void main() {
            vec2 tc = (vec4(u_facelet_coords.xy+UV,0.0,0.0)*u_texture_transform).xy;
            FragColor = texture(u_texture,tc);
            float b = u_border_size;
            if (UV.x < b || UV.x > (1.0-b) || UV.y < b || UV.y > (1.0-b)){
                FragColor = vec4(u_base_cols[u_cur_face],1.0);
            }
        }
        "#
    ,{
        // uniforms go here
        u_screen_transform: UniformMat4F,
        u_transform: UniformMat4F,
        u_texture: UniformSampler2D,
        u_texture_transform: UniformMat4F,
        u_base_cols: Uniform3FV,
        u_cur_face: Uniform1I,
        u_facelet_coords: Uniform2F,
        u_border_size: Uniform1F,
    }
}

#[cfg(feature="output_mode_jumbotron")]
shader_struct!{
    Shader
    ,r#"
        #version 330 core
        layout (location = 0) in vec4 aPos;
        uniform mat4 u_screen_transform;
        out vec2 px_pos;
        void main() {
            gl_Position = aPos * u_screen_transform;
            px_pos = aPos.xy;
        }
        "#
    ,include_str!("cube.frag.glsl")
    ,{
        // uniforms go here
        u_screen_transform: UniformMat4F,
        u_anim_pos: Uniform1F,
        u_facelet_px: Uniform1F,
        u_prev_colours: Uniform1UIV,
        u_cur_colours: Uniform1UIV,
        u_mapping: Uniform1UIV,
        u_rotation_map: Uniform1UIV,
        u_map_facenum: Uniform1UIV,
        u_map_subfacenum: Uniform1UIV,
        u_base_cols: Uniform3FV,
        u_twist_face: Uniform1UI,
        u_twist_dir: Uniform1F,
        u_debug_arrow: Uniform1UI,
    }
}

#[cfg(feature="output_mode_jumbotron")]
fn jumbotron_thread_main(
        config: &CubeConfig,
        cube_state: Arc<Mutex<Cube>>,
        prev_cube_state: Arc<Mutex<Cube>>,
        last_twist: Arc<Mutex<TwistInfo>>,
        led_map: Arc<Mutex<LedMap>>,
        calibration_mode: Arc<Mutex<bool>>,
    ) {

    fn colour_num(c: cube_model::Colors) -> u32{
        use cube_model::Colors::*;
        match c {
            White => 0,
            Red => 1,
            Blue => 2,
            Green => 3,
            Yellow => 4,
            Orange => 5,
            Blank => 6,
        }
    }

    // TODO make these config options
    let fp = config.facelet_px.unwrap_or(48);
    // always have a 5x9 arrangement of facelets
    let show_preview = true;
    let nw = 9;
    let nh = 5;
    let width = fp * nw;
    let height = fp * nh;
    let mut winw = width;
    let mut winh = height;
    if show_preview{
        winh = (winh as f32 * 2.5) as u32;
    }
    const start_fullscreen: bool = false;

    println!("Starting video output...");
    use glfw::fail_on_errors;
    let mut glfw = glfw::init(fail_on_errors!()).unwrap();

    let (mut window, events) = glfw.with_primary_monitor(|glfw,m| {
        glfw.create_window(winw, winh, "Jumbotron output window", 
            if start_fullscreen {glfw::WindowMode::FullScreen(m.expect("No primary monitor"))} else {glfw::WindowMode::Windowed}
        ).expect("Failed to create GLFW window.")
    });

    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 3));
    glfw.window_hint(glfw::WindowHint::OpenGlProfile(glfw::OpenGlProfileHint::Core));
    #[cfg(target_os = "macos")]
    glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));

    gl::load_with(|s| window.get_proc_address(s).unwrap() as *const _);

    // TODO vsync as config option??
    //glfw.set_swap_interval(glfw::SwapInterval::None);
    glfw.set_swap_interval(glfw::SwapInterval::Sync(1));
    window.set_framebuffer_size_polling(true);

    unsafe{
        gl::Enable(gl::CULL_FACE);
        gl::FrontFace(gl::CCW);
        gl::CullFace(gl::BACK);
        gl::Enable(gl::DEPTH_TEST);
        gl::DepthFunc(gl::LESS);
    }

    let mut main_buffer = 0;
    unsafe{ gl::GetIntegerv(gl::DRAW_FRAMEBUFFER_BINDING, &mut main_buffer); }
    let main_buffer = main_buffer as u32;

    let shader = Shader::new();
    let preview_cube = PreviewCube::new();

    let vert_array: [f32;8] = [
        width as f32, 0.0,
        0.0, 0.0,
        0.0, height as f32,
        width as f32, height as f32,
    ];

    let mut vbo = 0;
    let mut verts = 0;
    unsafe{ gl::GenVertexArrays(1, &mut verts); gl::GenBuffers(1, &mut vbo); }
    let bind_screen_rect = ||unsafe { gl::BindVertexArray(verts); gl::BindBuffer(gl::ARRAY_BUFFER, vbo); };

    unsafe{
        bind_screen_rect();
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (vert_array.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
            &vert_array[0] as *const f32 as *const c_void,
            gl::STATIC_DRAW,
        );

        // position attribute
        gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, (mem::size_of::<GLfloat>() * 2) as GLsizei, ptr::null());
        gl::EnableVertexAttribArray(0);
    }

    // -------------------------------------------------
    let facelet_vert_array: [f32;12] = [
        -0.5, -0.5, 0.0,
        0.5, -0.5, 0.0,
        0.5, 0.5, 0.0,
        -0.5, 0.5, 0.0,
    ];
    let facelet_norm_array: [f32;12] = [
        0.0, 1.0, 0.0,
        0.0, 1.0, 0.0,
        0.0, 1.0, 0.0,
        0.0, 1.0, 0.0,
    ];
    let facelet_uv_array: [f32;8] = [
        0.0,0.0,
        0.0,1.0,
        1.0,1.0,
        1.0,0.0,
    ];

    let mut fl_verts = 0;
    let mut fl_vbo = 0;
    let mut fl_nbo = 0;
    let mut fl_uvbo = 0;
    let mut cube_framebuffer = 0;
    let mut cube_texture = 0;
    unsafe{
        gl::GenVertexArrays(1, &mut fl_verts);
        gl::GenBuffers(1, &mut fl_vbo);
        gl::GenBuffers(1, &mut fl_nbo);
        gl::GenBuffers(1, &mut fl_uvbo);
    }
    let bind_cube_facelet = ||unsafe {
        gl::BindVertexArray(fl_verts);
    };

    unsafe{
        gl::BindVertexArray(fl_verts);
        // position attribute
        gl::EnableVertexAttribArray(0);
        gl::BindBuffer(gl::ARRAY_BUFFER, fl_vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (facelet_vert_array.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
            &facelet_vert_array[0] as *const f32 as *const c_void,
            gl::STATIC_DRAW,
        );
        gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, (mem::size_of::<GLfloat>() * 3) as GLsizei, ptr::null());


        // normal attribute
        gl::EnableVertexAttribArray(1);
        gl::BindBuffer(gl::ARRAY_BUFFER, fl_nbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (facelet_norm_array.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
            &facelet_norm_array[0] as *const f32 as *const c_void,
            gl::STATIC_DRAW,
        );
        gl::VertexAttribPointer(1, 3, gl::FLOAT, gl::FALSE, (mem::size_of::<GLfloat>() * 3) as GLsizei, ptr::null());

        // uv attribute
        gl::EnableVertexAttribArray(2);
        gl::BindBuffer(gl::ARRAY_BUFFER, fl_uvbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (facelet_uv_array.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
            &facelet_uv_array[0] as *const f32 as *const c_void,
            gl::STATIC_DRAW,
        );
        gl::VertexAttribPointer(2, 2, gl::FLOAT, gl::FALSE, (mem::size_of::<GLfloat>() * 2) as GLsizei, ptr::null());
        
        gl::GenFramebuffers(1, &mut cube_framebuffer);
        gl::BindFramebuffer(gl::FRAMEBUFFER, cube_framebuffer);
        gl::GenTextures(1, &mut cube_texture);
        gl::BindTexture(gl::TEXTURE_2D, cube_texture);
        gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGB as i32, width as i32, height as i32, 0, gl::RGB, gl::UNSIGNED_BYTE, ptr::null());
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
        gl::FramebufferTexture(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, cube_texture, 0);
        gl::DrawBuffers(1, &[gl::COLOR_ATTACHMENT0] as *const u32);
    }

    // spacing
    let sp = 1.5f32;

    // Rotations to different cube faces:
    //white red green orange blue
    //top left back right front
    let face_transforms = [
        &Transform::translate(0.0,sp,0.0)*&Transform::rotate_ypr(0.0, 0.0, std::f32::consts::TAU/4.0), // Top
        &Transform::translate(sp,0.0,0.0)*&Transform::rotate_ypr(0.0,std::f32::consts::TAU/-4.0,0.0), // Left
        &Transform::translate(0.0,0.0,sp)*&Transform::rotate_ypr(0.0,std::f32::consts::TAU/2.0,0.0), // Back
        &Transform::translate(-sp,0.0,0.0)*&Transform::rotate_ypr(0.0,std::f32::consts::TAU/4.0,0.0), // Right
        Transform::translate(0.0,0.0,-sp), // Front
    ];

    let facelet_translations: [Transform<f32>;9] = [
        Transform::translate(-1.0,-1.0,0.0),
        Transform::translate( 0.0,-1.0,0.0),
        Transform::translate( 1.0,-1.0,0.0),
        Transform::translate(-1.0, 0.0,0.0),
        Transform::none(), // Centre
        Transform::translate( 1.0, 0.0,0.0),
        Transform::translate(-1.0, 1.0,0.0),
        Transform::translate( 0.0, 1.0,0.0),
        Transform::translate( 1.0, 1.0,0.0),
    ];
    // -------------------------------------------------
    
    //let mut last_frame_start = Instant::now();

    let start_time = Instant::now();

    while !window.should_close() {
        let debug = *calibration_mode.lock().unwrap();
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, main_buffer);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }
        // Poll for and process events
        glfw.poll_events();
        for (_, event) in glfw::flush_messages(&events) {
            use glfw::WindowEvent::*;
            match event {
                FramebufferSize(w,h) => {winw = w as u32; winh = h as u32;},
                e => {println!("{:?}",e);}
            }
        }
        // Draw here
        unsafe { gl::Viewport(0,0,winw as i32,winh as i32); }
        shader.use_();
        bind_screen_rect();
        shader.u_facelet_px.set(fp as f32);
        shader.u_base_cols.set(&[
            1.0,1.0,1.0, // white
            1.0,0.0,0.0, // red
            0.0,0.0,1.0, // blue
            0.0,1.0,0.0, // green
            1.0,1.0,0.0, // yellow
            1.0,0.5,0.0, // orange
            0.0,0.0,0.0, // black (for blank cells)
        ]);
        let screen_transform = [
            2.0/winw as f32,0.0,0.0,-1.0,
            0.0,-2.0/winh as f32,0.0,1.0,
            0.0,0.0,1.0,0.0,
            0.0,0.0,0.0,1.0,
        ];
        shader.u_screen_transform.set(false,&screen_transform);
        let cols: Vec<u32> = {
            let cube = cube_state.lock().unwrap();
            (0..54).map(|i|{
                let f = i/9;
                let s = i%9;
                colour_num(cube.faces[f].subfaces[s].color)
            }).collect()
        };
        let prev_cols: Vec<u32> = {
            let cube = prev_cube_state.lock().unwrap();
            (0..54).map(|i|{
                let f = i/9;
                let s = i%9;
                colour_num(cube.faces[f].subfaces[s].color)
            }).collect()
        };
        {
            let lm = led_map.lock().unwrap();
            shader.u_mapping.set(&lm.indexmap);
            shader.u_map_facenum.set(&lm.facemap);
            shader.u_map_subfacenum.set(&lm.subfacemap);
            shader.u_rotation_map.set(&lm.rotationmap);
        }
        shader.u_prev_colours.set(prev_cols.as_slice());
        shader.u_cur_colours.set(cols.as_slice());
        let lt = last_twist.lock().unwrap();
        if let Some(t) = lt.time {
            let d = Instant::now() - t;
            let d = ((d.as_millis() as f32)/4000.0).min(1.0);
            shader.u_anim_pos.set(d);
        }
        else{
            shader.u_anim_pos.set(1.0);
        }
        if let Some(twist) = lt.twist{
            shader.u_twist_face.set(twist.face as u32);
            shader.u_twist_dir.set(if twist.reverse {-1.0} else {1.0});
        }
        
        shader.u_debug_arrow.set(if debug {1} else {0});

        unsafe { gl::DrawArrays(gl::TRIANGLE_FAN, 0, 4); }
        
        if show_preview{

            unsafe{
                // turn the pixel data in the frame buffer in to a texture for the preview cube to use
                gl::BindFramebuffer(gl::FRAMEBUFFER, cube_framebuffer);
                gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
                gl::Viewport(0,0,width as i32,height as i32);
                let tex_screen_transform = [
                    2.0/width as f32,0.0,0.0,-1.0,
                    0.0,-2.0/height as f32,0.0,1.0,
                    0.0,0.0,1.0,0.0,
                    0.0,0.0,0.0,1.0,
                ];
                shader.u_screen_transform.set(false,&tex_screen_transform);
                gl::DrawArrays(gl::TRIANGLE_FAN, 0, 4); // draw frame again, to texture this time
                // back to normal render settings
                gl::BindFramebuffer(gl::FRAMEBUFFER, main_buffer);
                gl::Viewport(0,0,winw as i32,winh as i32);
                // always be in front of the previous rendering
                gl::Clear(gl::DEPTH_BUFFER_BIT);
            }
            preview_cube.use_();
            let fww = winw as f32;
            let fwh = winh as f32;
            let fw = width as f32;
            let fh = height as f32;
            let ratio = fw/fh;
            let iswide = winw > winh;
            let xscale = ratio * if iswide {fwh/fww} else {1.0};
            let yscale =         if iswide {1.0} else {fww/fwh};
            let aw = fww * xscale / ratio;
            let ah = fwh * yscale;
            let tx = (fww-aw)/(1.0*fww);
            let ty = (fwh-ah)/(-1.0*fwh);
            let texture_transform = [
                1.0/nw as f32,0.0,0.0,0.0,
                0.0,-1.0/nh as f32,0.0,1.0,
                0.0,0.0,1.0,0.0,
                0.0,0.0,0.0,1.0,
            ];
            preview_cube.u_texture_transform.set(false,&texture_transform);
            preview_cube.u_screen_transform.set(false,&[
                xscale,0.0,0.0,tx,
                0.0,yscale,0.0,ty,
                0.0,0.0,1.0,0.0,
                0.0,0.0,0.0,1.0,
            ]);
            preview_cube.u_base_cols.set(&[
                0.8,0.8,0.8, // white
                1.0,0.0,0.0, // red
                0.0,0.0,1.0, // blue
                1.0,0.5,0.0, // orange
                //1.0,1.0,0.0, // yellow
                0.0,1.0,0.0, // green
                //0.0,0.0,0.0, // black (for blank cells)
            ]);
            preview_cube.u_border_size.set(if debug {0.05} else {-0.1});
            bind_cube_facelet();
            let sf = 0.3;
            let base_trans = &Transform::scale(height as f32/width as f32,1.0,1.0) * &Transform::scale(sf,sf,sf);
            let base_trans = &base_trans * &Transform::rotate_xyz(-0.25,0.0,0.0);
            //let base_trans = &base_trans * &Transform::rotate_xyz(0.00,(Instant::now()-start_time).as_millis() as f32 / 4000.0,0.0);
            let base_trans = &base_trans * &Transform::rotate_xyz(0.00,1.0,0.0);
            for (i, t1) in face_transforms.iter().enumerate(){
                preview_cube.u_cur_face.set(i.try_into().unwrap());
                for (j, t2) in facelet_translations.iter().enumerate(){
                    let t = Transform::none();
                    let t = &t * &base_trans;
                    let t = &t * &t1;
                    let t = &t * &t2;
                    preview_cube.u_transform.set(false, &t.data);
                    let fx = j as f32;
                    let fy = i as f32;
                    preview_cube.u_facelet_coords.set(fx,fy);
                    unsafe { gl::DrawArrays(gl::TRIANGLE_FAN, 0, 4); }
                }
            }
        }
        window.swap_buffers();
    }
}

struct TwistInfo{
    twist: Option<Twist>,
    time: Option<Instant>,
}

struct LedMap{
    indexmap: [u32;45],
    facemap: [u32;45],
    subfacemap: [u32;45],
    rotationmap: [u32;45],
}

impl LedMap{

    fn new() -> Self{
        let mut lm = LedMap{
            indexmap:[0;45],
            facemap:[0;45],
            subfacemap:[0;45],
            rotationmap:[0;45],
        };
        lm.set("000102030405060708101112131415161718202122232425262728303132333435363738404142434445464748505152535455565758");
        lm
    }

    fn set(&mut self, map: &str){
        let m = map.as_bytes();
        for i in 0..=44{
            let fnum = (m[i*2] - b'0') as u32;
            let sfnum = (m[(i*2)+1] - b'0') as u32;
            let num = fnum * 9 + sfnum;
            self.indexmap[i] = if num > 44 { 0 } else { num };
            self.facemap[i] = if num > 44 { 0 } else { fnum };
            self.subfacemap[i] = if num > 44 { 0 } else { sfnum };
        }   
    }

    fn rotate(&mut self, f: usize, sf: usize){
        let i = self.indexmap[(f*9)+sf] as usize;
        self.rotationmap[i] = (self.rotationmap[i]+3)%4;
    }

    fn set_rotations(&mut self, rotations: &str){
        let m = rotations.as_bytes();
        for i in 0..=44{
            let num = (m[i] - b'0') as u32;
            self.rotationmap[i] = num;
        }
    }
}

fn main() {
    println!("Cube service");

    let args = Args::parse();
    if args.tcp .is_none() && args.serial.is_none(){
        eprintln!("No interfaces specified");
        eprintln!("Specify at least one of --tcp and --serial");
        eprintln!("See --help for details");
        std::process::exit(1);
    }

    println!("Configuration:");
    println!("    Config file: {}", args.config);
    println!("    Device:      {}", args.device);
    println!("    TCP listen:  {}", args.tcp.as_ref().unwrap_or(&"(no TCP interface)".to_string()));
    println!("    Serial port: {}", args.serial.as_ref().unwrap_or(&"(no serial interface)".to_string()));

    let mut config: CubeConfig = {
        let p = Path::new(&args.config);
        match File::open(p) {
            Ok(f) => match serde_json::from_reader(f) {
                Ok(d) => d
                ,Err(e) => {println!("Failed to parse config file: {}", e); std::process::exit(1);}
            }
            // TODO handle secrets better
            ,Err(_) => serde_json::from_str(
                r#"{
                    "led_map": "000102030405060708101112131415161718202122232425262728303132333435363738404142434445464748505152535455565758"
                    ,"input_map": "000102030405060708091011121314151617"
                    ,"rotation_map": "000000000000000000000000000000000000000000000"
                    ,"secret": ""
                    ,"top_score": 0
                }"#
            ).unwrap()
        }
    };

    persist_config(&config, &args.config);

    let secret = config.secret.as_bytes().to_vec();

    let (sender, receiver) = channel::<Event>();
    let net_sender = sender.clone();
    let dev_sender = sender.clone();

    let device_name = args.device;

    let device = match CubeDevice::from_device_name(&device_name) {
        Ok(device) => device,
        Err(e) => {eprintln!("Device connection error. {:?} TODO better error message here", e); return},
    };

    let mut device_write = device.try_clone().expect("Failed to split serial connection into reader and writer, unsupported platform??");

    #[cfg(feature="debug_device_stream")]
    {
        use tee_readwrite::TeeWriter;
        let mut device_write = TeeWriter::new(device_write, std::io::stdout());
    }

    let device_thread = thread::spawn(move||{ device_thread_main(device, dev_sender); });

    if let Err(e) = (||{
        device_write.write(format!("ca{}\r\n", config.input_map).as_bytes())?;
        device_write.write(format!("cm{}\r\n", config.led_map).as_bytes())?;
        device_write.write(b"cuWWWWWWWWWRRRRRRRRRGGGGGGGGGOOOOOOOOOBBBBBBBBBYYYYYYYYYp\r\n")?;
        device_write.flush()?;
        Result::<(), std::io::Error>::Ok(())
    })(){
        println!("Failed to initialise device: {:?}", e);
    }

    let tcp_thread = if let Some(listen) = args.tcp {
        let listener = TcpListener::bind(listen);
        match listener{
            Err(e) => {println!("Failed to bind: {:?}", e); std::process::exit(1);}
            Ok(listener) => {
                Some(thread::spawn(move||{
                    println!("Listening on TCP");
                    for stream in listener.incoming(){
                        match stream {
                            Err(e) => println!("Incoming connection failed: {:?}", e)
                            ,Ok(read_stream) => {
                                println!("Connection from: {}", match read_stream.peer_addr() {Ok(addr)=>addr.to_string(), Err(e)=>e.to_string()});
                                match read_stream.try_clone() {
                                    Ok(write_stream) => {handle_stream(read_stream, write_stream, net_sender.clone(), secret.clone());}
                                    ,Err(e) => {println!("Stream failed: {:?}", e);}
                                }
                            }
                        }
                    }
                }))
            }
        }
    }
    else {
        None
    };

    let serial_thread = if let Some(_port) = args.serial {
        // TODO serial thread like the tcp thread
        Some(thread::spawn(||{}))
    }
    else{
        None
    };

    let (sound_sender, sound_events) = channel::<Sound>();
    //let sound_thread = std::thread::spawn(move||{ sound_thread_main(sound_events); });

    let cube = Arc::new(Mutex::new(Cube::new()));
    let prev_cube = Arc::new(Mutex::new(Cube::new()));
    let last_twist = Arc::new(Mutex::new(TwistInfo{twist:None, time: None}));

    let led_map = Arc::new(Mutex::new(LedMap::new()));
    let calibration_mode = Arc::new(Mutex::new(false));
    
    {
        let mut map = led_map.lock().unwrap();
        map.set(&config.led_map);
        map.set_rotations(&config.rotation_map);
    }

    #[cfg(feature="output_mode_jumbotron")]
    let jumbotron_thread = if args.jumbotron{
        let cube_state = Arc::clone(&cube);
        let prev_cube_state = Arc::clone(&prev_cube);
        let led_map = Arc::clone(&led_map);
        let config = config.clone();
        let last_twist = last_twist.clone();
        let calibration_mode = calibration_mode.clone();
        Some(std::thread::spawn(move||{ jumbotron_thread_main(&config, cube_state, prev_cube_state, last_twist, led_map, calibration_mode); }))
    } else { None };

    let mut gui_sender: Option<Sender<StreamEvent>> = None;
    let mut game_state = TimerState::default();

    for event in receiver.iter(){
        match event {
            Event::Client(c_ev) => {
                let r: Result<(), EvStreamError> = (|c_ev|{
                    match c_ev {
                        ClientEvent::SetState(state) =>{
                            let mut c = cube.lock().unwrap();
                            let mut cp = prev_cube.lock().unwrap();
                            match c.deserialise(&state) {
                                Ok(_) => {
                                    device_write.write(b"u")?;
                                    device_write.write(state.as_bytes())?;
                                    device_write.flush()?;
                                }
                                ,Err(msg) => {
                                    println!("Unable to deserialise cube state: {}", msg);
                                }
                            }
                            match cp.deserialise(&state) {
                                Ok(_) => {}
                                ,Err(msg) => {
                                    println!("Unable to deserialise cube state: {}", msg);
                                }
                            }
                        }
                        ,ClientEvent::StartDetectSwitches() => {
                            println!("Detect Switches");
                            device_write.write(b"c")?;
                            device_write.flush()?;
                        }
                        ,ClientEvent::EnableCalibrationView() => {
                            *calibration_mode.lock().unwrap() = true;
                        }
                        ,ClientEvent::DisableCalibrationView() => {
                            *calibration_mode.lock().unwrap() = false;
                        }
                        ,ClientEvent::RotateSubface(f,sf) => {
                            let mut lm = led_map.lock().unwrap();
                            lm.rotate(f, sf);
                            let rotstr = lm.rotationmap.iter().map(|a|a.to_string()).collect();
                            config.rotation_map = rotstr;
                            persist_config(&config, &args.config);
                        }
                        ,ClientEvent::ApplyTwist(t) => {
                            todo!();
                        }
                        ,ClientEvent::StartDetectLED() => {
                            println!("Detect LEDs");
                            // Configuration mode
                            device_write.write(b"c")?;
                            // Blank mapping
                            device_write.write(b"m000102030405060708101112131415161718202122232425262728303132333435363738404142434445464748505152535455565758")?;
                            // All subfaces blank
                            device_write.write(b"u                                                      ")?;
                            device_write.flush()?;
                            let mut lm = led_map.lock().unwrap();
                            lm.set("000102030405060708101112131415161718202122232425262728303132333435363738404142434445464748505152535455565758");
                        }
                        ,ClientEvent::UpdateLEDMap(new_map) => {
                            println!("led map update");
                            device_write.write(b"cm")?;
                            device_write.write(new_map.as_bytes())?;
                            device_write.flush()?;
                            config.led_map = new_map.clone();
                            let mut lm = led_map.lock().unwrap();
                            lm.set(&new_map);
                            persist_config(&config, &args.config);
                        }
                        ,ClientEvent::UpdateInputMap(new_map) => {
                            println!("input map update");
                            device_write.write(b"ca")?;
                            device_write.write(new_map.as_bytes())?;
                            device_write.flush()?;
                            config.input_map = new_map;
                            persist_config(&config, &args.config);
                        }
                        ,ClientEvent::Play() => {
                            device_write.write(b"p")?;
                            device_write.flush()?;
                        }
                        ,ClientEvent::StartTimedGame() => {
                            game_state.reset();
                            game_state.start();
                            if let Some(sender) = gui_sender.as_ref(){
                                sender.send(StreamEvent::SyncTimers(game_state.serialise()))?;
                            }
                        }
                        ,ClientEvent::CancelTimedGame() => {
                            game_state.reset();
                            if let Some(sender) = gui_sender.as_ref(){
                                sender.send(StreamEvent::SyncTimers(game_state.serialise()))?;
                            }
                        }
                        ,ClientEvent::Connected(sender) => {
                            gui_sender = Some(sender);
                        }
                        ,ClientEvent::GetState() => {
                            send_state_to_client(gui_sender.as_ref(), *cube.lock().unwrap(), config.top_score)?;
                        }
                        ,ClientEvent::SetBrightness(b) => {
                            device_write.write(b"%")?;
                            device_write.write(&[b])?;
                            device_write.flush()?;
                        }
                    }
                    Ok(())
                })(c_ev);
                match r {
                    Ok(_) =>{}
                    ,Err(e) => {println!("Error while handling client event: {:?}", e);}
                }
            }
            Event::Device(d_ev) => {
                match d_ev {
                    DeviceEvent::Twist(twist) => {
                        if game_state.twist(){
                            if let Some(sender) = gui_sender.as_ref(){
                                // Timer sync events are best-effort, ignore errors
                                let _ignored = sender.send(StreamEvent::SyncTimers(game_state.serialise()));
                            }
                        }
                        let _ignored = sound_sender.send(Sound::Twist());
                        let mut c = cube.lock().unwrap();
                        prev_cube.lock().unwrap().deserialise(&c.serialise()).unwrap();
                        c.twist(twist);
                        last_twist.lock().unwrap().twist = Some(twist);
                        last_twist.lock().unwrap().time = Some(Instant::now());
                    }
                    ,DeviceEvent::Solved() => {
                        let is_win = game_state.solved();
                        if let Some(sender) = gui_sender.as_ref(){
                            // Timer syc events are best-effort, ignore errors
                            let _ignored = sender.send(StreamEvent::SyncTimers(game_state.serialise()));
                        }
                        if is_win{
                            let _ignored = sound_sender.send(Sound::Win());
                            match game_state.recorded_time(){
                                Some(time) => {
                                    if let Some(sender) = gui_sender.as_ref(){
                                        // TODO do I even need this event??
                                        let _ignored = sender.send(StreamEvent::ReportTime(time));
                                    }
                                    let t = time.as_millis();
                                    if (config.top_score == 0) || (t < config.top_score){
                                        config.top_score = t;
                                        persist_config(&config, &args.config);
                                        if let Some(sender) = gui_sender.as_ref(){
                                            let _ignored = sender.send(StreamEvent::RecordState(t));
                                        }
                                    }
                                }
                                ,_=>{}
                            }
                        }
                    }
                    ,_=>{}
                };
                if let Some(sender) = gui_sender.as_ref(){
                    match sender.send(StreamEvent::GUI(d_ev)) {
                        Err(e) => {println!("Failed to send device event to client, client disconnected?: {:?}", e)}
                        ,Ok(_) => {}
                    }
                }
            }
        }
    }

    let _ignored = device_thread.join();
    #[cfg(feature="output_mode_jumbotron")]
    if let Some(t) = jumbotron_thread { let _ignored = t.join(); }
    if let Some(t) = tcp_thread { let _ignored = t.join(); }
    if let Some(t) = serial_thread { let _ignored = t.join(); }
    sound_sender.send(Sound::NoMoreSounds()).expect("sound thread crashed?");
    //let _ignored = sound_thread.join();
}
