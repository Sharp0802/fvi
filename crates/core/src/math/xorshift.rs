/// Do xorshift for [`u16`].
#[must_use]
pub const fn xorshift16(mut x: u16) -> u16 {
    x ^= x << 7;
    x ^= x >> 9;
    x ^= x << 8;
    x
}

/// Do xorshift for [`u32`].
#[must_use]
pub const fn xorshift32(mut x: u32) -> u32 {
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    x
}

/// Do xorshift for [`u64`].
#[must_use]
pub const fn xorshift64(mut x: u64) -> u64 {
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x
}

/// Do xorshift for [`usize`].
#[must_use]
pub const fn xorshift(x: usize) -> usize {
    #![expect(clippy::cast_possible_truncation, reason = "pointer width checked")]

    #[cfg(not(any(
        target_pointer_width = "64",
        target_pointer_width = "32",
        target_pointer_width = "16"
    )))]
    compile_error!("unsupported target pointer width");

    #[cfg(target_pointer_width = "64")]
    return xorshift64(x as u64) as usize;
    #[cfg(target_pointer_width = "32")]
    return xorshift32(x as u32) as usize;
    #[cfg(target_pointer_width = "16")]
    return xorshift16(x as u16) as usize;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exhaustive() {
        _ = xorshift16(0);
        _ = xorshift32(0);
        _ = xorshift64(0);
    }
}
