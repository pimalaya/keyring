//! Module gathering I/O-free, composable and iterable state machines.
//!
//! Flows emit [`crate::Io`] requests that need to be processed by
//! [`crate::handlers`] in order to continue their progression.

mod delete;
mod read;
mod write;

#[doc(inline)]
pub use self::{delete::Delete, read::Read, write::Write};
