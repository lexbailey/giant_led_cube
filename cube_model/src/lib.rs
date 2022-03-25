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

    fn copy_from(&mut self, other: [SubFace;9], doffset: isize, dstep: isize, soffset: isize, sstep: isize) {
        for i in 0..3{
            let s = (soffset + (i * sstep)) as usize;
            let d = (doffset + (i * dstep)) as usize;
            self.subfaces[d].next_color = other[s].color;
        }
    }

    fn update(&mut self){
        for s in &mut self.subfaces{
            s.color = s.next_color;
        }
    }

    fn twist(&mut self, reverse: bool){
        let f = &self.subfaces;
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
#[derive(Copy, Clone, Debug)]
pub struct Twist{
    pub face: usize
    ,pub reverse: bool
}

impl Twist{
    pub fn from_string(s: &str) -> Result<Twist, &'static str>{
        let s = s.as_bytes();
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
        let faces = [
            Face::new(Colors::White, [(BACK, 0,1), (RIGHT, 0,1), (FRONT, 0,1), (LEFT, 0,1)])
            ,Face::new(Colors::Red, [(TOP, 6,1), (RIGHT, 0,3), (BOTTOM, 6,1), (LEFT, 8,-3)])
            ,Face::new(Colors::Green, [(TOP, 0,3), (FRONT, 0,3), (BOTTOM, 8,-3), (BACK, 8,-3)])
            ,Face::new(Colors::Orange, [(TOP, 2,-1), (LEFT, 0,3), (BOTTOM, 2,-1), (RIGHT, 8,-3)])
            ,Face::new(Colors::Blue, [(TOP, 8,-3), (BACK, 0,3), (BOTTOM, 0,3), (FRONT, 8,-3)])
            ,Face::new(Colors::Yellow, [(FRONT, 6,1), (RIGHT, 6,1), (BACK, 6,1), (LEFT, 6,1)])
            // Fake faces, for manipulating centres, colour doesn't matter, it's never seen and represents nothing
            ,Face::new(Colors::White, [(LEFT, 7,-3), (TOP, 3,1), (RIGHT, 1,3), (BOTTOM, 3,1)])
            ,Face::new(Colors::White, [(BACK, 7,-3), (TOP, 1,3), (FRONT, 1,3), (BOTTOM, 7,-3)])
            ,Face::new(Colors::White, [(LEFT, 3,1), (FRONT, 3,1), (RIGHT, 3,1), (BACK, 3,1)])
        ];
        Cube{faces:faces}
    }

    pub fn get_color(&self, f: Output) -> Colors{
        self.faces[f.face].subfaces[f.subface].color
    }

    pub fn deserialise(&mut self, data: &str) {
        let mut i: usize = 0;
        for face in &mut self.faces[0..6]{
            for sface in &mut face.subfaces{
                let col = Colors::from_shortname(&data[i..i+1]);
                sface.color = col;
                sface.next_color = col;
                i+=1;
            }
        }
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

    pub fn twist(&mut self, twist: Twist){
        let face = twist.face;
        let reverse = twist.reverse;
        if face < FAKE_FACE_MIN {
            (&mut self.faces[face]).twist(reverse);
        }

        for i in 0..4{
            let (adj, doffset, dstep) = self.faces[face].adjacent[i];
            let (next, soffset, sstep) = self.faces[face].adjacent[((((i as isize) + if reverse {1} else {-1})+4)%4)as usize];
            let subs = self.faces[next].subfaces;
            self.faces[adj].copy_from(subs, doffset, dstep, soffset, sstep);
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
}

pub type SwitchMap5Faces = [Twist;48];

pub type OutputMap5Faces = [Output;45];

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
    fn ser_deser(){
        let mut c = Cube::new();
        c.twist(Twist{face:TOP, reverse:false});
        let text = c.serialise();
        assert_eq!(&text, "WWWWWWWWWBBBRRRRRRRRRGGGGGGGGGOOOOOOOOOBBBBBBYYYYYYYYY");
        let mut c = Cube::new();
        c.deserialise(&text);
        let text2 = c.serialise();
        assert_eq!(&text, &text2);
        c.twist(Twist{face:TOP, reverse:true});
        let text = c.serialise();
        assert_eq!(&text, "WWWWWWWWWRRRRRRRRRGGGGGGGGGOOOOOOOOOBBBBBBBBBYYYYYYYYY");
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
        c.twists(superflip);
        assert_eq!(c.simple_string(), "Top:\nYBY\nRYO\nYGY\nFront:\nGYG\nRGO\nGWG\nLeft:\nRYR\nBRG\nRWR\nBack:\nBYB\nOBR\nBWB\nRight:\nOYO\nGOB\nOWO\nBottom:\nWBW\nOWR\nWGW")
    }
}

