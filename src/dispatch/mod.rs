#[cfg(feature = "parallel")]
pub use self::async_dispatcher::AsyncDispatcher;
#[cfg(feature = "parallel")]
pub use self::par_seq::{Par, ParSeq, RunWithPool, Seq};
pub use self::{builder::DispatcherBuilder, dispatcher::Dispatcher};

#[cfg(feature = "parallel")]
mod async_dispatcher;
mod builder;
mod dispatcher;
#[cfg(feature = "parallel")]
mod par_seq;
mod stage;
mod util;
