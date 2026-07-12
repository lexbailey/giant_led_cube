mod client;
use client::{start_client, ToGUI, ClientState};

use cube_model as cube;
use cube_model::Cube;

use std::str;
use std::process::Command;
use std::sync::mpsc::{channel,SendError};
use std::thread;
use std::str::FromStr;
use std::path::Path;
use std::fs::File;

use serde::{Deserialize, Serialize};

use rustyline::error::ReadlineError;
use rustyline::Editor;
use rustyline::ExternalPrinter;

macro_rules! mprintln {
    ($p:expr, $fmt:literal) => {
        $p.print(format!(concat!($fmt,"\n"))).unwrap()
    };
    ($p:expr, $fmt:literal, $($args:expr),+) => {
        $p.print(format!(concat!($fmt,"\n"), $($args),+)).unwrap()
    };
}

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

fn draw(gfx: &RenderData, state: &ClientState, printer: &mut impl ExternalPrinter){
    let p = printer;
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

    mprintln!(p, "              Back ({})", cube::BACK);
    mprintln!(p, "              ┏━━━━━━━━━━━━━┓");
    mprintln!(p, "              ┃ {} {} {} ┃", nb(ba, 8), nb(ba, 7), nb(ba, 6));
    mprintln!(p, "              ┃ {} {} {} ┃", bb(ba, 8), bb(ba, 7), bb(ba, 6));
    mprintln!(p, "              ┃             ┃");
    mprintln!(p, "              ┃ {} {} {} ┃", nb(ba, 5), nb(ba, 4), nb(ba, 3));
    mprintln!(p, "              ┃ {} {} {} ┃", bb(ba, 5), bb(ba, 4), bb(ba, 3));
    mprintln!(p, "              ┃             ┃");
    mprintln!(p, "              ┃ {} {} {} ┃", nb(ba, 2), nb(ba, 1), nb(ba, 0));
    mprintln!(p, "Left ({})      ┃ {} {} {} ┃    Right ({})      Bottom ({})", cube::LEFT, bb(ba, 2), bb(ba, 1), bb(ba, 0), cube::RIGHT, cube::BOTTOM);
    mprintln!(p, "┏━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━━━━━━━━━┳━━━━━━━━━━━━━┓");
    mprintln!(p, "┃ {} {} {} ┃ {} {} {} ┃ {} {} {} ┃ {} {} {} ┃", nb(l,6), nb(l,3), nb(l,0),   nb(t,0), nb(t,1), nb(t,2),   nb(r,2), nb(r,5), nb(r,8),   nb(bo,0), nb(bo,1), nb(bo,2));
    mprintln!(p, "┃ {} {} {} ┃ {} {} {} ┃ {} {} {} ┃ {} {} {} ┃", bb(l,6), bb(l,3), bb(l,0),   bb(t,0), bb(t,1), bb(t,2),   bb(r,2), bb(r,5), bb(r,8),   bb(bo,0), bb(bo,1), bb(bo,2));
    mprintln!(p, "┃             ┃    Top ({})  ┃             ┃             ┃", cube::TOP);
    mprintln!(p, "┃ {} {} {} ┃ {} {} {} ┃ {} {} {} ┃ {} {} {} ┃", nb(l,7), nb(l,4), nb(l,1),   nb(t,3), nb(t,4), nb(t,5),   nb(r,1), nb(r,4), nb(r,7),   nb(bo,3), nb(bo,4), nb(bo,5));
    mprintln!(p, "┃ {} {} {} ┃ {} {} {} ┃ {} {} {} ┃ {} {} {} ┃", bb(l,7), bb(l,4), bb(l,1),   bb(t,3), bb(t,4), bb(t,5),   bb(r,1), bb(r,4), bb(r,7),   bb(bo,3), bb(bo,4), bb(bo,5));
    mprintln!(p, "┃             ┃             ┃             ┃             ┃");
    mprintln!(p, "┃ {} {} {} ┃ {} {} {} ┃ {} {} {} ┃ {} {} {} ┃", nb(l,8), nb(l,5), nb(l,2),   nb(t,6), nb(t,7), nb(t,8),   nb(r,0), nb(r,3), nb(r,6),   nb(bo,6), nb(bo,7), nb(bo,8));
    mprintln!(p, "┃ {} {} {} ┃ {} {} {} ┃ {} {} {} ┃ {} {} {} ┃", bb(l,8), bb(l,5), bb(l,2),   bb(t,6), bb(t,7), bb(t,8),   bb(r,0), bb(r,3), bb(r,6),   bb(bo,6), bb(bo,7), bb(bo,8));
    mprintln!(p, "┗━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━━━━━━━━━┻━━━━━━━━━━━━━┛");
    mprintln!(p, "              ┃ {} {} {} ┃", nb(f, 0), nb(f, 1), nb(f, 2));
    mprintln!(p, "              ┃ {} {} {} ┃", bb(f, 0), bb(f, 1), bb(f, 2));
    mprintln!(p, "              ┃             ┃");
    mprintln!(p, "              ┃ {} {} {} ┃", nb(f, 3), nb(f, 4), nb(f, 5));
    mprintln!(p, "              ┃ {} {} {} ┃", bb(f, 3), bb(f, 4), bb(f, 5));
    mprintln!(p, "              ┃             ┃");
    mprintln!(p, "              ┃ {} {} {} ┃", nb(f, 6), nb(f, 7), nb(f, 8));
    mprintln!(p, "              ┃ {} {} {} ┃", bb(f, 6), bb(f, 7), bb(f, 8));
    mprintln!(p, "              ┗━━━━━━━━━━━━━┛");
    mprintln!(p, "              Front ({})", cube::FRONT);

    if state.input_detect_state.active{
        mprintln!(p,"Detecting switch input for twist: {}", state.input_detect_state.twist);
        mprintln!(p,"Push the switch between the RED and GREEN LEDs towards the GREEN LED");
    }
    if state.led_detect_state.active{
        mprintln!(p,"Currently detecting LEDs. Use the `map <face_num> <subface_num>` command to map the currently lit LED.");
        mprintln!(p,"Currently detecting LED number {}", state.led_detect_state.led_num);
    }
}

#[derive(Serialize, Deserialize)]
struct CLIConfig{
    server: String
    ,secret: String
}

fn main() {

    let config: CLIConfig= {
        let p = Path::new("cli_config");
        match File::open(p) {
            Ok(f) => match serde_json::from_reader(f) {
                Ok(d) => d
                ,Err(e) => {println!("Failed to parse config file {}: {}", p.display(), e); std::process::exit(1);}
            }
            ,Err(e) => {println!("Failed to load config file {}: {}", p.display(), e); std::process::exit(1);}
        }
    };

    let gfx = init_render_data();

    // Current thread generates user input events
    let mut rl = Editor::<(),rustyline::history::DefaultHistory>::new().unwrap();
    let _ignored = rl.load_history(".cube_control_history");
    let mut printer = rl.create_external_printer().unwrap();

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

    let secret = config.secret.as_bytes().to_vec();
    let addr = config.server;

    let (sync_sender, sync_receiver) = channel();


    // Main event loop handles both kinds of events
    let event_loop = thread::spawn(move||{
        let p = &mut printer;
        use client::FromGUI::*;
        if let Err(e) = sender.send(Connect(secret.clone(), addr.clone())){
            mprintln!(p, "Failed to start: {:?}", e);
            return;
        }
        let gui_release = move||{let _ignored = sync_sender.send(());};
        enum TwistMode{
            Client(),
            Server(),
        }
        let mut twist_mode = TwistMode::Client();
        for ev in receiver.iter() {
            let result: Result<bool, SendError<client::FromGUI>> = (||{
                match ev {
                    Client(ev) => {
                        use client::ToGUI::*;
                        match ev {
                            Connected(is_connected) => {
                                if is_connected{
                                    mprintln!(p, "Connected to server");
                                }
                                else{
                                    mprintln!(p, "Disconnected from server. Some events may have been dropped.");
                                }
                            }
                            ,MissingConnection() => {
                                mprintln!(p, "Error: Not connected to server.");
                            }
                            ,StateUpdate() => {
                                let data = state.lock().unwrap();
                                draw(&gfx, &*data, p);
                            }
                            ,GameEnd() => {mprintln!(p, "TODO game end");}
                        }
                    }
                    ,UserInput(command) => {
                        match command.as_ref(){
                            "help" => {
                                mprintln!(p, "Commands:");
                                mprintln!(p, "\tanim <TWIST>             - show the animation used when performing the specified twist");
                                mprintln!(p, "\tbrightness <VALUE>       - set the brightness of the LEDs (value in range 0 to 255)");
                                mprintln!(p, "\tcal [on|off]             - enable or disable calibration mode");
                                mprintln!(p, "\tconnect                  - establish a connection to the remote cube service");
                                mprintln!(p, "\tdetect inputs            - start the input switch configuration sequence");
                                mprintln!(p, "\tdetect leds              - start the LED configuration sequence");
                                mprintln!(p, "\t    map <FACE> <SUBFACE> - map currently lit LED during LED detection sequence to face FACE and subface SUBFACE");
                                mprintln!(p, "\t    map undo             - undo a step in the LED detection sequence");
                                mprintln!(p, "\tdetect skip              - skip the current detection step (will result in a broken calibration, but useful for testing partially built systems)");
                                mprintln!(p, "\tdetect abort             - stop detecting LEDs or inputs, and return to normal operation");
                                mprintln!(p, "\texit                     - quit the CLI");
                                mprintln!(p, "\trot <FACE> <SUBFACE>     - rotate the specified subface by 90 degrees (use 'cal on' to see rotation guides)");
                                mprintln!(p, "\tshow                     - show the state of the cube");
                                mprintln!(p, "\tsolved                   - move to the solved state");
                                mprintln!(p, "\tstart                    - start the game (applies a scramble too)");
                                mprintln!(p, "\ttwist <TWIST>            - execute a twist on the cube. eg: \"twist U'\"");
                                mprintln!(p, "\ttwistmode client         - twists happen in the client, and the resulting raw state sent to the server");
                                mprintln!(p, "\ttwistmode server         - twists happend on the server, and the resulting raw state is pulled from the server");
                            }
                            ,"show" => {
                                let data = state.lock().unwrap();
                                draw(&gfx, &*data, p);
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
                            ,"detect skip" => {
                                sender.send(DetectionSkip())?;
                            }
                            ,"detect abort" => {
                                sender.send(DetectionAbort())?;
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
                                    "anim" => {
                                        if args.len() != 1{
                                            mprintln!(p, "anim requires one parameter");
                                        }
                                        let mut data = state.lock().unwrap();
                                        let t = cube::Twist::from_string(args[0]);
                                        match t{
                                            Err(_) => { mprintln!(p, "bad argument"); }
                                            ,Ok(t) => {
                                                let anim = data.cube.twist(t);
                                                let oldcube = data.cube;
                                                let ms = 30;
                                                data.cube = anim[0];
                                                draw(&gfx, &data, p);
                                                thread::sleep(std::time::Duration::from_millis(ms));
                                                data.cube = anim[1];
                                                draw(&gfx, &data, p);
                                                thread::sleep(std::time::Duration::from_millis(ms));
                                                data.cube = anim[2];
                                                draw(&gfx, &data, p);
                                                thread::sleep(std::time::Duration::from_millis(ms));
                                                data.cube = oldcube;
                                                draw(&gfx, &data, p);
                                                thread::sleep(std::time::Duration::from_millis(ms));
                                            }
                                        }
                                    }
                                    "twist" => {
                                        if args.len() != 1{
                                            mprintln!(p, "twist requires one parameter");
                                        }
                                        let mut data = state.lock().unwrap();
                                        match twist_mode{
                                            TwistMode::Client() => {
                                                match data.cube.twists(args_str){
                                                    Err(msg) => {mprintln!(p, "Error: {}", msg);}
                                                    ,Ok(_) => {
                                                        sender.send(SyncState())?;
                                                        draw(&gfx, &data, p);
                                                    }
                                                }
                                            },
                                            TwistMode::Server() => {
                                                if let Ok(t) = cube_model::Twist::seq_from_string(args_str){
                                                    for t in t{
                                                        sender.send(DoTwist(t))?;
                                                    }
                                                    sender.send(GetState())?;
                                                }
                                                else{
                                                    mprintln!(p, "Invalid twist sequence.");
                                                }
                                            },
                                        }
                                    }
                                    "twistmode" => {
                                        if args.len() != 1{
                                            mprintln!(p, "twistmode requires one parameter");
                                        }
                                        match args[0]{
                                            "client" => {twist_mode = TwistMode::Client();},
                                            "server" => {twist_mode = TwistMode::Server();},
                                            a => {mprintln!(p, "unknown twist mode '{}'", a)},
                                        }
                                    }
                                    ,"map" => {
                                        if args.len() != 2{
                                            mprintln!(p, "map requires two parameters");
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
                                                mprintln!(p, "mapped led {} to (face, subface) = ({}, {})", state.led_detect_state.led_num, f, s);
                                            }
                                        }
                                    }
                                    ,"brightness" => {
                                        if args.len() != 1{
                                            mprintln!(p, "brightness requires one parameter, a number in the range 0 to 255");
                                        }
                                        else{
                                            sender.send(SetBrightness(args[0].to_string()))?;
                                        }
                                    },
                                    "connect" => {
                                        sender.send(Connect(secret.clone(),addr.clone()))?;
                                    },
                                    "cal" => {
                                        if args.len() != 1{
                                            mprintln!(p, "cal requires one parameter");
                                        }
                                        match args[0]{
                                            "on" => {sender.send(EnableCalibrationView())?;}
                                            "off" => {sender.send(DisableCalibrationView())?;}
                                            _ => {mprintln!(p, "cal command expects 'on' or 'off' as parameter");}
                                        }
                                    },
                                    "rot" => {
                                        if args.len() != 2{
                                            mprintln!(p, "rot requires two parameters");
                                        }
                                        else{
                                            if let Ok((f, s)) = (||{
                                                Result::<(usize, usize), std::num::ParseIntError>::Ok((
                                                    usize::from_str(args[0])?
                                                    ,usize::from_str(args[1])?
                                                ))
                                            })() {
                                                sender.send(RotateSubface(f, s))?;
                                                mprintln!(p, "rotated subface ({}, {})", f, s);
                                            }
                                        }
                                    }
                                    _ => {mprintln!(p, "Unknown command: {}",cmd);},
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
                ,Err(e) => {mprintln!(p, "Internal event loop error: {:?}", e)}
            }
        }
    });


    'repl: loop {
        let readline = rl.readline("Cube Control> ");
        match readline {
            Ok(line) => {
                let _ = rl.add_history_entry(line.as_str()); // best-effort, ignore errors
                for line in line.lines(){
                    match u_sender.send(UserInput(line.to_string())){
                        Err(e) => {println!("Internal error: {:?}", e);}
                        ,Ok(_) => {}
                    }
                    if let Err(_e) = sync_receiver.recv(){
                        // other end disconnected, normally because of graceful exit. terminate.
                        break 'repl
                    }
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
