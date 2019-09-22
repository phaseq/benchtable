#![feature(proc_macro_hygiene)]
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
            engines
                .tera
                .register_function("to_color", comparison::tera_to_color());
            engines
                .tera
                .register_function("relative_change", comparison::tera_relative_change());
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

struct StaticFileDir(String);

#[get("/<path..>")]
fn static_file(
    path: std::path::PathBuf,
    static_file_dir: rocket::State<StaticFileDir>,
) -> Option<NamedFile> {
    NamedFile::open(std::path::Path::new(&static_file_dir.0).join(path)).ok()
}
