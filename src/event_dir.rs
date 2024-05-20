use std::path::PathBuf;
use inotify::{Inotify, WatchDescriptor};
use std::error::Error;
use std::collections::HashMap;

use crate::watcher::watch_directory_recursive;



pub fn dir_moved_from(
    inotify: &Inotify,
    complete_path: &PathBuf,
    watched_dirs: &mut HashMap<WatchDescriptor, PathBuf>,
) -> Result<(), Box<dyn Error>> {

    let mut keys_to_remove: Vec<WatchDescriptor> = Vec::new();

    for (key, value) in watched_dirs.iter() {
        if value.starts_with(complete_path) {
            inotify.watches().remove(key.clone())?;
            keys_to_remove.push(key.clone());
        }
    }

    for key in keys_to_remove.iter() {
        watched_dirs.remove(key);
    }

    Ok(())
}

pub fn dir_moved_to(
    inotify: &Inotify,
    complete_path: &PathBuf,
    watched_dirs: &mut HashMap<WatchDescriptor, PathBuf>,
) -> Result<(), Box<dyn Error>> {

    watch_directory_recursive(&inotify, &complete_path, watched_dirs)?;

    Ok(())
}

pub fn dir_delete(
    complete_path: &PathBuf,
    watched_dirs: &mut HashMap<WatchDescriptor, PathBuf>,
) -> Result<(), Box<dyn Error>> {

    let to_remove: WatchDescriptor;

    for (key, value) in watched_dirs.iter() {
        if value.to_path_buf() == complete_path.clone() {
            to_remove = key.clone();
            watched_dirs.remove(&to_remove);
            break;
        }
    }

    Ok(())
}

pub fn dir_create(
    inotify: &Inotify,
    complete_path: &PathBuf,
    watched_dirs: &mut HashMap<WatchDescriptor, PathBuf>,
) -> Result<(), Box<dyn Error>> {

    watch_directory_recursive(inotify, complete_path, watched_dirs)?;

    Ok(())
}