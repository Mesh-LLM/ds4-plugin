mod serving;

use anyhow::Result;
use clap::{Args as ClapArgs, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about = "Managed DwarfStar inference plugin for Mesh")]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(ClapArgs)]
#[group(required = true, multiple = false)]
struct Source {
    /// ds4 model name (as accepted by upstream download_model.sh, e.g. ds4f-q2,
    /// ds41f-q2, glm53-q2, qwen38-q4k). Downloaded on first start if missing.
    #[arg(long)]
    model: Option<String>,
    /// Existing GGUF weight file to serve instead of a named model.
    #[arg(long)]
    weights: Option<PathBuf>,
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
    /// Run as a Mesh plugin: fetch the model if needed, then serve it.
    Serve {
        #[command(flatten)]
        source: Source,
        /// Where named models are stored (default ~/.mesh-llm/models/ds4).
        #[arg(long)]
        model_dir: Option<PathBuf>,
        /// Context size passed to ds4-server; omit to use its default.
        #[arg(long)]
        context: Option<u32>,
        #[arg(long, hide = true)]
        runtime: Option<PathBuf>,
        /// Run a direct loopback trial without Mesh (HTTP port printed on stderr).
        #[arg(long)]
        standalone: bool,
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
        Commands::Serve {
            source,
            model_dir,
            context,
            runtime,
            standalone,
        } => {
            let runtime = runtime.unwrap_or(
                std::env::current_exe()?
                    .parent()
                    .expect("executable parent")
                    .join("runtime"),
            );
            let weights = match (source.model, source.weights) {
                (Some(model), _) => serving::Weights::Model(model),
                (None, Some(path)) => serving::Weights::Path(path),
                (None, None) => unreachable!("clap requires --model or --weights"),
            };
            anyhow::ensure!(context != Some(0), "--context must be positive");
            serving::run(
                runtime,
                weights,
                model_dir,
                context.unwrap_or(0),
                standalone,
            )
            .await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serve_needs_exactly_one_source() {
        assert!(Args::try_parse_from(["ds4"]).is_err());
        assert!(Args::try_parse_from(["ds4", "serve"]).is_err());
        assert!(
            Args::try_parse_from(["ds4", "serve", "--model", "ds4f-q2", "--weights", "/m"])
                .is_err()
        );
        assert!(Args::try_parse_from(["ds4", "serve", "--model", "glm53-q2"]).is_ok());
        assert!(
            Args::try_parse_from(["ds4", "serve", "--weights", "/m", "--context", "100000"])
                .is_ok()
        );
    }
}
