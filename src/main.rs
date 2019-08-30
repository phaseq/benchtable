use rusqlite::Connection;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::iter::FromIterator;

fn main() -> std::io::Result<()> {
    let first_revision = 800_000;
    let second_revision = 896_000;

    let conn = Connection::open("cutsim-testreport.db").unwrap();

    let csb_first = get_csb_runs(&conn, first_revision).unwrap();
    let csb_second = get_csb_runs(&conn, second_revision).unwrap();
    let ini_first = get_ini_runs(&conn, first_revision).unwrap();
    let ini_second = get_ini_runs(&conn, second_revision).unwrap();

    let mut out_file = File::create("report.html")?;
    let style = r#"
    body {font-family:monospace;}
    th,td{padding:0.3em 1em;text-align:right}
    th{font-weight:bold}
    "#;
    let script = r#"
    window.onload = function() {
        for (let element of document.querySelectorAll(".toggle-table")) {
            let name = element.parentElement.getAttribute("data-js-name");
            let in_body = [];
            let next = element.parentElement.parentElement.nextElementSibling;
            while (next && next.getAttribute("data-field-start") !== "true") {
                in_body.push(next);
                next = next.nextElementSibling;
            }
            for (let detail of in_body) {
                detail.style.display = "none";
            }
            element.addEventListener("toggle", evt => {
                for (let detail of in_body) {
                    if (element.open) {
                        detail.style.display = "";
                    } else {
                        detail.style.display = "none";
                    }
                }
            });
        }
    }
    "#;
    write!(
        out_file,
        "<html><head><title>{}</title><style>{}</style><script>{}</script></head><body>",
        "Benchmarks", style, script
    )?;

    let rows: Vec<(&str, &str, Box<dyn Fn(&(f64, f64)) -> f64>)> = vec![
        ("time", "s", Box::new(|r| r.0)),
        ("mem", " MB", Box::new(|r| r.1)),
    ];
    print_table(
        &mut out_file,
        "Csb Time",
        first_revision,
        second_revision,
        csb_first,
        csb_second,
        rows,
    )?;

    let rows: Vec<(&str, &str, Box<dyn Fn(&(f64, f64, f64)) -> f64>)> = vec![
        ("cut", "s", Box::new(|r| r.0)),
        ("draw", "s", Box::new(|r| r.1)),
        ("mem", " MB", Box::new(|r| r.2)),
    ];
    print_table(
        &mut out_file,
        "Ini Time",
        first_revision,
        second_revision,
        ini_first,
        ini_second,
        rows,
    )?;
    out_file.write_all(b"</body></html>")?;
    Ok(())
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

fn print_table<S>(
    out_file: &mut File,
    title: &str,
    first_revision: u32,
    second_revision: u32,
    csb_first: HashMap<String, Vec<S>>,
    csb_second: HashMap<String, Vec<S>>,
    row_printers: Vec<(&str, &str, Box<dyn Fn(&S) -> f64>)>,
) -> std::io::Result<()> {
    write!(
        out_file,
        "<h1>{}</h1><table><thead><tr><td /><th>r{}</th><th>r{}</th><th /></tr></thead><tbody>",
        title, first_revision, second_revision
    )?;

    let mut test_names = Vec::from_iter(csb_second.keys());
    test_names.sort_unstable();

    for test_name in test_names {
        let fst = csb_first.get(test_name);
        let snd = csb_second.get(test_name);
        if let Some(fst) = fst {
            if let Some(snd) = snd {
                write!(
                    out_file,
                    "<tr data-field-start=\"true\"><th data-js-name=\"{}\"><details class=\"toggle-table\"><summary>{}</summary></details></th>",
                    test_name, test_name
                )?;
                let data: Vec<_> = row_printers
                    .iter()
                    .map(|p| (p.0, p.1, averages_and_change(fst, snd, &p.2)))
                    .collect();
                for d in &data {
                    write!(
                        out_file,
                        "<td>{}: {}</td>",
                        d.0,
                        to_relative_change_html((d.2).2)
                    )?;
                }
                for d in data {
                    out_file.write_all(b"<tr>")?;
                    write!(
                        out_file,
                        "<tr><td>{}</td><td>{:.1}{}</td><td>{:.1}{}</td></tr>",
                        d.0,
                        (d.2).0,
                        d.1,
                        (d.2).1,
                        d.1
                    )?;
                }
                out_file.write_all(b"<tr><td colspan=\"3\">&nbsp;</td></tr>")?;
            }
        }
    }
    out_file.write_all(b"</table>")?;
    Ok(())
}

fn averages_and_change<C>(v1: &Vec<C>, v2: &Vec<C>, f: &dyn Fn(&C) -> f64) -> (f64, f64, f64) {
    let fst = average(v1, f);
    let snd = average(v2, f);
    (fst, snd, snd / fst)
}

fn average<C>(v: &Vec<C>, f: &dyn Fn(&C) -> f64) -> f64 {
    v.iter().map(|a| f(a)).sum::<f64>() / (v.len() as f64)
}

fn to_relative_change_html(change: f64) -> String {
    let color = match () {
        _ if change < 0.95 => "#090",
        _ if change > 1.05 => "#f00",
        _ => "#000",
    };
    let change_percent = match () {
        _ if change < 1.0 => format!("-{:.1}%", 100.0 * (1.0 - change)),
        _ if change > 1.0 => format!("+{:.1}%", 100.0 * (change - 1.0)),
        _ => "0%".to_owned(),
    };
    format!("<span style=\"color:{}\">{}</span>", color, change_percent)
}
