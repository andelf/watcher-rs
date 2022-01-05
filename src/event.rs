use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug)]
pub enum Event {
    // unused for now
    // Created(HashSet<PathBuf>),
    Modified(HashSet<PathBuf>),
    Deleted(HashSet<PathBuf>),
    // Rename(PathBuf, PathBuf),
    // Root changed, better restart watcher App.
    RootChanged(PathBuf),
    // fatal error, needs to restart
    Error,
}

unsafe impl Send for Event {}
unsafe impl Sync for Event {}

impl Event {
    pub fn kind(&self) -> &'static str {
        match self {
            // Event::Created(_) => "created",
            Event::Modified(_) => "modified",
            Event::Deleted(_) => "deleted",
            Event::RootChanged(_) => "rootChanged",
            Event::Error => "error",
        }
    }

    pub fn paths<'a>(&'a self) -> impl Iterator<Item = &'a PathBuf> {
        match self {
            // Event::Created(paths) => paths.iter(),
            Event::Modified(paths) => paths.iter(),
            Event::Deleted(paths) => paths.iter(),
            Event::RootChanged(_) => unimplemented!(),
            Event::Error => unimplemented!(),
        }
    }
}
