use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use litchi_api::LitchiApi;
use litchitool::{
    csv_format::{csv, read_from_csv},
    mission::LitchiMission,
};
use serde::Deserialize;
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
    /// Use pretty and more detailed log
    #[arg(short)]
    pretty_logs: bool,
}

#[derive(Deserialize)]
pub struct UploadConfig {
    username: String,
    password: String,
}

#[derive(Subcommand)]
pub enum Command {
    /// Convert a CSV file to a litchi mission file
    ConvertCsv {
        /// Input CSV file
        input: PathBuf,
        /// Output file path
        output: PathBuf,
    },
    /// Upload a CSV file to the litch cloud
    Upload {
        /// File to upload
        input: PathBuf,
        #[arg(short, long)]
        config: PathBuf,
        #[arg(short, long)]
        name: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let formatter = tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env());

    if cli.pretty_logs {
        formatter
            .pretty()
            .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
            .init();
    } else {
        formatter.init();
    }

    match cli.command {
        Command::ConvertCsv { input, output } => {
            let mission = read_csv_to_mission(&input);
            std::fs::write(output, mission.to_binary()).expect("Could not write mission to file");
        }
        Command::Upload {
            input,
            config,
            name,
        } => {
            let mission = read_csv_to_mission(&input);
            let config: UploadConfig = serde_json::from_str(
                &std::fs::read_to_string(config).expect("Could not read upload configuration"),
            )
            .unwrap();

            let api = LitchiApi::login(&config.username, &config.password)
                .await
                .expect("Authentication with litchi api failed");
            api.upload(&mission, &name)
                .await
                .expect("Failed to uploda mission to Litchi");
            api.sync_devices().await.expect("Failed to sync deices");
        }
    }
}

fn read_csv_to_mission(csv_path: &Path) -> LitchiMission {
    let csv_file = csv::Reader::from_path(csv_path).expect("Failed to create reader over file");

    read_from_csv(csv_file).expect("Failed to parse CSV")
}
