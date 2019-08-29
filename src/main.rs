use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::iter::FromIterator;

#[derive(Deserialize)]
struct Report {
    data: HashMap<String, Vec<TestRun>>,
    title: String,
}

#[derive(Deserialize, Clone)]
struct TestRun(u32, String, String, f32, f32, f32, u32, f32, f32, f32, u32);

fn main() -> std::io::Result<()> {
    let input_file = File::open("data.json")?;
    let buf_reader = std::io::BufReader::new(input_file);
    let report: Report = serde_json::from_reader(buf_reader)?;

    let first_revision = 750_000;
    let second_revision = 896_000;

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
        "<html><head><title>{}</title><style>{}</style><script>{}</script></head><body><table><thead><tr><td />",
        report.title, style, script
    )?;

    write!(
        out_file,
        "<th>r{}</th><th>r{}</th><th /></tr></thead><tbody>",
        first_revision, second_revision
    )?;

    let mut test_names = Vec::from_iter(report.data.keys());
    test_names.sort_unstable();
    for test_name in test_names {
        write!(
            out_file,
            "<tr data-field-start=\"true\"><th data-js-name=\"{}\"><details class=\"toggle-table\"><summary>{}</summary></details></th>",
            test_name, test_name
        )?;
        let info = &report.data[test_name];
        let fst = info.iter().find(|run| run.0 == first_revision);
        let snd = info.iter().find(|run| run.0 == second_revision);
        if fst.is_some() && snd.is_some() {
            let change = snd.unwrap().4 / fst.unwrap().4;
            write!(
                out_file,
                "<td /><td>avg: {}</td></tr>",
                to_relative_change_html(change)
            )?;
        } else {
            out_file.write_all(b"<td colspan=\"2\" /></tr>")?;
        }

        match fst {
            Some(run) => write!(out_file, "<tr><td /><td>{:.1}s</td>", run.4)?,
            None => out_file.write_all(b"<tr><td />")?,
        }
        match snd {
            Some(run) => write!(out_file, "<td>{:.1}s</td></tr>", run.4)?,
            None => out_file.write_all(b"<td /></tr>")?,
        }
        out_file.write_all(b"<tr><td colspan=\"3\">&nbsp;</td></tr>")?;
    }
    out_file.write_all(b"</table></body></html>")?;
    Ok(())
}

fn to_relative_change_html(change: f32) -> String {
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
