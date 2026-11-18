//! A small persistent worker pool: `run(f)` calls `f(i)` once for every worker index i in 0..threads, the
//! calling thread taking index 0, and returns when all have finished. Workers sleep between jobs; after
//! a job they spin briefly first, so back-to-back calls do not pay a wake-up each.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

type Job = dyn Fn(usize) + Sync;

struct Shared {
    lock: Mutex<Slot>,
    wake: Condvar,
    done: Condvar,
    /// bumped for every job; workers spin on it before sleeping
    gen: AtomicUsize,
    /// workers still running the current job
    running: AtomicUsize,
}

struct Slot {
    /// the current job, lifetime erased: valid until `running` is 0 (`run` waits for that)
    job: Option<*const Job>,
    gen: usize,
    quit: bool,
    /// a worker panicked in the current job
    panicked: bool,
}

// the raw job pointer is only read while `run` keeps the job alive
unsafe impl Send for Slot {}

pub struct Pool {
    shared: Arc<Shared>,
    handles: Vec<JoinHandle<()>>,
}

/// How long a worker spins on the job counter before it sleeps.
const SPIN: Duration = Duration::from_micros(200);

impl Pool {
    /// A pool of `threads` workers in all (`threads - 1` spawned; the caller is worker 0).
    pub fn new(threads: usize) -> Pool {
        let shared = Arc::new(Shared {
            lock: Mutex::new(Slot { job: None, gen: 0, quit: false, panicked: false }),
            wake: Condvar::new(),
            done: Condvar::new(),
            gen: AtomicUsize::new(0),
            running: AtomicUsize::new(0),
        });
        let handles = (1..threads.max(1))
            .map(|i| {
                let sh = shared.clone();
                std::thread::Builder::new().name(format!("cg-worker-{}", i)).spawn(move || worker(sh, i)).expect("spawn worker")
            })
            .collect();
        Pool { shared, handles }
    }

    pub fn threads(&self) -> usize {
        self.handles.len() + 1
    }

    /// Runs `f(i)` for every worker index; `f(0)` on the calling thread. A panic in any worker is raised
    /// here after all have finished.
    pub fn run(&self, f: &(dyn Fn(usize) + Sync)) {
        let n = self.handles.len();
        if n == 0 {
            f(0);
            return;
        }
        let sh = &*self.shared;
        {
            let mut s = sh.lock.lock().unwrap();
            // the pointer outlives no call: `run` returns only once every worker is done with it
            let p: *const Job = unsafe { std::mem::transmute::<&(dyn Fn(usize) + Sync), &'static Job>(f) };
            s.job = Some(p);
            s.gen = s.gen.wrapping_add(1);
            s.panicked = false;
            sh.running.store(n, Ordering::SeqCst);
            sh.gen.store(s.gen, Ordering::SeqCst);
            sh.wake.notify_all();
        }
        let mine = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(0)));
        // wait for the others: spin a little, then sleep
        let t0 = Instant::now();
        while sh.running.load(Ordering::Acquire) != 0 && t0.elapsed() < SPIN {
            std::hint::spin_loop();
        }
        let mut s = sh.lock.lock().unwrap();
        while sh.running.load(Ordering::Acquire) != 0 {
            s = sh.done.wait(s).unwrap();
        }
        s.job = None;
        let panicked = s.panicked;
        drop(s);
        if let Err(p) = mine {
            std::panic::resume_unwind(p);
        }
        if panicked {
            panic!("a worker thread panicked");
        }
    }
}

fn worker(sh: Arc<Shared>, index: usize) {
    let mut seen = 0usize;
    loop {
        // spin briefly for the next job, then sleep on the condition variable
        let t0 = Instant::now();
        while sh.gen.load(Ordering::Acquire) == seen && t0.elapsed() < SPIN {
            std::hint::spin_loop();
        }
        let job = {
            let mut s = sh.lock.lock().unwrap();
            while s.gen == seen && !s.quit {
                s = sh.wake.wait(s).unwrap();
            }
            if s.quit {
                return;
            }
            seen = s.gen;
            s.job
        };
        if let Some(p) = job {
            let f: &Job = unsafe { &*p };
            if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(index))).is_err() {
                sh.lock.lock().unwrap().panicked = true;
            }
        }
        if sh.running.fetch_sub(1, Ordering::AcqRel) == 1 {
            let _g = sh.lock.lock().unwrap();
            sh.done.notify_all();
        }
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        {
            let mut s = self.shared.lock.lock().unwrap();
            s.quit = true;
            self.shared.wake.notify_all();
        }
        for h in self.handles.drain(..) {
            let _ = h.join();
        }
    }
}

/// Hands out chunk indices 0..n to whoever asks next.
pub struct Chunks {
    next: AtomicUsize,
    n: usize,
}

impl Chunks {
    pub fn new(n: usize) -> Chunks {
        Chunks { next: AtomicUsize::new(0), n }
    }
    pub fn take(&self) -> Option<usize> {
        let i = self.next.fetch_add(1, Ordering::Relaxed);
        (i < self.n).then_some(i)
    }
}

/// A pointer to a slice that workers write disjoint parts of.
#[derive(Clone, Copy)]
pub struct Shards<T> {
    ptr: *mut T,
    len: usize,
}

unsafe impl<T: Send> Send for Shards<T> {}
unsafe impl<T: Send> Sync for Shards<T> {}

impl<T> Shards<T> {
    pub fn new(s: &mut [T]) -> Shards<T> {
        Shards { ptr: s.as_mut_ptr(), len: s.len() }
    }
    /// Elements a..b.
    ///
    /// # Safety
    /// No two live results may overlap, and the slice given to `new` must outlive them.
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn part(&self, a: usize, b: usize) -> &mut [T] {
        assert!(a <= b && b <= self.len);
        std::slice::from_raw_parts_mut(self.ptr.add(a), b - a)
    }
}
