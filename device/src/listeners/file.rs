use crate::wrappers::{FilePath, INotify, ResourceID};
use std::collections::HashMap;

#[derive(Copy, Clone)]
pub enum Tag {
    DisplayBrightness,
    KeyboardBacklight,
    PlatformProfile,
}

pub struct FileListener {
    inotify: INotify,
    map: HashMap<ResourceID, Tag>,
}

impl FileListener {
    pub fn init(capacity: usize) -> Self {
        Self {
            inotify: INotify::new(),
            map: HashMap::with_capacity(capacity),
        }
    }

    pub fn get_fd(&self) -> ResourceID {
        self.inotify.get_fd()
    }

    pub fn add_file(&mut self, file: &FilePath, tag: Tag) {
        self.map.insert(self.inotify.add_file(file), tag);
    }

    pub fn get_tag(&self) -> Option<Tag> {
        self.inotify.get_target_fd().map(|fd| self.map[&fd])
    }
}
