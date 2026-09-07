/// Do keyed split-mix for 64-bit integer.
#[must_use]
pub const fn shuffle64(mut x: u64, key: u64) -> u64 {
    x = x.wrapping_add(key);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    x
}

/// Do keyed split-mix for 32-bit integer.
#[must_use]
pub const fn shuffle32(mut x: u32, key: u32) -> u32 {
    x = x.wrapping_add(key);
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^= x >> 16;
    x
}

/// Shuffles [`usize`] with key, using bijection function.
#[must_use]
pub const fn shuffle(x: usize, key: usize) -> usize {
    #![expect(clippy::cast_possible_truncation, reason = "pointer width checked")]

    #[cfg(not(any(target_pointer_width = "64", target_pointer_width = "32",)))]
    compile_error!("unsupported target pointer width");

    #[cfg(target_pointer_width = "64")]
    return shuffle64(x as u64, key as u64) as usize;
    #[cfg(target_pointer_width = "32")]
    return shuffle32(x as u32, key as u32) as usize;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exhaustive() {
        _ = shuffle32(1, 0);
        _ = shuffle64(1, 0);
    }
}
