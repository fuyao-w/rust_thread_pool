use std::sync::{Arc, Condvar, Mutex};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvError, Sender, SyncSender};

#[cfg(test)]
mod test {
    use std::thread;
    use std::thread::Thread;

    use super::*;

    #[test]
    fn tt() {
        let b = NonBlockQueue::new();
        let b1 = b.clone();

        let at = thread::spawn(move || {
            for i in 0..1000 {
                b1.push(i);
                // println!("push")
            }
        });

        let b2 = b.clone();
        let bt = thread::spawn(move || {
            for i in 0..1000 {
                // println!("{}", b2.pop());
            }
        });


        at.join();
        bt.join();
    }
}


pub struct NonBlockQueue<T> {
    sender: Sender<T>,
    receiver: Arc<Mutex<Receiver<T>>>,
}

unsafe impl<T> Send for NonBlockQueue<T> {}

impl<T> Clone for NonBlockQueue<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            receiver: Arc::clone(&self.receiver),
        }
    }
}

impl<T> NonBlockQueue<T> {
    pub fn new() -> NonBlockQueue<T> {
        let (sender, receiver) = channel();
        NonBlockQueue {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }
    pub fn push(&self, t: T) -> bool {
        self.sender.send(t).expect("");
        true
    }
    pub fn pop(&self) -> Result<T, RecvError> {
        self.receiver.lock().unwrap().recv()
    }
}


trait Queue<T> {
    fn push(&mut self, t: T) -> bool;
    fn pop(&mut self) -> T;
}
