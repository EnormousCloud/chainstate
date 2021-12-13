pub mod args;
pub mod chainstate;
pub mod network;
pub mod telemetry;

use std::sync::Arc;

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
    let atag = Arc::new(args.tag.clone());
    if args.server > 0 {
        let state = State {
            eth1: args.network.clone(),
        };
        let mut app = tide::with_state(state);
        app.with(telemetry::TraceMiddleware::new());
        // app.with(ServeMiddleware {});
        app.at("/api/chainstate").get(chainstate::get);
        app.listen(args.addr.as_str()).await?;
    } else if args.networks_file.len() > 0 {
        let mut threads = vec![];
        for network in network::from_file(&args.networks_file).unwrap() {
            let tag = Arc::clone(&atag);
            // for each network spawn a thread that logs its status
            threads.push(std::thread::spawn(move || {
                let addr = network.endpoint.clone();
                if tag.len() == 0 || network.tags.contains(&tag.to_string()) {
                    crate::chainstate::get_evm_status(addr.clone()).log_with_address(&addr);
                }
            }));
        }
        // wait for result
        for t in threads {
            let _ = t.join();
        }
    } else {
        let network = args.network.clone();
        crate::chainstate::get_evm_status(network.clone()).log();
    }
    Ok(())
}
