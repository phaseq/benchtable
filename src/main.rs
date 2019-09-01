use actix_web::{error, middleware, web, App, HttpResponse, HttpServer};
use itertools::Itertools;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::json;
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

#[derive(Deserialize)]
struct IndexRequest {
    r1: Option<u32>,
    r2: Option<u32>,
    sort: Option<String>,
}
fn index(
    //tmpl: web::Data<tera::Tera>,
    conn: web::Data<Connection>,
    query: web::Query<IndexRequest>,
) -> actix_web::Result<HttpResponse> {
    let second_revision = query
        .r1
        .unwrap_or_else(|| db_latest_revision(&conn, "processed_csb").unwrap());
    let first_revision = query.r2.unwrap_or(second_revision - 2000);
    let sort = query.sort.clone().unwrap_or("cut time".to_string());

    let (csb_tests, ini_tests) =
        db_revision_comparison(&conn, first_revision, second_revision, &sort)
            .map_err(error::ErrorInternalServerError)?;

    let mut context = Context::new();
    context.insert("title", "CutSim benchmarks");
    context.insert("revision_low", &first_revision);
    context.insert("revision_high", &second_revision);
    context.insert("sort", &sort);
    context.insert("csb_tests", &csb_tests);
    context.insert("ini_tests", &ini_tests);
    let mut tmpl = compile_templates!(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*"));
    tmpl.register_function("relative_change", tera_relative_change());
    tmpl.register_function("to_color", tera_to_color());
    let s = tmpl
        .render("index.html", &context)
        .map_err(error::ErrorInternalServerError)?;
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

fn db_latest_revision(conn: &Connection, table: &str) -> rusqlite::Result<u32> {
    Ok(conn
        .prepare(&format!("SELECT MAX(revision) FROM {}", table))?
        .query_map(rusqlite::NO_PARAMS, |row| Ok(row.get(0)?))?
        .next()
        .unwrap()?)
}

fn db_revision_comparison(
    conn: &Connection,
    revision1: u32,
    revision2: u32,
    order_by: &str,
) -> rusqlite::Result<(Vec<CsbTest>, Vec<IniTest>)> {
    let csb_order_by = match order_by {
        "cut time" | "draw time" => "AVG(a.player_total_time) / AVG(b.player_total_time)",
        "memory" => "AVG(a.memory_peak) / AVG(b.memory_peak)",
        _ => "a.config_file",
    };
    let ini_order_by = match order_by {
        "cut time" => "AVG(a.cutting_time) / AVG(b.cutting_time)",
        "draw time" => "AVG(a.draw_time) / AVG(b.draw_time)",
        "memory" => "AVG(a.memory_peak) / AVG(b.memory_peak)",
        _ => "a.config_file",
    };
    Ok((
        db_revision_comparison_csb(&conn, revision1, revision2, csb_order_by)?,
        db_revision_comparison_ini(&conn, revision1, revision2, ini_order_by)?,
    ))
}

fn db_revision_comparison_csb(
    conn: &Connection,
    revision1: u32,
    revision2: u32,
    order_by: &str,
) -> rusqlite::Result<Vec<CsbTest>> {
    let query = concat!(
        "SELECT a.config_file, ",
        "AVG(a.player_total_time), AVG(b.player_total_time), ",
        "AVG(a.memory_peak), AVG(b.memory_peak) ",
        "FROM processed_csb a, processed_csb b ",
        "WHERE a.config_file = b.config_file ",
        "AND a.revision=?1 AND b.revision=?2 GROUP BY a.config_file "
    )
    .to_string()
        + &format!("ORDER BY {}", order_by);

    Ok(conn
        .prepare_cached(&query)?
        .query_map(&[&revision1, &revision2], |row| {
            let name: String = row.get(0)?;
            let name = name.split("\\testcases\\").last().unwrap_or(&name);
            Ok(CsbTest {
                name: name.to_string(),
                time0: row.get(1)?,
                time1: row.get(2)?,
                time_change: to_rel_change(row.get(1)?, row.get(2)?),
                memory0: row.get(3)?,
                memory1: row.get(4)?,
                memory_change: to_rel_change(row.get(3)?, row.get(4)?),
            })
        })?
        .filter_map(|r| r.ok())
        .collect())
}

fn db_revision_comparison_ini(
    conn: &Connection,
    revision1: u32,
    revision2: u32,
    order_by: &str,
) -> rusqlite::Result<Vec<IniTest>> {
    let query = concat!(
        "SELECT a.config_file, ",
        "AVG(a.cutting_time), AVG(b.cutting_time), ",
        "AVG(a.draw_time), AVG(b.draw_time), ",
        "AVG(a.memory_peak), AVG(b.memory_peak) ",
        "FROM processed_ini a, processed_ini b ",
        "WHERE a.config_file = b.config_file ",
        "AND a.revision=?1 AND b.revision=?2 GROUP BY a.config_file "
    )
    .to_string()
        + &format!("ORDER BY {}", order_by);

    Ok(conn
        .prepare_cached(&query)?
        .query_map(&[&revision1, &revision2], |row| {
            let name: String = row.get(0)?;
            let name = name.split("\\testcases\\").last().unwrap_or(&name);
            Ok(IniTest {
                name: name.to_string(),
                cut_time0: row.get(1)?,
                cut_time1: row.get(2)?,
                cut_time_change: to_rel_change(row.get(1)?, row.get(2)?),
                draw_time0: row.get(3)?,
                draw_time1: row.get(4)?,
                draw_time_change: to_rel_change(row.get(3)?, row.get(4)?),
                memory0: row.get(5)?,
                memory1: row.get(6)?,
                memory_change: to_rel_change(row.get(5)?, row.get(6)?),
            })
        })?
        .filter_map(|r| r.ok())
        .collect())
}

fn to_rel_change(t1: f64, t2: f64) -> f64 {
    t2 / t1 - 1.0
}

fn tera_relative_change() -> tera::GlobalFn {
    Box::new(move |args| -> tera::Result<tera::Value> {
        match args.get("val") {
            Some(val) => match tera::from_value::<f64>(val.clone()) {
                Ok(v) => {
                    let s = if v.is_nan() || v == -1.0 {
                        "?".to_string()
                    } else if v > 0.0 {
                        format!("+{:.1}%", 100.0 * v)
                    } else {
                        format!("{:.1}%", 100.0 * v)
                    };
                    Ok(tera::to_value(s).unwrap())
                }
                Err(_) => Ok("?".into()),
            },
            None => Err("oops".into()),
        }
    })
}

fn tera_to_color() -> tera::GlobalFn {
    Box::new(move |args| -> tera::Result<tera::Value> {
        match args.get("val") {
            Some(val) => match tera::from_value::<f64>(val.clone()) {
                Ok(v) => {
                    let s = if v.is_nan() || v == -1.0 {
                        "#f00"
                    } else if v > 0.05 {
                        "#f00"
                    } else if v < -0.05 {
                        "#0a0"
                    } else {
                        "#000"
                    };
                    Ok(tera::to_value(s).unwrap())
                }
                Err(_) => Ok("#f00".into()),
            },
            None => Err("oops".into()),
        }
    })
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
    let db_data = db_revision_history_for_file(&conn, table, &sql_columns, config_file)
        .map_err(error::ErrorInternalServerError)?;
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
    let json = json!({"labels": labels, "datasets": datasets}).to_string();
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
    let mut stmt = conn.prepare_cached(&format!(
        "SELECT revision, {} FROM {} WHERE config_file LIKE ?1 AND revision >= 800000 GROUP BY revision ORDER BY revision",
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

fn main() -> std::io::Result<()> {
    //std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    HttpServer::new(|| {
        //let mut tera = compile_templates!(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*"));
        //tera.register_function("relative_change", tera_relative_change());
        //tera.register_function("to_color", tera_to_color());
        let conn = Connection::open("cutsim-testreport.db").unwrap();
        App::new()
            //.data(tera)
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
