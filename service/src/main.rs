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
use gl::types::*;
#[cfg(feature="output_mode_jumbotron")]
use glfw::{Action, Context, GlfwReceiver, WindowEvent, PWindow, Glfw};
#[cfg(feature="output_mode_jumbotron")]
use std::{mem,ptr};
#[cfg(feature="output_mode_jumbotron")]
use std::ffi::c_void;
#[cfg(feature="output_mode_jumbotron")]
use gl_abstractions as gla;
#[cfg(feature="output_mode_jumbotron")]
use gla::{UniformMat4, UniformF32, shader_struct, impl_shader};

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
    led_map: String
    ,input_map: String
    ,secret: String
    ,top_score: u128
    ,facelet_px: Option<u32>
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
    Connected(Sender<StreamEvent>)
    ,SetState(String)
    ,GetState()
    ,StartDetectLED()
    ,StartDetectSwitches()
    ,UpdateLEDMap(String)
    ,UpdateInputMap(String)
    ,Play()
    ,StartTimedGame()
    ,CancelTimedGame()
    ,SetBrightness(u8)
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
                                            }
                                            ,_=>{
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
}

impl Read for CubeDevice{
    fn read(&mut self, data: &mut [u8]) -> Result<usize, std::io::Error> {
        use CubeDevice::*;
        match self{
            TestDevice{sequence, buffer, next_twist} => {
                if sequence.len() <= 0 {
                    // Generate a new sequence
                    for _ in 0..8{
                        sequence.push(Twist::get_random())
                    }
                    for i in 0..8{
                        sequence.push(sequence[7-i].inverse())
                    }
                    *next_twist = Instant::now() + Duration::from_secs(15);
                }
                while *next_twist < Instant::now() {
                    let next = sequence.remove(0);
                    *next_twist = *next_twist + Duration::from_millis(500);
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
        }
    }
}

impl Write for CubeDevice{
    fn write(&mut self, data: &[u8]) -> Result<usize, std::io::Error> {
        use CubeDevice::*;
        match self{
            TestDevice{..} => {Ok(data.len())}, // Currently does nothing, could make this display a test output perhaps?
            PhysicalDevice{serial_port} => {serial_port.write(data)},
        }
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        use CubeDevice::*;
        match self{
            TestDevice{..} => {Ok(())}, // Does nothing on the test device
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
        else{
            let result = serialport::new(name, 115200).timeout(Duration::from_secs(10)).open();
            match result{
                Ok(device) => Ok(PhysicalDevice{serial_port:device}),
                Err(e) => Err(()) // TODO better error handling here
            }
        }
    }

    fn try_clone(&self) -> Result<CubeDevice, ()>{
        use CubeDevice::*;
        match self{
            TestDevice{sequence, buffer, next_twist} => Ok(TestDevice{sequence:sequence.clone(), buffer:buffer.clone(), next_twist: *next_twist}),
            PhysicalDevice{serial_port} => match serial_port.try_clone() {
                Ok(cloned) => Ok(PhysicalDevice{serial_port: cloned}),
                Err(e) => Err(()), // TODO better error handling here
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
    ,r#"
        #version 330 core
        in vec2 px_pos;
        uniform float u_facelet_px;
        out vec4 FragColor;
        void main() {
           float fp = u_facelet_px;
           int ix = int(floor(px_pos.x / fp));
           int iy = int(floor(px_pos.y / fp));
           int i = iy * 8 + ix;
           if (i < 45 && px_pos.y > 0.0 && px_pos.x > 0.0 && px_pos.y < (fp*6.0) && px_pos.x < (fp*8)){
             FragColor = vec4(0.0,i * (1.0/45.0),0.0, 1.0);
           }
           else{
             FragColor = vec4(1.0,0.0,0.0, 1.0);
           }
        }
        "#
    ,{
        // uniforms go here
        u_screen_transform: UniformMat4,
        u_facelet_px: UniformF32,
    }
}

#[cfg(feature="output_mode_jumbotron")]
fn jumbotron_thread_main(config: &CubeConfig) {

    // TODO make these config options
    let fp = config.facelet_px.unwrap_or(48);
    // always have a 6x8 arrangement of facelets
    let width = fp * 8;
    let height = fp * 6;
    const start_fullscreen: bool = false;

    println!("Starting video output...");
    use glfw::fail_on_errors;
    let mut glfw = glfw::init(fail_on_errors!()).unwrap();

    let (mut window, events) = glfw.with_primary_monitor(|glfw,m| {
        glfw.create_window(width, height, "Jumbotron output window", 
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

    let mut shader = Shader::new();

    let vert_array: [f32;8] = [
        0.0, 0.0,
        width as f32, 0.0,
        width as f32, height as f32,
        0.0, height as f32,
    ];

    let mut vbo = 0;
    let mut verts = 0;

    unsafe{
        gl::GenVertexArrays(1, &mut verts);
        gl::GenBuffers(1, &mut vbo);

        gl::BindVertexArray(verts);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
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
    
    let mut last_frame_start = Instant::now();

    while !window.should_close() {
        // Poll for and process events
        glfw.poll_events();
        for (_, event) in glfw::flush_messages(&events) {
            match event {
                _ => {}
            }
        }
        // Draw here
        unsafe { gl::Viewport(0,0,width as i32,height as i32); }
        shader.use_();
        shader.u_facelet_px.set(fp as f32);
        shader.u_screen_transform.set(&[
            2.0/width as f32,0.0,0.0,-1.0,
            0.0,-2.0/height as f32,0.0,1.0,
            0.0,0.0,1.0,0.0,
            0.0,0.0,0.0,1.0,
        ]);

        //shader.u_screen_transform.set(&[
        //    width as f32/2.0,0.0,0.0,-(width as f32/2.0),
        //    0.0,height as f32/2.0,0.0,-(height as f32/2.0),
        //    0.0,0.0,1.0,0.0,
        //    0.0,0.0,0.0,1.0,
        //]);
        unsafe { gl::DrawArrays(gl::TRIANGLE_FAN, 0, 4); }
        window.swap_buffers();
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
    let sound_thread = std::thread::spawn(move||{ sound_thread_main(sound_events); });

    #[cfg(feature="output_mode_jumbotron")]
    let jumbotron_thread = if args.jumbotron{
        let config = config.clone();
        Some(std::thread::spawn(move||{ jumbotron_thread_main(&config); }))
    } else { None };

    let mut cube = Cube::new();
    let mut gui_sender: Option<Sender<StreamEvent>> = None;
    let mut game_state = TimerState::default();

    for event in receiver.iter(){
        match event {
            Event::Client(c_ev) => {
                let r: Result<(), EvStreamError> = (|c_ev|{
                    match c_ev {
                        ClientEvent::SetState(state) =>{
                            match cube.deserialise(&state) {
                                Ok(_) => {
                                    device_write.write(b"u")?;
                                    device_write.write(state.as_bytes())?;
                                    device_write.flush()?;
                                }
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
                        ,ClientEvent::StartDetectLED() => {
                            println!("Detect LEDs");
                            // Configuration mode
                            device_write.write(b"c")?;
                            // Blank mapping
                            device_write.write(b"m000102030405060708101112131415161718202122232425262728303132333435363738404142434445464748505152535455565758")?;
                            // All subfaces blank
                            device_write.write(b"u                                                      ")?;
                            device_write.flush()?;
                        }
                        ,ClientEvent::UpdateLEDMap(new_map) => {
                            println!("led map update");
                            device_write.write(b"cm")?;
                            device_write.write(new_map.as_bytes())?;
                            device_write.flush()?;
                            config.led_map = new_map;
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
                            send_state_to_client(gui_sender.as_ref(), cube, config.top_score)?;
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
                        cube.twist(twist);
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
    let _ignored = sound_thread.join();
}
