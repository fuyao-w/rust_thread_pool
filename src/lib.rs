use std::fmt::{Display, Formatter};
use std::sync::{Arc, Condvar, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::mpsc::channel;
use std::thread;

#[cfg(test)]
mod tests {
    // use std::thread::panicking;
    // use std::time::Duration;

    use super::*;

    #[test]
    fn test_job_box() {
        let job_box = Box::new(|| { println!("do") });
        job_box.work();
    }

    #[test]
    fn test_sentinel() {
        let (_, r) = channel();
        let data = Arc::new(PoolData {
            receiver: Mutex::new(r),
            pool_name: None,
            stack_size: 0,
            core_pool_size: Default::default(),
            active_pool_size: Default::default(),
            active_job_size: Default::default(),
            total_job_size: Default::default(),
            join_generation: Default::default(),
            no_worker_single: (Default::default(), Mutex::new(())),
        });
        let mut sen = Sentinel::new(&data);
        println!("before {}", sen.active);
        // panic!("test");
        sen.cancel();
        println!("before {}", sen.active);
    }
    macro_rules! __rust_force_expr {
        ($e:expr) => {
            $e
        };
    }

    #[test]
    fn test_new_data() {}

    #[test]
    fn test_pool_data() {
        let (_, r) = channel();
        let data = Arc::new(PoolData {
            receiver: Mutex::new(r),
            pool_name: None,
            stack_size: 0,
            core_pool_size: Default::default(),
            active_pool_size: Default::default(),
            active_job_size: Default::default(),
            total_job_size: Default::default(),
            join_generation: Default::default(),
            no_worker_single: (Default::default(), Mutex::new(())),
        });
        assert_eq!(data.has_worker(), false);
        data.no_worker_notify();
    }

    #[test]
    fn test_build() {
        let pool = ThreadPoolBuilder::new().
            with_pool_name("haha".to_string()).
            with_core_pool_size(4).
            build();
        for i in 0..100 {
            pool.execute(move || {
                println!("execute job {}", i);
            });
        }
        // pool.update_core_pool_size(0);
        // assert_eq!(pool.data.has_worker(), true);
        pool.join();
    }
}

trait WorkerFN {
    fn work(self: Box<Self>);
}

impl<F: FnOnce()> WorkerFN for F {
    fn work(self: Box<Self>) {
        (*self)()
    }
}

type Job = Box<dyn WorkerFN + Sync + Send + 'static>;

// type RejectHandler = fn(job_name: String);


#[derive(Default)]
struct ThreadPoolBuilder {
    core_pool_size: usize,
    pool_name: Option<String>,
    stack_size: usize,
}

struct ThreadPool {
    data: Arc<PoolData>,
    sender: Sender<Job>,
}


struct Sentinel<'a> {
    data: &'a Arc<PoolData>,
    active: bool,
}

impl<'a> Sentinel<'a> {
    fn new(data: &'a Arc<PoolData>) -> Self {
        Self {
            data,
            active: true,
        }
    }

    fn cancel(&mut self) {
        self.active = false
    }
}

impl<'a> Drop for Sentinel<'a> {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        let data = self.data;
        sub_atom(&data.active_pool_size, 1);
        sub_atom(&data.active_job_size, 1);
        if thread::panicking() {
            println!("panic 了");
        }
        self.data.no_worker_notify();
        thread_spawn(self.data.clone());
    }
}


struct PoolData {
    receiver: Mutex<Receiver<Job>>,
    pool_name: Option<String>,
    stack_size: usize,
    core_pool_size: AtomicUsize,
    active_pool_size: AtomicUsize,
    active_job_size: AtomicUsize,
    total_job_size: AtomicUsize,
    join_generation: AtomicUsize,
    no_worker_single: (Condvar, Mutex<()>),
}

impl PoolData {
    fn has_worker(&self) -> bool {
        get_atom(&self.total_job_size) > 0 ||
            get_atom(&self.active_job_size) > 0
    }
    fn no_worker_notify(&self) {
        if self.has_worker() {
            return;
        }
        let (cond, lock) = &self.no_worker_single;
        let _ = lock.lock().expect("不对了");
        cond.notify_all();
    }
}


fn thread_spawn(data: Arc<PoolData>) {
    let mut builder = thread::Builder::new();
    if let Some(ref name) = data.pool_name {
        builder = builder.name(name.clone());
    }
    if data.stack_size > 0 {
        builder = builder.stack_size(data.stack_size);
    }
    add_atom(&data.active_pool_size, 1);
    let data = data.clone();
    builder.spawn(move || {
        let mut sentinel = Sentinel::new(&data);
        loop {
            let active_pool_size = get_atom(&data.active_pool_size);
            let core_pool_size = get_atom(&data.core_pool_size);
            if active_pool_size > core_pool_size {
                break;
            }
            let lock = data.receiver.lock().unwrap();
            let job = {
                match lock.recv() {
                    Ok(job) => job,
                    Err(_) => break
                }
            };
            // add_atom_v1!(&data.active_job_size);
            add_atom(&data.active_job_size, 1);
            job.work();
            sub_atom(&data.active_job_size, 1);
            sub_atom(&data.total_job_size, 1);

            data.no_worker_notify();
        }

        sentinel.cancel()
    }).unwrap();
}


fn add_atom(atom: &AtomicUsize, val: usize) {
    atom.fetch_add(val, Ordering::SeqCst);
}

fn get_atom(atom: &AtomicUsize) -> usize {
    atom.load(Ordering::SeqCst)
}

fn sub_atom(atom: &AtomicUsize, val: usize) {
    atom.fetch_sub(val, Ordering::SeqCst);
}

impl ThreadPoolBuilder {
    fn new() -> Self {
        // let thread_num =;
        Self {
            core_pool_size: num_cpus::get(),
            pool_name: None,
            stack_size: 0,
        }
    }
    fn with_core_pool_size(mut self, size: usize) -> Self {
        self.core_pool_size = size;
        self
    }
    fn with_pool_name(mut self, name: String) -> Self {
        self.pool_name = Some(name);
        self
    }
    fn with_stack_size(mut self, size: usize) -> Self {
        self.stack_size = size;
        self
    }


    fn build(self) -> ThreadPool {
        let (sender, receiver) = channel::<Job>();
        let pool = ThreadPool {
            data: Arc::new(PoolData {
                receiver: Mutex::new(receiver),
                pool_name: self.pool_name,
                stack_size: self.stack_size,
                core_pool_size: AtomicUsize::new(self.core_pool_size),
                active_pool_size: Default::default(),
                active_job_size: Default::default(),
                total_job_size: Default::default(),
                join_generation: Default::default(),
                no_worker_single: (Condvar::new(), Mutex::new(())),
            }),
            // queue: Arc::new(NonBlockQueue::new()),
            sender,
        };
        for _ in 0..self.core_pool_size {
            thread_spawn(pool.data.clone());
        }
        pool
    }
}

#[derive(Debug)]
struct OverflowErr();

impl Display for OverflowErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "overflow")
    }
}

// impl Error for OverflowErr {}

// type Result<E: Error> = core::result::Result<(), E>;


impl ThreadPool {
    fn execute<F>(&self, job: F)
        where F: FnOnce() + Send + Sync + 'static {
        add_atom(&self.data.total_job_size, 1);
        self.sender.send(Box::new(job)).expect("TODO: panic message");
    }

    fn update_core_pool_size(&self, size: usize) {
        let ori_pool_size = self.data.core_pool_size.load(Ordering::SeqCst);
        self.data.core_pool_size.store(size, Ordering::SeqCst);
        if size > ori_pool_size {
            for _ in 0..size - ori_pool_size {
                println!("new thread");
                thread_spawn(self.data.clone());
            }
        }
    }
    fn join(&self) {
        if !self.data.has_worker() {
            return;
        }
        let gen_gen = || get_atom(&self.data.join_generation);
        let generation = gen_gen();

        let (cond, lock) = &self.data.no_worker_single;

        while generation == gen_gen() &&
            self.data.has_worker() {
            let _ = cond.wait(lock.lock().unwrap());
        }
        let _ = self.data.join_generation.compare_exchange(
            generation, generation.wrapping_add(1), Ordering::SeqCst, Ordering::SeqCst);
    }
}



