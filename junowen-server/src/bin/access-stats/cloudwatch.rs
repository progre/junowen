use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Context, Result};
use aws_sdk_cloudwatchlogs::Client;
use tokio::time;

use crate::{args::Args, output::QueryOutput};

pub async fn run_query(client: &Client, args: &Args) -> Result<QueryOutput> {
    let start_query_output = client
        .start_query()
        .log_group_name(format!("/aws/lambda/{}", args.common.function_name))
        .start_time(args.start_time)
        .end_time(args.end_time)
        .query_string(&args.query)
        .send()
        .await
        .context("failed to start CloudWatch Logs Insights query")?;
    let query_id = start_query_output
        .query_id()
        .ok_or_else(|| anyhow!("CloudWatch Logs did not return a query id"))?
        .to_owned();

    let deadline = time::Instant::now() + args.common.timeout;
    loop {
        let output = client
            .get_query_results()
            .query_id(&query_id)
            .send()
            .await
            .context("failed to get CloudWatch Logs Insights query results")?;
        let status = output
            .status()
            .map(|status| status.as_str().to_owned())
            .unwrap_or_else(|| "Unknown".to_owned());

        match status.as_str() {
            "Complete" => {
                return Ok(QueryOutput {
                    query_id,
                    status,
                    rows: collect_rows(output.results()),
                });
            }
            "Failed" | "Cancelled" | "Timeout" => {
                bail!("CloudWatch Logs Insights query ended with status {status}");
            }
            _ => {
                if time::Instant::now() >= deadline {
                    bail!("timed out waiting for query {query_id}; last status was {status}");
                }
                time::sleep(args.common.poll_interval).await;
            }
        }
    }
}

fn collect_rows(
    results: Option<&[Vec<aws_sdk_cloudwatchlogs::types::ResultField>]>,
) -> Vec<BTreeMap<String, String>> {
    results
        .unwrap_or_default()
        .iter()
        .map(|fields| {
            fields
                .iter()
                .filter_map(|field| {
                    Some((
                        field.field()?.to_owned(),
                        field.value().unwrap_or_default().to_owned(),
                    ))
                })
                .collect()
        })
        .collect()
}
