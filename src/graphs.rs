use crate::{SqliteDb, LOWEST_REVISION};
use itertools::Itertools;
use rocket::http::{RawStr, Status};
use rocket::response::{content, status};
use rusqlite::Connection;
use serde_json::json;
use std::collections::HashMap;
use std::iter::FromIterator;

#[get("/file/<file_type>?<id>")]
pub fn api_file_graph_json(
    conn: SqliteDb,
    file_type: &RawStr,
    id: &RawStr,
) -> Result<content::Json<String>, status::Custom<String>> {
    let (table, columns): (&str, Vec<(&str, &str)>) = match file_type.as_str() {
        "csb" => (
            "processed_csb",
            vec![("Memory", "memory_peak"), ("Run Time", "player_total_time")],
        ),
        "ini" => (
            "processed_ini",
            vec![
                ("Memory", "memory_peak"),
                ("Cut Time", "cutting_time"),
                ("Draw Time", "draw_time"),
            ],
        ),
        _ => {
            return Err(status::Custom(
                Status::BadRequest,
                "unexpected table type".to_string(),
            ));
        }
    };
    let id = match id.url_decode() {
        Ok(id) => id,
        _ => {
            return Err(status::Custom(
                Status::BadRequest,
                "couldn't decode id".to_string(),
            ))
        }
    };
    let sql_columns: Vec<_> = columns.iter().map(|c| c.1).collect();
    let db_data = db_revision_history_for_file(&conn, table, &sql_columns, &id)
        .map_err(|e| status::Custom(Status::InternalServerError, e.to_string()))?;
    let labels: Vec<_> = db_data.iter().map(|r| r.0).collect();
    let colors = vec![
        "rgb(54, 162, 235)",
        "rgb(255, 159, 64)",
        "rgb(75, 192, 192)",
    ];
    let datasets: Vec<_> = columns
        .into_iter()
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
    Ok(content::Json(
        json!({"labels": labels, "datasets": datasets}).to_string(),
    ))
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
        concat!(
            "SELECT revision, {} FROM {} ",
            "WHERE config_file LIKE ?1 ",
            "AND revision >= {} ",
            "GROUP BY revision ",
            "ORDER BY revision"
        ),
        column_str, table, LOWEST_REVISION
    ))?;
    let results = stmt.query_map(&[&config_file], |r| {
        let mut stats = Vec::new();
        for i in 0..columns.len() {
            stats.push(r.get(i + 1));
        }
        (r.get(0), stats)
    })?;
    Ok(results.filter_map(|r| r.ok()).collect())
}

#[get("/all/<file_type>?<r0>&<r1>")]
pub fn api_all_graph_json(
    conn: SqliteDb,
    file_type: &RawStr,
    r0: u32,
    r1: u32,
) -> Result<content::Json<String>, status::Custom<String>> {
    /*
    red: "rgb(255, 99, 132)",
    orange: "rgb(255, 159, 64)",
    yellow: "rgb(255, 205, 86)",
    green: "rgb(75, 192, 192)",
    blue: "rgb(54, 162, 235)",
    purple: "rgb(153, 102, 255)",
    grey: "rgb(201, 203, 207)"*/
    let info = match file_type.as_str() {
        "csb_memory" => (
            "Memory",
            "rgb(54, 162, 235)",
            "processed_csb",
            "memory_peak",
        ),
        "csb_play_time" => (
            "Run Time",
            "rgb(255, 205, 86)",
            "processed_csb",
            "player_total_time",
        ),
        "ini_memory" => (
            "Memory",
            "rgb(54, 162, 235)",
            "processed_ini",
            "memory_peak",
        ),
        "ini_cut_time" => (
            "Cut Time",
            "rgb(255, 159, 64)",
            "processed_ini",
            "cutting_time",
        ),
        "ini_draw_time" => (
            "Draw Time",
            "rgb(75, 192, 192)",
            "processed_ini",
            "draw_time",
        ),
        _ => {
            return Err(status::Custom(
                Status::BadRequest,
                "unexpected table type".to_string(),
            ));
        }
    };
    let db_data = db_revision_history_for_files(&conn, info.2, info.3, r0, r1)
        .map_err(|e| status::Custom(Status::InternalServerError, e.to_string()))?;

    let mut labels = std::collections::HashSet::new();
    let datasets: Vec<_> = db_data
        .into_iter()
        .map(|(test_name, runs)| {
            let first_value = runs[0].1;
            let data: Vec<_> = runs
                .into_iter()
                .map(|r| {
                    labels.insert(r.0);
                    json!({"x": r.0, "y": r.1 / first_value})
                })
                .collect();
            let name = test_name
                .split("\\testcases\\")
                .last()
                .unwrap_or(&test_name);
            json!({
                "label": name,
                "backgroundColor": info.1,
                "borderColor": info.1,
                "fill": false,
                "data": data
            })
        })
        .collect();
    let mut labels = Vec::from_iter(labels.iter());
    labels.sort();
    Ok(content::Json(
        json!({
            "labels": labels,
            "datasets": datasets
        })
        .to_string(),
    ))
}

fn db_revision_history_for_files(
    conn: &Connection,
    table: &str,
    column: &str,
    low_revision: u32,
    high_revision: u32,
) -> rusqlite::Result<HashMap<String, Vec<(u32, f64)>>> {
    let mut stmt = conn.prepare_cached(&format!(
        "SELECT config_file, revision, AVG({}) FROM {} WHERE revision >= ?1 AND revision <= ?2 GROUP BY revision, config_file ORDER BY revision",
        column, table
    ))?;
    let results = stmt
        .query_map(&[&low_revision, &high_revision], |r| {
            (r.get(0), r.get(1), r.get(2))
        })?
        .filter_map(|r| r.ok());
    let mut result = HashMap::new();
    for (config_file, revision, stats) in results {
        let t = result.entry(config_file).or_insert_with(Vec::new);
        t.push((revision, stats));
    }
    Ok(result)
}
