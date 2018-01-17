use std::borrow::Borrow;
use std::sync::Arc;

use parking_lot::{Condvar, Mutex};
use rayon::ThreadPool;

use dispatch::dispatcher::ThreadLocal;
use dispatch::stage::Stage;
use res::Resources;

const ERR_NO_DISPATCH: &str = "wait() called before dispatch";

/// Like, `Dispatcher` but works
/// asynchronously.
pub struct AsyncDispatcher<'a, R> {
    res: Arc<R>,
    lock: Arc<Locker>,
    stages: Arc<Mutex<Vec<Stage<'static>>>>,
    thread_local: ThreadLocal<'a>,
    thread_pool: Arc<ThreadPool>,
}

pub fn new_async<'a, R>(
    res: R,
    stages: Vec<Stage<'static>>,
    thread_local: ThreadLocal<'a>,
    thread_pool: Arc<ThreadPool>,
) -> AsyncDispatcher<'a, R> {
    AsyncDispatcher {
        res: Arc::new(res),
        lock: Arc::new(Locker::new()),
        stages: Arc::new(Mutex::new(stages)),
        thread_local: thread_local,
        thread_pool: thread_pool,
    }
}

impl<'a, R> AsyncDispatcher<'a, R>
where
    R: Borrow<Resources> + Send + Sync + 'static,
{
    /// Dispatches the systems asynchronously.
    /// Does not execute thread local systems.
    ///
    /// If you want to wait for the systems to finish,
    /// call `wait()`.
    pub fn dispatch(&mut self) {
        let lock = self.lock.clone();
        let stages = self.stages.clone();
        let res = self.res.clone();

        lock.start();
        self.thread_pool.spawn(move || {
            {
                let stages = stages;
                let mut stages = stages.lock();

                let res = res.as_ref().borrow();

                for stage in &mut *stages {
                    stage.execute(res);
                }
            }
            lock.done();
        })
    }

    /// Waits for all the asynchronously dispatched systems to finish
    /// and executes thread local systems (if there are any).
    pub fn wait(&mut self) {
        self.wait_without_tl();

        let res = self.res.as_ref().borrow();

        for sys in &mut self.thread_local {
            sys.run_now(res);
        }
    }

    /// Waits for all the asynchronously dispatched systems to finish
    /// without executing thread local systems.
    pub fn wait_without_tl(&mut self) {
        self.lock.wait();
    }

    /// Checks if any of the asynchronously dispatched systems are running.
    pub fn is_running(&self) -> bool {
        self.lock.is_running()
    }

    /// Dispatch only thread local systems sequentially.
    ///
    /// If there are asynchronously dispatched systems running this method will wait for them.
    pub fn dispatch_thread_local(&mut self) {
        self.wait_without_tl();

        let res = self.res.as_ref().borrow();

        for sys in &mut self.thread_local {
            sys.run_now(res);
        }
    }

    /// Returns the resources.
    ///
    /// If there are asynchronously dispatched systems running this method will wait for them.
    pub fn mut_res(&mut self) -> &mut R {
        self.wait_without_tl();

        Arc::get_mut(&mut self.res).expect(ERR_NO_DISPATCH)
    }
}

struct Locker {
    running: Mutex<bool>,
    cvar: Condvar,
}

impl Locker {
    fn new() -> Self {
        Locker {
            running: Mutex::new(false),
            cvar: Condvar::new(),
        }
    }

    fn is_running(&self) -> bool {
        *self.running.lock()
    }

    fn start(&self) {
        *self.running.lock() = true;
    }

    fn done(&self) {
        *self.running.lock() = false;
        self.cvar.notify_one();
    }

    fn wait(&self) {
        let mut running = self.running.lock();
        if *running {
            self.cvar.wait(&mut running);
        }
    }
}
