extern crate gl;
extern crate glutin;

use glutin::dpi::PhysicalPosition;
use glutin::event::{ElementState, MouseButton};

use std::collections::HashMap;

use gl::types::*;
use std::mem;
use std::ptr;
use std::os::raw::c_void;
use std::ffi::CString;
use std::time::{Instant,Duration};
use std::cell::RefCell;

use gl_abstractions as gla;
use gla::{UniformMat4, UniformVec3, UniformVec4, UniformSampler2D, shader_struct, impl_shader};

#[cfg(feature="gles")]
use gla::UniformVec2;

pub mod client;
use client::{start_client, ToGUI, FromGUI, ClientState};

use cube_model as cube;
use cube::Cube;

use std::sync::{Arc,Mutex};
use std::sync::mpsc::{Sender, Receiver};

use fontdue::Font;

use std::thread;

// Shaders for main OpenGL version
#[cfg(not(feature="gles"))]
shader_struct!{
    CubeShader
    ,r#"
        #version 330 core
        layout (location = 0) in vec4 aPos;
        uniform mat4 u_global_transform;
        uniform mat4 u_face_transform;
        uniform mat4 u_offset;
        uniform mat4 u_transform;
        void main()
        {
            gl_Position = aPos  * u_offset * u_face_transform * u_transform * u_global_transform;
        }
        "#
    ,r#"
        #version 330 core
        out vec4 FragColor;
        uniform vec3 u_color;
        void main()
        {
           // Set the fragment color to the color passed from the vertex shader
           FragColor = vec4(u_color, 1.0);
        }
        "#
    ,{
        u_face_transform: UniformMat4,
        u_offset: UniformMat4,
        u_transform: UniformMat4,
        u_color: UniformVec3,
        u_global_transform: UniformMat4,
    }
}

#[cfg(not(feature="gles"))]
shader_struct!{
    ImageShader 
    ,r#"
        #version 330 core
        layout (location = 0) in vec2 screenpos;
        layout (location = 1) in vec2 texcoord_in;

        uniform mat4 u_global_transform;
        uniform mat4 u_pix_transform;
        uniform mat4 u_image_geom;
        uniform mat4 u_translate;
        uniform mat4 u_scale;
        uniform vec4 u_glyph_select;

        out vec2 texcoord;

        void main()
        {
            gl_Position = vec4(screenpos, 0.0, 1.0) * u_image_geom * u_translate * u_global_transform * u_pix_transform;
            texcoord = (u_glyph_select.xy + (texcoord_in * u_glyph_select.zw));
        }
        "#
    ,r#"
        #version 330 core
        out vec4 FragColor;
        
        in vec2 texcoord;
    
        uniform vec4 u_color;
        uniform sampler2D u_texture;

        void main()
        {
           FragColor =
             min(1.0, (u_color.a * texture(u_texture, texcoord / textureSize(u_texture, 0)).r) + (1.0-u_color.a))
             *
             vec4(u_color.rgb, 1.0)
           ;
        }
        "#
    ,{
        u_color: UniformVec4,
        u_texture: UniformSampler2D,
        u_image_geom: UniformMat4,
        u_translate: UniformMat4,
        u_global_transform: UniformMat4,
        u_pix_transform: UniformMat4,
        u_scale: UniformMat4,
        u_glyph_select: UniformVec4,
    }
}

// Shaders for OpenGLES version
#[cfg(feature="gles")]
shader_struct!{
    CubeShader
    ,r#"
        #version 300 es
        precision mediump float;
        layout (location = 0) in vec4 aPos;
        uniform mat4 u_global_transform;
        uniform mat4 u_face_transform;
        uniform mat4 u_offset;
        uniform mat4 u_transform;
        void main()
        {
            gl_Position = aPos  * u_offset * u_face_transform * u_transform * u_global_transform;
        }
        "#
    ,r#"
        #version 300 es
        precision mediump float;
        out vec4 FragColor;
        uniform vec3 u_color;
        void main()
        {
           // Set the fragment color to the color passed from the vertex shader
           FragColor = vec4(u_color, 1.0);
        }
        "#
    ,{
        u_face_transform: UniformMat4,
        u_offset: UniformMat4,
        u_transform: UniformMat4,
        u_color: UniformVec3,
        u_global_transform: UniformMat4,
    }
}

#[cfg(feature="gles")]
shader_struct!{
    ImageShader 
    ,r#"
        #version 300 es
        precision mediump float;
        layout (location = 0) in vec2 screenpos;
        layout (location = 1) in vec2 texcoord_in;

        uniform mat4 u_global_transform;
        uniform mat4 u_pix_transform;
        uniform mat4 u_image_geom;
        uniform mat4 u_translate;
        uniform mat4 u_scale;
        uniform vec4 u_glyph_select;

        out vec2 texcoord;

        void main()
        {
            gl_Position = vec4(screenpos, 0.0, 1.0) * u_image_geom * u_translate * u_global_transform * u_pix_transform;
            texcoord = (u_glyph_select.xy + (texcoord_in * u_glyph_select.zw));
        }
        "#
    ,r#"
        #version 300 es
        precision mediump float;
        out vec4 FragColor;
        
        in vec2 texcoord;
    
        uniform vec4 u_color;
        uniform sampler2D u_texture;
        uniform vec2 u_tex_size;

        void main()
        {
           FragColor =
             min(1.0, (u_color.a * texture(u_texture, texcoord / u_tex_size).r) + (1.0-u_color.a))
             *
             vec4(u_color.rgb, 1.0)
           ;
        }
        "#
    ,{
        u_color: UniformVec4,
        u_texture: UniformSampler2D,
        u_image_geom: UniformMat4,
        u_translate: UniformMat4,
        u_global_transform: UniformMat4,
        u_pix_transform: UniformMat4,
        u_scale: UniformMat4,
        u_glyph_select: UniformVec4,
        u_tex_size: UniformVec2,
    }
}

struct DataModel{
    // TODO move d, r, diff, and frames into the renderdata for the opengl version
    d: f32
    ,r: f32
    ,diff: f32
    ,frames: i32
}


use glutin::ContextWrapper;
use glutin::PossiblyCurrent;

#[derive(Clone)]
struct Button{
    x: f32
    ,y: f32
    ,w: f32
    ,h: f32
    ,label: String
    ,id: String
    ,click_state: u32
    ,pt: f32
}

impl Button {
    fn new(x: f32, y: f32, w: f32, h: f32, label: String, id: String, pt: f32) -> Button{
        Button{
            x
            ,y
            ,w
            ,h
            ,label
            ,click_state: 0
            ,pt
            ,id
        }
    }
}

struct GlyphSheet{
    locations: HashMap<fontdue::layout::GlyphRasterConfig, (usize, usize, usize, usize)>
    ,bitmap: Vec<u8>
    ,pos_x: usize
    ,pos_y: usize
    ,max_used_y: usize
    ,width: usize
    ,height: usize
}

impl GlyphSheet{
    fn new(tex_size: usize) -> Self{
        let bitmap_size = tex_size * tex_size;
        let mut bitmap = Vec::with_capacity(bitmap_size);
        bitmap.resize(bitmap_size, 0);
        GlyphSheet{
            locations: HashMap::new()
            ,bitmap
            ,pos_x: 0
            ,pos_y: 0
            ,max_used_y: 0
            ,width: tex_size
            ,height: tex_size
        }
    }
}

struct RenderData{
    shader: CubeShader
    ,image_shader: ImageShader
    ,cube_verts: u32
    ,image_verts: u32
    ,offset: affine::Transform<f32>
    ,offset_subface: affine::Transform<f32>
    ,faces: Vec<affine::Transform<f32>>
    ,subfaces: Vec<affine::Transform<f32>>
    ,window: ContextWrapper<PossiblyCurrent, glutin::window::Window>
    ,events_loop: RefCell<Option<glutin::event_loop::EventLoop<ToGUI>>>
    ,font: Font
    ,texture: u32
    ,cur: PhysicalPosition<f64>
    ,s_cur: PhysicalPosition<f64>
    ,pressed: bool
    ,released: bool
    ,buttons: RefCell<Vec<Button>>
    ,font_cache: RefCell<GlyphSheet>
}

fn init_render_data() -> RenderData{
    let events_loop = glutin::event_loop::EventLoop::with_user_event();
    let window = glutin::window::WindowBuilder::new()
        .with_title("Giant cube")
        .with_inner_size(glutin::dpi::PhysicalSize::new(1120,630));
    let context = glutin::ContextBuilder::new().with_vsync(true);
    let gl_window = unsafe {
        let win = context.build_windowed(window, &events_loop).unwrap().make_current().unwrap();
        gl::load_with(|s| win.get_proc_address(s) as *const _);
        win
    };

    let font = include_bytes!("../resources/MPLUSRounded1c-Regular.ttf") as &[u8];
    let font = Font::from_bytes(font, fontdue::FontSettings::default()).unwrap();

    let cube_shader = CubeShader::new();
    let image_shader = ImageShader::new();

    let gfx_objs = unsafe{ 

        let offset = affine::Transform::<f32>::translate(0.0,0.0,-0.5);
        // a square
        let vertices: [f32;16] = [
            -0.5, -0.5, 0.0, 1.0
            ,-0.5, 0.5, 0.0, 1.0
            ,0.5, 0.5, 0.0, 1.0
            ,0.5, -0.5, 0.0, 1.0
        ];

        let image_vert_array: [f32;16] = [
            0.0, 0.0, 0.0, 1.0
            ,1.0, 0.0, 1.0, 1.0
            ,1.0, 1.0, 1.0, 0.0
            ,0.0, 1.0, 0.0, 0.0
        ];

        // Build the cube by transforming the square into five different orientations (bottom is missing)
        use std::f32::consts::TAU;

        // Top front left back right
        let face_transforms = vec![
            affine::Transform::<f32>::rotate_xyz(TAU/4.0,0.0,0.0),
            affine::Transform::<f32>::none(),
            affine::Transform::<f32>::rotate_xyz(0.0,(TAU/4.0)*1.0,0.0),
            affine::Transform::<f32>::rotate_xyz(0.0,(TAU/4.0)*2.0,0.0),
            affine::Transform::<f32>::rotate_xyz(0.0,(TAU/4.0)*3.0,0.0),
        ];

        let subface_transforms = vec![
            affine::Transform::<f32>::translate(-0.33, 0.33, 0.0),
            affine::Transform::<f32>::translate(0.0, 0.33, 0.0),
            affine::Transform::<f32>::translate(0.33, 0.33, 0.0),
            affine::Transform::<f32>::translate(-0.33, 0.0, 0.0),
            affine::Transform::<f32>::none(),
            affine::Transform::<f32>::translate(0.33, 0.0, 0.0),
            affine::Transform::<f32>::translate(-0.33, -0.33, 0.0),
            affine::Transform::<f32>::translate(0.0, -0.33, 0.0),
            affine::Transform::<f32>::translate(0.33, -0.33, 0.0),
        ];

        let mut vbo = 0;
        let mut cube_verts = 0;

        let mut image_vbo = 0;
        let mut image_verts = 0;

        gl::GenVertexArrays(1, &mut cube_verts);
        gl::GenBuffers(1, &mut vbo);
        gl::GenVertexArrays(1, &mut image_verts);
        gl::GenBuffers(1, &mut image_vbo);

        gl::BindVertexArray(cube_verts);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (vertices.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
            &vertices[0] as *const f32 as *const c_void,
            gl::STATIC_DRAW,
        );

        gl::EnableVertexAttribArray(0);
        gl::VertexAttribPointer(
            0,
            4,
            gl::FLOAT,
            gl::FALSE,
            4 * mem::size_of::<GLfloat>() as GLsizei,
            ptr::null(),
        );


        gl::BindVertexArray(image_verts);
        gl::BindBuffer(gl::ARRAY_BUFFER, image_vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (image_vert_array.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
            &image_vert_array[0] as *const f32 as *const c_void,
            gl::STATIC_DRAW,
        );

        // position attribute
        gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, (mem::size_of::<GLfloat>() * 4) as GLsizei, ptr::null());
        gl::EnableVertexAttribArray(0);
        // texture coord attribute
        gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, (mem::size_of::<GLfloat>() * 4) as GLsizei, ptr::null::<c_void>().add(2*mem::size_of::<GLfloat>()));
        gl::EnableVertexAttribArray(1);

        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        gl::BindVertexArray(0);

        let mut texture = 0;
        gl::GenTextures(1, &mut texture);
    
        let offset_subface = &((&affine::Transform::<f32>::scale(1.01,1.01,1.01)) * &offset) * (&affine::Transform::<f32>::scale(0.3,0.3,0.3));

        let left = -1920.0/2.0;

        let scramble_button = Button::new(left + 10.0, 240.0, 540.0,110.0, "Scramble".to_string(), "scramble".to_string(), 80.0);
        let end_button = Button::new(left + 10.0, 100.0, 540.0,110.0, "Reset Cube".to_string(), "reset".to_string(), 80.0);

        let mut tex_size: i32 = 0;
        gl::GetIntegerv(gl::MAX_TEXTURE_SIZE, &mut tex_size as *mut i32);
        let tex_size = std::cmp::min(tex_size, 1000) as usize; //shouldn't need more than this
        RenderData{
            shader: cube_shader
            ,image_shader: image_shader
            ,cube_verts: cube_verts
            ,image_verts: image_verts
            ,offset: offset
            ,offset_subface: offset_subface
            ,faces: face_transforms
            ,subfaces: subface_transforms
            ,window: gl_window
            ,events_loop: RefCell::new(Some(events_loop))
            ,font: font
            ,texture: texture
            ,cur: PhysicalPosition{x:0.0,y:0.0}
            ,s_cur: PhysicalPosition{x:0.0,y:0.0}
            ,buttons: RefCell::new(vec![scramble_button, end_button])
            ,pressed: false
            ,released: false
            ,font_cache: RefCell::new(GlyphSheet::new(tex_size))
        }
    };
    gfx_objs
}

type Tf = affine::Transform<GLfloat>;

fn render_text(gfx: &RenderData, global_transform: &Tf, win_pix_transform: &Tf, s: &str, x: f32, y: f32, pt: f32, color: (f32,f32,f32)){
    unsafe{
        gl::Disable(gl::DEPTH_TEST); //text always on top
        gfx.image_shader.use_();

        // Blend textured glyphs (transparency)
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::ONE, gl::ONE_MINUS_SRC_ALPHA);

        // Set up texture
        gl::BindTexture(gl::TEXTURE_2D, gfx.texture);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_LOD, 0 as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAX_LOD, 0 as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
        gl::ActiveTexture(gl::TEXTURE0);
        // Set up image shader data
        gfx.image_shader.u_texture.set(0);
        gfx.image_shader.u_color.set(color.0, color.1, color.2, 1.0);
        gfx.image_shader.u_global_transform.set(&global_transform.data);
        gfx.image_shader.u_pix_transform.set(&win_pix_transform.data);
        //let scale = Tf::scale(gfx.font_scale, gfx.font_scale, gfx.font_scale);
        gfx.image_shader.u_scale.set(&win_pix_transform.data);

        gl::BindVertexArray(gfx.image_verts);

        use fontdue::layout;
        let mut l: layout::Layout<()> = layout::Layout::new(layout::CoordinateSystem::PositiveYUp);
        l.append(&[&gfx.font], &layout::TextStyle::new(s, pt, 0));
        for c in l.glyphs(){
            let mut glyphs = gfx.font_cache.borrow_mut();
            let mut location: Option<(usize, usize, usize, usize)> = glyphs.locations.get(&c.key).cloned();
            if location.is_none() {
                let (metrics, bitmap) = gfx.font.rasterize_config(c.key);
                // If this would overflow the line, go to the next line down
                if glyphs.pos_x + metrics.width > glyphs.width{
                    glyphs.pos_y = glyphs.max_used_y + 2;
                    glyphs.pos_x = 0;
                }
                // Now we know where this character goes, cache the coords
                let a_location = (glyphs.pos_x, glyphs.pos_y, metrics.width, metrics.height);
                location = Some(a_location);
                glyphs.locations.insert(c.key, a_location);
                
                glyphs.max_used_y = std::cmp::max(glyphs.max_used_y, glyphs.pos_y + metrics.height);

                //println!("Cache-render {} at {},{} with size {},{}", c.parent, gfx.fmap_pos_x, gfx.fmap_pos_y, metrics.width, metrics.height);

                // Copy the glyph bitmap to the main bitmap sheet
                for y in 0..metrics.height{
                    for x in 0..metrics.width{
                        let p = bitmap[(y*metrics.width) + x];
                        let t = ((glyphs.pos_y + y) * glyphs.width) + (glyphs.pos_x + x);
                        glyphs.bitmap[t] = p;
                    }
                }

                glyphs.pos_x += metrics.width + 1;
                gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RED as i32, glyphs.width as i32, glyphs.height as i32, 0, gl::RED, gl::UNSIGNED_BYTE, glyphs.bitmap.as_ptr() as *const c_void);
            }
            let image_geom = Tf::scale(c.width as f32, c.height as f32, 0.0);
            let image_translate = Tf::translate(c.x + x, c.y + y,0.0) ;
            gfx.image_shader.u_image_geom.set(&image_geom.data);
            gfx.image_shader.u_translate.set(&image_translate.data);
            #[cfg(feature="gles")]{
                gfx.image_shader.u_tex_size.set(glyphs.width as f32, glyphs.height as f32);
            }
            if let Some((gx,gy,gw,gh)) = location{
                gfx.image_shader.u_glyph_select.set(gx as f32, gy as f32, gw as f32, gh as f32);
            }
            gl::DrawArrays(gl::TRIANGLE_FAN, 0, 4);
        }
    }
}

fn render_button(gfx: &RenderData, global_transform: &Tf, win_pix_transform: &Tf, s: &str, x: f32, y: f32, width: f32, height: f32, pt: f32, text_color: (f32,f32,f32), clicked: bool) -> bool{
    let hover = unsafe {
        gl::Disable(gl::DEPTH_TEST); //text always on top
        gfx.image_shader.use_();
        gl::Disable(gl::BLEND);

        // Set up texture
        gl::BindTexture(gl::TEXTURE_2D, 0);
        gl::ActiveTexture(gl::TEXTURE0);
        // Set up image shader data
        gfx.image_shader.u_texture.set(0);
        gfx.image_shader.u_global_transform.set(&global_transform.data);
        gfx.image_shader.u_pix_transform.set(&win_pix_transform.data);
        gfx.image_shader.u_scale.set(&win_pix_transform.data);

        gl::BindVertexArray(gfx.image_verts);

        let image_geom = Tf::scale(width, height, 1.0);
        let image_translate = Tf::translate(x, y-height,0.0) ;
        gfx.image_shader.u_image_geom.set(&image_geom.data);
        gfx.image_shader.u_translate.set(&image_translate.data);
        if clicked { 
            gfx.image_shader.u_color.set(1.0,1.0,1.0, 0.0);
        }
        else{
            gfx.image_shader.u_color.set(0.3,0.3,0.3, 0.0);
        }
        #[cfg(feature="gles")] {
            gfx.image_shader.u_tex_size.set(1.0, 1.0);
        }
        gl::DrawArrays(gl::TRIANGLE_FAN, 0, 4);
        let image_geom = Tf::scale(width-20.0, height-20.0, 1.0);
        let image_translate = Tf::translate(x+10.0, (y-height)+10.0,0.0) ;
        gfx.image_shader.u_image_geom.set(&image_geom.data);
        gfx.image_shader.u_translate.set(&image_translate.data);
        let (cx, cy) = (gfx.s_cur.x as f32, gfx.s_cur.y as f32);
        let hover = cx > x && cx < (x + width) && cy < y && cy > (y - height);
        if hover{ 
            gfx.image_shader.u_color.set(0.9,1.0,0.9, 0.0);
        }
        else{
            gfx.image_shader.u_color.set(0.5,1.0,0.5, 0.0);
        }
        #[cfg(feature="gles")] {
            gfx.image_shader.u_tex_size.set(1.0, 1.0);
        }
        gl::DrawArrays(gl::TRIANGLE_FAN, 0, 4);
        hover
    };
    render_text(gfx, global_transform, win_pix_transform, s, x+20.0, y-0.0, pt, text_color);
    hover
}

impl Button{
    fn render(&self, gfx: &RenderData, global_transform: &Tf, win_pix_transform: &Tf) -> bool{
        render_button(gfx, global_transform, win_pix_transform, &self.label, self.x, self.y, self.w, self.h, self.pt, (0.0,0.0,0.0), self.click_state == 1)
    }
}

fn format_time(d:Duration) -> String {
    let total = d.as_secs_f64();
    let mins = (total / 60.0).floor();
    if mins < 1.0 {
        format!("{:.3}s", total)
    }
    else {
        format!("{}:{:06.3}", mins, total - (mins * 60.0))
    }
}

fn ui_loop(mut gfx: RenderData, state: Arc<Mutex<ClientState>>, sender: Sender<FromGUI>, receiver: Receiver<ToGUI>){

    let mut data = DataModel{
        d:0.0
        ,r:0.0
        ,diff: 0.0
        ,frames:0
    };

    fn update(data: &mut DataModel){
        data.d += data.diff;
        data.r += data.diff.abs()/2.0;
        data.r %= 1.0;
        data.frames += 1;
        if data.d >= 1.0 {data.diff = -0.01;}
        if data.d <= 0.0 {data.diff = 0.01;}
    }


    fn draw(data: &mut DataModel, mut gfx: &mut RenderData, state: &Arc<Mutex<ClientState>>, sender: &Sender<FromGUI>){
        let state = state.lock().unwrap();
        let sz = gfx.window.window().inner_size();
        let ww = sz.width as f32;
        let wh = sz.height as f32;
        const RATIO: f32 = 16.0/9.0; // Y ranges from -1.0 to +1.0, X ranges from -RATIO to +RATIO

        let global_transform = if ww < (wh * RATIO){
            Tf::scale(1.0/RATIO, ww / (wh * RATIO),1.0)
        }
        else{
            Tf::scale((wh / (ww / RATIO))/RATIO,1.0,1.0)
        };

        const FW:f32  = 1920.0;
        const FH:f32  = 1080.0;

        // Calculate the transform to 1:1 window pixel scale, applied after global transform, pretend the screen is 1920x1080
        let pscale = if ww < (wh * RATIO) {
             //(1.0*RATIO)/ww // absolute pixel size (items on screen maintain absoluse size as window scales)
             (1.0*RATIO)/(FW/2.0) // fixed "fake" screen size, scaled to fit. (whole window image is scaled)
        }
        else{
             //1.0/wh
             2.0/FH
        };
        let win_pix_transform = Tf::scale(pscale, pscale, 1.0);
        gfx.s_cur = PhysicalPosition{
            x: gfx.cur.x / ww as f64 * FW as f64
            ,y: -gfx.cur.y / wh as f64 * FH as f64
        };
        if ww < (wh * RATIO){
            gfx.s_cur.y /= (ww / (wh * RATIO)) as f64;
        }
        else{
            gfx.s_cur.x /= ((wh / (ww / RATIO))) as f64;
        }

        
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::ClearColor(data.d, 0.58, 0.92, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            gfx.shader.use_();
            gfx.shader.u_global_transform.set(&global_transform.data);
            gl::BindVertexArray(gfx.cube_verts);
            let transform = Tf::rotate_xyz((3.14*2.0)/16.0, (3.14*2.0)*(data.r as f32), 0.0);
            gfx.shader.u_transform.set(&transform.data);

            for i in 0..5{
                gfx.shader.u_offset.set(&gfx.offset.data);
                gfx.shader.u_color.set(0.0,0.0,0.0);
                gfx.shader.u_face_transform.set(&gfx.faces[i].data);
                gl::DrawArrays(gl::TRIANGLE_FAN, 0, 4);
                gfx.shader.u_offset.set(&gfx.offset_subface.data);
                let f = &state.cube.faces[i];
                for j in 0..9{
                    let col = f.subfaces[j].color;
                    //println!("{:?}", col);
                    match col {
                        cube::Colors::Red => gfx.shader.u_color.set(1.0,0.0,0.0),
                        cube::Colors::Green => gfx.shader.u_color.set(0.0,1.0,0.0),
                        cube::Colors::Orange => gfx.shader.u_color.set(1.0,0.6,0.2),
                        cube::Colors::Blue => gfx.shader.u_color.set(0.0,0.0,1.0),
                        cube::Colors::White => gfx.shader.u_color.set(1.0,1.0,1.0),
                        cube::Colors::Yellow => gfx.shader.u_color.set(1.0,1.0,0.0),
                        cube::Colors::Blank => gfx.shader.u_color.set(0.0,0.0,0.0),
                    }
                    let sft = &gfx.subfaces[j];
                    gfx.shader.u_face_transform.set(&((&gfx.faces[i] * sft).data));
                    gl::DrawArrays(gl::TRIANGLE_FAN, 0, 4);
                }
            }
            gfx.shader.u_color.set(1.0,1.0,1.0);
            gfx.shader.u_offset.set(&gfx.offset.data);
            for i in 0..5{
                gfx.shader.u_face_transform.set(&gfx.faces[i].data);
                gl::DrawArrays(gl::LINE_LOOP, 0, 4);
            }

            let mut text = |s, x, y, pt, col|{
                render_text(&mut gfx, &global_transform, &win_pix_transform, s, x, y, pt, col);
            };
            let mut black_text = |s, x, y, pt|{
                text(s,x,y,pt,(0.0,0.0,0.0));
            };
            black_text("Giant Cube!", -1920.0/2.0, 1080.0/2.0, 150.0);
            black_text("⇩click to play⇩", -1920.0/2.0, 350.0, 70.0);
            let now = Instant::now();

            let timer_msg = if !state.timer_state.is_started() {
                "Ready to start".to_string()
            }
            else if state.timer_state.is_inspecting(Some(now)) {
                format!("Inspection: {}s", 15 - state.timer_state.inspection_so_far(Some(now)).as_secs())
            }
            else{
                if state.timer_state.is_ended() {
                    let flash = (data.frames % 30) > 10;
                    if flash {
                        format_time(state.timer_state.solve_so_far())
                    }
                    else {
                        "".to_string()
                    }
                }
                else {
                    format_time(state.timer_state.solve_so_far())
                }
            };
            black_text(&timer_msg, -1920.0/2.0, (-1080.0/2.0)+250.0, 170.0);
            if state.record_time > 0 {
                black_text(&format!("Current\nRecord:\n{}", format_time(Duration::from_millis(state.record_time.try_into().unwrap_or(0)))), 1920.0/2.0 - 500.0, 1080.0/2.0, 100.0);
            }
            let mut do_hover = false;
            for button in &mut*gfx.buttons.borrow_mut(){
                let hover = button.render(&gfx, &global_transform, &win_pix_transform);
                if hover{
                    do_hover = true;
                    if gfx.pressed{
                        button.click_state = 1;
                    }
                    if gfx.released && button.click_state == 1{
                        button.click_state = 2;
                    }
                }
            }
            use glutin::window::CursorIcon;
            if do_hover{
                gfx.window.window().set_cursor_icon(CursorIcon::Hand);
            }
            else{
                gfx.window.window().set_cursor_icon(CursorIcon::Default);
            }
            for button in &mut*gfx.buttons.borrow_mut(){
                if button.click_state == 1 && gfx.released{
                    button.click_state = 0;
                }
                if button.click_state == 2{
                    button.click_state = 0;
                    use client::FromGUI::*;
                    match button.id.as_ref(){
                        "scramble" => {
                            sender.send(StartGame());
                        }
                        ,"reset" => {
                            sender.send(CancelTimer());
                            sender.send(SetState(Cube::new()));
                        }
                        ,_=>{}
                    }
                }
            }
            gfx.pressed = false;
            gfx.released = false;
            // Cursor pos debug
            //black_text("X", gfx.s_cur.x as f32 -35.0, gfx.s_cur.y as f32+35.0, 70.0);
        }
        gfx.window.swap_buffers().unwrap();
    }

    let target_fps = 30.0;
    let frame_dur_ms: f32 = 1000.0/target_fps;

    let mut last_frame_start = Instant::now();

    let frame_duration = Duration::from_millis(frame_dur_ms.floor() as u64);

    use glutin::event::{Event, WindowEvent};
    use glutin::event_loop::ControlFlow;
    let events_loop = gfx.events_loop.take().unwrap();

    let proxy = events_loop.create_proxy();

    let _client_event_thread = thread::spawn(move||{
        for ev in receiver{
            let _ignored = proxy.send_event(ev);
        }
    });

    events_loop.run(move |event, _win_target, cf|
        match event {
            Event::WindowEvent{ event: ev,..} => {
                match ev {
                    WindowEvent::CloseRequested => {*cf = ControlFlow::Exit;}
                    ,WindowEvent::Resized(newsize) => {gfx.window.resize(newsize);}
                    ,WindowEvent::CursorMoved{ position: p,.. } => {
                        let sz = gfx.window.window().inner_size();
                        let ww = sz.width as f64;
                        let wh = sz.height as f64;
                        gfx.cur = PhysicalPosition::<f64>{x: p.x - (ww/2.0), y: p.y - (wh/2.0)};
                    }
                    ,WindowEvent::MouseInput{button:b, state:s, ..} => {
                        gfx.pressed |= b == MouseButton::Left && s == ElementState::Pressed;
                        gfx.released |= b == MouseButton::Left && s == ElementState::Released;
                    }
                    ,_=>{}
                }
            }
            ,Event::RedrawRequested(_win) => {
                draw(&mut data, &mut gfx, &state, &sender);
            }
            ,Event::RedrawEventsCleared => {
                let start = Instant::now();
                update(&mut data);
                draw(&mut data, &mut gfx, &state, &sender);
                *cf = ControlFlow::WaitUntil(last_frame_start+frame_duration); last_frame_start = start;
            }
            ,Event::UserEvent(client_ev) => {
                use ToGUI::*;
                match client_ev {
                    StateUpdate() => { println!("state update"); }
                    ,GameEnd() => { println!("game end"); }
                    ,Connected(b) => {
                        if b {
                           sender.send(FromGUI::GetState());
                        }
                    }
                    ,MissingConnection() => { println!("missing connection"); }
                }
            }
            ,_ => {}
        }
    );

}

fn main() {
    let gfx = init_render_data();

    let (state, sender, receiver, _client) = start_client();

    let secret = b"secret".to_vec(); // TODO load from file
    let addr = "localhost:9876".to_string(); // TODO load from tile

    use client::FromGUI::*;
    sender.send(Connect(secret, addr));
    ui_loop(gfx, state, sender, receiver);
}
