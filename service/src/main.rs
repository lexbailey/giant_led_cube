use clap::Parser as CLIParser;
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::io::{Write,Read};
use plain_authentic_commands::{AuthState, CommandParser, Rule};
extern crate pest;
use pest::Parser;

#[derive(CLIParser, Debug)]
struct Args{
    #[clap()]
    device: String,
    #[clap(long)]
    tcp: Option<String>,
    #[clap(long)]
    serial: Option<String>,
}

fn send_challenge<T: Write>(stream: &mut T, auth_state: &mut AuthState){
    auth_state.step();
    let msg = auth_state.construct_message("+challenge", &vec![]);
    stream.write(msg.as_bytes());
}

enum ParseStatus {
    Success()
    ,BadClient()
    ,Unauthorised()
}

fn parse_command<T: Write>(cmd: &Vec<u8>, stream: &mut T, auth_state: &mut AuthState) -> ParseStatus{
    let s = std::str::from_utf8(cmd);
    if let Ok(cmd) = s {
        let result = CommandParser::parse(Rule::command, cmd);
        match result{
            Err(cmd) => {
                println!("Unparesable: Bad syntax: {}", cmd.to_string());
                ParseStatus::BadClient()
            }
            ,Ok(cmd) => {
                for c in cmd{
                    let mut c = c.into_inner();
                    let checked = c.next().unwrap();
                    let checked_string = checked.as_str();
                    let mut checked = checked.into_inner();
                    let command_name = checked.next().unwrap().as_str();
                    let command_args: Vec<&str> = checked.next().unwrap().into_inner().map(|a|a.as_str()).collect();
                    let msg_salt = checked.next().unwrap().as_str();
                    let mac = c.next().unwrap().as_str();
                    println!("Parsed command: name: {}, args: {:?}", command_name, command_args);
                    if command_name == "next_challenge" {
                        // Don't check if this is authentic, challenges can be requested by anyone
                        println!("Sending next challenge");
                        send_challenge(stream, auth_state);
                    }
                    else {
                        if auth_state.command_is_authentic(checked_string, msg_salt, mac){
                            auth_state.step();
                            send_challenge(stream, auth_state);
                        }
                        else{
                            println!("Refusing to execute command: command is not authentic forcing this client to disconnect.");
                            return ParseStatus::Unauthorised();
                        }
                    }
                }
                ParseStatus::Success()
            }
        }
    }
    else{
        println!("Unparseable: Command is not valid UTF8.");
        ParseStatus::BadClient()
    }
}

fn handle_stream<T: Write + Read>(stream:&mut T){
    let mut s: Vec<u8> = Vec::with_capacity(200);
    let mut auth: AuthState = AuthState::new(b"secret text".to_vec());
    loop{
        let mut buf: [u8;100] = [0;100];
        let sz = stream.read(&mut buf);
        match sz{
            Ok(0) => {break;} // End of stream, client disconnected
            ,Ok(n) => { // Got some bytes to read
                println!("Read {} bytes: {}", n, String::from_utf8_lossy(&buf));
                let mut j = 0;
                loop{
                    let section = &buf[j..n];
                    match section.iter().position(|&c|c==b'\n'){
                        None => {
                            s.extend_from_slice(section);
                            // If the remote is just sending unparseable garbage we'll just discard it instead of crashing
                            if s.len() > 4000 {
                                s.clear();
                            }
                            break;
                        }
                        Some(i) => {
                            s.extend_from_slice(&section[0..=i]);
                            match parse_command(&s, stream, &mut auth) {
                                ParseStatus::Success() => ()
                                ,ParseStatus::BadClient() => {stream.write(b"+malformed_command:a#a\n"); return;}
                                ,ParseStatus::Unauthorised() => {stream.write(b"+auth_fail:a#a\n"); return;}
                            };
                            s.clear();
                            if i+1 >= n{
                                break;
                            }
                            j += i+1;
                        }
                    }
                }
            }
            ,Err(e) => {
                // Also the end of the stream, but less expectedly
                println!("Unable to read from remote: {:?}", e);
                break;
            }
        }
    }
    println!("Client stream ended, disconnected.")
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
                            ,Ok(mut stream) => {
                                println!("Connection from: {}", match stream.peer_addr() {Ok(addr)=>addr.to_string(), Err(e)=>e.to_string()});
                                handle_stream(&mut stream);
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

    if let Some(t) = tcp_thread { t.join(); };
    if let Some(t) = serial_thread { t.join(); };
}
