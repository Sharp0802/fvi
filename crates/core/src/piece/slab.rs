use alloc::boxed::Box;
use alloc::vec;

use crate::piece::Node;
use crate::piece::ptr::Ptr;

#[derive(Debug, Clone, Copy)]
struct Slot {
    free: u16,
    node: Option<Node>,
}

const _: () = const {
    assert!(size_of::<Slot>() == 20);
    assert!(align_of::<Slot>() == 4);
};

impl Slot {
    const fn uninit() -> Self {
        Self {
            free: 0,
            node: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Slab {
    free: u16,
    len: u16,
    buf: Box<[Slot]>,
}

impl Slab {
    #[must_use]
    pub fn new(n: u16) -> Self {
        let mut buf = vec![const { Slot::uninit() }; n as usize].into_boxed_slice();

        for (i, slot) in buf.iter_mut().enumerate() {
            #[expect(clippy::cast_possible_truncation, reason = "N < u16::MAX")]
            (slot.free = (i + 1) as u16);
        }

        Self {
            free: 0,
            len: 0,
            buf,
        }
    }

    #[inline]
    #[must_use]
    pub const fn len(&self) -> u16 {
        self.len
    }

    #[inline]
    #[must_use]
    pub const fn insert(&mut self, node: Node) -> Ptr {
        if (self.len as usize) >= self.buf.len() {
            return Ptr::NIL;
        }

        let curr = self.free;

        let slot = &mut self.buf[curr as usize];
        let old = slot.node.replace(node);
        debug_assert!(old.is_none(), "free points non-free slot");

        self.free = slot.free;
        self.len += 1;

        Ptr::new(curr)
    }

    #[inline]
    #[must_use]
    pub const fn remove(&mut self, at: Ptr) -> Option<Node> {
        let Some(pos) = at.get() else {
            return None;
        };

        if self.len == 0 || (pos as usize) >= self.buf.len() {
            return None;
        }

        let ret = self.buf[pos as usize].node.take();
        if ret.is_some() {
            self.buf[pos as usize].free = core::mem::replace(&mut self.free, pos);
            self.len -= 1;
        }

        ret
    }

    #[inline]
    #[must_use]
    pub const fn get(&self, at: Ptr) -> Option<&Node> {
        match at.get() {
            Some(pos) => self.buf[pos as usize].node.as_ref(),
            None => None,
        }
    }

    #[inline]
    #[must_use]
    pub const fn get_mut(&mut self, at: Ptr) -> Option<&mut Node> {
        match at.get() {
            Some(pos) => self.buf[pos as usize].node.as_mut(),
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::piece::Ptr;
    use crate::view::{View, ViewSize};
    use proptest::collection::vec;
    use proptest::prelude::*;

    const N16: u16 = 1024;
    const N: usize = N16 as usize;

    #[derive(Debug, Clone, Copy)]
    enum Op {
        Insert(u16),
        Remove(u16),
    }

    fn arb_op() -> impl Strategy<Value = Op> {
        prop_oneof![
            any::<u16>().prop_map(Op::Insert),
            any::<u16>().prop_map(Op::Remove),
        ]
    }

    fn gen_actual(seed: u16) -> Node {
        Node {
            size: ViewSize::MAX,
            prv: Ptr::new(seed),
            lhs: Ptr::new(seed),
            rhs: Ptr::new(seed),
            #[expect(clippy::unwrap_used, reason = "cannot be failed")]
            desc: View::new(true, 0, 0, ViewSize::MAX).unwrap(),
        }
    }

    proptest! {
        #[test]
        fn match_models(ops in vec(arb_op(), 1..1024).prop_map(|mut ops| {
            ops.splice(0..0, [
                Op::Remove(0),
                Op::Insert(0xBEEF),
                Op::Remove(N16),
                Op::Remove(u16::MAX),
            ]);
            ops
        })) {
            let mut mock = [const { None }; N];
            let mut actual = Slab::new(N16);
            let mut len = 0;

            for op in ops {
                match op {
                    Op::Insert(val) => {
                        if let Some(id) = actual.insert(gen_actual(val)).get() {
                            let node = actual.get(Ptr::new(id)).expect("occupied slot expected");
                            prop_assert_eq!(node.lhs.get(), Ptr::new(val).get());
                            prop_assert!(mock[id as usize].replace(val).is_none());
                            len += 1;
                        } else {
                            prop_assert!(mock.iter().all(Option::is_some));
                        }
                    }
                    Op::Remove(pos) => {
                        let cmp = actual.remove(Ptr::new(pos)).map(|val| val.lhs);
                        prop_assert_eq!(mock.get_mut(pos as usize).and_then(Option::take), cmp.and_then(Ptr::get));

                        if cmp.is_some() {
                            len -= 1;
                        }
                    }
                }

                prop_assert_eq!(len, actual.len());
            }
        }
    }
}
