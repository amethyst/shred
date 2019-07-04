#[cfg(feature = "parallel")]
pub use self::async_dispatcher::AsyncDispatcher;
#[cfg(feature = "parallel")]
pub use self::par_seq::{Par, ParSeq, RunWithPool, Seq};
pub use self::{
    batch_builder::BatchBuilder, batch_builder::BatchController, batch_builder::BatchSystem,
    builder::DispatcherBuilder, dispatcher::Dispatcher,
};

#[cfg(feature = "parallel")]
mod async_dispatcher;
mod batch_builder;
mod builder;
mod dispatcher;
#[cfg(feature = "parallel")]
mod par_seq;
mod stage;
mod util;
