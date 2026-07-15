use core::fmt::Debug;
use core::mem::MaybeUninit;

use crate::alloc::NIL;

pub struct Slot<T> {
    next: usize,
    value: MaybeUninit<T>,
}

impl<T> Slot<T> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            next: 0,
            value: MaybeUninit::uninit(),
        }
    }

    pub const fn next(&self) -> usize {
        self.next
    }

    pub const fn link(&mut self, value: usize) {
        assert!(self.next != NIL);
        self.next = value;
    }

    pub const fn write(&mut self, value: T) {
        assert!(self.next != NIL);
        self.value.write(value);
        self.next = NIL;
    }

    #[must_use]
    pub const fn take(&mut self) -> Option<T> {
        if self.next == NIL {
            self.next = 0;
            #[expect(unsafe_code, reason = "invariant")]
            Some(unsafe { self.value.assume_init_read() })
        } else {
            None
        }
    }

    #[must_use]
    pub const fn as_ref(&self) -> Option<&T> {
        if self.next == NIL {
            #[expect(unsafe_code, reason = "invariant")]
            Some(unsafe { self.value.assume_init_ref() })
        } else {
            None
        }
    }

    #[must_use]
    pub const fn as_mut(&mut self) -> Option<&mut T> {
        if self.next == NIL {
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
            .field("free", &self.next)
            .field("value", &self.as_ref())
            .finish()
    }
}

impl<T: Clone> Clone for Slot<T> {
    fn clone(&self) -> Self {
        let value = if let Some(value) = self.as_ref() {
            MaybeUninit::new(value.clone())
        } else {
            MaybeUninit::uninit()
        };

        Self {
            next: self.next,
            value,
        }
    }
}
