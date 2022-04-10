use cube_model as cube;
use cube::{Cube, Output, OutputMap5Faces, Twist};

use std::str;
use std::time::{Instant,Duration};
use rand::Rng;
use std::process::Command;
use std::io::{self,Read,Write,BufRead,BufReader};
use std::net::TcpStream;
use std::sync::mpsc::{channel,Sender,Receiver};
use std::sync::{Arc,Mutex};
use std::thread::{self,Thread,JoinHandle};
use std::collections::VecDeque;
use std::str::FromStr;
use std::collections::HashSet;

use plain_authentic_commands::{MessageHandler, ParseStatus};

pub struct DetectState {
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

pub struct ClientState {
    pub cube: Cube
    ,pub detect_state: DetectState
}

impl ClientState {
    fn new() -> ClientState{
        ClientState {
            cube: Cube::new()
            ,detect_state: DetectState::new()
        }
    }
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



#[derive(Debug)]
pub enum FromGUI {
    Connect(Vec<u8>, String) // secret, address
    ,DetectLEDs()
    ,DetectInputs()
    ,StartGame()
}

#[derive(Debug)]
pub enum ToGUI {
    StateUpdate()
    ,GameEnd()
    ,Connected(bool)
    ,MissingConnection()
}

#[derive(Debug)]
enum Event {
    ServiceMessage(Vec<u8>)
    ,FromGUI(FromGUI)
}

fn handle_responses<T: Read>(stream: &mut T, events: Sender<Event>) {
    let mut reader = BufReader::new(stream);
    for line in reader.split(b'\n'){
        if let Ok(line) = line {
            events.send(Event::ServiceMessage(line));
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

/*
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
*/

pub fn start_client() -> (Arc<Mutex<ClientState>>, Sender<FromGUI>, Receiver<ToGUI>, JoinHandle<()>) {
    let state = Arc::new(Mutex::new(ClientState::new()));
    let gui_state = Arc::clone(&state);
    let (sender, receiver) = channel();
    let (from_gui_sender, from_gui_receiver) = channel();
    let internal_gui_sender = sender.clone();
    let service_sender = sender.clone();
    let (to_gui_sender, to_gui_receiver) = channel();

    let from_gui_thread = thread::spawn(move||{
        for event in from_gui_receiver.iter(){
            internal_gui_sender.send(Event::FromGUI(event));
        }
    });

    let mut msg: Option<TcpMessenger> = None;

    let thread = thread::spawn(move||{
        let mut command_queue: VecDeque<(String, Vec<String>)> = VecDeque::new();
        let mut got_challenge = false;


        let mut net_thread: Option<JoinHandle<()>> = None;

        fn start_service_handler(net_thread: &mut Option<JoinHandle<()>>, service_sender: Sender<Event>, mut reader: TcpStream) {
            if net_thread.is_some() {
                net_thread.take().unwrap().join();
            }
            *net_thread = Some(thread::spawn(move||{
                handle_responses(&mut reader, service_sender);
            }));
        }


        fn send_events(got_challenge: &mut bool, command_queue: &mut VecDeque<(String, Vec<String>)>, msg: Option<&mut TcpMessenger>) -> Vec<ToGUI>{
            println!("Send events...");
            let mut results = vec![];
            if msg.is_none(){
                *got_challenge = false;
                results.push(ToGUI::MissingConnection());
                command_queue.clear();
            }
            if *got_challenge {
                if let Some((command, args)) = command_queue.pop_front() {
                    let m_args = args.iter().map(|a|a.as_ref()).collect();
                    *got_challenge = false;
                    let msg = msg.unwrap();
                    match msg.send_command(&command, &m_args) {
                        Ok(_) => {}
                        ,Err(e) => { // Probably no longer connected
                            match msg.connect() {
                                Ok(_) => { results.push(ToGUI::Connected(true)); command_queue.push_front((command, args)); }
                                ,Err(e) => { results.push(ToGUI::Connected(false)); command_queue.clear(); }
                            }
                        }
                    }
                }
            }
            results
        }

        //command_queue.push_back(("set_state".to_string(), vec![test_state]));

        for event in receiver.iter(){
            use Event::*;
            match event {
                ServiceMessage(s) => {
                    if let Some(msg) = msg.as_mut(){
                        match msg.handler.parse_response(&s) {
                            ParseStatus::Success(response, args) => {
                                match response.as_ref() {
                                    "challenge" => {
                                        println!("got challenge");
                                        got_challenge = true;
                                    }   
                                    ,"input" => {
                                        println!("user applied input: {}", args[0]);
                                        if let Ok(input) = u32::from_str(&args[0]){
                                            use DetectMessage::*;
                                            let mut state = state.lock().unwrap();
                                            match state.detect_state.sample_input(input){
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
                }
                ,FromGUI(e) => {
                    use self::FromGUI::*;
                    use ToGUI::*;
                    match e {
                        Connect(secret, addr) => {
                            let mut m = TcpMessenger::new(secret, &addr);
                            let r = m.connect();
                            match r {
                                Ok(_) => {
                                    start_service_handler(&mut net_thread, service_sender.clone(), m.stream.as_ref().unwrap().try_clone().unwrap());
                                    to_gui_sender.send(ToGUI::Connected(true));
                                }
                                Err(e) => {
                                    to_gui_sender.send(ToGUI::Connected(false));
                                }
                            }
                            msg = Some(m);
                        }

                        ,DetectLEDs() => {println!("TODO leds detect");}
                        ,DetectInputs() => {println!("TODO inputs detect");}
                        ,StartGame() => {
                            let mut state = state.lock().unwrap();
                            state.cube = cube_model::Cube::new();
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
                                state.cube.twist(twist);
                            }
                            println!("done start");
                            command_queue.push_back(("set_state".to_string(), vec![state.cube.serialise()]));
                            command_queue.push_back(("play".to_string(), vec![]));
                            println!("done start2");
                        }
                    }
                }
            }
            let replies = send_events(&mut got_challenge, &mut command_queue, msg.as_mut());
            for reply in replies{
                to_gui_sender.send(reply);
            }
        }
        from_gui_thread.join();
    });

    (gui_state, from_gui_sender, to_gui_receiver, thread)
}

