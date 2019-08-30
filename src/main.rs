#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]
extern crate rocket;
#[macro_use]
extern crate tera;

use rocket::http::RawStr;
use rocket_contrib::templates::Template;
//use rusqlite::Connection;
use serde_derive::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::iter::FromIterator;
use tera::Context;

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

#[get("/?<r0>&<r1>")]
fn index(r0: Option<&RawStr>, r1: Option<&RawStr>) -> Template {
    let first_revision = r0.and_then(parse_revision).unwrap_or(800_000);
    let second_revision = r1.and_then(parse_revision).unwrap_or(896_000);

    /*let conn = Connection::open("cutsim-testreport.db").unwrap();

    let csb_first = get_csb_runs(&conn, first_revision).unwrap();
    let csb_second = get_csb_runs(&conn, second_revision).unwrap();
    let ini_first = get_ini_runs(&conn, first_revision).unwrap();
    let ini_second = get_ini_runs(&conn, second_revision).unwrap();*/

    /*let mut csb_first = HashMap::new();
    csb_first.insert("test1".to_string(), vec![(1.5, 1240.0)]);
    let mut csb_second = HashMap::new();
    csb_second.insert("test1".to_string(), vec![(1.3, 220.0)]);*/

    let csb_tests = vec![CsbTest {
        name: "test1".to_string(),
        time0: 1.5,
        time1: 1.3,
        time_change: -1.4,
        memory0: 240.0,
        memory1: 1220.0,
        memory_change: 90.0,
    }];

    let ini_tests = vec![IniTest {
        name: "test_ini".to_string(),
        cut_time0: 1.5,
        cut_time1: 1.3,
        cut_time_change: -1.4,
        draw_time0: 11.5,
        draw_time1: 15.3,
        draw_time_change: 10.4,
        memory0: 240.0,
        memory1: 1220.0,
        memory_change: 90.0,
    }];

    let mut context = Context::new();
    context.insert("title", "CutSim benchmarks");
    context.insert("revision_low", &first_revision);
    context.insert("revision_high", &second_revision);
    context.insert("csb_tests", &csb_tests);
    context.insert("ini_tests", &ini_tests);
    Template::render("index", &context)
}

fn parse_revision(revision: &RawStr) -> Option<u32> {
    revision
        .percent_decode()
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
}

fn main() {
    rocket::ignite()
        .mount("/", routes![index])
        .attach(Template::fairing())
        .launch();
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
/*
fn get_csb_runs(
    conn: &Connection,
    revision: u32,
) -> rusqlite::Result<HashMap<String, Vec<(f64, f64)>>> {
    let mut stmt = conn.prepare(
        "SELECT config_file, player_total_time, memory_peak FROM processed_csb WHERE revision=?1",
    )?;
    let runs = stmt.query_map(&[&revision], |row| {
        let name: String = row.get(0)?;
        let name = name.split('\\').last().unwrap_or(&name);
        Ok(CsbRow {
            name: name.to_string(),
            time: row.get(1)?,
            memory: row.get(2)?,
        })
    })?;

    let mut results = HashMap::new();
    for run in runs {
        if let Ok(run) = run {
            let list = results.entry(run.name).or_insert(vec![]);
            list.push((run.time, run.memory));
        }
    }
    return Ok(results);
}

fn get_ini_runs(
    conn: &Connection,
    revision: u32,
) -> rusqlite::Result<HashMap<String, Vec<(f64, f64, f64)>>> {
    let mut stmt = conn.prepare(
        "SELECT config_file, cutting_time, draw_time, memory_peak FROM processed_ini WHERE revision=?1",
    )?;
    let runs = stmt.query_map(&[&revision], |row| {
        let name: String = row.get(0)?;
        let name = name.split('\\').last().unwrap_or(&name);
        Ok(IniRow {
            name: name.to_string(),
            cut_time: row.get(1)?,
            draw_time: row.get(2)?,
            memory: row.get(3)?,
        })
    })?;

    let mut results = HashMap::new();
    for run in runs {
        if let Ok(run) = run {
            let list = results.entry(run.name).or_insert(vec![]);
            list.push((run.cut_time, run.draw_time, run.memory));
        }
    }
    return Ok(results);
}
*/
