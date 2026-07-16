//! A low-level piece table implemented with an implicit treap.

mod iter;
mod main;
mod node;
mod state;

const NIL: usize = usize::MAX;

pub use iter::{Iter, IterAll};
pub use main::Treap;
