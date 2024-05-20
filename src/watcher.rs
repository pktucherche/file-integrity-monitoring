use std::fs;
use std::path::{Path, PathBuf};
use inotify::{Inotify, WatchDescriptor, WatchMask};
use std::error::Error;
use std::collections::HashMap;



fn add_dir_watch(
    inotify: &Inotify,
    dir: &Path,
    watched_dirs: &mut HashMap<WatchDescriptor, PathBuf>,
) -> Result<(), Box<dyn Error>> {

    let wd = inotify
        .watches()
        .add(
            dir,
            WatchMask::MODIFY | WatchMask::DELETE | WatchMask::CREATE | WatchMask::MOVED_FROM | WatchMask::MOVED_TO,
        )?;

    watched_dirs.insert(wd, dir.to_path_buf());

    Ok(())
}

pub fn watch_directory_recursive(
    inotify: &Inotify,
    dir: &Path,
    watched_dirs: &mut HashMap<WatchDescriptor, PathBuf>,
) -> Result<(), Box<dyn Error>> {

    let dir_metadata = match fs::metadata(dir) {
        Ok(m) => m,
        Err(_) => {
            return Ok(());
        }
    };

    if dir_metadata.is_dir() {
        match add_dir_watch(inotify, dir, watched_dirs) {
            Ok(_) => (),
            Err(_) => {
                return Ok(());
            }
        };

        let dir_entries = fs::read_dir(dir)?;
        for entry in dir_entries {
            if let Ok(entry) = entry {
                watch_directory_recursive(inotify, &entry.path(), watched_dirs)?;
            }
        }
    }

    Ok(())
}