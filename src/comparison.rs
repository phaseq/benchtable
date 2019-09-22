use crate::{SqliteDb, LOWEST_REVISION};
use rocket::http::{RawStr, Status};
use rocket::response::status;
use rocket_contrib::templates::Template;
use rusqlite::{Connection, NO_PARAMS};
use serde::Serialize;
use tera::Context;

#[get("/?<r1>&<r2>&<sort>")]
pub fn index(
    conn: SqliteDb,
    r1: Option<u32>,
    r2: Option<u32>,
    sort: Option<&RawStr>,
) -> Result<Template, status::Custom<String>> {
    let revisions = db_all_revisions(&conn, "processed_csb")
        .map_err(|e| status::Custom(Status::InternalServerError, e.to_string()))?;

    let first_revision = r1.unwrap_or_else(|| revisions[revisions.len() - 5]);
    let second_revision = r2.unwrap_or_else(|| *revisions.last().unwrap());
    let sort = sort
        .and_then(|s| s.url_decode().ok())
        .unwrap_or_else(|| "cut time".to_string());

    let (csb_tests, ini_tests) =
        db_revision_comparison(&conn, first_revision, second_revision, &sort)
            .map_err(|e| status::Custom(Status::InternalServerError, e.to_string()))?;

    let mut context = Context::new();
    context.insert("title", "CutSim benchmarks");
    context.insert("revision_low", &first_revision);
    context.insert("revision_high", &second_revision);
    context.insert("revisions", &revisions);
    context.insert("sort", &sort);
    context.insert("csb_tests", &csb_tests);
    context.insert("ini_tests", &ini_tests);
    Ok(Template::render("index", &context))
}

fn db_all_revisions(conn: &Connection, table: &str) -> rusqlite::Result<Vec<u32>> {
    Ok(conn
        .prepare(&format!(
            "SELECT DISTINCT revision FROM {} WHERE revision >= {} ORDER BY revision",
            table, LOWEST_REVISION
        ))?
        .query_map(NO_PARAMS, |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect())
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
        "FROM processed_csb A ",
        "INNER JOIN processed_csb b ON a.config_file = b.config_file ",
        "WHERE a.revision=?1 AND b.revision=?2 GROUP BY a.config_file "
    )
    .to_string()
        + &format!("ORDER BY {}", order_by);

    Ok(conn
        .prepare_cached(&query)?
        .query_map(&[&revision1, &revision2], |row| {
            let name: String = row.get(0);
            let name = name.split("\\testcases\\").last().unwrap_or(&name);
            CsbTest {
                name: name.to_string(),
                time0: row.get(1),
                time1: row.get(2),
                memory0: row.get(3),
                memory1: row.get(4),
            }
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
        "FROM processed_ini a ",
        "INNER JOIN processed_ini b ON a.config_file = b.config_file ",
        "WHERE a.revision=?1 AND b.revision=?2 GROUP BY a.config_file "
    )
    .to_string()
        + &format!("ORDER BY {}", order_by);

    Ok(conn
        .prepare_cached(&query)?
        .query_map(&[&revision1, &revision2], |row| {
            let name: String = row.get(0);
            let name = name.split("\\testcases\\").last().unwrap_or(&name);
            IniTest {
                name: name.to_string(),
                cut_time0: row.get(1),
                cut_time1: row.get(2),
                draw_time0: row.get(3),
                draw_time1: row.get(4),
                memory0: row.get(5),
                memory1: row.get(6),
            }
        })?
        .filter_map(|r| r.ok())
        .collect())
}

#[derive(Serialize)]
struct CsbTest {
    name: String,
    time0: f64,
    time1: f64,
    memory0: f64,
    memory1: f64,
}

#[derive(Serialize)]
struct IniTest {
    name: String,
    cut_time0: f64,
    cut_time1: f64,
    draw_time0: f64,
    draw_time1: f64,
    memory0: f64,
    memory1: f64,
}
