use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use serde::Serialize;

use crate::args::OutputFormat;

#[derive(Serialize)]
pub struct QueryOutput {
    pub query_id: String,
    pub status: String,
    pub rows: Vec<BTreeMap<String, String>>,
}

pub fn render_output(output: &QueryOutput, format: OutputFormat) -> Result<String> {
    match format {
        OutputFormat::Table => Ok(render_table(output)),
        OutputFormat::Json => Ok(format!("{}\n", serde_json::to_string_pretty(output)?)),
        OutputFormat::Csv => Ok(render_csv(&output.rows)),
    }
}

fn render_table(output: &QueryOutput) -> String {
    if output.rows.is_empty() {
        return format!(
            "status: {}\nquery_id: {}\nrows: 0\n",
            output.status, output.query_id
        );
    }

    let rows = &output.rows;
    let columns = columns(rows);
    let mut widths: Vec<usize> = columns.iter().map(|column| column.len()).collect();
    for row in rows {
        for (index, column) in columns.iter().enumerate() {
            let len = row.get(column).map(|value| value.len()).unwrap_or_default();
            widths[index] = widths[index].max(len);
        }
    }

    let mut result = String::new();
    push_table_row(&mut result, &columns, &widths);
    push_table_separator(&mut result, &widths);
    for row in rows {
        let values = columns
            .iter()
            .map(|column| row.get(column).cloned().unwrap_or_default())
            .collect::<Vec<_>>();
        push_table_row(&mut result, &values, &widths);
    }
    result
}

fn push_table_row(output: &mut String, values: &[String], widths: &[usize]) {
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            output.push_str("  ");
        }
        output.push_str(value);
        output.push_str(&" ".repeat(widths[index] - value.len()));
    }
    output.push('\n');
}

fn push_table_separator(output: &mut String, widths: &[usize]) {
    for (index, width) in widths.iter().enumerate() {
        if index > 0 {
            output.push_str("  ");
        }
        output.push_str(&"-".repeat(*width));
    }
    output.push('\n');
}

fn render_csv(rows: &[BTreeMap<String, String>]) -> String {
    let columns = columns(rows);
    let mut result = String::new();
    push_csv_row(&mut result, columns.iter().map(String::as_str));
    for row in rows {
        push_csv_row(
            &mut result,
            columns
                .iter()
                .map(|column| row.get(column).map(String::as_str).unwrap_or_default()),
        );
    }
    result
}

fn push_csv_row<'a>(output: &mut String, values: impl Iterator<Item = &'a str>) {
    for (index, value) in values.enumerate() {
        if index > 0 {
            output.push(',');
        }
        push_csv_value(output, value);
    }
    output.push('\n');
}

fn push_csv_value(output: &mut String, value: &str) {
    if value.contains([',', '"', '\n', '\r']) {
        output.push('"');
        output.push_str(&value.replace('"', "\"\""));
        output.push('"');
    } else {
        output.push_str(value);
    }
}

fn columns(rows: &[BTreeMap<String, String>]) -> Vec<String> {
    let mut columns = BTreeSet::new();
    for row in rows {
        columns.extend(row.keys().cloned());
    }

    let preferred_columns = [
        "day",
        "unique_ip_hashes",
        "shared_matches",
        "reserved_matches",
        "room_type",
        "action",
        "cw_room_type",
        "cw_action",
        "events",
        "@timestamp",
        "@message",
    ];
    let mut ordered_columns = Vec::new();
    for column in preferred_columns {
        if columns.remove(column) {
            ordered_columns.push(column.to_owned());
        }
    }
    ordered_columns.extend(columns);
    ordered_columns
}
