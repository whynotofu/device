use crate::wrappers::{Connection, EPoll, EPollAction, ResourceID};
use std::collections::HashMap;

#[derive(Copy, Clone)]
pub enum Event {
    Connection,
    Message(Connection),
    FileChange,
    BatteryPoll,
    Netlink,
    KeyboardBacklightPoll,
    Signal,
}

pub struct EventListener {
    epoll: EPoll,
    map: HashMap<ResourceID, Event>,
}

impl EventListener {
    pub fn init(capacity: usize) -> Self {
        EventListener {
            epoll: EPoll::new(),
            map: HashMap::with_capacity(capacity),
        }
    }

    pub fn add(&mut self, fd: ResourceID, event: Event) {
        self.epoll.control(EPollAction::Add, fd);
        self.map.insert(fd, event);
    }

    pub fn remove(&mut self, fd: ResourceID) {
        self.epoll.control(EPollAction::Remove, fd);
        self.map.remove(&fd);
    }

    pub fn event(&self) -> Event {
        loop {
            if let Some(fd) = self.epoll.wait() {
                return self.map[&fd];
            }
        }
    }
}
