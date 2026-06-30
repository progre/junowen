mod args;
mod cloudwatch;
mod output;
mod room_activity;

use std::fs;

use anyhow::{Context, Result};
use args::{Args, Cli, PostProcess};
use aws_config::profile::ProfileFileCredentialsProvider;
use aws_credential_types::provider::ProvideCredentials;
use aws_sdk_cloudwatchlogs::Client;
use aws_types::region::Region;
use clap::Parser;
use output::render_output;
use room_activity::summarize_room_activity;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::try_from(Cli::parse())?;
    let credentials_provider = ProfileFileCredentialsProvider::builder()
        .profile_name(&args.common.profile)
        .build();
    credentials_provider
        .provide_credentials()
        .await
        .with_context(|| {
            format!(
                "failed to load AWS credentials for profile `{}`. Check `aws sts get-caller-identity --profile {}` and make sure the profile contains aws_access_key_id and aws_secret_access_key in ~/.aws/credentials, or in a [profile {}] section of ~/.aws/config",
                args.common.profile, args.common.profile, args.common.profile
            )
        })?;

    let mut config_loader = aws_config::from_env().credentials_provider(credentials_provider);
    if let Some(region) = &args.common.region {
        config_loader = config_loader.region(Region::new(region.clone()));
    }
    let config = config_loader.load().await;
    let client = Client::new(&config);

    let output = cloudwatch::run_query(&client, &args).await?;
    let output = match args.post_process {
        PostProcess::None => output,
        PostProcess::RoomActivity => summarize_room_activity(output)?,
    };
    let rendered = render_output(&output, args.common.format)?;
    if let Some(path) = args.common.out {
        fs::write(&path, rendered)
            .with_context(|| format!("failed to write {}", path.display()))?;
    } else {
        print!("{rendered}");
    }

    Ok(())
}
