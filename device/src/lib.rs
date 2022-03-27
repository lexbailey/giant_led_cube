#![no_std]

use cube_model::{Cube, OutputMap5Faces, Output, Colors, Twist};
use core::panic::PanicInfo;
use core::slice;
use core::str;

#[panic_handler]
fn panic(_: &PanicInfo) -> !{
    unsafe{ core::arch::asm!("bkpt"); }
    loop{}
}

#[no_mangle]
pub extern "C" fn init_cube(cube_store: *mut Cube, mapping_store: *mut OutputMap5Faces) {
    let cube = Cube::new();
    
    let mut mapping: OutputMap5Faces = [
        Output{face:0,subface:0};45
    ];
    
    for i in 0..5{
        let f = i * 9;
        for j in 0..9{
            mapping[f+j].face = i;
            mapping[f+j].subface = j;
        }
    }
    unsafe {
        core::ptr::copy::<Cube>(&cube, cube_store, 1);
        core::ptr::copy::<OutputMap5Faces>(&mapping, mapping_store, 1);
    }
}

#[no_mangle]
pub extern "C" fn get_data(cube: *mut Cube, mapping: *const OutputMap5Faces, data_out: *mut u32) {
    for i in 0..5{
        for j in 0..9{
            let pos = (i*9)+j;
            unsafe {
                let output = (*mapping)[pos];
                let color = (*cube).faces[output.face].subfaces[output.subface].color;
                *(data_out.add(pos)) = match color {
                    Colors::Red => 0xff0000
                    ,Colors::Green => 0x00ff00
                    ,Colors::Blue => 0x0000ff
                    ,Colors::White => 0xffffff
                    ,Colors::Yellow => 0xffff00
                    ,Colors::Orange => 0xff3000
                    ,Colors::Blank => 0x0
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn update_from_string(cube: *mut Cube, s: *const u8) {
    unsafe{
        (*cube).deserialise(&str::from_utf8(slice::from_raw_parts(s, 6*9)).unwrap());
    }
}

#[no_mangle]
pub extern "C" fn twist_cube(cube: *mut Cube, s: *const u8, l: u32){
    unsafe{
        let t = &str::from_utf8(slice::from_raw_parts(s, l as usize)).unwrap();
        if let Ok(t) = Twist::from_string(t){
            (*cube).twist(t);
        }
    }
}

#[no_mangle]
pub extern "C" fn remap_outputs(mapping: *mut OutputMap5Faces, newmap: *const u8){
    for i in 0..5{
        let f = i * 9;
        for j in 0..9{
            let index = f+j;
            let index2 = index * 2;
            unsafe {
                (*mapping)[index].face = (*newmap.add(index2)) as usize;
                (*mapping)[index].subface = (*newmap.add(index2+1)) as usize;
            }
        }
    }
}
