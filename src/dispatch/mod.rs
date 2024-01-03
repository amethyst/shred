#[cfg(feature = "parallel")]
pub use self::async_dispatcher::AsyncDispatcher;
#[cfg(feature = "parallel")]
pub use self::par_seq::{Par, ParSeq, RunWithPool, Seq};
pub use self::{
    batch::{
        BatchAccessor, BatchController, BatchUncheckedWorld, MultiDispatchController,
        MultiDispatcher,
    },
    builder::DispatcherBuilder,
    dispatcher::Dispatcher,
    send_dispatcher::SendDispatcher,
};

#[cfg(feature = "parallel")]
mod async_dispatcher;
mod batch;
mod builder;
mod dispatcher;
#[cfg(feature = "parallel")]
mod par_seq;
mod send_dispatcher;
mod stage;
mod util;
