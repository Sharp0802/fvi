//! Private GPU resources and pipelines.

mod args;
mod buffer;
mod bundle;
mod pipeline;
mod shape;
mod state;

use args::*;
pub use args::{Args, ArgsBuffer};
use buffer::*;
pub use bundle::*;
pub use pipeline::PipelineDescriptor;
use pipeline::*;
pub use shape::*;
pub use state::*;
