use actix_web::{error, middleware, web, App, HttpResponse, HttpServer};
use itertools::Itertools;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::iter::FromIterator;
use tera::{compile_templates, Context};

#[derive(Serialize)]
struct CsbTest {
    name: String,
    time0: f64,
    time1: f64,
    time_change: f64,
    memory0: f64,
    memory1: f64,
    memory_change: f64,
}

#[derive(Serialize)]
struct IniTest {
    name: String,
    cut_time0: f64,
    cut_time1: f64,
    cut_time_change: f64,
    draw_time0: f64,
    draw_time1: f64,
    draw_time_change: f64,
    memory0: f64,
    memory1: f64,
    memory_change: f64,
}

fn index(
    _tmpl: web::Data<tera::Tera>,
    conn: web::Data<Connection>,
    query: web::Query<HashMap<String, String>>,
) -> actix_web::Result<HttpResponse> {
    let first_revision = query
        .get("r0")
        .and_then(|s| s.parse().ok())
        .unwrap_or(800_000);
    let second_revision = query
        .get("r1")
        .and_then(|s| s.parse().ok())
        .unwrap_or(896_000);

    let csb_tests = get_csb_runs_cmp(&conn, first_revision, second_revision);
    if let Err(e) = csb_tests {
        return Ok(HttpResponse::InternalServerError()
            .content_type("text/http")
            .body(e.to_string()));
    }
    let csb_tests = csb_tests.unwrap();

    let ini_tests = get_ini_runs_cmp(&conn, first_revision, second_revision);
    if let Err(e) = ini_tests {
        return Ok(HttpResponse::InternalServerError()
            .content_type("text/http")
            .body(e.to_string()));
    }
    let ini_tests = ini_tests.unwrap();

    let mut context = Context::new();
    context.insert("title", "CutSim benchmarks");
    context.insert("revision_low", &first_revision);
    context.insert("revision_high", &second_revision);
    context.insert("csb_tests", &csb_tests);
    context.insert("ini_tests", &ini_tests);
    let tmpl = compile_templates!(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*"));
    let s = tmpl
        .render("index.html", &context)
        .map_err(|e| error::ErrorInternalServerError(format!("{:?}", e)))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

#[derive(Deserialize)]
struct GraphJsonRequest {
    id: String,
}
fn csb_graph_json(
    conn: web::Data<Connection>,
    query: web::Query<GraphJsonRequest>,
) -> actix_web::Result<HttpResponse> {
    graph_json(
        conn,
        "processed_csb",
        &[("Memory", "memory_peak"), ("Run Time", "player_total_time")],
        &query.id,
    )
}

fn ini_graph_json(
    conn: web::Data<Connection>,
    query: web::Query<GraphJsonRequest>,
) -> actix_web::Result<HttpResponse> {
    graph_json(
        conn,
        "processed_ini",
        &[
            ("Memory", "memory_peak"),
            ("Cut Time", "cutting_time"),
            ("Draw Time", "draw_time"),
        ],
        &query.id,
    )
}

fn graph_json(
    conn: web::Data<Connection>,
    table: &str,
    columns: &[(&str, &str)],
    config_file: &str,
) -> actix_web::Result<HttpResponse> {
    let sql_columns: Vec<_> = columns.iter().map(|c| c.1).collect();
    let db_data = match db_revision_history_for_file(&conn, table, &sql_columns, config_file) {
        Ok(data) => data,
        Err(e) => {
            return Ok(HttpResponse::InternalServerError()
                .content_type("text/html")
                .body(e.to_string()))
        }
    };
    let labels: Vec<_> = db_data.iter().map(|r| r.0).collect();
    let colors = vec![
        "rgb(54, 162, 235)",
        "rgb(255, 159, 64)",
        "rgb(75, 192, 192)",
    ];
    let datasets: Vec<_> = columns
        .iter()
        .enumerate()
        .map(|(i, (title, _))| {
            let data: Vec<_> = db_data
                .iter()
                .map(|r| json!({"x": r.0, "y": r.1[i] / db_data[0].1[i], "v": r.1[i]}))
                .collect();
            json!({
                "label": title,
                "backgroundColor": colors[i],
                "borderColor": colors[i],
                "fill": false,
                "data": data
            })
        })
        .collect();
    let json = json!({"labels": labels,
    "datasets": datasets})
    .to_string();
    Ok(HttpResponse::Ok().content_type("text/json").body(json))
}

fn db_revision_history_for_file(
    conn: &Connection,
    table: &str,
    columns: &[&str],
    config_file: &str,
) -> rusqlite::Result<Vec<(u32, Vec<f64>)>> {
    let column_str = columns
        .iter()
        .format_with(",", |v, f| f(&format_args!("AVG({})", v)));
    let mut stmt = conn.prepare(&format!(
        "SELECT revision, {} FROM {} WHERE revision >= 800000 AND config_file LIKE ?1 GROUP BY revision ORDER BY revision",
        column_str, table
    ))?;
    let results = stmt.query_map(&[config_file], |r| {
        let mut stats = Vec::new();
        for i in 0..columns.len() {
            stats.push(r.get(i + 1)?);
        }
        Ok((r.get(0)?, stats))
    })?;
    Ok(results.filter_map(|r| r.ok()).collect())
}

#[derive(Debug)]
struct CsbRow {
    name: String,
    time: f64,
    memory: f64,
}

#[derive(Debug)]
struct IniRow {
    name: String,
    cut_time: f64,
    draw_time: f64,
    memory: f64,
}

fn get_csb_runs_cmp(
    conn: &Connection,
    revision1: u32,
    revision2: u32,
) -> rusqlite::Result<Vec<CsbTest>> {
    let results1 = get_csb_runs(conn, revision1)?;
    let results2 = get_csb_runs(conn, revision2)?;

    let mut results = Vec::new();
    for (name, r) in results1 {
        if let Some(r2) = results2.get(&name) {
            results.push(CsbTest {
                name: r.name,
                time0: r.time,
                time1: r2.time,
                time_change: to_rel_change(r.time, r2.time),
                memory0: r.memory,
                memory1: r2.memory,
                memory_change: to_rel_change(r.memory, r2.memory),
            });
        }
    }
    results.sort_by(|r1, r2| r1.name.cmp(&r2.name));
    Ok(results)
}

fn get_csb_runs(conn: &Connection, revision: u32) -> rusqlite::Result<HashMap<String, CsbRow>> {
    let mut stmt = conn.prepare(
        "SELECT config_file, AVG(player_total_time), AVG(memory_peak) FROM processed_csb WHERE revision=?1 GROUP BY config_file",
    )?;
    let result = HashMap::from_iter(
        stmt.query_map(&[&revision], |row| {
            let name: String = row.get(0)?;
            let name = name.split('\\').last().unwrap_or(&name);
            Ok(CsbRow {
                name: name.to_string(),
                time: row.get(1)?,
                memory: row.get(2)?,
            })
        })?
        .filter_map(|r| r.ok().map(|r| (r.name.clone(), r))),
    );
    Ok(result)
}

fn get_ini_runs_cmp(
    conn: &Connection,
    revision1: u32,
    revision2: u32,
) -> rusqlite::Result<Vec<IniTest>> {
    let results1 = get_ini_runs(conn, revision1)?;
    let results2 = get_ini_runs(conn, revision2)?;

    let mut results = Vec::new();
    for (name, r) in results1 {
        if let Some(r2) = results2.get(&name) {
            results.push(IniTest {
                name: r.name,
                cut_time0: r.cut_time,
                cut_time1: r2.cut_time,
                cut_time_change: to_rel_change(r.cut_time, r2.cut_time),
                draw_time0: r.draw_time,
                draw_time1: r2.draw_time,
                draw_time_change: to_rel_change(r.draw_time, r2.draw_time),
                memory0: r.memory,
                memory1: r2.memory,
                memory_change: to_rel_change(r.memory, r2.memory),
            });
        }
    }
    results.sort_by(|r1, r2| r1.name.cmp(&r2.name));

    Ok(results)
}

fn get_ini_runs(conn: &Connection, revision: u32) -> rusqlite::Result<HashMap<String, IniRow>> {
    let mut stmt = conn.prepare(
        "SELECT config_file, AVG(cutting_time), AVG(draw_time), AVG(memory_peak) FROM processed_ini WHERE revision=?1 GROUP BY config_file",
    )?;
    let result = HashMap::from_iter(
        stmt.query_map(&[&revision], |row| {
            let name: String = row.get(0)?;
            let name = name.split('\\').last().unwrap_or(&name);
            Ok(IniRow {
                name: name.to_string(),
                cut_time: row.get(1)?,
                draw_time: row.get(2)?,
                memory: row.get(3)?,
            })
        })?
        .filter_map(|r| r.ok().map(|r| (r.name.clone(), r))),
    );
    Ok(result)
}

fn to_rel_change(t1: f64, t2: f64) -> f64 {
    if t1 > t2 {
        t2 / t1 - 1.0
    } else {
        1.0 - t1 / t2
    }
}

fn main() -> std::io::Result<()> {
    //std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    HttpServer::new(|| {
        let tera = compile_templates!(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*"));
        let conn = Connection::open("cutsim-testreport.db").unwrap();

        App::new()
            .data(tera)
            .data(conn)
            .wrap(middleware::Logger::default()) // enable logger
            .service(web::resource("/").route(web::get().to(index)))
            .service(web::resource("/csb_graph.json").route(web::get().to(csb_graph_json)))
            .service(web::resource("/ini_graph.json").route(web::get().to(ini_graph_json)))
            .service(actix_files::Files::new("/static", "static").show_files_listing())
    })
    .bind("127.0.0.1:8000")?
    .run()
}
