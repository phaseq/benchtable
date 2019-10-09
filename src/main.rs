#[macro_use]
extern crate tower_web;

use flate2::Compression;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::{io, path::PathBuf};
use tokio::{fs::File, prelude::Future};
use tower_web::middleware::deflate::DeflateMiddleware;
use tower_web::ServiceBuilder;

mod comparison;
mod graphs;

pub static LOWEST_REVISION: u32 = 800_000;

#[derive(Clone, Debug)]
pub struct TowerWeb {
    db_pool: Pool<SqliteConnectionManager>,
}

impl_web! {
    impl TowerWeb {
        pub fn new(db_pool: Pool<SqliteConnectionManager>) -> Self {
            Self { db_pool }
        }

        #[get("/")]
        #[content_type("text/html")]
        fn index(&self, query_string: comparison::IndexQuery) -> Result<String, tower_web::Error> {
            comparison::index(&self.db_pool, query_string)
        }

        #[get("/api/file/:file_type")]
        #[content_type("text/json")]
        fn api_file(&self, file_type: String, query_string: graphs::FileGraphQuery) -> Result<String, tower_web::Error> {
            graphs::api_file_graph_json(&self.db_pool, file_type, query_string)
        }

        #[get("/api/all/:file_type")]
        #[content_type("text/json")]
        fn api_all(&self, file_type: String, query_string: graphs::AllGraphQuery) -> Result<String, tower_web::Error> {
            graphs::api_all_graph_json(&self.db_pool, file_type, query_string)
        }

        #[get("/static/*rel_path")]
        fn static_files(&self, rel_path: PathBuf) -> impl Future<Item = File, Error = io::Error> {
            let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            path.push("static");
            path.push(rel_path);
            File::open(path)
        }
    }
}

#[derive(Deserialize)]
struct Config {
    sqlite_db: String,
}

fn load_config() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("config.toml");
    let config: String = std::fs::read_to_string(path).unwrap();
    let config: Config = toml::from_str(&config).unwrap();
    PathBuf::from(config.sqlite_db)
}

fn main() {
    let db_path = load_config();

    let addr = "127.0.0.1:8000".parse().expect("Invalid IP");
    println!("Listening on http://{}", addr);

    let manager = r2d2_sqlite::SqliteConnectionManager::file(db_path);
    let pool = r2d2::Pool::new(manager).unwrap();

    ServiceBuilder::new()
        .resource(TowerWeb::new(pool))
        .middleware(DeflateMiddleware::new(Compression::fast()))
        .run(&addr)
        .unwrap();
}
