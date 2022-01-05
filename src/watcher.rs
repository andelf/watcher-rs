use gitignore::Pattern;
// use notify::{EventKind, RecursiveMode, Result, Watcher};
use notify::{DebouncedEvent, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::iter;
use std::iter::IntoIterator;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::sleep;
use std::time::Duration;

use super::event::Event;

pub struct WatcherBuilder {
    pub root: PathBuf,
    pub patterns: Vec<String>,
}

impl WatcherBuilder {
    pub fn new(root: PathBuf) -> WatcherBuilder {
        WatcherBuilder {
            root,
            patterns: Vec::new(),
        }
    }

    pub fn add_pattern(&mut self, pattern: &str) -> &mut WatcherBuilder {
        self.patterns.push(pattern.to_string());
        self
    }

    pub fn add_patterns<T: AsRef<str>, P: IntoIterator<Item = T>>(
        &mut self,
        patterns: P,
    ) -> &mut WatcherBuilder {
        for pattern in patterns.into_iter() {
            self.add_pattern(pattern.as_ref());
        }
        self
    }

    pub fn build(&mut self) -> Result<FileWatcher, String> {
        let (tx, rx) = mpsc::channel();

        //let raw_tx = tx.clone();
        //let event_handler = move |event: notify::Result<notify::Event>| {
        //    tx.send(event.ok()).unwrap();
        //};

        let watcher = FileWatcher {
            root: self.root.clone(),
            patterns: self.patterns.clone(),
            started: false,
            watcher: notify::watcher(tx.clone(), Duration::from_millis(500))
                .map_err(|e| format!("E(file-watcher): {:?}", e))?,
            raw_rx: rx,
            raw_tx: tx,
        };

        Ok(watcher)
    }
}

// Watcher fo a directory
pub struct FileWatcher {
    pub root: PathBuf,
    pub patterns: Vec<String>,
    started: bool,
    watcher: notify::RecommendedWatcher,
    raw_rx: Receiver<DebouncedEvent>,
    raw_tx: Sender<DebouncedEvent>,
}

unsafe impl Sync for FileWatcher {}
unsafe impl Send for FileWatcher {}

impl FileWatcher {
    pub fn watch(root: &str) -> WatcherBuilder {
        let top_path = Path::new(root).canonicalize().unwrap();
        WatcherBuilder::new(top_path)
    }

    pub fn stop(&mut self) -> Result<(), String> {
        if self.started {
            self.watcher
                .unwatch(&self.root)
                .map_err(|_| "E(file-watcher): unwatch failed".to_owned())?;
        }
        // FIXME: use rescan to denote watch stopped
        self.raw_tx
            .send(DebouncedEvent::Rescan)
            .map_err(|_| "E(file-watcher): unwatch failed".to_owned())?;
        Ok(())
    }

    pub fn start<F>(&mut self, callback: F)
    where
        F: Fn(Event) + 'static,
    {
        let top_path: &Path = &self.root;

        println!("top_path is {:?}", top_path);
        self.watcher
            .watch(top_path, RecursiveMode::Recursive)
            .unwrap();

        let patterns = self
            .patterns
            .iter()
            .map(|p| Pattern::new(p, top_path))
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();

        let is_ignored = |path: &Path| patterns.iter().any(|p| p.is_excluded(&path, path.is_dir()));
        self.started = true;
        loop {
            let first_event = self.raw_rx.recv().unwrap();
            sleep(Duration::from_millis(100));
            let rest_events = self.raw_rx.try_iter();

            let events = iter::once(first_event).chain(rest_events);

            let mut modified = HashSet::new();
            let mut deleted = HashSet::new();

            for event in events {
                // println!(format!("D event: {:?}", event));
                match event {
                    DebouncedEvent::Create(path) | DebouncedEvent::Write(path) => {
                        if !is_ignored(&path) && path.is_file() {
                            modified.insert(path);
                        }
                    }
                    DebouncedEvent::Remove(path) => {
                        if !is_ignored(&path) {
                            deleted.insert(path);
                        }
                    }
                    DebouncedEvent::Rename(from, to) => {
                        if !is_ignored(&to) && to.is_file() {
                            if !is_ignored(&from) {
                                deleted.insert(from);
                            }
                            modified.insert(to);
                        }
                    }
                    DebouncedEvent::Error(err, _) => {
                        println!("E: watcher error: {:?}", err);
                        break;
                    }
                    DebouncedEvent::Rescan => {
                        println!("D: watcher stopped");
                        break;
                    }
                    DebouncedEvent::NoticeRemove(_)
                    | DebouncedEvent::NoticeWrite(_)
                    | DebouncedEvent::Chmod(_) => (),
                }
            }

            if !deleted.is_empty() {
                callback(Event::Deleted(deleted));
            }
            if !modified.is_empty() {
                callback(Event::Modified(modified));
            }
        }
    }
}
