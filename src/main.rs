mod serving;
mod setup;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about = "Managed DwarfStar plugin; downloads are explicit")]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(hide = true)]
    WatchBackend {
        runtime: PathBuf,
        weights: PathBuf,
        context: u32,
        port: u16,
        parent: u32,
    },
    /// Show the initial pinned model catalog; does not download anything.
    Catalog,
    /// Download and verify weights. Never starts inference.
    Download {
        #[arg(long, value_parser = ["ds4f-q2"])]
        model: String,
        #[arg(long)]
        directory: PathBuf,
        #[arg(long)]
        accept_download: bool,
    },
    /// Run as a Mesh plugin using an already provisioned runtime and weights.
    Serve {
        #[arg(long)]
        runtime: Option<PathBuf>,
        #[arg(long)]
        weights: PathBuf,
        /// Run a direct loopback trial without Mesh (HTTP port printed on stderr).
        #[arg(long)]
        standalone: bool,
        #[arg(long, default_value_t = 4096, value_parser = clap::value_parser!(u32).range(512..=32768))]
        context: u32,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    match Args::parse().command {
        Commands::WatchBackend {
            runtime,
            weights,
            context,
            port,
            parent,
        } => serving::watch_backend(runtime, weights, context, port, parent).await,
        Commands::Catalog => {
            println!(
                "ds4f-q2: DeepSeek V4 Flash Q2; {} bytes (~81 GiB).\nAllow additional RAM for context/runtime; resident use targets 96+ GB machines.\nReview model licence: https://huggingface.co/antirez/deepseek-v4-gguf",
                setup::BYTES
            );
            Ok(())
        }
        Commands::Download {
            model: _,
            directory,
            accept_download,
        } => setup::download(&directory, accept_download).await,
        Commands::Serve {
            runtime,
            weights,
            context,
            standalone,
        } => {
            let runtime = runtime.unwrap_or(
                std::env::current_exe()?
                    .parent()
                    .expect("executable parent")
                    .join("runtime"),
            );
            serving::run(runtime, weights, context, standalone).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_cannot_download_implicitly() {
        assert!(Args::try_parse_from(["ds4"]).is_err());
        assert!(
            Args::try_parse_from([
                "ds4",
                "serve",
                "--runtime",
                "/r",
                "--weights",
                "/m",
                "--context",
                "0"
            ])
            .is_err()
        );
        assert!(
            Args::try_parse_from(["ds4", "download", "--directory", "/m", "--model", "unknown"])
                .is_err()
        );
    }
}
