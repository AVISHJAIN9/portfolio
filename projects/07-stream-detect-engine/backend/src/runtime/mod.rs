//! Backpressure & Stream Runtime (Features 07-092..07-104)

use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::mpsc::{self, Receiver, Sender};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeHealth {
    Healthy,
    Degraded,
    Shedding,
    Unhealthy,
}

/// 07-092..07-097: Bounded Queue & Load Shedding Manager
pub struct StreamRuntime<T> {
    tx: Sender<T>,
    rx: parking_lot::Mutex<Receiver<T>>,
    capacity: usize,
    dropped_count: AtomicUsize,
}

impl<T> StreamRuntime<T> {
    pub fn new(capacity: usize) -> Self {
        let (tx, rx) = mpsc::channel(capacity);
        Self {
            tx,
            rx: parking_lot::Mutex::new(rx),
            capacity,
            dropped_count: AtomicUsize::new(0),
        }
    }

    pub fn try_pop(&self) -> Option<T> {
        self.rx.lock().try_recv().ok()
    }

    /// 07-096: Push event with backpressure and load shedding
    pub fn try_push(&self, item: T, priority: bool) -> Result<(), &'static str> {
        match self.tx.try_send(item) {
            Ok(()) => Ok(()),
            Err(mpsc::error::TrySendError::Full(_)) => {
                if priority {
                    // Critical items apply backpressure or bypass queue
                    self.dropped_count.fetch_add(1, Ordering::Relaxed);
                    Err("Queue full: priority backpressure triggered")
                } else {
                    self.dropped_count.fetch_add(1, Ordering::Relaxed);
                    Err("Queue full: low-priority event shed")
                }
            }
            Err(mpsc::error::TrySendError::Closed(_)) => Err("Runtime channel closed"),
        }
    }

    /// 07-104: Runtime Health State Machine
    pub fn health_status(&self) -> RuntimeHealth {
        let max_cap = self.capacity;
        let available = self.tx.capacity();
        let utilized = max_cap.saturating_sub(available);
        let ratio = utilized as f64 / max_cap as f64;

        if ratio < 0.70 {
            RuntimeHealth::Healthy
        } else if ratio < 0.90 {
            RuntimeHealth::Degraded
        } else if ratio < 1.0 {
            RuntimeHealth::Shedding
        } else {
            RuntimeHealth::Unhealthy
        }
    }

    pub fn dropped_events(&self) -> usize {
        self.dropped_count.load(Ordering::Relaxed)
    }
}
