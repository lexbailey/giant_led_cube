#version 330 core
in vec2 px_pos;
uniform float u_facelet_px;
uniform float u_global_time;
uniform float u_anim_pos;
uniform uint u_prev_colours[54];
uniform uint u_cur_colours[54];
uniform uint u_mapping[45];
uniform uint u_rotation_map[45];
uniform uint u_map_facenum[45];
uniform uint u_map_subfacenum[45];
uniform uint u_inverse_facemap[45];
uniform uint u_adjacent[45];
uniform usamplerBuffer u_data_table;
uniform vec3 u_base_cols[7];
uniform uint u_twist_face;
uniform float u_twist_dir;
uniform uint u_debug_arrow;
uniform uint u_anim_style;
out vec4 FragColor;
layout(location=0) out vec3 tFragColor;

const float PI = 3.1415;

float bulge(float a){
    if (a < -0.5){return 0.0;}
    if (a > 0.5){return 0.0;}
    a = a*2.0;
    return 1.0-a*a*a*a;
}

float bulge2(float a){
    if (a < -0.5){return 0.0;}
    if (a > 0.5){return 0.0;}
    a = a*2.0;
    return 1.0-a*a;
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

mat3 rotate(float a){
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

uint get_index(uint f, uint sf){
    uint i = (f*9u)+sf;
    return u_inverse_facemap[i];
}

vec4 arrowhead(vec2 fl_coord, vec3 base, uint rot){
    vec3 cfl = vec3(fl_coord,1.0);
    cfl *= rotate((rot+1u)*PI/2);
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


vec4 render_tile(uint face_id, uint sf_id, uint m_face, vec2 fl_coord, vec3 base, uint rot, bool is_centre, bool is_edge, bool is_corner){
    vec3 c = vec3(0.0,0.0,0.0);
    bool mask = (fl_coord.x > -0.5 && fl_coord.x < 0.5) && (fl_coord.y > -0.5 && fl_coord.y < 0.5);
    //if (face_id == 5u){
    //    if (mask){
    //        vec2 uv = fl_coord;
    //        mat3 r = rotate(PI/2);
    //        uv.y = 0.0-uv.y;
    //        uv = (vec3(uv,1.0) * r).xy;
    //        uv += vec2(0.5,0.5);
    //        c = jupiter(uv).rgb * bulge(fl_coord.x) * bulge(fl_coord.y);;
    //    }
    //}
    //else{
        c = base * bulge(fl_coord.x) * bulge(fl_coord.y);
    //}

    if (u_debug_arrow > 0u && !is_centre){
        vec3 cfl = vec3(fl_coord,1.0);
        cfl *= rotate((rot+1u)*PI/2);
        if (is_corner){
            cfl *= rotate(-PI/4);
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
    return vec4(c,1.0);
}

vec4 render_tile_centre(uint face_id, uint sf_id, uint m_face, vec2 fl_coord, vec3 base, uint rot){ return render_tile(face_id, sf_id, m_face, fl_coord, base, rot, true, false, false); }
vec4 render_tile_edge(uint face_id, uint sf_id, uint m_face, vec2 fl_coord, vec3 base, uint rot){ return render_tile(face_id, sf_id, m_face, fl_coord, base, rot, false, true, false); }
vec4 render_tile_corner(uint face_id, uint sf_id, uint m_face, vec2 fl_coord, vec3 base, uint rot){ return render_tile(face_id, sf_id, m_face, fl_coord, base, rot, false, false, true); }

bool adjacent(uint f1, uint sf1 ,uint f2){
    return (u_adjacent[(f1*9u)+sf1] & (1u<<f2)) > 0u;
}

vec4 unseen_area(vec2 px_pos){
    vec4 col = vec4(0,0,0,1);
    vec2 grid = px_pos / 20;
    if (mod(grid.x + floor(mod(grid.y, 2)), 2) < 1){
        col = vec4(1.0,0.0,1.0, 1.0);
    }
    return col;
}

void main() {
    uint sf_cw[9];
    uint sf_ccw[9];
    sf_cw[0] = 1u;
    sf_cw[1] = 2u;
    sf_cw[2] = 5u;
    sf_cw[3] = 0u;
    sf_cw[4] = 4u;
    sf_cw[5] = 8u;
    sf_cw[6] = 3u;
    sf_cw[7] = 6u;
    sf_cw[8] = 7u;
    sf_ccw[0] = 3u;
    sf_ccw[1] = 0u;
    sf_ccw[2] = 1u;
    sf_ccw[3] = 6u;
    sf_ccw[4] = 4u;
    sf_ccw[5] = 2u;
    sf_ccw[6] = 7u;
    sf_ccw[7] = 8u;
    sf_ccw[8] = 5u;
    float fp = u_facelet_px;
    // get tile coordinates for this tile
    int ix = int(floor(px_pos.x / fp));
    int iy = int(floor(px_pos.y / fp));
    // Determine which face this is on, if any
    uint f = 999u;
    if ((ix >= 3) && (ix < 6)){ // middle column
        f = 0u;                 // top
        if (iy <  3){ f = 3u; } // back
        if (iy >= 6){ f = 1u; } // front
    }
    else{
        if ((iy > 2) && (iy < 6)) {
            f = 2u;                // left
            if (ix > 5){ f = 4u; } // right
            if (ix > 8){ f = 5u; } // bottom
        }
    }
    //bool should_render = f >= 0 && f != 5; // if not on a face, nothing to do. Bottom face is also skipped
    bool should_render = f < 999u; // alternatively, when debugging, this might be useful
    if (!should_render){
        FragColor = unseen_area(px_pos);
    }
    else{
        uint sf = 0u;
        vec2 fl_coord = vec2(px_pos.x - (ix*u_facelet_px), px_pos.y - (iy*u_facelet_px)) / u_facelet_px;
        uint i = (f * 9u) + sf;
        FragColor = render_tile(f, sf, f, fl_coord, u_base_cols[u_cur_colours[i]], 0u,true,false,false);
        //if (i < 45 && px_pos.y > 0.0 && px_pos.x > 0.0 && px_pos.y < (fp*5) && px_pos.x < (fp*9)){
        //    uint f = u_map_facenum[i];
        //    uint sf = u_map_subfacenum[i];
        //    uint sf_rot = u_rotation_map[i];
        //    bool this_face = f == u_twist_face;
        //    bool is_centre = sf == 4u;
        //    bool is_edge = sf == 1u || sf == 3u || sf == 5u || sf == 7u;
        //    bool is_corner = sf == 0u || sf == 2u || sf == 6u || sf == 8u;
        //    uint j = u_mapping[i];

        //    // Get the base colour for this facelet and the one before this twist
        //    uint cur_face_id = u_cur_colours[j];
        //    uint prev_face_id = u_prev_colours[j];
        //    vec3 base = u_base_cols[cur_face_id];
        //    vec3 prev_base = u_base_cols[prev_face_id];
        //    
        //    vec2 fl_coord = vec2(px_pos.x - (ix*u_facelet_px), px_pos.y - (iy*u_facelet_px)) / u_facelet_px;
        //    fl_coord -= vec2(0.5,0.5);

        //    FragColor = vec4(0.0,0.0,0.0,1.0);
        //    if (this_face && u_anim_pos < 1.0){
        //        // Animations for subfaces on the twisted face
        //        float anim = u_anim_pos;
        //        //float anim = 0.5;
        //        float a = (PI/2) * anim * -u_twist_dir;
        //        float a2 = (PI/2) * (1.0-anim) * u_twist_dir;
        //        if (is_centre){
        //            mat3 rot = rotate(a);
        //            vec2 fl_coord1 = (vec3(fl_coord,1.0) * rot).xy;
        //            FragColor += render_tile(cur_face_id, sf, f, fl_coord1, base, sf_rot, is_centre, is_edge, is_corner);
        //            vec3 r = vec3(1.0,0.0,1.0) * rotate(sf_rot * PI/2);
        //            uint edge_id_1 = u_prev_colours[get_index(f, 1u)];
        //            uint edge_id_2 = u_prev_colours[get_index(f, 3u)];
        //            uint edge_id_3 = u_prev_colours[get_index(f, 7u)];
        //            uint edge_id_4 = u_prev_colours[get_index(f, 5u)];
        //            vec3 edge_base1 = u_base_cols[edge_id_1];
        //            vec3 edge_base2 = u_base_cols[edge_id_2];
        //            vec3 edge_base3 = u_base_cols[edge_id_3];
        //            vec3 edge_base4 = u_base_cols[edge_id_4];
        //            vec3 edge_coord1 = vec3(fl_coord,1.0) * rotate(a) * translate(-r.x,r.y);
        //            vec3 edge_coord2 = vec3(fl_coord,1.0) * rotate(a + (1*PI/2)) * translate(-r.x,r.y);
        //            vec3 edge_coord3 = vec3(fl_coord,1.0) * rotate(a + (2*PI/2)) * translate(-r.x,r.y);
        //            vec3 edge_coord4 = vec3(fl_coord,1.0) * rotate(a + (3*PI/2)) * translate(-r.x,r.y);
        //            FragColor += render_tile_edge(edge_id_1, 1u, f, edge_coord1.xy, edge_base1, 0u);
        //            FragColor += render_tile_edge(edge_id_2, 3u, f, edge_coord2.xy, edge_base2, 0u);
        //            FragColor += render_tile_edge(edge_id_3, 7u, f, edge_coord3.xy, edge_base3, 0u);
        //            FragColor += render_tile_edge(edge_id_4, 5u, f, edge_coord4.xy, edge_base4, 0u);
        //        }
        //        if (is_edge) {
        //            vec3 r = vec3(1.0,0.0,1.0) * rotate(sf_rot * PI/2);
        //            // edge 1
        //            mat3 rot = translate(-r.x,r.y) * rotate(a2) * translate(r.x,-r.y);
        //            vec2 fl_coord1 = (vec3(fl_coord,1.0) * rot).xy;
        //            FragColor += render_tile(cur_face_id, sf, f, fl_coord1, base, sf_rot, is_centre, is_edge, is_corner);
        //            // edge 2
        //            mat3 rot2 = translate(-r.x,r.y) * rotate(a) * translate(r.x,-r.y);
        //            vec2 fl_coord2 = (vec3(fl_coord,1.0) * rot2).xy;
        //            FragColor += render_tile(prev_face_id, sf, f, fl_coord2, prev_base, sf_rot, is_centre, is_edge, is_corner);

        //            uint centre_index = get_index(f, 4u);
        //            uint centre_id = u_cur_colours[centre_index];
        //            vec3 centre_base = u_base_cols[centre_id];
        //            vec3 centre_coord = vec3(fl_coord,1.0) * translate(-r.x,r.y) * rotate(a);
        //            FragColor += render_tile_centre(centre_id, 4u, f, centre_coord.xy, centre_base, 0u);

        //            
        //            vec3 r2 = vec3(1.0,1.0,1.0) * rotate((-sf_rot+3u) * PI/2);
        //            uint corner_index = get_index(f, sf_cw[sf]);
        //            uint corner_id = u_cur_colours[corner_index];
        //            vec3 corner_base = u_base_cols[corner_id];
        //            vec3 corner_coord = centre_coord + r2;
        //            uint corner_rotation = (sf_rot+1u)%4u;
        //            // TODO do not hard code subface id 0 in this next line
        //            FragColor += render_tile_corner(corner_id, 0u, f, corner_coord.xy, corner_base, corner_rotation);

        //            vec3 r3 = vec3(-1.0,1.0,1.0) * rotate((-sf_rot+3u) * PI/2);
        //            uint corner_index2 = get_index(f, sf_ccw[sf]);
        //            uint corner_id_2 = u_cur_colours[corner_index2];
        //            vec3 corner_base2 = u_base_cols[corner_id_2];
        //            vec3 corner_coord2 = centre_coord + r3;
        //            uint corner_rotation2 = (sf_rot+2u)%4u;
        //            // TODO do not hard code subface id 2 in this next line
        //            FragColor += render_tile_corner(corner_id_2, 2u, f, corner_coord2.xy, corner_base2, corner_rotation2);

        //        }
        //        if (is_corner) {
        //            vec3 r = vec3(1.0,-1.0,1.0) * rotate(sf_rot * PI/2);
        //            // Corner 1
        //            mat3 rot = translate(-r.x,r.y) * rotate(a2) * translate(r.x,-r.y);
        //            vec2 fl_coord1 = (vec3(fl_coord,1.0) * rot).xy;
        //            FragColor += render_tile(cur_face_id, sf, f, fl_coord1, base, sf_rot, is_centre, is_edge, is_corner);
        //            // Corner 2
        //            mat3 rot2 = translate(-r.x,r.y) * rotate(a) * translate(r.x,-r.y);
        //            vec2 fl_coord2 = (vec3(fl_coord,1.0) * rot2).xy;
        //            FragColor += render_tile(prev_face_id, sf, f, fl_coord2, prev_base, sf_rot, is_centre, is_edge, is_corner);
        //            // Edge
        //            vec3 centre_coord = vec3(fl_coord,1.0) * translate(-r.x,r.y) * rotate(a);

        //            vec3 r2 = vec3(0.0,1.0,1.0) * rotate((-sf_rot+3u) * PI/2);
        //            uint edge_index = get_index(f, sf_cw[sf]);
        //            vec3 edge_coord = centre_coord + r2;
        //            uint edge_id = u_cur_colours[edge_index];
        //            vec3 edge_base = u_base_cols[edge_id];
        //            uint edge_rotation = sf_rot;
        //            // TODO do not hard code subface id 1 in this next line
        //            FragColor += render_tile_edge(edge_id, 1u, f, edge_coord.xy, edge_base, edge_rotation);

        //            vec3 r3 = vec3(-1.0,0.0,1.0) * rotate((-sf_rot+3u) * PI/2);
        //            uint edge_index2 = get_index(f, sf_ccw[sf]);
        //            vec3 edge_coord2 = centre_coord + r3;
        //            uint edge_id2 = u_cur_colours[edge_index2];
        //            vec3 edge_base2 = u_base_cols[edge_id2];
        //            uint edge_rotation2 = (sf_rot+1u)%4u;
        //            // TODO do not hard code subface id 1 in this next line
        //            FragColor += render_tile_edge(edge_id2, 1u, f, edge_coord2.xy, edge_base2, edge_rotation2);
        //        }
        //    }
        //    else{
        //        if (u_anim_pos >= 1.0 || !adjacent(f, sf, u_twist_face)){
        //            FragColor += render_tile(cur_face_id, sf, f, fl_coord, base, sf_rot, is_centre, is_edge, is_corner);
        //        }
        //        else {
        //            // This is the code path for tiles around the edge of a face that is twisting.
        //            float anim = u_anim_pos * 3;
        //            uint angle = sf_rot+1u;
        //            int dir_index = (int(u_twist_face) * 54) + (int(f) * 9) + int(sf);
        //            uvec4 data_entry = uvec4(texelFetch(u_data_table, dir_index));
        //            uint other_col_a;
        //            uint other_col_b;
        //            angle += (data_entry.a >> 6u) & 3u;
        //            if (u_twist_dir < 0){
        //                angle += 2u;
        //                other_col_a = data_entry.g;// & 0x3fu;
        //                other_col_b = data_entry.r;// & 0x3fu;
        //            }
        //            else {
        //                other_col_a = data_entry.a & 0x3fu;
        //                other_col_b = data_entry.b;// & 0x3fu;
        //            }

        //            vec2 fl_coord_a;
        //            vec2 fl_coord_a2;
        //            vec2 fl_coord_a3;
        //            vec2 fl_coord_a4;

        //            if (u_anim_style == 0u){
        //                mat3 rot = rotate(angle * (PI/2));
        //                fl_coord_a  = fl_coord + (vec3(      anim ,0.0,1.0)*rot).xy;
        //                fl_coord_a2 = fl_coord + (vec3(-(3.0-anim),0.0,1.0)*rot).xy;
        //                fl_coord_a3 = fl_coord + (vec3(-(1.0-anim),0.0,1.0)*rot).xy;
        //                fl_coord_a4 = fl_coord + (vec3(-(2.0-anim),0.0,1.0)*rot).xy;
        //            }
        //            if (u_anim_style == 1u){
        //                mat3 rot = rotate(angle * (PI/2));
        //                vec3 fl_coord3_a  = vec3(fl_coord,1.5) + (vec3( 0.0,0.0,1.0)*rot);
        //                vec3 fl_coord3_a2 = vec3(fl_coord,1.5) + (vec3(-3.0,0.0,1.0)*rot);
        //                vec3 fl_coord3_a3 = vec3(fl_coord,1.5) + (vec3(-1.0,0.0,1.0)*rot);
        //                vec3 fl_coord3_a4 = vec3(fl_coord,1.5) + (vec3(-2.0,0.0,1.0)*rot);
        //                //fl_coord_a = (vec4(fl_coord_a, 0.0, 1.0) * translate4(0.0,0.0,1.5) * rotate4(0.0,anim*(PI/2),0.0) * translate4(0.0,0.0,-1.5)).xy;
        //                float d = 0;
        //                float fd = 0;
        //                fl_coord_a = (vec4(fl_coord3_a,  1.0) * translate4(-fd,-fd,-d) * rotate4(0.0,u_anim_pos*(PI/2),0.0) * translate4(fd,fd,d)).xy;
        //                fl_coord_a2= (vec4(fl_coord3_a2, 1.0) * translate4(-fd,-fd,-d) * rotate4(0.0,u_anim_pos*(PI/2),0.0) * translate4(fd,fd,d)).xy;
        //                fl_coord_a3= (vec4(fl_coord3_a3, 1.0) * translate4(-fd,-fd,-d) * rotate4(0.0,u_anim_pos*(PI/2),0.0) * translate4(fd,fd,d)).xy;
        //                fl_coord_a4= (vec4(fl_coord3_a4, 1.0) * translate4(-fd,-fd,-d) * rotate4(0.0,u_anim_pos*(PI/2),0.0) * translate4(fd,fd,d)).xy;
        //            }
        //            // previous colour, slides out
        //            FragColor += render_tile(prev_face_id, sf, f, fl_coord_a, prev_base, sf_rot, is_centre, is_edge, is_corner);
        //
        //            // two intermediate colours slide through
        //            uint other_face_b_id = u_prev_colours[other_col_b];
        //            uint other_face_a_id = u_prev_colours[other_col_a];
        //            // TODO what is the subface for each of these
        //            FragColor += render_tile(other_face_a_id, 1u, f, fl_coord_a3, u_base_cols[other_face_b_id], sf_rot, is_centre, is_edge, is_corner);
        //            FragColor += render_tile(other_face_b_id, 2u, f, fl_coord_a4, u_base_cols[other_face_a_id], sf_rot, is_centre, is_edge, is_corner);

        //            // next colour, slides in
        //            FragColor += render_tile(cur_face_id, sf, f, fl_coord_a2, base, sf_rot, is_centre, is_edge, is_corner);
        //        }
        //    }
        //    if (is_centre && (u_debug_arrow > 0u)){
        //        float a2 = 0;
        //        if (this_face && u_anim_pos < 1.0){
        //            a2 = (PI/2) * (1.0-u_anim_pos) * u_twist_dir;
        //        }
        //        mat3 rot = rotate(a2);
        //        vec2 c = (vec3(fl_coord,1.0) * rot).xy;
        //        vec3 edge_base1 = u_base_cols[u_cur_colours[get_index(f, 1u)]];
        //        vec3 edge_base2 = u_base_cols[u_cur_colours[get_index(f, 3u)]];
        //        vec3 edge_base3 = u_base_cols[u_cur_colours[get_index(f, 7u)]];
        //        vec3 edge_base4 = u_base_cols[u_cur_colours[get_index(f, 5u)]];
        //        vec4 ah1 = arrowhead(c, edge_base1, (sf_rot + 0u)%4u);
        //        vec4 ah2 = arrowhead(c, edge_base2, (sf_rot + 1u)%4u);
        //        vec4 ah3 = arrowhead(c, edge_base3, (sf_rot + 2u)%4u);
        //        vec4 ah4 = arrowhead(c, edge_base4, (sf_rot + 3u)%4u);
        //        float mask = ah1.a * ah2.a * ah3.a * ah4.a;
        //        vec4 arrows = vec4(ah1.rgb+ah2.rgb+ah3.rgb+ah4.rgb, 0.0);
        //        FragColor = (FragColor * mask) + arrows;
        //    }

        //}
        //else{
        //    FragColor = unseen_area(px_pos);
        //}
    }
    tFragColor = FragColor.rgb;
}
