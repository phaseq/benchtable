#![feature(proc_macro_hygiene)]

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;

use rocket::response::NamedFile;
use rocket::response::{self, Responder};
use rocket::Request;

mod comparison;
mod graphs;

pub static LOWEST_REVISION: u32 = 800_000;

#[database("sqlite_db")]
pub struct SqliteDb(rusqlite::Connection);

fn main() {
    rocket::ignite()
        .attach(SqliteDb::fairing())
        //.attach(Compression::fairing())
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
        .launch()
        .expect("launch error!")
}

struct StaticFileDir(String);

#[get("/<path..>")]
fn static_file(
    path: std::path::PathBuf,
    static_file_dir: rocket::State<StaticFileDir>,
) -> Option<CachedFile> {
    NamedFile::open(std::path::Path::new(&static_file_dir.0).join(path))
        .ok()
        .map(CachedFile)
}

struct CachedFile(NamedFile);

impl<'r> Responder<'r> for CachedFile {
    fn respond_to(self, req: &'r Request<'_>) -> response::ResultFuture<'r> {
        Box::pin(async move {
            let mut response = self.0.respond_to(req).await?;
            response.set_raw_header("Cache-control", "max-age=86400");
            Ok(response)
        })
    }
}
