use clap::{ArgAction, Parser};
use colored::Colorize;
use log::{info, LevelFilter};

/// Command line arguments
#[derive(Parser, Debug)]
#[command(about)]
pub struct Args {
    /// Certificate to check jetstream server against
    #[arg(short = 'c', long, default_value = "/etc/ssl/certs/ISRG_Root_X1.pem")]
    pub certificate: String,
    /// Override tokio threadpool size for async operations
    #[arg(long)]
    pub worker_threads: Option<usize>,
    /// Override amount of concurrent requests the full indexer may handle
    #[arg(long)]
    pub max_concurrent_requests: Option<usize>,
    /// Endpoint of the database server (including port and protocol)
    #[arg(short = 'D', long, default_value = "rocksdb://path/to/surreal.db")]
    pub db: String,
    /// Debug verbosity level
    #[arg(short, action = ArgAction::Count)]
    pub verbosity: u8,
    /// Indexer Mode (jetstream only or full)
    #[arg(long, default_value = "jetstream")]
    pub mode: String,
}

impl Args {
    /// Dump configuration to log
    pub fn dump(self: &Self) {
        // dump configuration
        info!("{}", "Configuration:".bold().underline().blue());
        info!("{}: {}", "Certificate".cyan(), self.certificate.green());
        info!(
            "{}: {}",
            "Worker Threads".cyan(),
            self.worker_threads.map_or_else(
                || "Not set, using CPU count".yellow(),
                |v| v.to_string().green()
            )
        );
        info!(
            "{}: {}",
            "Max concurrent requests".cyan(),
            self.max_concurrent_requests.map_or_else(
                || "Not set, using CPU count times 1000".yellow(),
                |v| v.to_string().green()
            )
        );
        info!("{}: {}", "Database".cyan(), self.db.green());
        info!(
            "{}: {}",
            "Verbosity Level".cyan(),
            self.log_level().to_string().green()
        );
        info!("{}: {}", "Mode".cyan(), self.mode.green());
    }

    /// Verbosity to log level
    pub fn log_level(self: &Self) -> LevelFilter {
        match self.verbosity {
            0 => LevelFilter::Info,
            1 => LevelFilter::Debug,
            _ => LevelFilter::Trace,
        }
    }
}

/// Parse command line arguments
pub fn parse_args() -> Args {
    Args::parse()
}
