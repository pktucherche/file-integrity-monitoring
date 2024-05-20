use std::sync::{Arc, Mutex};

mod web;
mod path;
mod app;
mod event_dir;
mod event_file;
mod watcher;

use crate::web::start_web;
use crate::app::AppFIM;



fn main() -> std::io::Result<()> {
    let app_fim = Arc::new(Mutex::new(AppFIM::new()));
    start_web(app_fim)?;

    return Ok(());
}