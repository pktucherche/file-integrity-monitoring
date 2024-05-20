use actix_web::{get, web, App, HttpServer, Responder, HttpResponse};
use std::sync::{Arc, Mutex};
use std::thread;
use std::fs;
use std::collections::HashMap;
use std::path::PathBuf;
use rusqlite::{Connection, Result, params, Error};

use crate::app::AppFIM;
use crate::path::check_path;



#[derive(Debug)]
struct EventPatch {
    diff_patch: Vec<u8>
}

fn fetch_events() -> Result<Vec<Result<(u32, String, String, String, Vec<u8>), Error>>> {
    let conn = Connection::open("database.db")?;
    let mut stmt = conn.prepare("SELECT event.id, path.file_path, event.type_event, strftime('%Y-%m-%d %H:%M:%S', event.date_event) as date_event, event.diff FROM event INNER JOIN path ON event.path_id = path.id ORDER BY event.date_event DESC")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Vec<u8>>(4)?,
        ))
    })?;
    
    let mut events = Vec::new();
    for row in rows {
        events.push(row);
    }
    Ok(events)
}

fn get_diff(
    id: u32
) -> Result<Vec<u8>, Error> {
    let conn = Connection::open("database.db")?;
    
    let mut stmt = conn.prepare("SELECT diff FROM event WHERE id = ?1")?;
    let patch: EventPatch = stmt.query_row(params![id], |row| {
        Ok(EventPatch {
            diff_patch: row.get(0)?
        })
    })?;

    let diff_patch = patch.diff_patch;
    Ok(diff_patch)
}

#[get("/")]
async fn index(data: web::Data<Arc<Mutex<AppFIM>>>) -> impl Responder {
    let app_fim_mutex = data.get_ref();
    let app_fim = app_fim_mutex.lock().unwrap();
    let file_state;

    if app_fim.state == false {
        file_state = String::from("offline");
    } else {
        file_state = String::from("online");
    }

    let html_content = match fs::read_to_string(format!("website/index_{}.html", file_state)) {
        Ok(content) => content,
        Err(_) => return HttpResponse::InternalServerError().finish()
    };
    let css_content = match fs::read_to_string(format!("website/style/style_{}.css", file_state)) {
        Ok(content) => content,
        Err(_) => return HttpResponse::InternalServerError().finish()
    };
    let mut html_with_css = html_content.replace("{}", &css_content);

    if app_fim.state == false {
        for item in &app_fim.path_from_web {
            html_with_css = html_with_css.replace("<!---->", "
            <!---->
            <form class=\"path-item\" action=\"http://127.0.0.1:6077/del\" method=\"get\">
                <input type=\"hidden\" name=\"path\" value=\"{}\">
                <span class=\"path-text\">{}</span>
                <button type=\"submit\" class=\"remove-btn\">Remove</button>
            </form>
            ");

            html_with_css = match item.to_str() {
                Some(path_str) => html_with_css.replace("{}", path_str),
                None => continue
            }
        }
    } else {
        match fetch_events() {
            Ok(events) => {
                for event in events {
                    match event {
                        Ok((id, file_path, type_event, date_event, diff)) => {
                            html_with_css = html_with_css.replace("<!---->", "
                            <div class=\"event\">
                                <div class=\"event-indicator indicator-{1}\">
                                    <div class=\"event-indicator-circle\"></div>
                                    <div class=\"event-indicator-label\">{2}</div>
                                </div>
                                <div class=\"event-path\">{3}</div>
                                <div class=\"event-date\">{4}</div>
                                {5}
                            </div>
                            <!---->
                            ");

                            html_with_css = match type_event.as_str() {
                                "CREATE" => html_with_css.replace("{1}", "create"),
                                "DELETE" => html_with_css.replace("{1}", "delete"),
                                "MODIFY" => html_with_css.replace("{1}", "modify"),
                                "MOVED_FROM" => html_with_css.replace("{1}", "moved_from"),
                                "MOVED_TO" => html_with_css.replace("{1}", "moved_to"),
                                _ => {
                                    continue;
                                }
                            };

                            html_with_css = html_with_css.replace("{2}", type_event.as_str());
                            html_with_css = html_with_css.replace("{3}", file_path.as_str());
                            html_with_css = html_with_css.replace("{4}", date_event.as_str());

                            if diff.is_empty() {
                                html_with_css = html_with_css.replace("{5}", "");
                            } else {
                                let tmp_link = "<a href=\"{6}\" class=\"event-link\">See more</a>";
                                let tmp_id = format!("http://127.0.0.1:6077/diffweb?id={}", id);
                                let tmp = tmp_link.replace("{6}", tmp_id.as_str());
                                html_with_css = html_with_css.replace("{5}", tmp.as_str());
                            }
                        },
                        _ => continue
                    }
                }
            },
            _ => ()
        }
    }

    HttpResponse::Ok().body(html_with_css)
}

#[get("/diffweb")]
async fn diffweb(data: web::Data<Arc<Mutex<AppFIM>>>, info: web::Query<HashMap<String, String>>) -> impl Responder {
    let app_fim_mutex = data.get_ref();
    let app_fim = app_fim_mutex.lock().unwrap();

    if app_fim.state == false {
        return HttpResponse::Found().append_header(("Location", "/")).finish();
    }

    let id_str = match info.get("id") {
        Some(i) => i,
        None => return HttpResponse::Found().append_header(("Location", "/")).finish()
    };

    let id = match id_str.parse::<u32>() {
        Ok(i) => i,
        Err(_) => return HttpResponse::Found().append_header(("Location", "/")).finish()
    };

    let diff = match get_diff(id) {
        Ok(d) => d,
        Err(_) => return HttpResponse::Found().append_header(("Location", "/")).finish()
    };

    HttpResponse::Ok().body(diff)
}

#[get("/start")]
async fn start(data: web::Data<Arc<Mutex<AppFIM>>>) -> impl Responder {
    let app_fim_mutex = data.get_ref();
    let mut app_fim = app_fim_mutex.lock().unwrap();
    app_fim.state = true;

    println!("Launch of the program...");

    let app_fim_clone = Arc::clone(&app_fim_mutex);
    let path_from_web = app_fim.path_from_web.clone();
    thread::spawn(move || {
        AppFIM::app(app_fim_clone, path_from_web).expect("Impossible to start app");
    });

    HttpResponse::Found().append_header(("Location", "/")).finish()
}

#[get("/stop")]
async fn stop(data: web::Data<Arc<Mutex<AppFIM>>>) -> impl Responder {
    let app_fim_mutex = data.get_ref();
    let mut app_fim = app_fim_mutex.lock().unwrap();
    app_fim.state = false;

    println!("Stopping the program...");

    HttpResponse::Found().append_header(("Location", "/")).finish()
}

#[get("/add")]
async fn add(data: web::Data<Arc<Mutex<AppFIM>>>, info: web::Query<HashMap<String, String>>) -> impl Responder {
    let app_fim_mutex = data.get_ref();
    let mut app_fim = app_fim_mutex.lock().unwrap();
    
    if app_fim.state == true {
        return HttpResponse::Found().append_header(("Location", "/")).finish();
    }

    let path = match info.get("path") {
        Some(p) => p,
        None => return HttpResponse::Found().append_header(("Location", "/")).finish()
    };

    match check_path(&path) {
        Err(_) => return HttpResponse::Found().append_header(("Location", "/")).finish(),
        _ => (),
    }

    let desired_path = match std::fs::canonicalize(path) {
        Ok(p) => p,
        Err(_) => return HttpResponse::Found().append_header(("Location", "/")).finish()
    };

    if !app_fim.path_from_web.contains(&desired_path) {
        let mut to_delete: Vec<PathBuf> = Vec::new();

        for item in &app_fim.path_from_web {
            if desired_path.starts_with(&item) {
                return HttpResponse::Found().append_header(("Location", "/")).finish();
            }
            if item.starts_with(&desired_path) {
                to_delete.push(item.to_path_buf());
            }
        }

        for item in &to_delete {
            app_fim.path_from_web.retain(|x| *x != PathBuf::from(item));
        }

        app_fim.path_from_web.push(desired_path);
    }

    HttpResponse::Found().append_header(("Location", "/")).finish()
}

#[get("/del")]
async fn del(data: web::Data<Arc<Mutex<AppFIM>>>, info: web::Query<HashMap<String, String>>) -> impl Responder {
    let app_fim_mutex = data.get_ref();
    let mut app_fim = app_fim_mutex.lock().unwrap();

    if app_fim.state == true {
        return HttpResponse::Found().append_header(("Location", "/")).finish();
    }

    let path = match info.get("path") {
        Some(p) => p,
        None => return HttpResponse::Found().append_header(("Location", "/")).finish()
    };

    app_fim.path_from_web.retain(|x| *x != PathBuf::from(path));

    HttpResponse::Found().append_header(("Location", "/")).finish()
}

#[actix_web::main]
pub async fn start_web(app_fim: Arc<Mutex<AppFIM>>) -> std::io::Result<()> {
    HttpServer::new(move || {
        App::new()
        .app_data(web::Data::new(Arc::clone(&app_fim)))
        .service(start)
        .service(stop)
        .service(index)
        .service(add)
        .service(del)
        .service(diffweb)
    })
    .bind("127.0.0.1:6077")?
    .run()
    .await
}