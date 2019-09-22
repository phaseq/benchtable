use crate::{SqliteDb, LOWEST_REVISION};
use itertools::Itertools;
use rocket::http::{RawStr, Status};
use rocket::response::{content, status};
use rusqlite::Connection;
use serde_json::json;
use std::collections::HashMap;
use std::iter::FromIterator;

/*
Color scheme (copied from Chart.js examples):

red: "rgb(255, 99, 132)",
orange: "rgb(255, 159, 64)",
yellow: "rgb(255, 205, 86)",
green: "rgb(75, 192, 192)",
blue: "rgb(54, 162, 235)",
purple: "rgb(153, 102, 255)",
grey: "rgb(201, 203, 207)"
*/

#[get("/file/<file_type>?<id>")]
pub fn api_file_graph_json(
    conn: SqliteDb,
    file_type: &RawStr,
    id: &RawStr,
) -> Result<content::Json<String>, status::Custom<String>> {
    let (sql_table, sql_columns, column_titles): (&str, Vec<&str>, Vec<&str>) =
        match file_type.as_str() {
            "csb" => (
                "processed_csb",
                vec!["memory_peak", "player_total_time"],
                vec!["Memory", "Run Time"],
            ),
            "ini" => (
                "processed_ini",
                vec!["memory_peak", "cutting_time", "draw_time"],
                vec!["Memory", "Cut Time", "Draw Time"],
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
    let revision_info = db_revision_history_for_file(&conn, sql_table, &sql_columns, &id)
        .map_err(|e| status::Custom(Status::InternalServerError, e.to_string()))?;
    let reference_stats = &revision_info[0].stats;
    let labels: Vec<_> = revision_info.iter().map(|r| r.revision).collect();
    let colors = vec![
        "rgb(54, 162, 235)",
        "rgb(255, 159, 64)",
        "rgb(75, 192, 192)",
    ];
    let datasets: Vec<_> = column_titles
        .into_iter()
        .enumerate()
        .map(|(i, title)| {
            let data: Vec<_> = revision_info
                .iter()
                .map(|r| {
                    json!({
                    "x": r.revision,
                    "y": r.stats[i] / reference_stats[i],
                    "v": r.stats[i]})
                })
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

struct RevisionInfos {
    revision: u32,
    stats: Vec<f64>,
}
fn db_revision_history_for_file(
    conn: &Connection,
    table: &str,
    columns: &[&str],
    config_file: &str,
) -> rusqlite::Result<Vec<RevisionInfos>> {
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
        RevisionInfos {
            revision: r.get(0),
            stats,
        }
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
            let first_value = runs[0].stat;
            let data: Vec<_> = runs
                .into_iter()
                .map(|r| {
                    labels.insert(r.revision);
                    json!({"x": r.revision, "y": r.stat / first_value})
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

struct RevisionInfo {
    revision: u32,
    stat: f64,
}
fn db_revision_history_for_files(
    conn: &Connection,
    table: &str,
    column: &str,
    low_revision: u32,
    high_revision: u32,
) -> rusqlite::Result<HashMap<String, Vec<RevisionInfo>>> {
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
    for (config_file, revision, stat) in results {
        let t = result.entry(config_file).or_insert_with(Vec::new);
        t.push(RevisionInfo { revision, stat });
    }
    Ok(result)
}
