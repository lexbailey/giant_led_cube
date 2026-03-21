use std::ops;
use std::fmt;

#[derive(Debug)]
/**
    A polymorphic 4x4 matrix that represents an affine transform.

    `T` must be `f32`, or another type that can be converted to and from `f32`

    for example:

    ```
    # use std::f32;
    # use affine::{Transform,Vec4};
    /* Rotate a vector by 45 degrees
           result
       (0.707, 0.707)
       ^
      /      < counterclockwise rotation by 45 degrees
     /
    o---->(1,0)
          input
    */
    let t = Transform::rotate_xyz(0.0, 0.0, f32::consts::PI/4.0);
    let v = Vec4::new([1.0,0.0,0.0,0.0]); // Unit vector in X axis
        
    assert_eq!(v.transform(&t), Vec4::new([0.70710677, 0.70710677, 0.0, 0.0]));
    ```
*/
#[derive(PartialEq)]
pub struct Transform<T: Copy + ops::Neg<Output=T> + ops::Add<Output=T> + ops::Mul<Output=T> + From<f32> + Into<f32> + fmt::Display> {
    pub data: [T;16]
}

/**
    A length 4 vector that can be transformed with a [`Transform<T>`]
*/
#[derive(PartialEq,Debug)]
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

    pub fn new(data: [T;4]) -> Vec4<T>{
        Vec4{
            data
        }
    }

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

    /// Create a new Transform from the raw matrix data (rather than building it from a transform operation)
    pub fn new(data:[T;16]) -> Transform<T>{
        Transform{ data }
    }

    /// Creates an identity matrix
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

    /// Creates a translation matrix. Translates the specified amount in x, y, and z
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

    /// Creates a scale transform matrix, with independent scale factors in x, y, and z
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

    /**
        Creates a rotation matrix with the given yaw, pitch, and roll.
        
        X is the horizontal (left to right, pitch) axis
        
        Y is the front to back (roll) axis
        
        Z is the vertical (yaw) axis

        yaw, pitch, and roll are specified in radians
        for example: one full rotation for a `Transform<f32>` is `f32::consts::TAU`

        for types `T` that are not `f32`, the value is converted to `f32` for the sine and cosine calculation, and the result is converted back to type `T`
    */
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

    /// Synonym for [`Transform<T>::rotate_ypr()`], except that the arguments are in reverse order
    pub fn rotate_xyz(x:T, y:T, z:T) -> Transform<T>{
        Transform::rotate_ypr(z,y,x)
    }
}
