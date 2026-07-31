use crate::wrappers::{ResourceID, TimerFD};
use std::time::Duration;

pub struct Timer {
    timer_fd: TimerFD,
    interval: Option<Duration>,
    limit: Option<u64>,
    count: u64,
    running: bool,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            timer_fd: TimerFD::new(),
            interval: None,
            limit: None,
            count: 0,
            running: false,
        }
    }

    pub fn set(&mut self, interval: Duration, limit: Option<u64>) -> &mut Self {
        if self.running {
            self.stop();
        }
        self.interval = Some(interval);
        self.limit = limit;
        self
    }

    pub fn update(&mut self) {
        self.timer_fd.read_missed();
        if let Some(limit) = self.limit {
            if self.count < limit {
                self.count += 1;
            } else {
                self.stop();
            }
        };
    }

    pub fn running(&self) -> bool {
        self.running
    }

    pub fn start(&mut self) {
        if let Some(interval) = self.interval {
            self.timer_fd.set(interval);
            self.running = true;
        }
    }

    pub fn stop(&mut self) {
        self.timer_fd.set(Duration::ZERO);
        self.count = 0;
        self.running = false;
    }

    pub fn get_fd(&self) -> ResourceID {
        self.timer_fd.get_fd()
    }
}
