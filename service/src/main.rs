use clap::Parser as CLIParser;
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::io::{Write,Read};
use rand_core::{RngCore, SeedableRng, CryptoRng};
use rand_chacha::{ChaCha20Rng};

extern crate pest;
#[macro_use]
extern crate pest_derive;
use pest::Parser;

use sha2::Sha256;
use hmac::{Hmac, Mac};
type HmacSha256 = Hmac<Sha256>;


#[derive(CLIParser, Debug)]
struct Args{
    #[clap()]
    device: String,
    #[clap(long)]
    tcp: Option<String>,
    #[clap(long)]
    serial: Option<String>,
}

#[derive(Parser)]
#[grammar = "command_parser.pest"]
struct CommandParser;

fn command_is_authentic(command: &str, auth_state: &AuthState, msg_salt: &str, given_mac: &str) -> bool{
    if auth_state.salt != msg_salt{ return false; } // Didn't use the most recent challenge value
    let given_mac = hex::decode(given_mac);
    if given_mac.is_err(){return false;} // Mac is not even a valid hex string
    let mut mac = HmacSha256::new_from_slice(&auth_state.secret).unwrap();
    mac.update(command.as_bytes());
    mac.verify_slice(&given_mac.unwrap()).is_ok()
}

fn authenticate_message(message: &str, auth_state: &AuthState) -> String{
    let mut mac = HmacSha256::new_from_slice(&auth_state.secret).unwrap();
    mac.update(message.as_bytes());
    mac.update(auth_state.salt.as_bytes());
    let sig = hex::encode(mac.finalize().into_bytes());
    format!("{}{}#{}\n", message, auth_state.salt, sig)
}

struct GenericAuthState<Rng: RngCore + SeedableRng + CryptoRng>{
    rng: Rng
    ,salt: String
    ,secret: Vec<u8>
}

type AuthState = GenericAuthState<ChaCha20Rng>;

impl<Rng: RngCore + SeedableRng + CryptoRng> GenericAuthState<Rng>{
    fn new(secret: Vec<u8>) -> GenericAuthState<Rng>{
        let mut a = GenericAuthState{
            rng: Rng::from_entropy()
            ,salt: "".to_string()
            ,secret: secret
        };
        a.step();
        a
    }

    fn step(&mut self){
        let mut bytes = [0;8];
        self.rng.fill_bytes(&mut bytes);
        self.salt = hex::encode(bytes);
    }
}

fn construct_message(command: &str, args: &Vec<&str>, auth_state: &AuthState) -> String{
    let mut arg_list = String::new();
    for a in args{
        arg_list.push_str(a);
        arg_list.push(',');
    }
    let message = format!("{}:{}", command, arg_list);
    authenticate_message(&message, auth_state)
}

fn send_challenge<T: Write>(stream: &mut T, auth_state: &mut AuthState){
    auth_state.step();
    let msg = construct_message("+challenge", &vec![], auth_state);
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
                    //println!("Check string: '{}' against mac: {}", checked_string, mac);
                    if command_name == "next_challenge" {
                        // Don't check if this is authentic, challenges can be requested by anyone
                        println!("Sending next challenge");
                        send_challenge(stream, auth_state);
                    }
                    else {
                        if command_is_authentic(checked_string, auth_state, msg_salt, mac){
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
