#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Unit(i32);

impl Unit {
    // 4 unit per 1 physical pixel
    const PRECI: i32 = 4;
    const PRECF: f32 = 4.0;

    pub fn fract(&self) -> Self {
        Self(self.0 % Unit::PRECI)
    }
}

impl From<f32> for Unit {
    fn from(value: f32) -> Self {
        Self((value.fract() * Unit::PRECF) as i32)
    }
}

impl From<Unit> for f32 {
    fn from(value: Unit) -> Self {
        let fract = (value.0 % Unit::PRECI) as f32;
        let int = (value.0 / Unit::PRECI) as f32;
        fract + int
    }
}
