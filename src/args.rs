use structopt::StructOpt;

#[derive(Debug, StructOpt, Clone)]
#[structopt(
    name = "enormous-chainstate",
    about = "Enormous Cloud chainstate API server"
)]
pub struct Args {
    #[structopt(
        long,
        default_value = "http://127.0.0.1:8545",
        env = "ETHEREUM_RPC_ENDPOINT"
    )]
    pub eth1: String,
    #[structopt(long, default_value = "./dist", env = "STATIC_DIR")]
    pub static_dir: String,
    #[structopt(short, long, default_value = "0.0.0.0:8000", env = "LISTEN")]
    pub addr: String,
}

pub fn parse() -> anyhow::Result<Args> {
    let res = Args::from_args();
    tracing::info!("{:?}", res);
    // todo: check static dir exists
    Ok(res)
}
