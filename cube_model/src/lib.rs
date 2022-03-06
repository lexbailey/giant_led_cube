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

#[derive(Copy, Clone, Debug)]
pub struct Twist{
    pub face: usize
    ,pub reverse: bool
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

    pub fn deserialise(data: &str) -> Cube {
        let mut c = Cube::new();
        let mut i: usize = 0;
        for face in &mut c.faces[0..6]{
            for sface in &mut face.subfaces{
                let col = Colors::from_shortname(&data[i..i+1]);
                sface.color = col;
                sface.next_color = col;
                i+=1;
            }
        }
        c
    }

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


        for face in &self.faces{
            for s in &face.subfaces{
                assert!(s.color == s.next_color);
            }
        }
    }

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

type SwitchMap5Faces = [Twist;48];

type OutputMap5Faces = [Output;45];

#[cfg(test)]
mod tests {
    use crate::Cube;

    #[test]
    fn cube_init() {
        let cube = Cube::new();
        let result = cube.simple_string();
        assert_eq!(result, "Top:\nWWW\nWWW\nWWW\nFront:\nRRR\nRRR\nRRR\nLeft:\nGGG\nGGG\nGGG\nBack:\nOOO\nOOO\nOOO\nRight:\nBBB\nBBB\nBBB\nBottom:\nYYY\nYYY\nYYY".to_string());
    }

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

    #[test]
    fn ser_deser(){
        use crate::{Twist, TOP};
        let mut c = Cube::new();
        c.twist(Twist{face:TOP, reverse:false});
        let text = c.serialise();
        assert_eq!(&text, "WWWWWWWWWBBBRRRRRRRRRGGGGGGGGGOOOOOOOOOBBBBBBYYYYYYYYY");
        let mut c = Cube::deserialise(&text);
        let text2 = c.serialise();
        assert_eq!(&text, &text2);
        c.twist(Twist{face:TOP, reverse:true});
        let text = c.serialise();
        assert_eq!(&text, "WWWWWWWWWRRRRRRRRRGGGGGGGGGOOOOOOOOOBBBBBBBBBYYYYYYYYY");
    }
}

