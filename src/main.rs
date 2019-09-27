use actix_web::{middleware, web, App, HttpServer};

mod comparison;
mod graphs;

pub static LOWEST_REVISION: u32 = 800_000;

fn main() -> std::io::Result<()> {
    let manager = r2d2_sqlite::SqliteConnectionManager::file("cutsim-testreport.db");
    let pool = r2d2::Pool::new(manager).unwrap();

    HttpServer::new(move || {
        App::new()
            .data(pool.clone())
            .wrap(middleware::Compress::default())
            .service(web::resource("/").route(web::get().to_async(comparison::index)))
            .service(
                web::resource("/api/file/{type}")
                    .route(web::get().to_async(graphs::api_file_graph_json)),
            )
            .service(
                web::resource("/api/all/{type}")
                    .route(web::get().to_async(graphs::api_all_graph_json)),
            )
            .service(actix_files::Files::new("/static", "static").show_files_listing())
    })
    .bind("127.0.0.1:8000")?
    .run()
}
