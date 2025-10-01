#![feature(async_fn_traits)]
#![feature(unboxed_closures)]

mod event;
mod span;
mod channel;
mod macros;
mod handler;

pub use event::Event;
pub use event::Level;

pub use span::Span;
pub use span::SpanScope;
pub use span::SpanData;

pub use handler::Handler;
pub use handler::GlobalHandler;
