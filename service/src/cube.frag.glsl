#version 330 core
in vec2 px_pos;
uniform float u_facelet_px;
uniform float u_anim_pos;
uniform uint u_prev_colours[54];
uniform uint u_cur_colours[54];
uniform uint u_mapping[45];
uniform uint u_rotation_map[45];
uniform uint u_map_facenum[45];
uniform uint u_map_subfacenum[45];
uniform uint u_inverse_facemap[45];
uniform vec3 u_base_cols[6];
uniform uint u_twist_face;
uniform float u_twist_dir;
uniform uint u_debug_arrow;
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

mat3 rotate(float a){
    return mat3(
        cos(a), -sin(a),0.0,
        sin(a), cos(a),0.0,
        0.0,0.0,1.0
    );
}

uint get_index(uint f, uint sf){
    uint i = (f*9u)+sf;
    return u_inverse_facemap[i];
}

vec4 render_tile(vec2 fl_coord, vec3 base, uint rot, bool is_centre, bool is_edge, bool is_corner){
    vec3 c = base * bulge(fl_coord.x) * bulge(fl_coord.y);

    if (u_debug_arrow > 0u){
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

vec4 render_tile_centre(vec2 fl_coord, vec3 base, uint rot){ return render_tile(fl_coord, base, rot, true, false, false); }
vec4 render_tile_edge(vec2 fl_coord, vec3 base, uint rot){ return render_tile(fl_coord, base, rot, false, true, false); }
vec4 render_tile_corner(vec2 fl_coord, vec3 base, uint rot){ return render_tile(fl_coord, base, rot, false, false, true); }

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
    int ix = int(floor(px_pos.x / fp));
    int iy = int(floor(px_pos.y / fp));
    int i = iy * 9 + ix;
    if (i < 45 && px_pos.y > 0.0 && px_pos.x > 0.0 && px_pos.y < (fp*5) && px_pos.x < (fp*9)){
        uint f = u_map_facenum[i];
        uint sf = u_map_subfacenum[i];
        uint sf_rot = u_rotation_map[i];
        bool this_face = u_map_facenum[i] == u_twist_face;
        bool is_centre = sf == 4u;
        bool is_edge = sf == 1u || sf == 3u || sf == 5u || sf == 7u;
        bool is_corner = sf == 0u || sf == 2u || sf == 6u || sf == 8u;
        uint j = u_mapping[i];

        // Get the base colour for this facelet and the one before this twist
        vec3 base = u_base_cols[u_cur_colours[j]];
        vec3 prev_base = u_base_cols[u_prev_colours[j]];
        
        vec2 fl_coord = vec2(px_pos.x - (ix*u_facelet_px), px_pos.y - (iy*u_facelet_px)) / u_facelet_px;
        fl_coord -= vec2(0.5,0.5);
        if (this_face && u_anim_pos < 1.0){
            // Animations for subfaces on the twisted face
            float anim = u_anim_pos;
            //float anim = 0.5;
            float a = (PI/2) * anim * -u_twist_dir;
            float a2 = (PI/2) * (1.0-anim) * u_twist_dir;
            FragColor = vec4(0.0,0.0,0.0,1.0);
            if (is_centre){
                mat3 rot = rotate(a);
                vec2 fl_coord1 = (vec3(fl_coord,1.0) * rot).xy;
                FragColor += render_tile(fl_coord1, base, sf_rot, is_centre, is_edge, is_corner);
                vec3 r = vec3(1.0,0.0,1.0) * rotate(sf_rot * PI/2);
                // TODO calculate actual edge base colours
                vec3 edge_base1 = u_base_cols[u_prev_colours[get_index(f, 1u)]]; // TODO determine centre colour to render here
                vec3 edge_base2 = u_base_cols[u_prev_colours[get_index(f, 3u)]]; // TODO determine centre colour to render here
                vec3 edge_base3 = u_base_cols[u_prev_colours[get_index(f, 7u)]]; // TODO determine centre colour to render here
                vec3 edge_base4 = u_base_cols[u_prev_colours[get_index(f, 5u)]]; // TODO determine centre colour to render here
                vec3 edge_coord1 = vec3(fl_coord,1.0) * rotate(a) * translate(-r.x,r.y);
                vec3 edge_coord2 = vec3(fl_coord,1.0) * rotate(a + (1*PI/2)) * translate(-r.x,r.y);
                vec3 edge_coord3 = vec3(fl_coord,1.0) * rotate(a + (2*PI/2)) * translate(-r.x,r.y);
                vec3 edge_coord4 = vec3(fl_coord,1.0) * rotate(a + (3*PI/2)) * translate(-r.x,r.y);
                FragColor += render_tile_edge(edge_coord1.xy, edge_base1, 0u);
                FragColor += render_tile_edge(edge_coord2.xy, edge_base2, 0u);
                FragColor += render_tile_edge(edge_coord3.xy, edge_base3, 0u);
                FragColor += render_tile_edge(edge_coord4.xy, edge_base4, 0u);
            }
            if (is_edge) {
                vec3 r = vec3(1.0,0.0,1.0) * rotate(sf_rot * PI/2);
                // edge 1
                mat3 rot = translate(-r.x,r.y) * rotate(a2) * translate(r.x,-r.y);
                vec2 fl_coord1 = (vec3(fl_coord,1.0) * rot).xy;
                FragColor += render_tile(fl_coord1, base, sf_rot, is_centre, is_edge, is_corner);
                // edge 2
                mat3 rot2 = translate(-r.x,r.y) * rotate(a) * translate(r.x,-r.y);
                vec2 fl_coord2 = (vec3(fl_coord,1.0) * rot2).xy;
                FragColor += render_tile(fl_coord2, prev_base, sf_rot, is_centre, is_edge, is_corner);

                uint centre_index = get_index(f, 4u);
                vec3 centre_base = u_base_cols[u_cur_colours[centre_index]];
                vec3 centre_coord = vec3(fl_coord,1.0) * translate(-r.x,r.y) * rotate(a);
                FragColor += render_tile_centre(centre_coord.xy, centre_base, 0u);

                
                vec3 r2 = vec3(1.0,1.0,1.0) * rotate((-sf_rot+3u) * PI/2);
                uint corner_index = get_index(f, sf_cw[sf]);
                vec3 corner_base = u_base_cols[u_cur_colours[corner_index]];
                vec3 corner_coord = centre_coord + r2;
                uint corner_rotation = 0u;
                FragColor += render_tile_corner(corner_coord.xy, corner_base, corner_rotation);

                vec3 r3 = vec3(-1.0,1.0,1.0) * rotate((-sf_rot+3u) * PI/2);
                uint corner_index2 = get_index(f, sf_ccw[sf]);
                vec3 corner_base2 = u_base_cols[u_cur_colours[corner_index2]];
                vec3 corner_coord2 = centre_coord + r3;
                uint corner_rotation2 = 0u;
                FragColor += render_tile_corner(corner_coord2.xy, corner_base2, corner_rotation2);

            }
            if (is_corner) {
                vec3 r = vec3(1.0,-1.0,1.0) * rotate(sf_rot * PI/2);
                // Corner 1
                mat3 rot = translate(-r.x,r.y) * rotate(a2) * translate(r.x,-r.y);
                vec2 fl_coord1 = (vec3(fl_coord,1.0) * rot).xy;
                FragColor += render_tile(fl_coord1, base, sf_rot, is_centre, is_edge, is_corner);
                // Corner 2
                mat3 rot2 = translate(-r.x,r.y) * rotate(a) * translate(r.x,-r.y);
                vec2 fl_coord2 = (vec3(fl_coord,1.0) * rot2).xy;
                FragColor += render_tile(fl_coord2, prev_base, sf_rot, is_centre, is_edge, is_corner);
                // Edge
                vec3 centre_coord = vec3(fl_coord,1.0) * translate(-r.x,r.y) * rotate(a);

                vec3 r2 = vec3(0.0,1.0,1.0) * rotate((-sf_rot+3u) * PI/2);
                uint edge_index = get_index(f, sf_cw[sf]);
                vec3 edge_coord = centre_coord + r2;
                vec3 edge_base = u_base_cols[u_cur_colours[edge_index]];
                uint edge_rotation = 0u;
                FragColor += render_tile_edge(edge_coord.xy, edge_base, edge_rotation);

                vec3 r3 = vec3(-1.0,0.0,1.0) * rotate((-sf_rot+3u) * PI/2);
                uint edge_index2 = get_index(f, sf_ccw[sf]);
                vec3 edge_coord2 = centre_coord + r3;
                vec3 edge_base2 = u_base_cols[u_cur_colours[edge_index2]];
                uint edge_rotation2 = 0u;
                FragColor += render_tile_edge(edge_coord2.xy, edge_base2, edge_rotation2);
            }
        }
        else{
            // TODO remove fades from other faces
            FragColor = render_tile(fl_coord, prev_base, sf_rot, is_centre, is_edge, is_corner) * (1.0-u_anim_pos);
            FragColor += render_tile(fl_coord, base, sf_rot, is_centre, is_edge, is_corner) * (u_anim_pos);
        }
        //if (is_centre){
        //    mat3 rot = rotate(0);
        //    vec2 fl_coord1 = (vec3(fl_coord,1.0) * rot).xy;
        //    //FragColor += render_tile(fl_coord1, base, sf_rot, is_centre, is_edge, is_corner);
        //    vec3 r = vec3(0.7,0.0,1.0) * rotate(sf_rot * PI/2);
        //    vec3 edge_base1 = u_base_cols[u_cur_colours[get_index(f, 1u)]];
        //    vec3 edge_base2 = u_base_cols[u_cur_colours[get_index(f, 3u)]];
        //    vec3 edge_base3 = u_base_cols[u_cur_colours[get_index(f, 7u)]];
        //    vec3 edge_base4 = u_base_cols[u_cur_colours[get_index(f, 5u)]];
        //    vec3 edge_coord1 = vec3(fl_coord,1.0) * rotate(0) * translate(-r.x,r.y);
        //    vec3 edge_coord2 = vec3(fl_coord,1.0) * rotate(0 + (1*PI/2)) * translate(-r.x,r.y);
        //    vec3 edge_coord3 = vec3(fl_coord,1.0) * rotate(0 + (2*PI/2)) * translate(-r.x,r.y);
        //    vec3 edge_coord4 = vec3(fl_coord,1.0) * rotate(0 + (3*PI/2)) * translate(-r.x,r.y);
        //    FragColor += render_tile_edge(edge_coord1.xy, edge_base1, 0u);
        //    FragColor += render_tile_edge(edge_coord2.xy, edge_base2, 0u);
        //    FragColor += render_tile_edge(edge_coord3.xy, edge_base3, 0u);
        //    FragColor += render_tile_edge(edge_coord4.xy, edge_base4, 0u);
        //}

    }
    else{
        FragColor = vec4(1.0,0.0,0.0, 1.0);
    }
    tFragColor = FragColor.rgb;
}
