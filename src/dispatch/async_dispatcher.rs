use std::{
    borrow::Borrow,
    sync::{mpsc, Arc, RwLock},
};

use crate::{
    dispatch::{
        dispatcher::{ThreadLocal, ThreadPoolWrapper},
        stage::Stage,
    },
    world::World,
};
use std::borrow::BorrowMut;

pub fn new_async<'a, R>(
    world: R,
    stages: Vec<Stage<'static>>,
    thread_local: ThreadLocal<'a>,
    thread_pool: Arc<RwLock<ThreadPoolWrapper>>,
) -> AsyncDispatcher<'a, R> {
    AsyncDispatcher {
        data: Data::Inner(Inner { world, stages }),
        thread_local,
        thread_pool,
    }
}

/// Like, `Dispatcher` but works asynchronously.
pub struct AsyncDispatcher<'a, R> {
    data: Data<R>,
    thread_local: ThreadLocal<'a>,
    thread_pool: Arc<RwLock<ThreadPoolWrapper>>,
}

impl<'a, R> AsyncDispatcher<'a, R>
where
    R: Borrow<World> + Send + Sync + 'static,
{
    /// Sets up all the systems which means they are gonna add default values
    /// for the resources they need.
    pub fn setup(&mut self)
    where
        R: BorrowMut<World>,
    {
        let inner = self.data.inner();
        let stages = &mut inner.stages;
        let world = inner.world.borrow_mut();

        for stage in stages {
            stage.setup(world);
        }

        for sys in &mut self.thread_local {
            sys.setup(world);
        }
    }

    /// Dispatches the systems asynchronously.
    /// Does not execute thread local systems.
    ///
    /// If you want to wait for the systems to finish,
    /// call `wait()`.
    pub fn dispatch(&mut self) {
        let (snd, mut inner) = self.data.sender();

        self.thread_pool
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .spawn(move || {
                let world: &World = inner.world.borrow();

                for stage in &mut inner.stages {
                    stage.execute(world);
                }

                let _ = snd.send(inner);
            });
    }

    /// Waits for all the asynchronously dispatched systems to finish
    /// and executes thread local systems (if there are any).
    pub fn wait(&mut self) {
        let world = self.data.inner().world.borrow();

        for sys in &mut self.thread_local {
            sys.run_now(world);
        }
    }

    /// Waits for all the asynchronously dispatched systems to finish
    /// without executing thread local systems.
    ///
    /// See `wait` for executing thread local systems.
    pub fn wait_without_tl(&mut self) {
        self.data.inner();
    }

    /// Checks if any of the asynchronously dispatched systems are running.
    pub fn running(&mut self) -> bool {
        self.data.inner_noblock().is_none()
    }

    /// Returns the `World`.
    ///
    /// This will wait for the asynchronous systems to finish.
    ///
    /// Renamed to `self.world()`.
    #[deprecated(since = "0.8.0", note = "renamed to `world`")]
    pub fn res(&mut self) -> &R {
        &self.world()
    }

    /// Returns the `World`.
    ///
    /// This will wait for the asynchronous systems to finish.
    pub fn world(&mut self) -> &R {
        &self.data.inner().world
    }

    /// Borrows the `World` mutably.
    ///
    /// This will wait for the asynchronous systems to finish.
    ///
    /// Renamed to `self.world_mut()`.
    #[deprecated(since = "0.8.0", note = "renamed to `world_mut`")]
    pub fn mut_res(&mut self) -> &mut R {
        &mut self.data.inner().world
    }

    /// Borrows the `World` mutably.
    ///
    /// This will wait for the asynchronous systems to finish.
    pub fn world_mut(&mut self) -> &mut R {
        &mut self.data.inner().world
    }
}

enum Data<R> {
    Inner(Inner<R>),
    Rx(mpsc::Receiver<Inner<R>>),
}

impl<R> Data<R> {
    fn inner(&mut self) -> &mut Inner<R> {
        let new_self;

        match *self {
            Data::Inner(ref mut inner) => return inner,
            Data::Rx(ref mut rx) => {
                let inner = rx.recv().expect("Sender dropped");
                new_self = Data::Inner(inner);
            }
        }

        *self = new_self;

        self.inner()
    }

    fn inner_noblock(&mut self) -> Option<&mut Inner<R>> {
        use std::sync::mpsc::TryRecvError;

        let new_self;

        match *self {
            Data::Inner(ref mut inner) => return Some(inner),
            Data::Rx(ref mut rx) => {
                let inner = rx
                    .try_recv()
                    .map(Some)
                    .or_else(|e| match e {
                        TryRecvError::Empty => Ok(None),
                        TryRecvError::Disconnected => Err(e),
                    })
                    .expect("Sender dropped");
                match inner {
                    Some(inner) => new_self = Data::Inner(inner),
                    None => return None,
                }
            }
        }

        *self = new_self;

        self.inner_noblock()
    }

    fn sender(&mut self) -> (mpsc::Sender<Inner<R>>, Inner<R>) {
        use std::mem::replace;

        self.inner();

        let (snd, rx) = mpsc::channel();
        let inner = replace(&mut *self, Data::Rx(rx));
        let inner = match inner {
            Data::Inner(inner) => inner,
            Data::Rx(_) => unreachable!(),
        };

        (snd, inner)
    }
}

struct Inner<R> {
    stages: Vec<Stage<'static>>,
    world: R,
}
