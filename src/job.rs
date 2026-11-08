use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use crate::launcher::Notify;

pub(crate) struct Job<T> {
    result: Arc<Mutex<Option<T>>>,
}

impl<T: Send + 'static> Job<T> {
    pub(crate) fn spawn(notify: &Notify, work: impl FnOnce() -> T + Send + 'static) -> Self {
        let result = Arc::new(Mutex::new(None));
        let slot = Arc::clone(&result);
        let notify = Arc::clone(notify);
        thread::spawn(move || {
            let done = work();
            *slot.lock().unwrap_or_else(PoisonError::into_inner) = Some(done);
            notify();
        });
        Self { result }
    }

    pub(crate) fn take(&self) -> Option<T> {
        self.result
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }
}
