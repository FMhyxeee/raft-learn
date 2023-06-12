use std::sync::{Arc, Mutex, mpsc::{self, TryRecvError}};

pub enum Signal {
    Terminate,
}


pub fn check_singals(receiver: &Arc<Mutex<mpsc::Receiver<Signal>>>) -> bool {
    match receiver.lock().unwrap().try_recv() {
        Ok(Signal::Terminate) => true,
        Err(TryRecvError::Empty) => false,
        Err(TryRecvError::Disconnected) => true,
    }
}


