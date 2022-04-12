use std::net::TcpListener;
use std::thread;
use std::sync::mpsc::{channel,Sender,SendError};
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
use std::time::{Instant, Duration};
use std::cmp::min;
use std::fmt::{self,Display};

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
}

#[derive(Serialize, Deserialize)]
struct CubeConfig{
    led_map: String
    ,input_map: String
    ,secret: String
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
}

enum ClientEvent{
    Connected(Sender<StreamEvent>)
    ,SetState(String)
    ,StartDetectLED()
    ,StartDetectSwitches()
    ,UpdateLEDMap(String)
    ,UpdateInputMap(String)
    ,Play()
    ,StartTimedGame()
}

enum Event{
    Client(ClientEvent)
    ,Device(DeviceEvent)
}

#[derive(Error, Debug)]
enum EvStreamError {
    #[error("IO Error: {0}")]
    IO(#[from] std::io::Error)
    ,#[error("Sender Error: {0}")]
    Sender(#[from] std::sync::mpsc::SendError<Event>)
}

#[derive(Default, Debug)]
struct GameState{
    started: Option<Instant>
    ,inspection_end: Option<Instant>
    ,ended: Option<Instant>
}

impl Display for GameState{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self.started{
            Some(s) => "t0"
            ,None => "?"
        };
        let i = match (self.started, self.inspection_end) {
            (Some(start), Some(end)) => {format!("{:#?}", end-start)}
            ,_=>{"?".to_string()}
        };
        let e = match (self.started, self.ended) {
            (Some(start), Some(end)) => {format!("{:#?}", end-start)}
            ,_=>{"?".to_string()}
        };
        write!(f, "(s:{}, i:{}, e:{})", s,i,e)
    }
}

impl GameState{
    fn is_inspecting(&self) -> bool {
        self.started.is_some() && self.inspection_end.is_none()
    }

    fn is_started(&self) -> bool {
        self.started.is_some()
    }

    fn is_ended(&self) -> bool {
        self.ended.is_some()
    }

    fn can_start(&self) -> bool {
        self.is_ended() || !self.is_started()
    }

    fn recorded_time(&self) -> Option<Duration>{
        match (self.started, self.inspection_end, self.ended) {
            (Some(start), Some(inspect_end), Some(end)) => {
                const FIFTEEN: Duration = Duration::from_secs(15);
                Some((end - start)- min(inspect_end - start, FIFTEEN))
            }
            ,_=>{
                None
            }
        }
    }

    fn twist(&mut self) -> bool {
        if self.is_inspecting(){
            self.inspection_end = Some(Instant::now());
            true
        }
        else {
            false
        }
    }

    fn start(&mut self) -> bool {
        if self.can_start() {
            self.started = Some(Instant::now());
            self.inspection_end = None;
            self.ended = None;
            true
        }
        else {
            false
        }
    }

    fn solved(&mut self) {
        self.ended = Some(Instant::now());
    }

    fn serialise(&self) -> (String, String, String){
        match self.started{
            None => {("X".to_string(), "X".to_string(), "X".to_string())}
            Some(start) => {
                match self.inspection_end {
                    None => {("0".to_string(), "X".to_string(), "X".to_string())}
                    Some(inspect) => {
                        let d_in = format!("{}", (inspect - start).as_millis());
                        match self.ended{
                            None => { ("0".to_string(),d_in,"X".to_string()) }
                            Some(end) => {
                                let d_tot = format!("{}", (end - start).as_millis());
                                ("0".to_string(), d_in, d_tot)
                             }
                        }
                    }
                }
            }
        }
    }
}

fn handle_stream<R: 'static + Read + Send + Sync, W: 'static + Write + Send + Sync>(read_stream: R, mut write_stream: W, sender: Sender<Event>){
    let mut auth = MessageHandler::new(b"secret".to_vec());
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
                }"#
            ).unwrap()
        }
    };

    let (sender, receiver) = channel::<Event>();

    let net_sender = sender.clone();
    let ser_sender = sender.clone();
    let dev_sender = sender.clone();

    let device_name = args.device;

    let mut device = serialport::new(&device_name, 115200).open().expect("Failed to open cube device serial port.");

    let mut device_write = device.try_clone().expect("Failed to split serial connection into reader and writer, unsupported platform??");

    let device_thread = thread::spawn(move||{
        let _ignored = device.set_timeout(Duration::from_secs(10));
        let mut switch_num: [u8;2] = [0,0];
        let mut num_pos = 0;
        let mut twist_id: [u8;2] = [0,0];
        let mut twist_pos = 0;
        #[derive(Debug)]
        enum Mode {Normal, ParseNum, ParseTwist}
        use Mode::*;
        let mut mode = Normal;
        loop{
            let mut s = [0u8;50];
            let r = device.read(&mut s);
            match r {
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                ,Err(_) => {break;}
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
                                        println!("Twist: {}", t);
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
    });
    match (||{
        device_write.write(format!("ca{}\r\n", config.input_map).as_bytes())?;
        device_write.write(format!("cm{}\r\n", config.led_map).as_bytes())?;
        device_write.write(b"cuWWWWWWWWWRRRRRRRRRGGGGGGGGGOOOOOOOOOBBBBBBBBBYYYYYYYYYp\r\n")?;
        device_write.flush()?;
        Result::<(), std::io::Error>::Ok(())
    })(){
        Err(e) => {println!("Failed to initialise device: {:?}", e);}
        ,Ok(_) => {}
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
                                    Ok(write_stream) => {handle_stream(read_stream, write_stream, net_sender.clone());}
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

    let serial_thread = if let Some(port) = args.serial {
        // TODO serial thread like the tcp thread
        Some(thread::spawn(||{}))
    }
    else{
        None
    };

    let mut cube = Cube::new();

    let mut gui_sender: Option<Sender<StreamEvent>> = None;

    let mut game_state = GameState::default();

    for event in receiver.iter(){
        match event {
            Event::Client(c_ev) => {
                let r: Result<(), std::io::Error> = (|c_ev|{
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
                            game_state.start();
                            if let Some(sender) = gui_sender.as_ref(){
                                sender.send(StreamEvent::SyncTimers(game_state.serialise()));
                            }
                        }
                        ,ClientEvent::Connected(sender) => {
                            gui_sender = Some(sender);
                            // TODO sync state on connect: sender.send(StreamEvent::GUI(GUIEvent::SyncState(somethingsomething)));
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
                if let Some(sender) = gui_sender.as_ref(){
                    match d_ev {
                        DeviceEvent::Twist(_) => {
                            if game_state.twist(){
                                sender.send(StreamEvent::SyncTimers(game_state.serialise()));
                            }
                        }
                        ,DeviceEvent::Solved() => {
                            game_state.solved();
                            sender.send(StreamEvent::SyncTimers(game_state.serialise()));
                        }
                        ,_=>{}
                    }
                    match sender.send(StreamEvent::GUI(d_ev)) {
                        Err(e) => {println!("Failed to send device event to client, client disconnected?: {:?}", e)}
                        ,Ok(_) => {}
                    }
                }
            }
        }
    }

    let _ignored = device_thread.join();
    if let Some(t) = tcp_thread { let _ignored = t.join(); };
    if let Some(t) = serial_thread { let _ignored = t.join(); };
}
