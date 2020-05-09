use std::error::Error;
use std::path::Path;

use log::{error, debug};
use regex::Regex;
use inotify::{Inotify, WatchMask, EventStream};
use tokio::{
    fs::File,
    stream::StreamExt,
    io::AsyncReadExt,
};

pub async fn read_file(file_path: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    debug!("Reading full file");
    let mut content = vec![];
    let mut file = File::open(file_path).await?;
    file.read_to_end(&mut content).await?;
    Ok(content)
}

pub struct InotifyFileWatcher {
    _inotify: Inotify,
    stream: EventStream<Vec<u8>>,
    filter: Option<Regex>,
}

impl InotifyFileWatcher {
    pub fn new(path: &Path, filter: Option<Regex>) -> Result<Self, Box<dyn Error>> {
        debug!("Creating new inotify file watcher at {:?}", path);
        let mut inotify = Inotify::init()?;
        inotify.add_watch(path, WatchMask::CREATE)?;

        // 32 size issue: https://github.com/hannobraun/inotify/issues/125
        let buffer = vec![0u8; 4096];

        let stream = inotify.event_stream(buffer)?;

        Ok(Self { _inotify: inotify, stream, filter })
    }

    pub async fn get_new_file(&mut self) -> Option<String> {
        let result = loop {
            let event_or_error = self.stream.next().await?;
            debug!("Received inotify event: {:?}", event_or_error);
            if let Err(e) = event_or_error {
                error!("Inotify stream error: {:?}", e);
                break None
            } else {
                let file_name = event_or_error.unwrap().name?.into_string().unwrap();
                if self.filter.as_ref().map(|r| r.is_match(&file_name)) == Some(true) {
                    break Some(file_name)
                }
                debug!("Skipping {:?}, due to filter match", file_name);
                continue;
            }
        };
        result
    }
}
