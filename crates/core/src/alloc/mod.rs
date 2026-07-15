pub(crate) extern crate alloc as std;

mod slab;
mod slot;

const NIL: usize = usize::MAX;

pub use slab::Slab;
