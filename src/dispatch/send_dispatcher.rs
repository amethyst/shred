#[cfg(feature = "parallel")]
use crate::dispatch::dispatcher::ThreadPoolWrapper;
use crate::{dispatch::stage::Stage, system::RunNow, world::World};

/// `Send`able version of [`Dispatcher`](crate::dispatch::Dispatcher).
///
/// Can't hold thread local systems.
///
/// Create using [`Dispatcher::try_into_sendable`](crate::dispatch::Dispatcher::try_into_sendable).
pub struct SendDispatcher<'a> {
    pub(super) stages: Vec<Stage<'a>>,
    #[cfg(feature = "parallel")]
    pub(super) thread_pool: ::std::sync::Arc<::std::sync::RwLock<ThreadPoolWrapper>>,
}

impl SendDispatcher<'_> {
    /// Sets up all the systems which means they are gonna add default values
    /// for the resources they need.
    pub fn setup(&mut self, world: &mut World) {
        for stage in &mut self.stages {
            stage.setup(world);
        }
    }

    /// Calls the `dispose` method of all systems and allows them to release
    /// external resources. It is common this method removes components and
    /// / or resources from the `World` which are associated with external
    /// resources.
    pub fn dispose(self, world: &mut World) {
        for stage in self.stages {
            stage.dispose(world);
        }
    }

    /// Dispatch all the systems with given resources and context
    /// and then run thread local systems.
    ///
    /// This function automatically redirects to
    ///
    /// * [SendDispatcher::dispatch_par] in case it is supported
    /// * [SendDispatcher::dispatch_seq] otherwise
    ///
    /// and runs `dispatch_thread_local` afterwards.
    ///
    /// Please note that this method assumes that no resource
    /// is currently borrowed. If that's the case, it panics.
    pub fn dispatch(&mut self, world: &World) {
        #[cfg(feature = "parallel")]
        self.dispatch_par(world);

        #[cfg(not(feature = "parallel"))]
        self.dispatch_seq(world);
    }

    /// Dispatches the systems (except thread local systems)
    /// in parallel given the resources to operate on.
    ///
    /// This operation blocks the
    /// executing thread.
    ///
    /// Only available with "parallel" feature enabled.
    ///
    /// Please note that this method assumes that no resource
    /// is currently borrowed. If that's the case, it panics.
    #[cfg(feature = "parallel")]
    pub fn dispatch_par(&mut self, world: &World) {
        let stages = &mut self.stages;

        self.thread_pool
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .install(move || {
                for stage in stages {
                    stage.execute(world);
                }
            });
    }

    /// Dispatches the systems (except thread local systems) sequentially.
    ///
    /// This is useful if parallel overhead is
    /// too big or the platform does not support multithreading.
    ///
    /// Please note that this method assumes that no resource
    /// is currently borrowed. If that's the case, it panics.
    pub fn dispatch_seq(&mut self, world: &World) {
        for stage in &mut self.stages {
            stage.execute_seq(world);
        }
    }

    /// This method returns the largest amount of threads this dispatcher
    /// can make use of. This is mainly for debugging purposes so you can see
    /// how well your systems can make use of multi-threading.
    #[cfg(feature = "parallel")]
    pub fn max_threads(&self) -> usize {
        self.stages
            .iter()
            .map(Stage::max_threads)
            .max()
            .unwrap_or(0)
    }
}

impl RunNow<'_> for SendDispatcher<'_> {
    fn run_now(&mut self, world: &World) {
        self.dispatch(world);
    }

    fn setup(&mut self, world: &mut World) {
        self.setup(world);
    }

    fn dispose(self: Box<Self>, world: &mut World) {
        (*self).dispose(world);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn send_dispatcher_is_send() {
        fn is_send<T: Send>() {}
        is_send::<super::SendDispatcher>();
    }
}
