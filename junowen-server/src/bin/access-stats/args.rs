use std::{path::PathBuf, time::Duration};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand, ValueEnum};

const DEFAULT_QUERY: &str = "stats count(*) as events";
const IAM_CHECK_QUERY: &str = DEFAULT_QUERY;
const ROOM_ACTIVITY_QUERY: &str = r#"fields @timestamp, @message
| filter @message like /INFO/ and (@message like /\[Shared Room\]/ or @message like /\[Reserved Room\]/)
| parse @message /\[(?<cw_room_type>Shared|Reserved) Room\] (?<cw_action>Created|Removed|Answered|Join|Spectate): (?<cw_room_name>.*) ip_hash="(?<cw_ip_hash>[^"]+)"/
| sort @timestamp asc"#;

#[derive(Debug, Parser)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
    #[arg(long, global = true)]
    profile: String,
    #[arg(long, global = true)]
    region: Option<String>,
    #[arg(long, global = true)]
    function_name: String,
    #[arg(long, global = true)]
    out: Option<PathBuf>,
    #[arg(long, global = true, value_enum, default_value = "table")]
    format: OutputFormat,
    #[arg(long = "poll-interval-ms", global = true, default_value_t = 1000)]
    poll_interval_ms: u64,
    #[arg(long = "timeout-secs", global = true, default_value_t = 60)]
    timeout_secs: u64,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(name = "iam-check")]
    IamCheck {
        #[arg(long)]
        start: Option<String>,
        #[arg(long)]
        end: Option<String>,
    },
    #[command(name = "query")]
    Query {
        #[arg(long)]
        start: String,
        #[arg(long)]
        end: String,
        #[arg(long)]
        query: Option<String>,
    },
    #[command(name = "room-activity")]
    RoomActivity {
        #[arg(long)]
        start: String,
        #[arg(long)]
        end: String,
    },
}

#[derive(Debug)]
pub struct Args {
    pub common: CommonArgs,
    pub query: String,
    pub start_time: i64,
    pub end_time: i64,
    pub post_process: PostProcess,
}

#[derive(Debug)]
pub struct CommonArgs {
    pub profile: String,
    pub region: Option<String>,
    pub function_name: String,
    pub out: Option<PathBuf>,
    pub format: OutputFormat,
    pub poll_interval: Duration,
    pub timeout: Duration,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
    Csv,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PostProcess {
    None,
    RoomActivity,
}

impl TryFrom<Cli> for Args {
    type Error = anyhow::Error;

    fn try_from(cli: Cli) -> Result<Self> {
        let common = CommonArgs {
            profile: cli.profile,
            region: cli.region,
            function_name: cli.function_name,
            out: cli.out,
            format: cli.format,
            poll_interval: Duration::from_millis(cli.poll_interval_ms),
            timeout: Duration::from_secs(cli.timeout_secs),
        };
        let (query, start_time, end_time, post_process) = command_values(cli.command)?;

        Ok(Self {
            common,
            query,
            start_time,
            end_time,
            post_process,
        })
    }
}

fn command_values(command: Command) -> Result<(String, i64, i64, PostProcess)> {
    let values = match command {
        Command::IamCheck { start, end } => {
            let end_time = end
                .as_deref()
                .map(|value| parse_rfc3339("--end", value))
                .transpose()?
                .unwrap_or_else(|| Utc::now().timestamp());
            let start_time = start
                .as_deref()
                .map(|value| parse_rfc3339("--start", value))
                .transpose()?
                .unwrap_or(end_time - 300);
            (
                IAM_CHECK_QUERY.to_owned(),
                start_time,
                end_time,
                PostProcess::None,
            )
        }
        Command::Query { start, end, query } => {
            let start_time = parse_rfc3339("--start", &start)?;
            let end_time = parse_rfc3339("--end", &end)?;
            let query = query.unwrap_or_else(|| DEFAULT_QUERY.to_owned());
            (query, start_time, end_time, PostProcess::None)
        }
        Command::RoomActivity { start, end } => {
            let start_time = parse_rfc3339("--start", &start)?;
            let end_time = parse_rfc3339("--end", &end)?;
            (
                ROOM_ACTIVITY_QUERY.to_owned(),
                start_time,
                end_time,
                PostProcess::RoomActivity,
            )
        }
    };

    if values.2 <= values.1 {
        bail!("--end must be later than --start");
    }
    Ok(values)
}

fn parse_rfc3339(name: &str, value: &str) -> Result<i64> {
    DateTime::parse_from_rfc3339(value)
        .with_context(|| {
            format!("{name} must be an RFC 3339 timestamp, e.g. 2026-06-01T00:00:00+09:00")
        })
        .map(|timestamp| timestamp.timestamp())
}
