use crate::{SqliteDb, LOWEST_REVISION};
use rocket::http::{ContentType, RawStr, Status};
use rocket::response::{content::Content, status};
use rusqlite::{Connection, NO_PARAMS};

#[get("/?<r1>&<r2>&<sort>")]
pub fn index(
    conn: SqliteDb,
    r1: Option<u32>,
    r2: Option<u32>,
    sort: Option<&RawStr>,
) -> Result<Content<String>, status::Custom<String>> {
    let revisions = db_all_revisions(&conn, "processed_csb")
        .map_err(|e| status::Custom(Status::InternalServerError, e.to_string()))?;

    let revision_low = r1.unwrap_or_else(|| revisions[revisions.len() - 5]);
    let revision_high = r2.unwrap_or_else(|| *revisions.last().unwrap());
    let sort = sort
        .and_then(|s| s.url_decode().ok())
        .unwrap_or_else(|| "cut time".to_string());

    let (csb_tests, ini_tests) = db_revision_comparison(&conn, revision_low, revision_high, &sort)
        .map_err(|e| status::Custom(Status::InternalServerError, e.to_string()))?;

    let html = format!(
        "{}",
        Index {
            page: &Page {
                revisions,
                revision_low,
                revision_high,
                sort,
                csb_tests,
                ini_tests,
            }
        }
    );
    Ok(Content(ContentType::HTML, html))
}

pub struct Page {
    revisions: Vec<u32>,
    revision_low: u32,
    revision_high: u32,
    sort: String,
    csb_tests: Vec<CsbTest>,
    ini_tests: Vec<IniTest>,
}

markup::define! {
    Index<'a>(page: &'a Page) {
        {markup::doctype()}
        html {
            head {
                title { "CutSim Benchmarks" }
                script[src="static/Chart.min.js"] {}
                script[src="static/table.js"] {}
                link[rel="stylesheet", href="static/style.css"] {}
            }
            body {
                {Form { page }}
                div#summary_charts {
                    button[
                        onclick = format!("loadSummaryCharts({},{})",
                            page.revision_low, page.revision_high)
                    ] {
                        "Load Summary Charts"
                    }
                }
                {CsbTable { page }}
                {IniTable { page }}
            }
        }
    }
    Form<'a>(page: &'a Page) {
        form {
            "Revision range: "
            select[name="r1"] {
                {Revisions {page, selected_revision: page.revision_low}}
            }
            select[name="r2"] {
                {Revisions {page, selected_revision: page.revision_high}}
            }
            "Sort by: "
            select[name="sort"] {
                option[selected? = page.sort == "name"] { "name" }
                option[selected? = page.sort == "cut time"] { "cut time" }
                option[selected? = page.sort == "draw time"] { "draw time" }
                option[selected? = page.sort == "memory"] { "memory" }
            }
            " "
            input[type="submit", value="Ok"] {}
        }
    }
    Revisions<'a>(page: &'a Page, selected_revision: u32) {
        @for r in page.revisions.iter() {
            option[selected?=r == selected_revision] {{r}}
        }
    }
    CsbTable<'a>(page: &'a Page) {
        h1 { "CSB Benchmarks" }
        table.benchtable {
            tbody {
                @for test in page.csb_tests.iter() {
                    {CsbRow { page, test } }
                }
            }
        }
    }
    CsbRow<'a>(page: &'a Page, test: &'a CsbTest) {
        tr["data-field-start" = true] {
            th["data-js-name" = &test.name] {
                details."toggle-table" {
                    summary { {test.name} }
                }
            }
            td {
                "time: "
                span[style = to_style(test.time0, test.time1)] {
                    {relative_change(test.time0, test.time1)}
                }
            }
            td {
                "mem: "
                span[style = to_style(test.memory0, test.memory1)] {
                    {relative_change(test.memory0, test.memory1)}
                }
            }
        }
        tr[style = "display:none"] {
            th[style = "text-align:right"] { "r" {page.revision_low} }
            td { {format_time(test.time0)} }
            td { {format_mem(test.memory0)} }
        }
        tr[style = "display:none"] {
            th[style = "text-align:right"] { "r" {page.revision_high} }
            td { {format_time(test.time1)} }
            td { {format_mem(test.memory1)} }
        }
        tr[style = "display:none"] {
            td[colspan = 3, class="chart", "data-chart-id" = &test.name] { "&nbsp;" }
        }
    }
    IniTable<'a>(page: &'a Page) {
        h1 { "CSB Benchmarks" }
        table.benchtable {
            tbody {
                @for test in page.ini_tests.iter() {
                    {IniRow { page, test } }
                }
            }
        }
    }
    IniRow<'a>(page: &'a Page, test: &'a IniTest) {
        tr["data-field-start" = true] {
            th["data-js-name" = &test.name] {
                details."toggle-table" {
                    summary { {test.name} }
                }
            }
            td {
                "cut: "
                span[style = to_style(test.cut_time0, test.cut_time1)] {
                    {relative_change(test.cut_time0, test.cut_time1)}
                }
            }
            td {
                "draw: "
                span[style = to_style(test.draw_time0, test.draw_time1)] {
                    {relative_change(test.draw_time0, test.draw_time1)}
                }
            }
            td {
                "mem: "
                span[style = to_style(test.memory0, test.memory1)] {
                    {relative_change(test.memory0, test.memory1)}
                }
            }
        }
        tr[style = "display:none"] {
            th[style = "text-align:right"] { "r" {page.revision_low} }
            td { {format_time(test.cut_time0)} }
            td { {format_time(test.draw_time0)} }
            td { {format_mem(test.memory0)} }
        }
        tr[style = "display:none"] {
            th[style = "text-align:right"] { "r" {page.revision_high} }
            td { {format_time(test.cut_time1)} }
            td { {format_time(test.draw_time0)} }
            td { {format_mem(test.memory1)} }
        }
        tr[style = "display:none"] {
            td[colspan = 4, class="chart", "data-chart-id" = &test.name] { "&nbsp;" }
        }
    }
}

#[allow(clippy::float_cmp)]
pub fn relative_change(v1: f64, v2: f64) -> String {
    let v = v2 / v1 - 1.0;
    if v.is_nan() || v.is_infinite() || v == -1.0 {
        "?".to_string()
    } else if v > 0.0 {
        format!("+{:.1}%", 100.0 * v)
    } else {
        format!("{:.1}%", 100.0 * v)
    }
}

#[allow(clippy::float_cmp)]
pub fn to_style(v1: f64, v2: f64) -> &'static str {
    let v = v2 / v1 - 1.0;
    if v.is_nan() || v.is_infinite() || v == -1.0 || v > 0.05 {
        "color:#e00;font-weight:bold"
    } else if v < -0.05 {
        "color:#0a0;font-weight:bold"
    } else {
        "color:#aaa"
    }
}

fn format_time(t: f64) -> String {
    format!("{:.2}s", t)
}

fn format_mem(m: f64) -> String {
    format!("{:.0} MB", m)
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

pub struct CsbTest {
    name: String,
    time0: f64,
    time1: f64,
    memory0: f64,
    memory1: f64,
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

pub struct IniTest {
    name: String,
    cut_time0: f64,
    cut_time1: f64,
    draw_time0: f64,
    draw_time1: f64,
    memory0: f64,
    memory1: f64,
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
