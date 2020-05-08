use std::path::Path;
use std::error::Error;

use log::{error, debug, trace};
use inotify::{Inotify, WatchMask, EventStream};
use tokio::{
    fs,
    stream::StreamExt,
};

pub struct InotifyFileWatcher {
    _inotify: Inotify,
    stream: EventStream<Vec<u8>>,
}

impl InotifyFileWatcher {
    pub fn new(path: &Path) -> Result<Self, Box<dyn Error>> {
        debug!("Creating new inotify file watcher at {:?}", path);
        let mut inotify = Inotify::init()?;
        inotify.add_watch(path, WatchMask::CREATE)?;

        // 32 size issue: https://github.com/hannobraun/inotify/issues/125
        let buffer = vec![0u8; 4096];

        let stream = inotify.event_stream(buffer)?;

        Ok(Self { _inotify: inotify, stream })
    }

    pub async fn get_new_file(&mut self) -> Option<String> {
        let event_or_error = self.stream.next().await?;
        debug!("Received inotify event: {:?}", event_or_error);
        if let Err(e) = event_or_error {
            error!("Inotify stream error: {:?}", e);
            None
        } else {
            Some(event_or_error.unwrap().name?.into_string().unwrap())
        }
    }
}
