use core::fmt::Debug;
use core::mem::MaybeUninit;

pub(super) struct Slot<T> {
    next: usize,
    value: MaybeUninit<T>,
}

const MASK: usize = 1 << (usize::BITS - 1);

impl<T> Slot<T> {
    #[must_use]
    pub const fn new(next: usize, value: T) -> Self {
        assert!(next & MASK == 0);
        Self {
            next: MASK | next,
            value: MaybeUninit::new(value),
        }
    }

    pub const fn next(&self) -> usize {
        self.next & !MASK
    }

    pub const fn link(&mut self, value: usize) {
        assert!(value & MASK == 0, "bound exceeded");
        assert!(self.next & MASK == 0, "already written");
        self.next = value;
    }

    pub const fn write(&mut self, value: T) {
        assert!(self.next & MASK == 0, "already written");
        self.value.write(value);
        self.next |= MASK;
    }

    #[must_use]
    pub const fn take(&mut self) -> Option<T> {
        if self.next & MASK == MASK {
            self.next &= !MASK;
            #[expect(unsafe_code, reason = "invariant")]
            Some(unsafe { self.value.assume_init_read() })
        } else {
            None
        }
    }

    #[must_use]
    pub const fn as_ref(&self) -> Option<&T> {
        if self.next & MASK == MASK {
            #[expect(unsafe_code, reason = "invariant")]
            Some(unsafe { self.value.assume_init_ref() })
        } else {
            None
        }
    }

    #[must_use]
    pub const fn as_mut(&mut self) -> Option<&mut T> {
        if self.next & MASK == MASK {
            #[expect(unsafe_code, reason = "invariant")]
            Some(unsafe { self.value.assume_init_mut() })
        } else {
            None
        }
    }
}

impl<T> Drop for Slot<T> {
    fn drop(&mut self) {
        _ = self.take();
    }
}

impl<T: Debug> Debug for Slot<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Slot")
            .field("next", &(self.next & !MASK))
            .field("value", &self.as_ref())
            .finish()
    }
}

impl<T: Clone> Clone for Slot<T> {
    fn clone(&self) -> Self {
        let value = self
            .as_ref()
            .cloned()
            .map_or_else(MaybeUninit::uninit, MaybeUninit::new);

        Self {
            next: self.next,
            value,
        }
    }
}

#[cfg(test)]
mod tests {
    use core::fmt::Write;
    use std::string::String;

    use super::*;

    #[test]
    fn exhaustive() {
        let mut slot = Slot::new(0, 0u32);
        assert!(slot.take().is_some());
        assert!(slot.as_mut().is_none());

        let mut buf = String::new();
        write!(buf, "{slot:?}").expect("cannot fail");
        assert!(buf.contains("value: None"));
    }
}
