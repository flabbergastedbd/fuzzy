use std::io::{BufRead, Seek, SeekFrom};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use log::{trace, error, debug};
use regex::Regex;
use inotify::{Inotify, WatchMask, EventStream};
use tokio::{
    fs::{self, File},
    stream::StreamExt,
    io::AsyncReadExt,
};

use crate::utils::get_human_dt;

pub fn tail_n(file_path: &Path, bytes: u64) -> Result<Vec<String>, Box<dyn Error>> {
    let mut file = std::fs::File::open(file_path)?;
    let length = file.metadata()?.len();

    // Always seek from start
    // debug!("File {:?} length found to be {}", file_path.as_path(), length);
    file.seek(SeekFrom::Start(length - bytes))?;

    let reader = std::io::BufReader::new(file);
    let lines: Vec<String> = reader.lines().map(|line| { line.unwrap() }).collect();

    Ok(lines)
}

pub async fn mkdir_p(path: &Path) -> std::io::Result<()> {
    debug!("Creating directory tree {:?}", path);
    fs::create_dir_all(path).await?;
    Ok(())
}

pub async fn rm_r(path: &Path) -> std::io::Result<()> {
    debug!("Removing directory tree {:?}", path);
    fs::remove_dir_all(path).await?;
    Ok(())
}

pub async fn read_file(file_path: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    debug!("Reading full file: {:?}", file_path);
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

pub struct FileWatcher {
    path: PathBuf,
    last_sync: SystemTime,
    blacklist_filter: Option<Regex>,
    whitelist_filter: Option<Regex>,
}

impl FileWatcher {
    pub fn new(path: &Path, blacklist_filter: Option<Regex>, whitelist_filter: Option<Regex>, last_sync: SystemTime) -> Result<Self, Box<dyn Error>> {
        debug!("Creating new file watcher at {:?}", path);
        Ok(Self {
            path: path.to_path_buf(),
            last_sync,
            blacklist_filter,
            whitelist_filter,
        })
    }

    // TODO: This does lot of syscalls, fix this
    pub fn get_new_files(&mut self) -> Result<Vec<PathBuf>, Box<dyn Error>> {
        debug!("Trying to get new files at {:?} since {:?}", self.path, get_human_dt(self.last_sync));
        let mut new_files = vec![];
        // Dedup is handled on master anyways, this is to not miss anything
        let now = SystemTime::now();
        let entries = std::fs::read_dir(self.path.as_path())?;
        for entry in entries {
            let entry = entry?;
            if let Some(file_name) = entry.file_name().to_str() {
                let blacklist_match = self.blacklist_filter.as_ref().map(|r| r.is_match(&file_name)).unwrap_or(true);
                let whitelist_match = self.whitelist_filter.as_ref().map(|r| r.is_match(&file_name)).unwrap_or(true);
                if !blacklist_match && whitelist_match {
                    // This enables us to sync all files if filesystem doesnt support timestamps
                    let timestamp = entry.metadata()?.created().unwrap_or(std::time::UNIX_EPOCH);
                    if  self.last_sync <= timestamp {
                        new_files.push(entry.path());
                    } else {
                        trace!("Old timestamp detected should have been synced: {:?}", get_human_dt(timestamp));
                    }
                } else {
                    trace!("Skipping {} (Blacklist Match: {:?} - Whitelist Match: {:?}", file_name, blacklist_match, whitelist_match);
                }
            }
        }

        // Always update sync at end
        self.last_sync = now;
        Ok(new_files)
    }
}
