//! Runtime components
//!
//! The traits and types within this module are used to allow plugging in
//! runtime types. These include:
//!
//! - Executors
//! - Timers

mod timer;
pub mod tokio;

pub use futures_util::io::{AsyncRead as Read, AsyncWrite as Write};

pub use self::{
    timer::{Sleep, Timer},
    tokio::{TokioExecutor, TokioIo},
};

/// An executor of futures.
///
/// This trait allows abstract over async runtimes. Implement this trait for your own type.
pub trait Executor<Fut> {
    /// Place the future into the executor to be run.
    fn execute(&self, fut: Fut);
}
