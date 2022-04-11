mod client;
use client::{start_client, ToGUI, ClientState};

use cube_model as cube;
use cube_model::Cube;

use std::str;
use std::process::Command;
use std::sync::mpsc::{channel,SendError};
use std::thread;
use std::str::FromStr;

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

struct RenderData{
    tc: TermCols
}

fn tput (f:fn (&mut Command)-> &mut Command) -> String {
    String::from_utf8(f(&mut Command::new("tput")).output().expect("tput failed").stdout).unwrap()
}

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

fn init_render_data() -> RenderData{
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

fn draw(gfx: &RenderData, state: &ClientState){
    let mut cube = state.cube;
    if state.led_detect_state.active{
        cube = Cube::new();
    }
    fn nb (f: &cube::Face, i:usize, tc: &TermCols) -> String{ color_string(i.to_string(), f.subfaces[i].color, &tc) }
    fn bb (f: &cube::Face, i:usize, tc: &TermCols) -> String{ color_string("".to_string(), f.subfaces[i].color, &tc) }

    let ba = &cube.faces[cube::BACK];
    let l = &cube.faces[cube::LEFT];
    let t = &cube.faces[cube::TOP];
    let r = &cube.faces[cube::RIGHT];
    let bo = &cube.faces[cube::BOTTOM];
    let f = &cube.faces[cube::FRONT];

    let nb = |f,i|nb(f,i,&gfx.tc);
    let bb = |f,i|bb(f,i,&gfx.tc);

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

    if state.input_detect_state.active{
        println!("Detecting switch input for twist: {}", state.input_detect_state.twist);
        println!("Push the switch between the RED and GREEN LEDs towards the GREEN LED");
    }
    if state.led_detect_state.active{
        println!("Currently detecting LEDs. Use the `map <face_num> <subface_num>` command to map the currently lit LED.");
        println!("Currently detecting LED number {}", state.led_detect_state.led_num);
    }
}


fn main() {
    let gfx = init_render_data();
    use rustyline::error::ReadlineError;
    use rustyline::Editor;

    let (state, sender, c_receiver, client) = start_client();

    // The same event loop will handle client events and user events, so we need a type that encapsulates both
    enum CLIEvent{
        Client(ToGUI)
        ,UserInput(String)
    }
    use CLIEvent::*;
    let (u_sender, receiver) = channel::<CLIEvent>();
    let c_sender = u_sender.clone();

    // start a thread to repackage client events
    let client_event_forwarder = thread::spawn(move||{
        for ev in c_receiver.iter(){
            let _ignored = c_sender.send(Client(ev));
        }
    });

    let secret = b"secret".to_vec(); // TODO load from file
    let addr = "localhost:9876".to_string(); // TODO load from tile

    let (sync_sender, sync_receiver) = channel();

    // Main event loop handles both kinds of events
    let event_loop = thread::spawn(move||{
        use client::FromGUI::*;
        if let Err(e) = sender.send(Connect(secret, addr)){
            println!("Failed to start: {:?}", e);
            return;
        }
        let gui_release = move||{let _ignored = sync_sender.send(());};
        for ev in receiver.iter() {
            let result: Result<bool, SendError<client::FromGUI>> = (||{
                match ev {
                    Client(ev) => {
                        use client::ToGUI::*;
                        match ev {
                            Connected(is_connected) => {
                                if is_connected{
                                    println!("Connected to server");
                                }
                                else{
                                    println!("Disconnected from server. Some events may have been dropped. Trying to reconnect...");
                                    // TODO try to reconnect (with exponential backoff?)
                                }
                            }
                            ,MissingConnection() => {
                                println!("Internal error: Know known method of connecting to server.");
                            }
                            ,StateUpdate() => {
                                let data = state.lock().unwrap();
                                draw(&gfx, &*data);
                                
                            }
                            ,GameEnd() => {println!("TODO game end");}
                        }
                    }
                    ,UserInput(command) => {
                        match command.as_ref(){
                            "show" => {
                                let data = state.lock().unwrap();
                                draw(&gfx, &*data);
                            }
                            ,"solved" => {
                                sender.send(SetState(Cube::new()))?;
                            }
                            ,"detect leds" => {
                                sender.send(DetectLEDs())?;
                            }
                            ,"map undo" => {
                                sender.send(BacktrackLEDDetect())?;
                            }
                            ,"detect inputs" => {
                                sender.send(DetectInputs())?;
                            }
                            ,"start" => {
                                sender.send(StartGame())?;
                            }
                            ,"exit" => {
                                sender.send(ShutDown())?;
                                return Ok(true);
                            }
                            ,"" => {}
                            ,cmd => {
                                let mut parts = cmd.split(' ');
                                let name = parts.next().unwrap();
                                let args_str = &cmd[name.len()..cmd.len()];
                                let args = parts.collect::<Vec<&str>>();
                                match name.as_ref(){
                                    "twist" => {
                                        let mut data = state.lock().unwrap();
                                        match data.cube.twists(args_str){
                                            Err(msg) => {println!("Error: {}", msg)}
                                            ,Ok(_) => {
                                                sender.send(SyncState())?;
                                                draw(&gfx, &data);
                                            }
                                        }
                                    }
                                    ,"map" => {
                                        if args.len() != 2{
                                            println!("map requires two parameters");
                                        }
                                        else{
                                            let state = state.lock().unwrap();
                                            if let Ok((f, s)) = (||{
                                                Result::<(usize, usize), std::num::ParseIntError>::Ok((
                                                    usize::from_str(args[0])?
                                                    ,usize::from_str(args[1])?
                                                ))
                                            })() {
                                                sender.send(MapLED(f, s))?;
                                                println!("mapped led {} to (face, subface) = ({}, {})", state.led_detect_state.led_num, f, s);
                                            }
                                        }
                                    }
                                    ,_ => {println!("Unknown command: {}",cmd);}
                                }
                            }
                        }
                        gui_release();
                    }
                }
                Ok(false)
            })();
            match result{
                Ok(do_break) => {if do_break {break;}}
                ,Err(e) => {println!("Internal event loop error: {:?}", e)}
            }
        }
    });

    // Current thread generates user input events
    let mut rl = Editor::<()>::new();
    let _ignored = rl.load_history(".cube_control_history");
    loop {
        let readline = rl.readline("Cube Control> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str());
                for line in line.lines(){
                    match u_sender.send(UserInput(line.to_string())){
                        Err(e) => {println!("Internal error: {:?}", e);}
                        ,Ok(_) => {}
                    }
                    sync_receiver.recv().unwrap();
                }
            }
            ,Err(ReadlineError::Interrupted) => {
            }
            ,Err(ReadlineError::Eof) => {
                println!("exit");
                let _ignored = u_sender.send(UserInput("exit".to_string()));
                break
            }
            ,Err(err) => {
                println!("Error: {:?}", err);
                break
            }
        }
    }
    rl.save_history(".cube_control_history").unwrap();

    let _ignored = event_loop.join();
    let _ignored = client_event_forwarder.join();
    let _ignored = client.join();

}
