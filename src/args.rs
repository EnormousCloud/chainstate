use structopt::StructOpt;
use tracing_subscriber::prelude::*;

#[derive(Debug, StructOpt, Clone)]
#[structopt(
    name = "enormous-chainstate",
    about = "Enormous Cloud chainstate API server"
)]
pub struct Args {
    /// Optional - plain text file, containing the list of RPC addresses to be checked.
    /// Tag may be appled to restrict the list
    #[structopt(short, long, default_value = "", env = "NETWORKS_FILE")]
    pub networks_file: String,
    /// Filter chains by tag
    #[structopt(short, long, default_value = "")]
    pub tag: String,
    /// Check single network address (internally used tags: nosync, nogaps)
    #[structopt(long, default_value = "")]
    pub network: String,
    /// Return working endpoint (tag may be applied to restrict the list)
    #[structopt(long)]
    pub endpoints: bool,
    /// Whether to start HTTP API server
    #[structopt(short, long)]
    pub server: bool,
    /// In case of server, TCP address to be listened
    #[structopt(short, long, default_value = "0.0.0.0:8000", env = "LISTEN")]
    pub addr: String,
}

pub fn parse() -> anyhow::Result<Args> {
    let log_level: String = std::env::var("LOG_LEVEL").unwrap_or("info".to_owned());

    let fmt_layer = tracing_subscriber::fmt::layer()
        // .without_time()
        // .with_ansi(false)
        // .with_level(false)
        .with_target(false);
    let filter_layer = tracing_subscriber::EnvFilter::try_from_default_env()
        .or_else(|_| tracing_subscriber::EnvFilter::try_new(&log_level))
        .unwrap();
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();

    let res = Args::from_args();
    tracing::debug!("{:?}", res);
    Ok(res)
}
