#![feature(proc_macro_hygiene)]
#![allow(clippy::float_cmp)]
use rocket::response::NamedFile;
use rocket_contrib::{compression::Compression, templates::Template};
#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;
mod comparison;
mod graphs;

pub static LOWEST_REVISION: u32 = 800_000;

#[database("sqlite_db")]
pub struct SqliteDb(rusqlite::Connection);

fn main() {
    rocket::ignite()
        .attach(SqliteDb::fairing())
        .attach(Compression::fairing())
        .attach(Template::custom(|engines| {
            engines.tera.register_function("to_color", tera_to_color());
            engines
                .tera
                .register_function("relative_change", tera_relative_change());
        }))
        .attach(rocket::fairing::AdHoc::on_attach(
            "Static Files",
            |rocket| {
                let static_file_dir = rocket
                    .config()
                    .get_str("static_file_dir")
                    .unwrap()
                    .to_string();
                Ok(rocket.manage(StaticFileDir(static_file_dir)))
            },
        ))
        .mount("/", routes![comparison::index])
        .mount(
            "/api",
            routes![graphs::api_file_graph_json, graphs::api_all_graph_json],
        )
        .mount("/static", routes![static_file])
        .launch();
}

fn tera_relative_change() -> tera::GlobalFn {
    Box::new(move |args| -> tera::Result<tera::Value> {
        match (args.get("v1"), args.get("v2")) {
            (Some(v1), Some(v2)) => {
                match (
                    tera::from_value::<f64>(v1.clone()),
                    tera::from_value::<f64>(v2.clone()),
                ) {
                    (Ok(v1), Ok(v2)) => {
                        let v = v2 / v1 - 1.0;
                        let s = if v.is_nan() || v.is_infinite() || v == -1.0 {
                            "?".to_string()
                        } else if v > 0.0 {
                            format!("+{:.1}%", 100.0 * v)
                        } else {
                            format!("{:.1}%", 100.0 * v)
                        };
                        Ok(tera::to_value(s).unwrap())
                    }
                    _ => Ok("?".into()),
                }
            }
            _ => Err("oops".into()),
        }
    })
}

fn tera_to_color() -> tera::GlobalFn {
    Box::new(move |args| -> tera::Result<tera::Value> {
        match (args.get("v1"), args.get("v2")) {
            (Some(v1), Some(v2)) => {
                match (
                    tera::from_value::<f64>(v1.clone()),
                    tera::from_value::<f64>(v2.clone()),
                ) {
                    (Ok(v1), Ok(v2)) => {
                        let v = v2 / v1 - 1.0;
                        let s = if v.is_nan() || v.is_infinite() || v == -1.0 || v > 0.05 {
                            "#f00"
                        } else if v < -0.05 {
                            "#0a0"
                        } else {
                            "#000"
                        };
                        Ok(tera::to_value(s).unwrap())
                    }
                    _ => Ok("?".into()),
                }
            }
            _ => Err("oops".into()),
        }
    })
}

struct StaticFileDir(String);

#[get("/<path..>")]
fn static_file(
    path: std::path::PathBuf,
    static_file_dir: rocket::State<StaticFileDir>,
) -> Option<NamedFile> {
    NamedFile::open(std::path::Path::new(&static_file_dir.0).join(path)).ok()
}
