use std::fmt;
use std::ops::Add;

#[derive(Clone, Copy)]
pub struct V3I {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl V3I {
    pub fn create(x: i32, y: i32, z: i32) -> Self {
        V3I { x: x, y: y, z: z }
    }
    pub fn zero() -> Self {
        V3I::create(0, 0, 0)
    }
}

impl Add for V3I {
    type Output = V3I;

    fn add(self, other: V3I) -> V3I {
        V3I::create(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

impl fmt::Debug for V3I {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}
