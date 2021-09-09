use structopt::StructOpt;
use tracing_subscriber::prelude::*;

#[derive(Debug, StructOpt, Clone)]
#[structopt(
    name = "enormous-chainstate",
    about = "Enormous Cloud chainstate API server"
)]
pub struct Args {
    #[structopt(long, default_value = "http://127.0.0.1:8545", env = "RPC_ENDPOINT")]
    pub network: String,
    #[structopt(short, long, default_value = "0.0.0.0:8000", env = "LISTEN")]
    pub addr: String,
}

pub fn parse() -> anyhow::Result<Args> {
    let log_level: String = std::env::var("LOG_LEVEL").unwrap_or("info".to_owned());

    let fmt_layer = tracing_subscriber::fmt::layer()
        // .without_time()
        // .with_ansi(false)
        // .with_level(false)
        // .with_target(false)
    ;
    let filter_layer = tracing_subscriber::EnvFilter::try_from_default_env()
        .or_else(|_| tracing_subscriber::EnvFilter::try_new(&log_level))
        .unwrap();
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();

    let res = Args::from_args();
    tracing::info!("{:?}", res);
    // todo: check static dir exists
    Ok(res)
}
