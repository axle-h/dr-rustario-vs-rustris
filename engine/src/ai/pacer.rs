//! Presses one queued key at a time, so a speed limited agent plays at a human rate.

use std::collections::VecDeque;
use std::time::Duration;

pub struct KeyPacer<T> {
    key_delay: Duration,
    since_last_key: Duration,
    pending: VecDeque<T>,
}

impl<T> KeyPacer<T> {
    /// `key_delay` is the minimum time between presses; [Duration::ZERO] plays at full speed
    pub fn new(key_delay: Duration) -> Self {
        Self {
            key_delay,
            since_last_key: Duration::ZERO,
            pending: VecDeque::new(),
        }
    }

    pub fn tick(&mut self, delta: Duration) {
        self.since_last_key += delta;
    }

    pub fn queue(&mut self, inputs: impl IntoIterator<Item = T>) {
        self.pending.extend(inputs);
    }

    /// the next key to press, once enough time has passed since the last one
    pub fn next_key(&mut self) -> Option<T> {
        if self.since_last_key < self.key_delay {
            return None;
        }
        let key = self.pending.pop_front()?;
        self.since_last_key = Duration::ZERO;
        Some(key)
    }

    /// throw away whatever is left, the piece it was meant for has gone
    pub fn abandon(&mut self) {
        self.pending.clear();
    }

    pub fn is_idle(&self) -> bool {
        self.pending.is_empty()
    }
}
