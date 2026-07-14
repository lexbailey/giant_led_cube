#version 330 core
in vec2 px_pos;
uniform float u_facelet_px;
uniform float u_global_time;
uniform float u_anim_pos;
uniform uint u_prev_colours[54];
uniform uint u_cur_colours[54];
uniform vec3 u_base_cols[7];
uniform uint u_twist_face;
uniform float u_twist_dir;
uniform uint u_debug_arrow;
uniform uint u_style;
out vec4 FragColor;
layout(location=0) out vec3 tFragColor;

const float PI = 3.1415;

float bulge(float a){
    if (a < -0.5){return 0.0;}
    if (a > 0.5){return 0.0;}
    a = a*2.0;
    return 1.0-a*a*a*a;
}

mat3 translate(float x, float y){
    return mat3(
        1.0,0.0,x,
        0.0,1.0,y,
        0.0,0.0,1.0
    );
}

mat4 translate4(float x, float y, float z){
    return mat4(
        1.0,0.0,0.0,x,
        0.0,1.0,0.0,y,
        0.0,0.0,1.0,z,
        0.0,0.0,0.0,1.0
    );
}

mat2 rotate2(float t){
     float ct = cos(t);
     float st = sin(t);
     return mat2(
         ct, -st,
         st, ct
     );
}

mat3 rotate3(float a){
    return mat3(
        cos(a), -sin(a),0.0,
        sin(a), cos(a),0.0,
        0.0,0.0,1.0
    );
}

mat4 rotate4(float x, float y, float z){
    return mat4(
        cos(z), -sin(z),0.0,0.0,
        sin(z), cos(z),0.0,0.0,
        0.0,0.0,1.0,0.0,
        0.0,0.0,0.0,1.0
    )*
    mat4(
        cos(y), 0.0, sin(y),0.0,
        0.0,1.0,0.0,0.0,
        -sin(y), cos(y),1.0,0.0,
        0.0,0.0,0.0,1.0
    )*
    mat4(
        1.0,0.0,0.0,0.0,
        0.0,cos(x), -sin(x),0.0,
        0.0,sin(x), cos(x),0.0,
        0.0,0.0,0.0,1.0
    );
}

vec4 arrowhead(vec2 fl_coord, vec3 base, uint rot){
    vec2 cfl = fl_coord * rotate2((rot+1u)*PI/2);
    float x = cfl.x;
    float y = cfl.y;
    float ax = abs(cfl.x);
    float ay = abs(cfl.y);
    vec3 c = vec3(0.0,0.0,0.0);
    float a = 1.0;
    if ( y > 0.25 && y < 0.45 && ax < 0.45-y){
        c += base;
        a = 0.0;
    }
    return vec4(c,a);
}


// ===============================================
float logi(float x, float L, float k){
  float ex = -k*x;
  return L / (1+(exp(ex)));
}

vec3 red_spot(vec3 col, vec2 centre, vec2 uv, float edge, vec2 scale){
    vec2 d1 = centre - (uv);
    d1.y*=1.6;
    float d1m = sqrt((d1.x*d1.x) + (d1.y*d1.y));
    float l = logi(((edge*1.3)-d1m)/1.0, 1, 100);
    float a = (1.0-(l*0.3));
    return vec3(
      max(l,col.r),
      col.g*a,
      col.b*a
    );
}

vec3 bands(float f, vec2 spot_centre, vec2 uv, float spot_radius, vec2 scale){
     const int NUM_COLS = 13;
     vec4 cols[NUM_COLS];
     cols[0] = vec4(0.49,0.42,0.31,    0.0);
     cols[1] = vec4(0.42,0.34,0.20,    0.13);
     cols[2] = vec4(0.525,0.482,0.369, 0.25);
     cols[3] = vec4(0.74,0.74,0.74,    0.28);
     cols[4] = vec4(0.525,0.482,0.369, 0.32);
     cols[5] = vec4(0.74,0.74,0.74,    0.34);
     cols[6] = vec4(0.78,0.506,0.427,  0.38);
     cols[7] = vec4(0.945,0.702,0.447, 0.5);
     cols[8] = vec4(0.74,0.74,0.74,    0.6);
     cols[9] = vec4(0.745,0.537,0.431, 0.68);
     cols[10] = vec4(0.74,0.74,0.74,    0.75);
     cols[11] = vec4(0.525,0.482,0.369,1.0);
     cols[12] = vec4(0.525,0.482,0.369,2.0);
     int c1 = 0;
     int c2 = 1;
     for (int i = 0; i < NUM_COLS-1; i++){
         if (f >= cols[i].a && f <= cols[i+1].a){
             c1 = i;
             c2 = i+1;
         }
     }
     float d = cols[c2].a - cols[c1].a;
     float p = f - cols[c1].a;
     float c2m = p/d;
     float c1m = 1.0-c2m;
     return red_spot( ((cols[c1] * c1m) +
      (cols[c2] * c2m)).rgb, spot_centre, uv, spot_radius*1.2, scale);
}


vec2 rot(vec2 a, float t){
     float ct = cos(t);
     float st = sin(t);
     return a * mat2(
         ct, -st,
         st, ct
     );
}

vec2 swirl(vec2 centre, vec2 uv, float factor, float edge){
    vec2 d1 = centre - uv;
    float d1m = sqrt((d1.x*d1.x) + (d1.y*d1.y));
    float i = ((3.141/2)-d1m);
    if (i<0){i=0;}
    i=tan(clamp(i,0.0,3.141/2))*0.1;
    float l = logi((edge-d1m)/1.0, 1, 100);
    uv = rot(d1,(3.141592)+(i*factor*l))+centre;
    return uv;
}

float chaos(float x){  
  return mod(
  sin(x)+
  sin(3.3*x)+
  sin(5.8*x)+
  sin(2.5*x)+
  sin(30*x),1.0);
}

float bchaos(float x){  
  return (
  sin(x)+
  sin(3.3*x)+
  sin(5.8*x)+
  sin(2.5*x)+
  sin(30*x)
  )/5.0;
}

vec2 anoise2(float x){
  return vec2(chaos(x), chaos(chaos(x)*200.0));
}

vec3 fuzzy(vec3 c, vec2 uv){
  // TODO interesting filter
  return vec3(
    c.r,
    c.g,
    c.b
  );
}

vec4 jupiter(vec2 uv){
    float edgelen = u_facelet_px * 3;
    vec2 iResolution = vec2(edgelen, edgelen);
    float iTime = u_global_time;
    vec4 fragColor = vec4(1,1,1,1);
    uv -= vec2(0.5,0.5);
    vec2 uv_noswirl = uv;
    float d = uv.x * uv.x + uv.y * uv.y;
    vec3 col = vec3(1,0.63,0); // Main jupiter background colour
    vec2 spot_centre = vec2(clamp(-mod(iTime*0.1,2)+1, -0.7,0.7),-0.14);
    float spot_radius = 0.1;
    vec2 scale = vec2(1,1);
    uv.y += bchaos((iTime*0.1)+uv.x)*0.02;
    uv = swirl(spot_centre,uv,1+(sin(iTime)*0.3),spot_radius);
    for (int i = 0; i< 100; i++){
      vec2 c = anoise2(float(i))-vec2(0.5,0.5);
      c.x*=2;
      c.x = mod(c.x-(iTime*0.2),2.0)-1;
      uv = swirl(c,uv,(sin(iTime+float(i))*0.5),abs(chaos(i))*0.05);
    }
    float p = 1.0-(uv.y+0.5);
    if (d < 0.25){
         col = bands(p, spot_centre, uv, spot_radius,scale);
    }
     
    col = fuzzy(col,uv_noswirl);
    fragColor = vec4(col,1.0);
    return fragColor;
}

//==========================================================

vec4 unseen_area(vec2 px_pos){
    vec4 col = vec4(0,0,0,1);
    vec2 grid = px_pos / 20;
    if (mod(grid.x + floor(mod(grid.y, 2)), 2) < 1){
        col = vec4(1.0,0.0,1.0, 1.0);
    }
    return col;
}

bool is_adjacent(uint f1, uint f2){
    if (f1 == 0u) {return f2 != 5u;}
    if (f1 == 1u) {return f2 != 3u;}
    if (f1 == 2u) {return f2 != 4u;}
    if (f1 == 3u) {return f2 != 1u;}
    if (f1 == 4u) {return f2 != 2u;}
    if (f1 == 5u) {return f2 != 0u;}
}

int should_slide_x(uint face, int x){
    if (face == 0u && u_twist_face == 2u && x == 0) {return 1;}
    if (face == 0u && u_twist_face == 4u && x == 2) {return -1;}
    if (face == 1u && u_twist_face == 2u && x == 0) {return 1;}
    if (face == 1u && u_twist_face == 4u && x == 2) {return -1;}
    if (face == 2u && u_twist_face == 3u && x == 0) {return 1;}
    if (face == 2u && u_twist_face == 1u && x == 2) {return -1;}
    if (face == 3u && u_twist_face == 4u && x == 0) {return 1;}
    if (face == 3u && u_twist_face == 2u && x == 2) {return -1;}
    if (face == 4u && u_twist_face == 1u && x == 0) {return 1;}
    if (face == 4u && u_twist_face == 3u && x == 2) {return -1;}

    if (face == 0u && u_twist_face == 7u && x == 1) {return 1;}

    if (face == 1u && u_twist_face == 7u && x == 1) {return 1;}
    if (face == 2u && u_twist_face == 6u && x == 1) {return -1;}
    if (face == 3u && u_twist_face == 7u && x == 1) {return -1;}
    if (face == 4u && u_twist_face == 6u && x == 1) {return 1;}

    return 0;
}

int should_slide_y(uint face, int y){
    if (face == 0u && u_twist_face == 3u && y == 0) {return -1;}
    if (face == 0u && u_twist_face == 1u && y == 2) {return 1;}
    if (face == 1u && u_twist_face == 0u && y == 0) {return -1;}
    if (face == 1u && u_twist_face == 5u && y == 2) {return 1;}
    if (face == 2u && u_twist_face == 0u && y == 0) {return -1;}
    if (face == 2u && u_twist_face == 5u && y == 2) {return 1;}
    if (face == 3u && u_twist_face == 0u && y == 0) {return -1;}
    if (face == 3u && u_twist_face == 5u && y == 2) {return 1;}
    if (face == 4u && u_twist_face == 0u && y == 0) {return -1;}
    if (face == 4u && u_twist_face == 5u && y == 2) {return 1;}

    if (face == 0u && u_twist_face == 6u && y == 1) {return 1;}


    if (face == 1u && u_twist_face == 8u && y == 1) {return 1;}
    if (face == 2u && u_twist_face == 8u && y == 1) {return 1;}
    if (face == 3u && u_twist_face == 8u && y == 1) {return 1;}
    if (face == 4u && u_twist_face == 8u && y == 1) {return 1;}
    return 0;
}

vec3 render_subface(vec2 fl, vec3 base_col, uint f, uint sf){
     vec3 c = base_col * bulge(fl.x) * bulge(fl.y);
     bool mask = (fl.x > -0.5 && fl.x < 0.5) && (fl.y > -0.5 && fl.y < 0.5);
     bool is_centre = sf == 4u;
     bool is_edge = sf == 1u || sf == 3u || sf == 5u || sf == 7u;
     bool is_corner = sf == 0u || sf == 2u || sf == 6u || sf == 8u;
     uint rot = 0u;
     if (sf == 1u) {rot += 3u;}
     if (sf == 2u) {rot += 3u;}
     if (sf == 3u) {rot += 0u;}

     if (sf == 5u) {rot += 2u;}
     if (sf == 6u) {rot += 1u;}
     if (sf == 7u) {rot += 1u;}
     if (sf == 8u) {rot += 2u;}
     if (u_debug_arrow > 0u && !is_centre){
         vec3 cfl = vec3(fl,1.0);
         cfl *= rotate3((rot+1u)*PI/2);
         if (is_corner){
             cfl *= rotate3(-PI/4);
         }
         float x = cfl.x;
         float y = cfl.y;
         float ax = abs(cfl.x);
         float ay = abs(cfl.y);
         if (
             (ax < 0.1 && ay < 0.35)
             || (y > 0.25 && y < 0.45 && ax < 0.45-y)
         ){
             c += vec3(1.0,-1.0,1.0);
         }
     }
     if (is_centre && (u_debug_arrow > 0u)){
         vec2 ca = fl;
         vec3 edge_base1 = u_base_cols[u_cur_colours[f*9u + 5u]];
         vec3 edge_base2 = u_base_cols[u_cur_colours[f*9u + 1u]];
         vec3 edge_base3 = u_base_cols[u_cur_colours[f*9u + 3u]];
         vec3 edge_base4 = u_base_cols[u_cur_colours[f*9u + 7u]];
         vec4 ah1 = arrowhead(ca, edge_base1, 0u);
         vec4 ah2 = arrowhead(ca, edge_base2, 1u);
         vec4 ah3 = arrowhead(ca, edge_base3, 2u);
         vec4 ah4 = arrowhead(ca, edge_base4, 3u);
         float mask = ah1.a * ah2.a * ah3.a * ah4.a;
         vec3 arrows = vec3(ah1.rgb+ah2.rgb+ah3.rgb+ah4.rgb);
         c = (c * mask) + arrows;
     }
     return c;
}

vec4 render_face(vec2 ffc, uint face){
    vec3 o = vec3(0,0,0);
    for (int x = 0; x < 3; x++){
        int slide_x = should_slide_x(face, x);
        for (int y = 0; y < 3; y++){
            int slide_y = should_slide_y(face, y);
            vec2 ffc1 = ffc;
            vec2 ffc2 = ffc;
            if (slide_x != 0){
                ffc1 = ffc + (vec2(0.0,-1.0+u_anim_pos) * -u_twist_dir * slide_x);
                ffc2 = ffc + (vec2(0.0,u_anim_pos) * -u_twist_dir * slide_x);
            }
            if (slide_y != 0){
                ffc1 = ffc + (vec2(-1.0+u_anim_pos,0.0) * -u_twist_dir * slide_y);
                ffc2 = ffc + (vec2(u_anim_pos,0.0) * -u_twist_dir * slide_y);
            }
            uint sf = uint(y*3+x);
            vec2 p = vec2(0-(x-1),0-(y-1));
            uint f_id = face * 9u + sf;

            vec2 facelet1 = (ffc1 * 3) + p;
            vec3 base1 = u_base_cols[u_cur_colours[f_id]];
            o += render_subface(facelet1, base1, face, sf);
            if (slide_x != 0 || slide_y != 0){
                vec2 facelet2 = (ffc2 * 3) + p;
                vec3 base2 = u_base_cols[u_prev_colours[f_id]];
                o += render_subface(facelet2, base2, face, sf);
            }
        }
    }
    return vec4(o, 1.0);
}

void main() {
    float fp = u_facelet_px;
    float face_pixels = u_facelet_px * 3;;
    // get tile coordinates for this tile
    int ix = int(floor(px_pos.x / face_pixels));
    int iy = int(floor(px_pos.y / face_pixels));
    uint sfx = uint(ix) % 3u;
    uint sfy = uint(iy) % 3u;
    // Determine which face this is on, if any
    uint f = 999u;
    if (ix == 1){ // middle column
        f = 0u;                 // top
        if (iy == 0){ f = 3u; } // back
        if (iy == 2){ f = 1u; } // front
    }
    else{ // middle row
        if (iy == 1) {
            f = 2u;                // left
            if (ix == 2){ f = 4u; } // right
            if (ix == 3){ f = 5u; } // bottom
        }
    }

    bool should_render = f < 999u;
    if (!should_render){
        FragColor = unseen_area(px_pos);
    }
    else{
        bool this_face = f == u_twist_face;
        vec2 from_face_centre = vec2(px_pos.x - (ix*face_pixels), px_pos.y - (iy*face_pixels)) / face_pixels - vec2(0.5,0.5);
        float angle = 0;
        if (f == 2u) {angle += 3*PI/2;}
        if (f == 4u) {angle += PI/2;}
        if (f == 3u) {angle += PI;}
        if (this_face && u_anim_pos < 1.0){
            angle += (u_twist_dir*PI/2) - (u_anim_pos * u_twist_dir * PI/2);
        }
        mat2 rotation = rotate2(angle);
        from_face_centre *= rotation;
        FragColor = render_face(from_face_centre, f);
    }
    tFragColor = FragColor.rgb;
}
