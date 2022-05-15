use std::fs::File;
use std::io::Write;

fn main(){
    let mut sound_loader = File::create("src/sounds.rs").unwrap();
    sound_loader.write(("{\n").as_bytes());
    for i in 1..=11 {
        sound_loader.write(format!("\tlet _wav_bytes_{:02} = include_bytes!(\"../../sounds/twist-{:02}.wav\");\n", i, i).as_bytes());
    }
    sound_loader.write(format!("vec![\n").as_bytes());
    for i in 1..=11 {
        sound_loader.write(format!("\t_wav_bytes_{:02},\n", i).as_bytes());
    }
    sound_loader.write(format!("]\n").as_bytes());
    sound_loader.write(("}\n").as_bytes());
}
