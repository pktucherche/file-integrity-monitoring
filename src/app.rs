extern crate inotify;

use inotify::{Inotify, EventMask, WatchDescriptor};
use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::error::Error;
use std::io::ErrorKind;
use nix::fcntl::{fcntl, FcntlArg, OFlag};
use std::os::fd::AsRawFd;
use std::fs;
use rusqlite::{Connection, Result};

use crate::watcher::watch_directory_recursive;
use crate::event_dir::{dir_moved_from, dir_moved_to, dir_delete, dir_create};
use crate::event_file::{check_rec, check_file};



pub struct AppFIM {
    pub state: bool,
    pub path_from_web: Vec<PathBuf>
}

impl AppFIM {
    pub fn new() -> Self {
        Self {
            state: false,
            path_from_web: Vec::new()
        }
    }

    fn init_db() -> Result<(), Box<dyn Error>> {
        if !fs::metadata("database.db").is_ok() {
            fs::File::create("database.db")?;
            
            let conn = Connection::open("database.db")?;
            conn.execute(
                "CREATE TABLE IF NOT EXISTS path (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    file_path TEXT NOT NULL,
                    last_copy BLOB NOT NULL
                );",
                []
            )?;

            conn.execute(
                "CREATE TABLE IF NOT EXISTS event (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    type_event TEXT CHECK (type_event IN ('CREATE', 'DELETE', 'MODIFY', 'MOVED_FROM', 'MOVED_TO')),
                    date_event TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    diff BLOB,
                    path_id INTEGER NOT NULL,
                    FOREIGN KEY (path_id) REFERENCES path(id)
                );",
                []
            )?;
        }

        Ok(())
    }

    pub fn app(
        app_fim_mutex: Arc<Mutex<AppFIM>>,
        path_from_web: Vec<PathBuf>
    ) -> Result<(), Box<dyn Error>> {
        Self::init_db()?;

        let mut inotify = Inotify::init().expect("Failed to initialize inotify");
        let mut watched_dirs: HashMap<WatchDescriptor, PathBuf> = HashMap::new();
        for path in &path_from_web {
            match watch_directory_recursive(&inotify, path, &mut watched_dirs) {
                _ => ()
            }
            let conn = Connection::open("database.db")?;
            match check_rec(&conn, path) {
                _ => ()
            }
        }

        let fd = inotify.as_raw_fd();
        fcntl(fd, FcntlArg::F_SETFL(OFlag::O_NONBLOCK)).expect("Failed to set non-blocking mode");

        println!("OK!");
        println!("");
        
        let mut buffer = [0; 4096];
        loop {
            match inotify.read_events(&mut buffer) {
                Ok(events) => {
                    let conn = Connection::open("database.db")?;

                    for event in events {                                
                        let name = match event.name {
                            Some(name) => name,
                            None => continue
                        };
                                
                        let mut complete_path = match watched_dirs.get(&event.wd) {
                            Some(complete_path) => complete_path,
                            None => continue
                        }.clone();
                        complete_path.push(name);
                                
                        if event.mask.contains(EventMask::ISDIR) {
                            let flag = EventMask::ISDIR ^ event.mask;
                            match flag {
                                EventMask::CREATE => {
                                    println!("Dossier créé : {:?}", complete_path);
                                    dir_create(&inotify, &complete_path, &mut watched_dirs)?;
                                    //check_rec(&complete_path, &mut path_json)?;
                                }
                                EventMask::DELETE => {
                                    println!("Dossier supprimé : {:?}", complete_path);
                                    dir_delete(&complete_path, &mut watched_dirs)?;
                                }
                                EventMask::MOVED_FROM => {
                                    println!("Dossier from : {:?}", complete_path);
                                    dir_moved_from(&inotify, &complete_path, &mut watched_dirs)?;
                                }
                                EventMask::MOVED_TO => {
                                    println!("Dossier to : {:?}", complete_path);
                                    dir_moved_to(&inotify, &complete_path, &mut watched_dirs)?;
                                    //check_rec(&complete_path, &mut path_json)?;
                                }
                                _ => {}
                            }
                        } else {
                            match event.mask {
                                EventMask::MODIFY => {
                                    println!("Fichier modifié : {:?}", complete_path);
                                    check_file(&conn, &complete_path, "MODIFY")?;
                                }
                                EventMask::DELETE => {
                                    println!("Fichier supprimé : {:?}", complete_path);
                                    check_file(&conn, &complete_path, "DELETE")?;
                                }
                                EventMask::CREATE => {
                                    println!("Fichier crée : {:?}", complete_path);
                                    check_file(&conn, &complete_path, "CREATE")?;
                                }
                                EventMask::MOVED_FROM => {
                                    println!("Fichier moved from : {:?}", complete_path);
                                    check_file(&conn, &complete_path, "MOVED_FROM")?;
                                }
                                EventMask::MOVED_TO => {
                                    println!("Fichier moved to : {:?}", complete_path);
                                    check_file(&conn, &complete_path, "MOVED_TO")?;
                                }
                            _ => {}
                            }
                        }
                    }
                },
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    let app_fim = app_fim_mutex.lock().unwrap();
                    if app_fim.state == false {
                        println!("OK!");
                        println!("");
                        break Ok(());
                    }
                },
                Err(_) => {
                    break Ok(());
                }
            }
        }
    }
}