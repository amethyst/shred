#[cfg(feature = "parallel")]
pub use self::async::AsyncDispatcher;
pub use self::builder::DispatcherBuilder;
pub use self::dispatcher::Dispatcher;
#[cfg(feature = "parallel")]
pub use self::par_seq::{Par, ParSeq, RunWithPool, Seq};

#[cfg(feature = "parallel")]
mod async;
mod builder;
mod dispatcher;
#[cfg(feature = "parallel")]
mod par_seq;
mod stage;
mod util;
