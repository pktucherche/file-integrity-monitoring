use std::path::PathBuf;
use std::error::Error;
use std::fs::{self, File};
use rusqlite::{params, Connection, Result};
use std::io::prelude::*;
use diffy::create_patch_bytes;



#[derive(Debug)]
struct PatchLastCopy {
    last_copy: Vec<u8>
}

fn get_diff(
    conn: &Connection,
    path: &PathBuf
) -> Result<Vec<u8>, Box<dyn Error>> {

    let mut stmt = conn.prepare("SELECT last_copy FROM path WHERE file_path = ?1")?;
    let patch: PatchLastCopy = stmt.query_row(params![path.to_string_lossy()], |row| {
        Ok(PatchLastCopy {
            last_copy: row.get(0)?
        })
    })?;
    let buffer_save = patch.last_copy;

    let buffer_new = match File::open(&path) {
        Ok(mut file) => {
            let mut buffer = Vec::new();
            if let Err(_) = file.read_to_end(&mut buffer) {
                Vec::new()
            } else {
                buffer
            }
        }
        Err(_) => {
            Vec::new()
        }
    };

    if buffer_save != buffer_new {
        let diff = create_patch_bytes(&buffer_save, &buffer_new);

        return Ok(diff.to_bytes());
    }

    Ok(Vec::new())
}

fn is_path_present(
    conn: &Connection,
    path: &PathBuf
) -> Result<bool> {
    let mut stmt = conn.prepare("SELECT EXISTS(SELECT 1 FROM path WHERE file_path = ?1)")?;
    let exists: bool = stmt.query_row(params![path.to_string_lossy()], |row| row.get(0))?;
    Ok(exists)
}

fn update_copy(
    conn: &Connection,
    path: &PathBuf
) -> Result<(), Box<dyn Error>> {

    let copy = match File::open(&path) {
        Ok(mut file) => {
            let mut buffer = Vec::new();
            if let Err(_) = file.read_to_end(&mut buffer) {
                Vec::new()
            } else {
                buffer
            }
        }
        Err(_) => {
            Vec::new()
        }
    };

    conn.execute(
        "UPDATE path SET last_copy = ?1 WHERE file_path = ?2",
        params![&copy, path.to_string_lossy()],
    )?;

    Ok(())
}

fn create_file_db(
    conn: &Connection,
    path: &PathBuf
) -> Result<(), Box<dyn Error>> {

    let buffer_file_create = match File::open(&path) {
        Ok(mut file) => {
            let mut buffer = Vec::new();
            if let Err(_) = file.read_to_end(&mut buffer) {
                Vec::new()
            } else {
                buffer
            }
        }
        Err(_) => {
            Vec::new()
        }
    };

    conn.execute(
        "INSERT INTO path (file_path, last_copy) VALUES (?1, ?2)",
        params![path.to_string_lossy(), &buffer_file_create],
    )?;

    Ok(())
}

fn delete_file(
    conn: &Connection,
    path: &PathBuf
) -> Result<(), Box<dyn Error>> {

    conn.execute(
        "INSERT INTO event (type_event, diff, path_id) VALUES (?1, ?2, (SELECT id FROM path WHERE file_path = ?3))",
        params!["DELETE", Vec::new(), path.to_string_lossy()],
    )?;

    Ok(())
}

fn moved_from_file(
    conn: &Connection,
    path: &PathBuf
) -> Result<(), Box<dyn Error>> {

    println!("toto");

    conn.execute(
        "INSERT INTO event (type_event, diff, path_id) VALUES (?1, ?2, (SELECT id FROM path WHERE file_path = ?3))",
        params!["MOVED_FROM", Vec::new(), path.to_string_lossy()],
    )?;

    Ok(())
}

fn moved_to_file(
    conn: &Connection,
    path: &PathBuf
) -> Result<(), Box<dyn Error>> {

    let diff = get_diff(&conn, &path)?;
    for &byte in &diff {
        print!("{}", byte as char);
    }

    conn.execute(
        "INSERT INTO event (type_event, diff, path_id) VALUES (?1, ?2, (SELECT id FROM path WHERE file_path = ?3))",
        params!["MOVED_TO", &diff, path.to_string_lossy()],
    )?;

    update_copy(&conn, &path)?;

    Ok(())
}
    
fn modify_file(
    conn: &Connection,
    path: &PathBuf
) -> Result<(), Box<dyn Error>> {

    let diff = get_diff(&conn, &path)?;
    for &byte in &diff {
        print!("{}", byte as char);
    }

    conn.execute(
        "INSERT INTO event (type_event, diff, path_id) VALUES (?1, ?2, (SELECT id FROM path WHERE file_path = ?3))",
        params!["MODIFY", &diff, path.to_string_lossy()],
    )?;

    update_copy(&conn, &path)?;

    Ok(())
}

fn maybe_modify_file(
    conn: &Connection,
    path: &PathBuf
) -> Result<(), Box<dyn Error>> {

    let diff = get_diff(&conn, &path)?;

    if !diff.is_empty() {
        modify_file(&conn, &path)?;
    }

    Ok(())
}

fn create_file(
    conn: &Connection,
    path: &PathBuf
) -> Result<(), Box<dyn Error>> {

    let diff = get_diff(&conn, &path)?;
    for &byte in &diff {
        print!("{}", byte as char);
    }

    conn.execute(
        "INSERT INTO event (type_event, diff, path_id) VALUES (?1, ?2, (SELECT id FROM path WHERE file_path = ?3))",
        params!["CREATE", &diff, path.to_string_lossy()],
    )?;

    update_copy(&conn, &path)?;

    Ok(())
}

pub fn check_file(
    conn: &Connection,
    path: &PathBuf,
    event: &str
) -> Result<(), Box<dyn Error>> {

    if !is_path_present(&conn, &path)? {
        create_file_db(&conn, &path)?;
    }

    match event {
        "DELETE" => delete_file(&conn, &path)?,
        "MOVED_FROM" => moved_from_file(&conn, &path)?,
        "MOVED_TO" => moved_to_file(&conn, &path)?,
        "MAYBE_MODIFY" => maybe_modify_file(&conn, &path)?,
        "MODIFY" => modify_file(&conn, &path)?,
        "CREATE" => create_file(&conn, &path)?,
        _ => ()
    }

    Ok(())
}

pub fn check_rec(
    conn: &Connection,
    dir: &PathBuf,
) -> Result<(), Box<dyn Error>> {

    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                check_file(&conn, &path, "MAYBE_MODIFY")?;
            } else if path.is_dir() {
                check_rec(&conn, &path)?;
            }
        }
    }

    Ok(())
}