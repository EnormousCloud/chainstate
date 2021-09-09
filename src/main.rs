pub mod args;
pub mod chainstate;
pub mod telemetry;

use tracing::{info, info_span};

#[derive(Clone)]
pub struct State {
    pub eth1: String,
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    let args = match args::parse() {
        Ok(x) => x,
        Err(e) => {
            panic!("Args parsing error: {}", e);
        }
    };

    if args.server > 0 {
        let state = State {
            eth1: args.network.clone(),
        };
        let mut app = tide::with_state(state);
        app.with(telemetry::TraceMiddleware::new());
        // app.with(ServeMiddleware {});
        app.at("/api/chainstate").get(chainstate::get);
        app.listen(args.addr.as_str()).await?;
    } else {
        let network = args.network.clone();
        let chain_id = crate::chainstate::get_evm_chain_id(network.clone()).unwrap();
        info!("{}: {}", network, chain_id);
    }
    Ok(())
}
