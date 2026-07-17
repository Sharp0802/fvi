//! A piece table implemented with an implicit treap.

mod cx;
mod iter;
mod main;
mod node;
mod state;
mod version;

const NIL: usize = usize::MAX;

pub use cx::Context;
pub use iter::Iter;
pub use main::Table;
pub use version::Version;
