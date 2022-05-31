#![cfg_attr(feature="without_std", no_std)]

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Colors{
    White, Red, Blue, Green, Yellow, Orange, Blank
}

#[derive(Clone, Copy, Debug)]
pub struct SubFace{
    pub color: Colors
    ,next_color: Colors
}

#[derive(Clone, Copy, Debug)]
pub struct Face{
    pub subfaces: [SubFace; 9]
    ,adjacent: [(usize, isize, isize); 4]
}

#[derive(Clone, Copy, Debug)]
pub struct Cube{
    pub faces: [Face; 9]
}

impl Colors{
    pub fn shortname(&self) -> &'static str{
        match self{
            Colors::White => "W",
            Colors::Red => "R",
            Colors::Blue => "B",
            Colors::Green => "G",
            Colors::Yellow => "Y",
            Colors::Orange => "O",
            Colors::Blank => " ",
        }
    }

    pub fn from_shortname(name: &str) -> Colors{
        match name{
            "W" => Colors::White,
            "R" => Colors::Red,
            "B" => Colors::Blue,
            "G" => Colors::Green,
            "Y" => Colors::Yellow,
            "O" => Colors::Orange,
            _ => Colors::Blank,
        }
    }
}

impl Face{
    fn new(color: Colors, adjacent: [(usize, isize, isize);4]) -> Face {
        Face{
            subfaces: [
                SubFace{color: color, next_color: color};9
            ]
            ,adjacent: adjacent
        }
    }

    fn copy_from(&mut self, other: [SubFace;9], doffset: isize, dstep: isize, soffset: isize, sstep: isize) -> [[SubFace;9];2] {
        let window: [SubFace;6] = [
            self.subfaces[doffset as usize]
            ,self.subfaces[(doffset+dstep) as usize]
            ,self.subfaces[(doffset+dstep+dstep) as usize]
            ,other[soffset as usize]
            ,other[(soffset+sstep) as usize]
            ,other[(soffset+sstep+sstep) as usize]
        ];

        let mut intermediates = [self.subfaces;2];

        for f in 0..2{
            for i in 0..3{
                let d = (doffset + (i * dstep)) as usize;
                let c = window[f + i as usize + 1].color;
                intermediates[f][d].color = c;
                intermediates[f][d].next_color = c;
            }
        }

        for i in 0..3{
            let d = (doffset + (i * dstep)) as usize;
            self.subfaces[d].next_color = window[3+i as usize].color;
        }

        intermediates
    }

    fn update(&mut self){
        for s in &mut self.subfaces{
            s.color = s.next_color;
        }
    }

    fn twist(&mut self, reverse: bool) -> [SubFace;9]{
        let f = &self.subfaces;
        let intermediate = if !reverse {[
            f[3], f[0], f[1]
            ,f[6], f[4], f[2]
            ,f[7], f[8], f[5]
        ]}
        else {[
            f[1], f[2], f[5]
            ,f[0], f[4], f[8]
            ,f[3], f[6], f[7]
        ]};
        self.subfaces = if !reverse {[
            f[6],f[3],f[0]
            ,f[7],f[4],f[1]
            ,f[8],f[5],f[2]
        ]}
        else {[
            f[2],f[5],f[8]
            ,f[1],f[4],f[7]
            ,f[0],f[3],f[6]
        ]};
        intermediate
    }

    #[cfg(not(feature="without_std"))]
    pub fn simple_string(&self) -> String{
        let f = &self.subfaces;
        format!("{}{}{}\n{}{}{}\n{}{}{}"
            ,f[0].color.shortname()
            ,f[1].color.shortname()
            ,f[2].color.shortname()
            ,f[3].color.shortname()
            ,f[4].color.shortname()
            ,f[5].color.shortname()
            ,f[6].color.shortname()
            ,f[7].color.shortname()
            ,f[8].color.shortname()
        )
    }
}

// TODO make this an enum? how to handle FAKE_FACE_MIN for enum?
pub const TOP: usize = 0;
pub const FRONT: usize = 1;
pub const LEFT: usize = 2;
pub const BACK: usize = 3;
pub const RIGHT: usize = 4;
pub const BOTTOM: usize = 5;
pub const FAKE_FACE_MIN: usize = 6;
pub const CENTER_FB: usize = 6;
pub const CENTER_LR: usize = 7;
pub const CENTER_BT: usize = 8;

// Represents any turn that is a single turn according to Quarter Slice Turn Metric
// (This means that it can only represent 90-degree turns, and not 180-degree turns
// and that it also does not represent full-cube rotations)
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Twist{
    pub face: usize
    ,pub reverse: bool
}

impl Twist{

    pub fn from_bytes(s: &[u8]) -> Result<Twist, &'static str>{
        let l = s.len();
        if l < 1 || l > 3 {
            Err("Invalid twist string")
        }
        else{
            let reverse = s[s.len()-1] == b'\'';
            let face = match s[0] {
                b'T'|b't' => Ok(TOP),
                b'U'|b'u' => Ok(TOP),
                b'F'|b'f' => Ok(FRONT),
                b'L'|b'l' => Ok(LEFT),
                b'B'|b'b' => match (l, reverse) {
                    (1, false) | (2, true) => {Ok(BACK)},
                    (2, false) | (3, true) => {
                        match s[1] {
                            b'A' => {Ok(BACK)},
                            b'a' => {Ok(BACK)},
                            b' ' => {Ok(BACK)},
                            b'O' => {Ok(BOTTOM)},
                            b'o' => {Ok(BOTTOM)},
                            _ => {Err("Invalid twist string")}
                        }
                    },
                    _=>{Err("Invalid twist string")}
                },
                b'R'|b'r' => Ok(RIGHT),
                b'D'|b'd' => Ok(BOTTOM),
                b'S'|b's' => Ok(CENTER_FB),
                b'M'|b'm' => Ok(CENTER_LR),
                b'E'|b'e' => Ok(CENTER_BT),
                _=> Err("Invalid twist string"),
            }?;
            Ok(Twist{
                face: face
                ,reverse: reverse
            })
        }
    }

    pub fn from_string(s: &str) -> Result<Twist, &'static str>{
        let s = s.as_bytes();
        Twist::from_bytes(s)
    }

    #[cfg(not(feature="without_std"))]
    pub fn seq_from_string(s: &str) -> Result<Vec<Twist>, &'static str>{
        let mut seq = Vec::new();
        for m in s.split_whitespace(){
            let l = m.len();
            if l < 1{
                // wut?
            }
            else{
                let b = m.as_bytes();
                let last = b[b.len()-1];
                if last == b'2'{
                    let t = Twist::from_string(&m[0..l-1])?;
                    seq.push(t.clone());
                    seq.push(t);
                }
                else{
                    seq.push(Twist::from_string(m)?);
                }
            }
        }
        Ok(seq)
    }

}

#[cfg(not(feature="without_std"))]
use std::fmt;
#[cfg(not(feature="without_std"))]
impl fmt::Display for Twist{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f
            ,"{}{}"
            ,match self.face {
                TOP => "U"
                ,FRONT => "F"
                ,LEFT => "L"
                ,BACK => "B"
                ,RIGHT => "R"
                ,BOTTOM => "D"
                ,CENTER_FB => "S"
                ,CENTER_LR => "M"
                ,CENTER_BT => "E"
                ,_=>"?"
            }
            ,if self.reverse {"'"} else {""}
        )
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Output{
    pub face: usize
    ,pub subface: usize
}

pub const ALL_TWISTS: [Twist; 18] = [
    Twist{face:BOTTOM, reverse:false}
    ,Twist{face:CENTER_BT, reverse:false}
    ,Twist{face:TOP, reverse:false}
    ,Twist{face:LEFT, reverse:false}
    ,Twist{face:CENTER_LR, reverse:false}
    ,Twist{face:RIGHT, reverse:false}
    ,Twist{face:FRONT, reverse:false}
    ,Twist{face:CENTER_FB, reverse:false}
    ,Twist{face:BACK, reverse:false}
    ,Twist{face:BOTTOM, reverse:true}
    ,Twist{face:CENTER_BT, reverse:true}
    ,Twist{face:TOP, reverse:true}
    ,Twist{face:LEFT, reverse:true}
    ,Twist{face:CENTER_LR, reverse:true}
    ,Twist{face:RIGHT, reverse:true}
    ,Twist{face:FRONT, reverse:true}
    ,Twist{face:CENTER_FB, reverse:true}
    ,Twist{face:BACK, reverse:true}
];

impl Cube{
    pub fn new() -> Cube{
        // top front left back right bottom
        let faces = [
            Face::new(Colors::White, [(BACK, 0,1), (RIGHT, 0,1), (FRONT, 0,1), (LEFT, 0,1)])
            ,Face::new(Colors::Red, [(TOP, 8,-1), (RIGHT, 6,-3), (BOTTOM, 8,-1), (LEFT, 2,3)])
            ,Face::new(Colors::Green, [(TOP, 6,-3), (FRONT, 6,-3), (BOTTOM, 2,3), (BACK, 2,3)])
            ,Face::new(Colors::Orange, [(TOP, 0,1), (LEFT, 6,-3), (BOTTOM, 0,1), (RIGHT, 2,3)])
            ,Face::new(Colors::Blue, [(TOP, 2,3), (BACK, 6,-3), (BOTTOM, 6,-3), (FRONT, 2,3)])
            ,Face::new(Colors::Yellow, [(FRONT, 8,-1), (RIGHT, 8,-1), (BACK, 8,-1), (LEFT, 8,-1)])
            // Fake faces, for manipulating centres, colour doesn't matter, it's never seen and represents nothing
            ,Face::new(Colors::White, [(LEFT, 1,3), (TOP, 5,-1), (RIGHT, 7,-3), (BOTTOM, 5,-1)])
            ,Face::new(Colors::White, [(BACK, 1,3), (TOP, 7,-3), (FRONT, 7,-3), (BOTTOM, 1,3)])
            ,Face::new(Colors::White, [(LEFT, 5,-1), (FRONT, 5,-1), (RIGHT, 5,-1), (BACK, 5,-1)])
        ];
        Cube{faces:faces}
    }

    pub fn deserialise(&mut self, data: &str) -> Result<(), &'static str> {
        if data.len() < 54{
            return Err("not enough data, incomplete cube state");
        }
        let mut i: usize = 0;
        for face in &mut self.faces[0..6]{
            for sface in &mut face.subfaces{
                let col = Colors::from_shortname(&data[i..i+1]);
                sface.color = col;
                sface.next_color = col;
                i+=1;
            }
        }
        Ok(())
    }

    #[cfg(not(feature="without_std"))]
    pub fn serialise(&self) -> String {
        let mut s = String::with_capacity(54);
        for face in &self.faces[0..6]{
            for sface in face.subfaces{
                s.push_str(sface.color.shortname());
            }
        }
        s
    }

    pub fn twist(&mut self, twist: Twist) -> [Cube; 3]{
        let face = twist.face;
        let reverse = twist.reverse;
        let mut intermediates = [*self;3];

        if face < FAKE_FACE_MIN {
            let anim = (&mut self.faces[face]).twist(reverse);
            intermediates[1].faces[face].subfaces = anim;
            intermediates[2].faces[face].subfaces = anim;
        }

        for i in 0..4{
            let (adj, doffset, dstep) = self.faces[face].adjacent[i];
            let (next, soffset, sstep) = self.faces[face].adjacent[((((i as isize) + if reverse {1} else {-1})+4)%4)as usize];
            let subs = self.faces[next].subfaces;
            let edge_anim = if !reverse {
                self.faces[adj].copy_from(subs, doffset, dstep, soffset, sstep)
            }
            else {
                self.faces[adj].copy_from(subs, doffset+(dstep*2), -dstep, soffset+(sstep*2), -sstep)
            };
            intermediates[0].faces[adj].subfaces = edge_anim[0];
            intermediates[1].faces[adj].subfaces = edge_anim[0];
            intermediates[2].faces[adj].subfaces = edge_anim[1];
        }

        for i in 0..4{
            let (adj, _,_) = self.faces[face].adjacent[i];
            self.faces[adj].update();
        }


        #[cfg(not(feature="without_std"))]
        for face in &self.faces{
            for s in &face.subfaces{
                assert!(s.color == s.next_color);
            }
        }

        intermediates
    }

    #[cfg(not(feature="without_std"))]
    pub fn twists(&mut self, twists: &str) -> Result<(), &'static str>{
        let t = Twist::seq_from_string(twists)?;
        for t in t{
            self.twist(t);
        }
        Ok(())
    }

    #[cfg(not(feature="without_std"))]
    pub fn simple_string(&self) -> String{
        format!("Top:\n{}\nFront:\n{}\nLeft:\n{}\nBack:\n{}\nRight:\n{}\nBottom:\n{}"
            ,self.faces[TOP].simple_string()
            ,self.faces[FRONT].simple_string()
            ,self.faces[LEFT].simple_string()
            ,self.faces[BACK].simple_string()
            ,self.faces[RIGHT].simple_string()
            ,self.faces[BOTTOM].simple_string()
        )
    }

    pub fn is_solved(&self) -> bool {
        for f in 0..6{
            let face = self.faces[f];
            let col = face.subfaces[0].color;
            for s in 1..9{
                if face.subfaces[s].color != col{
                    return false;
                }
            }
        }
        true
    }
}

pub type SwitchMap5Faces = [Twist;48];

pub type OutputMap5Faces = [Output;45];

#[cfg(not(feature="without_std"))]
pub fn serialise_output_map(map: &OutputMap5Faces) -> String {
    let mut result = String::new();
    for i in 0..45{
        result.push_str(&format!("{}{}", map[i].face, map[i].subface));
    }
    result
}

#[cfg(test)]
mod tests {
    use crate::{Cube, Twist, TOP, LEFT, CENTER_FB, BACK, BOTTOM, RIGHT};

    #[cfg(not(feature="without_std"))]
    #[test]
    fn cube_init() {
        let cube = Cube::new();
        let result = cube.simple_string();
        assert_eq!(result, "Top:\nWWW\nWWW\nWWW\nFront:\nRRR\nRRR\nRRR\nLeft:\nGGG\nGGG\nGGG\nBack:\nOOO\nOOO\nOOO\nRight:\nBBB\nBBB\nBBB\nBottom:\nYYY\nYYY\nYYY".to_string());
    }

    #[cfg(not(feature="without_std"))]
    #[test]
    fn basic_twists() {
        use crate::ALL_TWISTS;
        assert!(ALL_TWISTS.len() > 0);
        for (twist, expected) in ALL_TWISTS.iter().zip([
            "Top:\nWWW\nWWW\nWWW\nFront:\nRRR\nRRR\nGGG\nLeft:\nGGG\nGGG\nOOO\nBack:\nOOO\nOOO\nBBB\nRight:\nBBB\nBBB\nRRR\nBottom:\nYYY\nYYY\nYYY",
            "Top:\nWWW\nWWW\nWWW\nFront:\nRRR\nGGG\nRRR\nLeft:\nGGG\nOOO\nGGG\nBack:\nOOO\nBBB\nOOO\nRight:\nBBB\nRRR\nBBB\nBottom:\nYYY\nYYY\nYYY",
            "Top:\nWWW\nWWW\nWWW\nFront:\nBBB\nRRR\nRRR\nLeft:\nRRR\nGGG\nGGG\nBack:\nGGG\nOOO\nOOO\nRight:\nOOO\nBBB\nBBB\nBottom:\nYYY\nYYY\nYYY",
            "Top:\nOWW\nOWW\nOWW\nFront:\nWRR\nWRR\nWRR\nLeft:\nGGG\nGGG\nGGG\nBack:\nOOY\nOOY\nOOY\nRight:\nBBB\nBBB\nBBB\nBottom:\nYYR\nYYR\nYYR",
            "Top:\nWOW\nWOW\nWOW\nFront:\nRWR\nRWR\nRWR\nLeft:\nGGG\nGGG\nGGG\nBack:\nOYO\nOYO\nOYO\nRight:\nBBB\nBBB\nBBB\nBottom:\nYRY\nYRY\nYRY",
            "Top:\nWWR\nWWR\nWWR\nFront:\nRRY\nRRY\nRRY\nLeft:\nGGG\nGGG\nGGG\nBack:\nWOO\nWOO\nWOO\nRight:\nBBB\nBBB\nBBB\nBottom:\nOYY\nOYY\nOYY",
            "Top:\nWWW\nWWW\nGGG\nFront:\nRRR\nRRR\nRRR\nLeft:\nGGY\nGGY\nGGY\nBack:\nOOO\nOOO\nOOO\nRight:\nWBB\nWBB\nWBB\nBottom:\nYYY\nYYY\nBBB",
            "Top:\nWWW\nGGG\nWWW\nFront:\nRRR\nRRR\nRRR\nLeft:\nGYG\nGYG\nGYG\nBack:\nOOO\nOOO\nOOO\nRight:\nBWB\nBWB\nBWB\nBottom:\nYYY\nBBB\nYYY",
            "Top:\nBBB\nWWW\nWWW\nFront:\nRRR\nRRR\nRRR\nLeft:\nWGG\nWGG\nWGG\nBack:\nOOO\nOOO\nOOO\nRight:\nBBY\nBBY\nBBY\nBottom:\nGGG\nYYY\nYYY",
            "Top:\nWWW\nWWW\nWWW\nFront:\nRRR\nRRR\nBBB\nLeft:\nGGG\nGGG\nRRR\nBack:\nOOO\nOOO\nGGG\nRight:\nBBB\nBBB\nOOO\nBottom:\nYYY\nYYY\nYYY",
            "Top:\nWWW\nWWW\nWWW\nFront:\nRRR\nBBB\nRRR\nLeft:\nGGG\nRRR\nGGG\nBack:\nOOO\nGGG\nOOO\nRight:\nBBB\nOOO\nBBB\nBottom:\nYYY\nYYY\nYYY",
            "Top:\nWWW\nWWW\nWWW\nFront:\nGGG\nRRR\nRRR\nLeft:\nOOO\nGGG\nGGG\nBack:\nBBB\nOOO\nOOO\nRight:\nRRR\nBBB\nBBB\nBottom:\nYYY\nYYY\nYYY",
            "Top:\nRWW\nRWW\nRWW\nFront:\nYRR\nYRR\nYRR\nLeft:\nGGG\nGGG\nGGG\nBack:\nOOW\nOOW\nOOW\nRight:\nBBB\nBBB\nBBB\nBottom:\nYYO\nYYO\nYYO",
            "Top:\nWRW\nWRW\nWRW\nFront:\nRYR\nRYR\nRYR\nLeft:\nGGG\nGGG\nGGG\nBack:\nOWO\nOWO\nOWO\nRight:\nBBB\nBBB\nBBB\nBottom:\nYOY\nYOY\nYOY",
            "Top:\nWWO\nWWO\nWWO\nFront:\nRRW\nRRW\nRRW\nLeft:\nGGG\nGGG\nGGG\nBack:\nYOO\nYOO\nYOO\nRight:\nBBB\nBBB\nBBB\nBottom:\nRYY\nRYY\nRYY",
            "Top:\nWWW\nWWW\nBBB\nFront:\nRRR\nRRR\nRRR\nLeft:\nGGW\nGGW\nGGW\nBack:\nOOO\nOOO\nOOO\nRight:\nYBB\nYBB\nYBB\nBottom:\nYYY\nYYY\nGGG",
            "Top:\nWWW\nBBB\nWWW\nFront:\nRRR\nRRR\nRRR\nLeft:\nGWG\nGWG\nGWG\nBack:\nOOO\nOOO\nOOO\nRight:\nBYB\nBYB\nBYB\nBottom:\nYYY\nGGG\nYYY",
        ]){
            let mut cube = Cube::new();
            cube.twist(*twist);
            let result = cube.simple_string();
            assert_eq!(result, expected.to_string());
        }
    }

    #[cfg(not(feature="without_std"))]
    #[test]
    fn twist_animation() {
        let mut cube = Cube::new();
        let anim = cube.twist(Twist::from_string("u").unwrap());
        assert_eq!(anim[0].simple_string(), "Top:\nWWW\nWWW\nWWW\nFront:\nRRB\nRRR\nRRR\nLeft:\nGGR\nGGG\nGGG\nBack:\nOOG\nOOO\nOOO\nRight:\nBBO\nBBB\nBBB\nBottom:\nYYY\nYYY\nYYY".to_string());
        assert_eq!(anim[1].simple_string(), "Top:\nWWW\nWWW\nWWW\nFront:\nRRB\nRRR\nRRR\nLeft:\nGGR\nGGG\nGGG\nBack:\nOOG\nOOO\nOOO\nRight:\nBBO\nBBB\nBBB\nBottom:\nYYY\nYYY\nYYY".to_string());
        assert_eq!(anim[2].simple_string(), "Top:\nWWW\nWWW\nWWW\nFront:\nRBB\nRRR\nRRR\nLeft:\nGRR\nGGG\nGGG\nBack:\nOGG\nOOO\nOOO\nRight:\nBOO\nBBB\nBBB\nBottom:\nYYY\nYYY\nYYY".to_string());
        let anim = cube.twist(Twist::from_string("f'").unwrap());
        assert_eq!(anim[0].simple_string(), "Top:\nWWW\nWWW\nWWO\nFront:\nBBB\nRRR\nRRR\nLeft:\nRRW\nGGR\nGGG\nBack:\nGGG\nOOO\nOOO\nRight:\nBOO\nBBB\nYBB\nBottom:\nYYY\nYYY\nYYG".to_string());
        assert_eq!(anim[1].simple_string(), "Top:\nWWW\nWWW\nWWO\nFront:\nBBR\nBRR\nRRR\nLeft:\nRRW\nGGR\nGGG\nBack:\nGGG\nOOO\nOOO\nRight:\nBOO\nBBB\nYBB\nBottom:\nYYY\nYYY\nYYG".to_string());
        assert_eq!(anim[2].simple_string(), "Top:\nWWW\nWWW\nWOB\nFront:\nBBR\nBRR\nRRR\nLeft:\nRRW\nGGW\nGGR\nBack:\nGGG\nOOO\nOOO\nRight:\nBOO\nYBB\nYBB\nBottom:\nYYY\nYYY\nYGG".to_string());
    }

    #[cfg(not(feature="without_std"))]
    #[test]
    fn ser_deser(){
        let mut c = Cube::new();
        c.twist(Twist{face:TOP, reverse:false});
        let text = c.serialise();
        assert_eq!(&text, "WWWWWWWWWBBBRRRRRRRRRGGGGGGGGGOOOOOOOOOBBBBBBYYYYYYYYY");
        let mut c = Cube::new();
        c.deserialise(&text).expect("deserialise failed");
        let text2 = c.serialise();
        assert_eq!(&text, &text2);
        c.twist(Twist{face:TOP, reverse:true});
        let text = c.serialise();
        assert_eq!(&text, "WWWWWWWWWRRRRRRRRRGGGGGGGGGOOOOOOOOOBBBBBBBBBYYYYYYYYY");
        let mut c = Cube::new();
        assert!(c.deserialise("BOOB").is_err());
    }

    #[test]
    fn parse_twists(){
        let t = Twist::from_string("t'").unwrap();
        assert_eq!(t.face, TOP);
        assert_eq!(t.reverse, true);
        let t = Twist::from_string("L").unwrap();
        assert_eq!(t.face, LEFT);
        assert_eq!(t.reverse, false);
        let t = Twist::from_string("s'").unwrap();
        assert_eq!(t.face, CENTER_FB);
        assert_eq!(t.reverse, true);
        let t = Twist::from_string("Ba'").unwrap();
        assert_eq!(t.face, BACK);
        assert_eq!(t.reverse, true);
        let t = Twist::from_string("bO").unwrap();
        assert_eq!(t.face, BOTTOM);
        assert_eq!(t.reverse, false);
    }

    #[cfg(not(feature="without_std"))]
    #[test]
    fn parse_twist_seq(){
        let j_b_pll = Twist::seq_from_string("R U2 R' U' R U'2 L' U R' U'").unwrap();
        let superflip1 = Twist::seq_from_string("U R2 F B R B2 R U2 L B2 R U' D' R2 F R' L B2 U2 F2").unwrap();
        let superflip2 = Twist::seq_from_string("S U B2 D2 M D' M2 S U R2 D M2 U B2 U S2").unwrap();
        // QSTM lengths:
        assert_eq!(j_b_pll.len(), 12);
        assert_eq!(superflip1.len(), 28);
        assert_eq!(superflip2.len(), 23);
        // Sequences
        for (a,b) in j_b_pll.iter().zip(vec![
                Twist{face:RIGHT, reverse: false},
                Twist{face:TOP, reverse: false},
                Twist{face:TOP, reverse: false},
                Twist{face:RIGHT, reverse: true},
                Twist{face:TOP, reverse: true},
                Twist{face:RIGHT, reverse: false},
                Twist{face:TOP, reverse: true},
                Twist{face:TOP, reverse: true},
                Twist{face:LEFT, reverse: true},
                Twist{face:TOP, reverse: false},
                Twist{face:RIGHT, reverse: true},
                Twist{face:TOP, reverse: true},
            ].iter()) {
            assert_eq!(a.face,b.face);
            assert_eq!(a.reverse,b.reverse);
        }
    }

    #[cfg(not(feature="without_std"))]
    #[test]
    fn do_twist_seq(){
        let mut c = Cube::new();
        let superflip = "S U B2 D2 M D' M2 S U R2 D M2 U B2 U S2";
        c.twists(superflip).expect("failed to twist");
        assert_eq!(c.simple_string(), "Top:\nYBY\nRYO\nYGY\nFront:\nGYG\nRGO\nGWG\nLeft:\nRYR\nBRG\nRWR\nBack:\nBYB\nOBR\nBWB\nRight:\nOYO\nGOB\nOWO\nBottom:\nWBW\nOWR\nWGW")
    }

    #[cfg(not(feature="without_std"))]
    #[test]
    fn test_twist_parse(){
        assert!(Twist::from_string("nonsense").is_err());
        assert!(Twist::from_string("Beeeeees").is_err());
        assert!(Twist::from_string("").is_err());
    }

    #[cfg(not(feature="without_std"))]
    #[test]
    fn test_twist_display(){
        assert_eq!("U'".to_string(), format!("{}", Twist::from_string("U'").unwrap()));
        assert_eq!("R".to_string(), format!("{}", Twist::from_string("R").unwrap()));
    }

    #[cfg(not(feature="without_std"))]
    #[test]
    fn solve_check(){
        let mut c = Cube::new();
        assert!(c.is_solved());
        c.twist(Twist::from_string("U").unwrap());
        assert!(!c.is_solved());
        c.twist(Twist::from_string("U'").unwrap());
        assert!(c.is_solved());
    }

    #[cfg(not(feature="without_std"))]
    #[test]
    fn output_map(){
        let out = crate::serialise_output_map(
            &[crate::Output{face:1, subface:2};45]
        );
        assert_eq!("121212121212121212121212121212121212121212121212121212121212121212121212121212121212121212".to_string(), out)
    }
}

