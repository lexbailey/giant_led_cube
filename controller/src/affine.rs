use std::ops;
use std::fmt;

#[derive(Debug)]
pub struct Transform<T: Copy + ops::Neg<Output=T> + ops::Add<Output=T> + ops::Mul<Output=T> + From<f32> + Into<f32> + fmt::Display> {
    pub data: [T;16]
}

pub struct Vec4<T: Copy + ops::Neg<Output=T> + ops::Add<Output=T> + ops::Mul<Output=T> + From<f32> + Into<f32> + fmt::Display> {
    pub data: [T;4]
}

impl<T: Copy + ops::Neg<Output=T> + ops::Add<Output=T> + ops::Mul<Output=T> + From<f32> + Into<f32> + fmt::Display> fmt::Display for Transform<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let a = self.data;
        write!(f, "Transform[\n{},{},{},{},\n{},{},{},{},\n{},{},{},{},\n{},{},{},{},\n]",
            a[0],a[1],a[2],a[3],a[4],a[5],a[6],a[7],a[8],a[9],a[10],a[11],a[12],a[13],a[14],a[15],
        )
    }
}

impl<T: Copy + ops::Neg<Output=T> + ops::Add<Output=T> + ops::Mul<Output=T> + From<f32> + Into<f32> + fmt::Display> ops::Mul for &Transform<T>{
    type Output = Transform<T>;

    fn mul(self, b: &Transform<T>) -> Transform<T> {
        let a = self.data;
        let b = b.data;
        Transform{data:[
            ((a[0]*b[0])+(a[1]*b[4])+(a[2]*b[8])+(a[3]*b[12])),((a[0]*b[1])+(a[1]*b[5])+(a[2]*b[9])+(a[3]*b[13])),((a[0]*b[2])+(a[1]*b[6])+(a[2]*b[10])+(a[3]*b[14])),((a[0]*b[3])+(a[1]*b[7])+(a[2]*b[11])+(a[3]*b[15])),
            ((a[4]*b[0])+(a[5]*b[4])+(a[6]*b[8])+(a[7]*b[12])),((a[4]*b[1])+(a[5]*b[5])+(a[6]*b[9])+(a[7]*b[13])),((a[4]*b[2])+(a[5]*b[6])+(a[6]*b[10])+(a[7]*b[14])),((a[4]*b[3])+(a[5]*b[7])+(a[6]*b[11])+(a[7]*b[15])),
            ((a[8]*b[0])+(a[9]*b[4])+(a[10]*b[8])+(a[11]*b[12])),((a[8]*b[1])+(a[9]*b[5])+(a[10]*b[9])+(a[11]*b[13])),((a[8]*b[2])+(a[9]*b[6])+(a[10]*b[10])+(a[11]*b[14])),((a[8]*b[3])+(a[9]*b[7])+(a[10]*b[11])+(a[11]*b[15])),
            ((a[12]*b[0])+(a[13]*b[4])+(a[14]*b[8])+(a[15]*b[12])),((a[12]*b[1])+(a[13]*b[5])+(a[14]*b[9])+(a[15]*b[13])),((a[12]*b[2])+(a[13]*b[6])+(a[14]*b[10])+(a[15]*b[14])),((a[12]*b[3])+(a[13]*b[7])+(a[14]*b[11])+(a[15]*b[15])),
        ]}
    }
}

impl<T: Copy + ops::Neg<Output=T> + ops::Add<Output=T> + ops::Mul<Output=T> + From<f32> + Into<f32> + fmt::Display> Vec4<T>{
    pub fn transform(self, b: &Transform<T>) -> Vec4<T> {
        let a = self.data;
        let b = b.data;
        Vec4{data:[
            ((a[0]*b[0]) + (a[1]*b[1]) + (a[2]*b[2]) + (a[3]*b[3]))
            ,((a[0]*b[4]) + (a[1]*b[5]) + (a[2]*b[6]) + (a[3]*b[7]))
            ,((a[0]*b[8]) + (a[1]*b[9]) + (a[2]*b[10]) + (a[3]*b[11]))
            ,((a[0]*b[12]) + (a[1]*b[13]) + (a[2]*b[14]) + (a[3]*b[15]))
        ]}
    }
}

impl<T: Copy + ops::Neg<Output=T> + ops::Add<Output=T> + ops::Mul<Output=T> + From<f32> + Into<f32> + fmt::Display> Transform<T>{
    pub fn none() -> Transform<T>{
        let one = T::from(1.0);
        let zero = T::from(0.0);
        Transform{data:[
            one,zero,zero,zero
            ,zero,one,zero,zero
            ,zero,zero,one,zero
            ,zero,zero,zero,one
        ]}
    }

    pub fn translate(x:T, y:T, z:T) -> Transform<T>{
        let one = T::from(1.0);
        let zero = T::from(0.0);
        Transform{data:[
            one, zero, zero, x
            ,zero, one, zero, y
            ,zero, zero, one, z
            ,zero, zero, zero, one
        ]}
    }

    pub fn scale(x:T, y:T, z:T) -> Transform<T>{
        let one = T::from(1.0);
        let zero = T::from(0.0);
        Transform{data:[
            x, zero, zero, zero
            ,zero, y, zero, zero
            ,zero, zero, z, zero
            ,zero, zero, zero, one
        ]}
    }

    pub fn rotate_ypr(yaw:T, pitch:T, roll:T) -> Transform<T>{
        let one = T::from(1.0);
        let zero = T::from(0.0);
        let sroll = T::from(roll.into().sin());
        let croll = T::from(roll.into().cos());
        let spitch = T::from(pitch.into().sin());
        let cpitch = T::from(pitch.into().cos());
        let syaw = T::from(yaw.into().sin());
        let cyaw = T::from(yaw.into().cos());
        let yaw = Transform{data:[
            cyaw, -syaw, zero, zero
            ,syaw, cyaw, zero, zero
            ,zero, zero, one, zero
            ,zero, zero, zero, one
        ]};
        let pitch = Transform{data:[
            cpitch, zero, spitch, zero
            ,zero, one, zero, zero
            ,-spitch, zero, cpitch, zero
            ,zero, zero, zero, one
        ]};
        let roll = Transform{data:[
            one, zero, zero, zero
            ,zero, croll, -sroll, zero
            ,zero, sroll, croll, zero
            ,zero, zero, zero, one
        ]};
        &(&yaw*&pitch)*&roll
    }

    pub fn rotate_xyz(x:T, y:T, z:T) -> Transform<T>{
        Transform::rotate_ypr(z,y,x)
    }
}
