use super::*;
use crate::process::{processor, current_thread};
use crate::thread;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;
use crate::process::Process;
use rcore_thread::std_thread::Thread;


#[derive(Default)]
pub struct Condvar {
    wait_queue: SpinNoIrqLock<VecDeque<Arc<thread::Thread>>>,
    pub epoll_queue: SpinNoIrqLock< VecDeque<(Arc<SpinNoIrqLock<Process>>, usize, usize, usize)> >,
}

impl Condvar {
    pub fn new() -> Self {
        Condvar::default()
    }

    /// Park current thread and wait for this condvar to be notified.
    #[deprecated(note = "this may leads to lost wakeup problem. please use `wait` instead.")]
    pub fn _wait(&self) {
        // The condvar might be notified between adding to queue and thread parking.
        // So park current thread before wait queue lock is freed.
        // Avoid racing
        let lock = self.add_to_wait_queue();
        thread::park_action(move || {
            drop(lock);
        });
    }

    fn add_to_wait_queue(&self) -> MutexGuard<VecDeque<Arc<thread::Thread>>, SpinNoIrq> {
        let mut lock = self.wait_queue.lock();
        lock.push_back(Arc::new(thread::current()));
        return lock;
    }

    /// Wait for condvar until condition() returns Some
    pub fn wait_event<T>(condvar: &Condvar, mut condition: impl FnMut() -> Option<T>) -> T {
        Self::wait_events(&[condvar], condition)
    }

    /// Wait for condvars until condition() returns Some
    pub fn wait_events<T>(condvars: &[&Condvar], mut condition: impl FnMut() -> Option<T>) -> T {
        let thread = thread::current();
        let tid = thread.id();
        let token = Arc::new(thread);
        for condvar in condvars {
            let mut lock = condvar.wait_queue.lock();
            lock.push_back(token.clone());
        }
        let mut locks = Vec::with_capacity(condvars.len());
        loop {
            for condvar in condvars {
                let mut lock = condvar.wait_queue.lock();
                locks.push(lock);
            }
            processor().manager().sleep(tid, 0);
            locks.clear();

            if let Some(res) = condition() {
                let _ = FlagsGuard::no_irq_region();
                processor().manager().cancel_sleeping(tid);
                for condvar in condvars {
                    let mut lock = condvar.wait_queue.lock();
                    lock.retain(|t| !Arc::ptr_eq(t, &token));
                }
                return res;
            }
            processor().yield_now();
        }
    }

    /// Park current thread and wait for this condvar to be notified.
    pub fn wait<'a, T, S>(&self, guard: MutexGuard<'a, T, S>) -> MutexGuard<'a, T, S>
    where
        S: MutexSupport,
    {
        let mutex = guard.mutex;
        let token = Arc::new(thread::current());
        let mut lock = self.wait_queue.lock();
        lock.push_back(token.clone());

        thread::park_action(move || {
            drop(lock);
            drop(guard);
        });
        let ret = mutex.lock();
        let mut lock = self.wait_queue.lock();
        lock.retain(|t| !Arc::ptr_eq(&t, &token));
        ret
    }

    pub fn notify_one(&self) {
        if let Some(t) = self.wait_queue.lock().front() {
            self.epoll_callback(t);
            t.unpark();
        }
    }


    pub fn notify_all(&self) {
        let queue = self.wait_queue.lock();
        for t in queue.iter() {
            self.epoll_callback(t);
            t.unpark();
        }
    }
    /// Notify up to `n` waiters.
    /// Return the number of waiters that were woken up.
    pub fn notify_n(&self, n: usize) -> usize {
        let mut count = 0;
        let queue = self.wait_queue.lock();
        for t in queue.iter() {
            if count >= n {
                break;
            }
            t.unpark();
            count += 1;
        }

        count
    }

    pub fn add_epoll_list(condvar: &Condvar, proc: Arc<SpinNoIrqLock<Process>>, tid :usize, epfd: usize, fd: usize){
        condvar.epoll_queue.lock().push_back((proc, tid, epfd, fd));
    }

    pub fn remove_epoll_list(condvar: &Condvar, tid :usize, epfd: usize, fd: usize) -> bool{
        let mut epoll_list = condvar.epoll_queue.lock();
        for idx in 0..epoll_list.len(){
            if epoll_list[idx].1 == tid && epoll_list[idx].2 == epfd && epoll_list[idx].3 == fd{
                epoll_list.remove(idx);
                return true;
            }
        }
        return false;
    }

    fn epoll_callback(&self, thread: &Arc<Thread>) {
        let epoll_list = self.epoll_queue.lock();
        if epoll_list.len() <= 0 {
            return;
        }
        for idx in 0..epoll_list.len(){
            let proc = &epoll_list[idx].0;
            let tid = &epoll_list[idx].1;
            let epfd = &epoll_list[idx].2;
            let fd = &epoll_list[idx].3;

            if thread.id() == *tid {
                let mut proc = proc.lock();
                match proc.get_epoll_instance(*epfd) {
                    Ok(instacne) => {
                        let mut readylist = instacne.readyList.lock();
                        if !readylist.contains(fd) {
                            readylist.push_back(*fd);
                        }
                    }
                    Err(r) => {}
                }
            }
        }
    }

}
