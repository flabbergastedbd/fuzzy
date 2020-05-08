use std::path::Path;
use std::error::Error;

use log::{debug, trace};
use inotify::{Inotify, WatchMask, EventStream};
use tokio::{
    fs,
    stream::StreamExt,
};

pub struct InotifyFileWatcher {
    _inotify: Inotify,
    stream: EventStream<[u8; 32]>,
}

impl InotifyFileWatcher {
    pub fn new(path: &Path) -> Result<Self, Box<dyn Error>> {
        debug!("Creating new inotify file watcher");
        let mut inotify = Inotify::init()?;
        inotify.add_watch(path, WatchMask::CREATE)?;
        let buffer = [0; 32];
        let stream = inotify.event_stream(buffer)?;

        Ok(Self { _inotify: inotify, stream })
    }

    pub async fn get_new_file(&mut self) -> Option<String> {
        let event_or_error = self.stream.next().await?;
        trace!("Received inotify event: {:?}", event_or_error);
        if let Ok(event) = event_or_error {
            Some(event.name?.into_string().unwrap())
        } else {
            None
        }
    }
}
