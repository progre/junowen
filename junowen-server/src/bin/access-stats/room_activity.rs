use std::collections::{BTreeMap, BTreeSet};

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone, Utc};

use crate::output::QueryOutput;

pub fn summarize_room_activity(output: QueryOutput) -> Result<QueryOutput> {
    let timezone = FixedOffset::east_opt(9 * 60 * 60).unwrap();
    let mut summaries = BTreeMap::<String, RoomActivitySummary>::new();

    for row in &output.rows {
        let timestamp = row
            .get("@timestamp")
            .ok_or_else(|| anyhow!("room activity row did not contain @timestamp"))?;
        let day = parse_cloudwatch_timestamp(timestamp)?
            .with_timezone(&timezone)
            .format("%Y-%m-%d")
            .to_string();
        let summary = summaries.entry(day).or_default();

        if let Some(ip_hash) = row.get("cw_ip_hash") {
            if !ip_hash.is_empty() {
                summary.ip_hashes.insert(ip_hash.to_owned());
            }
        }

        match (
            row.get("cw_room_type").map(String::as_str),
            row.get("cw_action").map(String::as_str),
        ) {
            (Some("Shared"), Some("Answered")) => summary.shared_matches += 1,
            (Some("Reserved"), Some("Join")) => summary.reserved_matches += 1,
            _ => {}
        }
    }

    Ok(QueryOutput {
        query_id: output.query_id,
        status: output.status,
        rows: summaries
            .into_iter()
            .map(|(day, summary)| {
                BTreeMap::from([
                    ("day".to_owned(), day),
                    (
                        "unique_ip_hashes".to_owned(),
                        summary.ip_hashes.len().to_string(),
                    ),
                    (
                        "shared_matches".to_owned(),
                        summary.shared_matches.to_string(),
                    ),
                    (
                        "reserved_matches".to_owned(),
                        summary.reserved_matches.to_string(),
                    ),
                ])
            })
            .collect(),
    })
}

#[derive(Default)]
struct RoomActivitySummary {
    ip_hashes: BTreeSet<String>,
    shared_matches: u64,
    reserved_matches: u64,
}

fn parse_cloudwatch_timestamp(value: &str) -> Result<DateTime<Utc>> {
    if let Ok(timestamp) = DateTime::parse_from_rfc3339(value) {
        return Ok(timestamp.with_timezone(&Utc));
    }

    let timestamp = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S%.f")
        .with_context(|| format!("failed to parse CloudWatch timestamp `{value}`"))?;
    Ok(Utc.from_utc_datetime(&timestamp))
}
