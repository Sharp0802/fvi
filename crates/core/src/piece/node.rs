use core::fmt::Debug;

use crate::piece::Ptr;
use crate::view::{View, ViewSize};

#[derive(Debug, Clone, Copy)]
pub struct Node {
    pub size: ViewSize,
    pub prv: Ptr,
    pub lhs: Ptr,
    pub rhs: Ptr,
    pub desc: View,
}

const _: () = const {
    assert!(size_of::<Node>() == size_of::<Option<Node>>());
};
