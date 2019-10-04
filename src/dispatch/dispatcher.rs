use smallvec::SmallVec;

use crate::{dispatch::stage::Stage, system::RunNow, world::World};

/// This wrapper is used to share a replaceable ThreadPool with other
/// dispatchers. Useful with batch dispatchers.
#[cfg(feature = "parallel")]
pub type ThreadPoolWrapper = Option<::std::sync::Arc<::rayon::ThreadPool>>;

/// The dispatcher struct, allowing
/// systems to be executed in parallel.
pub struct Dispatcher<'a, 'b> {
    stages: Vec<Stage<'a>>,
    thread_local: ThreadLocal<'b>,
    #[cfg(feature = "parallel")]
    thread_pool: ::std::sync::Arc<::std::sync::RwLock<ThreadPoolWrapper>>,
}

impl<'a, 'b> Dispatcher<'a, 'b> {
    /// Sets up all the systems which means they are gonna add default values
    /// for the resources they need.
    pub fn setup(&mut self, world: &mut World) {
        for stage in &mut self.stages {
            stage.setup(world);
        }

        for sys in &mut self.thread_local {
            sys.setup(world);
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

        for sys in self.thread_local {
            sys.dispose(world);
        }
    }

    /// Dispatch all the systems with given resources and context
    /// and then run thread local systems.
    ///
    /// This function automatically redirects to
    ///
    /// * [Dispatcher::dispatch_par] in case it is supported
    /// * [Dispatcher::dispatch_seq] otherwise
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

        self.dispatch_thread_local(world);
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

    /// Dispatch only thread local systems sequentially.
    ///
    /// Please note that this method assumes that no resource
    /// is currently borrowed. If that's the case, it panics.
    pub fn dispatch_thread_local(&mut self, world: &World) {
        for sys in &mut self.thread_local {
            sys.run_now(world);
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

impl<'a, 'b, 'c> RunNow<'a> for Dispatcher<'b, 'c> {
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

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SystemId(pub usize);

pub type SystemExecSend<'b> = Box<dyn for<'a> RunNow<'a> + Send + 'b>;
pub type ThreadLocal<'a> = SmallVec<[Box<dyn for<'b> RunNow<'b> + 'a>; 4]>;

#[cfg(feature = "parallel")]
pub fn new_dispatcher<'a, 'b>(
    stages: Vec<Stage<'a>>,
    thread_local: ThreadLocal<'b>,
    thread_pool: ::std::sync::Arc<::std::sync::RwLock<ThreadPoolWrapper>>,
) -> Dispatcher<'a, 'b> {
    Dispatcher {
        stages,
        thread_local,
        thread_pool,
    }
}

#[cfg(not(feature = "parallel"))]
pub fn new_dispatcher<'a, 'b>(
    stages: Vec<Stage<'a>>,
    thread_local: ThreadLocal<'b>,
) -> Dispatcher<'a, 'b> {
    Dispatcher {
        stages,
        thread_local,
    }
}

#[cfg(test)]
mod tests {
    use crate::{dispatch::builder::DispatcherBuilder, system::*, world::*};

    #[derive(Default)]
    struct Res(i32);

    struct Dummy(i32);

    impl<'a> System<'a> for Dummy {
        type SystemData = Write<'a, Res>;

        fn run(&mut self, mut data: Self::SystemData) {
            if self.0 == 4 {
                // In second stage

                assert_eq!(data.0, 6);
            } else if self.0 == 5 {
                // In second stage

                assert_eq!(data.0, 10);
            }

            data.0 += self.0;
        }
    }

    struct Panic;

    impl<'a> System<'a> for Panic {
        type SystemData = ();

        fn run(&mut self, _: Self::SystemData) {
            panic!("Propagated panic");
        }
    }

    fn new_builder() -> DispatcherBuilder<'static, 'static> {
        DispatcherBuilder::new()
            .with(Dummy(0), "0", &[])
            .with(Dummy(1), "1", &[])
            .with(Dummy(2), "2", &[])
            .with(Dummy(3), "3", &["1"])
            .with_barrier()
            .with(Dummy(4), "4", &[])
            .with(Dummy(5), "5", &["4"])
    }

    fn new_world() -> World {
        let mut world = World::empty();
        world.insert(Res(0));

        world
    }

    #[test]
    #[should_panic(expected = "Propagated panic")]
    fn dispatcher_panics() {
        DispatcherBuilder::new()
            .with(Panic, "p", &[])
            .build()
            .dispatch(&new_world())
    }

    #[test]
    fn stages() {
        let mut d = new_builder().build();

        d.dispatch(&new_world());
    }

    #[test]
    #[cfg(feature = "parallel")]
    fn stages_async() {
        let mut d = new_builder().build_async(new_world());

        d.dispatch();
    }
}
