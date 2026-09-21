#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Unit(i32);

impl Unit {
    // 4 unit per 1 physical pixel
    const PRECI: i32 = 4;
    const PRECF: f32 = 4.0;
}

impl From<f32> for Unit {
    fn from(value: f32) -> Self {
        Self((value * Unit::PRECF).round() as i32)
    }
}

impl From<Unit> for f32 {
    fn from(value: Unit) -> Self {
        let fract = (value.0 % Unit::PRECI) as f32;
        let int = (value.0 / Unit::PRECI) as f32;
        fract / Unit::PRECF + int
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_samples() {
        assert_eq!(Unit::from(0.1), Unit(0));
        assert_eq!(Unit::from(1.1), Unit(4));
        assert_eq!(Unit::from(0.2), Unit(1));
        assert_eq!(Unit::from(4.4), Unit(18));
    }
}
