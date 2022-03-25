use clap::Parser as CLIParser;
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::sync::mpsc::{channel,Sender,Receiver};
use std::io::{Write,Read,BufReader,BufRead};
use plain_authentic_commands::{MessageHandler, ParseStatus};
extern crate pest;
use pest::Parser;

use cube_model::Cube;

#[derive(CLIParser, Debug)]
struct Args{
    #[clap()]
    device: String,
    #[clap(long)]
    tcp: Option<String>,
    #[clap(long)]
    serial: Option<String>,
}

enum ClientEvent{
    SetState(String)
    ,StartDetectLED()
}

enum Event{
    Client(ClientEvent)
    ,Device()
}

struct LEDDetectState{
    cur_led: usize
    ,map: [Option<(usize, usize)>; 9*5] // five faces, tuple of face index and subface index
}

impl LEDDetectState{
    fn new() -> LEDDetectState{
        LEDDetectState{
            cur_led: 0
            ,map: [None; 9*5]
        }
    }
}

fn handle_stream<R: Read, W: Write>(read_stream: &mut R, write_stream: &mut W, sender: &mut Sender<Event>){
    let mut auth = MessageHandler::new(b"secret".to_vec());
    let mut buffer = BufReader::new(read_stream);
    for line_result in buffer.split(b'\n'){
        match line_result {
            Ok(line) => { // Got a line to read
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
                                    sender.send(Event::Client(ClientEvent::SetState(args[0].clone())));
                                }
                                else{
                                    let msg = auth.construct_reply("wrong_arguments", &vec![&command]);
                                    write_stream.write(msg.as_bytes());
                                }
                            }
                            ,"detect" => {
                                println!("TODO: logic for detecting LED or switch sequence");
                            }
                            ,_=>{
                                let msg = auth.construct_reply("unknown_command", &vec![&command]);
                                write_stream.write(msg.as_bytes());
                            }
                        };
                        auth.step();
                        let msg = auth.construct_reply("challenge", &vec![&auth.get_salt()]);
                        write_stream.write(msg.as_bytes());
                    }
                    // Dont sign replies to messages that are not authorised. If we don't trust the source, we won't sign things for them
                    ,ParseStatus::BadClient() => {write_stream.write(b"+malformed_command:a#a\n"); break;}
                    ,ParseStatus::Unauthorised() => {write_stream.write(b"+auth_fail:a#a\n"); break;}
                };
            }
            ,Err(e) => {
                println!("Unable to read from remote: {:?}", e);
                break;
            }
        }
    }
    println!("Client stream ended, disconnected.");
}

fn main() {
    println!("Cube service");

    let args = Args::parse();
    if args.tcp .is_none() && args.serial.is_none(){
        eprintln!("No interfaces specified");
        eprintln!("TODO: provide more useful help text here");
        std::process::exit(1);
    }

    println!("Configuration:");
    println!("    Device:      {}", args.device);
    println!("    TCP listen:  {}", args.tcp.as_ref().unwrap_or(&"(no TCP interface)".to_string()));
    println!("    Serial port: {}", args.serial.as_ref().unwrap_or(&"(no serial interface)".to_string()));

    let (tcp_thread, serial_thread) = {
        let (sender, receiver) = channel::<Event>();

        let mut net_sender = sender.clone();
        let mut ser_sender = sender.clone();

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
                                ,Ok(mut read_stream) => {
                                    println!("Connection from: {}", match read_stream.peer_addr() {Ok(addr)=>addr.to_string(), Err(e)=>e.to_string()});
                                    match read_stream.try_clone() {
                                        Ok(mut write_stream) => {handle_stream(&mut read_stream, &mut write_stream, &mut net_sender);}
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

        let mut leds = LEDDetectState::new();

        for event in receiver.iter(){
            match event {
                Event::Client(c_ev) => {
                    match c_ev {
                        ClientEvent::SetState(state) =>{
                            println!("Set cube state: {}", state);
                            cube.deserialise(&state);
                            println!("New state: {}", cube.simple_string());
                        }
                        ,ClientEvent::StartDetectLED() => {
                            println!("Detect LEDs");
                        }
                    }
                }
                Event::Device() => {println!("device event");}
            }
        }

        (tcp_thread, serial_thread)
    };

    if let Some(t) = tcp_thread { t.join(); };
    if let Some(t) = serial_thread { t.join(); };
}
