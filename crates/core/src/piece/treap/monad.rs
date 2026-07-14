use core::ops::Deref;

use crate::piece::node::Node;
use crate::piece::ptr::Ptr;
use crate::piece::slab::Slab;
use crate::unreachable;

/// Monad to express occupied slot.
///
/// Internal usage only:
/// It doesn't guarantee that slot is actually occupied after removal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub(super) struct Occupied(Ptr);

impl Deref for Occupied {
    type Target = Ptr;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub(super) trait Key {
    type Output<T>;

    fn get_from<const N: usize>(self, slab: &Slab<N>) -> Self::Output<&Node>;
    fn get_mut_from<const N: usize>(self, slab: &mut Slab<N>) -> Self::Output<&mut Node>;
}

impl Key for Occupied {
    type Output<T> = T;

    #[inline]
    fn get_from<const N: usize>(self, slab: &Slab<N>) -> &Node {
        slab.get(self.0).unwrap_or_else(|| unreachable())
    }

    #[inline]
    fn get_mut_from<const N: usize>(self, slab: &mut Slab<N>) -> &mut Node {
        slab.get_mut(self.0).unwrap_or_else(|| unreachable())
    }
}

impl Key for Ptr {
    type Output<T> = Option<(T, Occupied)>;

    #[inline]
    fn get_from<const N: usize>(self, slab: &Slab<N>) -> Self::Output<&Node> {
        slab.get(self).map(|t| (t, Occupied(self)))
    }

    #[inline]
    fn get_mut_from<const N: usize>(self, slab: &mut Slab<N>) -> Self::Output<&mut Node> {
        slab.get_mut(self).map(|t| (t, Occupied(self)))
    }
}
