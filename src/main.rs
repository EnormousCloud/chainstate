pub mod args;
pub mod chainstate;
pub mod network;
pub mod telemetry;

use crate::chainstate::{get_evm_status, EvmStatus};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct State {
    pub eth1: String,
}

pub fn tags_from_args(tags_str: &str) -> HashSet<String> {
    let mut tags: HashSet<String> = HashSet::new();
    let parts: Vec<&str> = tags_str.trim().split(",").collect();
    if parts.len() > 0 {
        for part in parts {
            if part.trim().len() > 0 {
                tags.insert(part.trim().to_string());
            }
        }
    }
    tags
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    let args = match args::parse() {
        Ok(x) => x,
        Err(e) => {
            panic!("Args parsing error: {}", e);
        }
    };

    if args.endpoints {
        // show working endpoints in plain text format
        let arc_tags = Arc::new(tags_from_args(&args.tag));
        let mut threads = vec![];
        let matches: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        for network in network::from_file(&args.networks_file).unwrap() {
            let tag = Arc::clone(&arc_tags);
            let amatches = Arc::clone(&matches);
            // for each network spawn a thread that logs its status
            threads.push(std::thread::spawn(move || {
                if network.has_all(&tag) {
                    let addr = network.endpoint.clone();
                    if let EvmStatus::Ok(_) = get_evm_status(addr.clone(), &network.tags) {
                        let mut m = amatches.lock().unwrap();
                        m.push(addr.clone());
                    }
                }
            }));
        }
        // wait for result
        for t in threads {
            let _ = t.join();
        }
        // return found endpoints into stdout
        let result = matches.lock().unwrap();
        for l in result.iter() {
            println!("{}", l);
        }
        return Ok(());
    }

    if args.network.len() > 0 {
        let network = args.network.clone();
        let tags = tags_from_args(&args.tag);
        get_evm_status(network.clone(), &tags).log();
        return Ok(());
    }

    if args.networks_file.len() > 0 {
        let arc_tag = Arc::new(tags_from_args(&args.tag));
        let mut threads = vec![];
        for network in network::from_file(&args.networks_file).unwrap() {
            let tags = Arc::clone(&arc_tag);
            // for each network spawn a thread that logs its status
            threads.push(std::thread::spawn(move || {
                if network.has_all(&tags) {
                    let addr = network.endpoint.clone();
                    get_evm_status(addr.clone(), &network.tags).log_with_address(&addr);
                }
            }));
        }
        // wait for result
        for t in threads {
            let _ = t.join();
        }
        return Ok(());
    }

    if args.server {
        let state = State {
            eth1: args.network.clone(),
        };
        let mut app = tide::with_state(state);
        app.with(telemetry::TraceMiddleware::new());
        // app.with(ServeMiddleware {});
        app.at("/api/chainstate").get(chainstate::get);
        app.listen(args.addr.as_str()).await?;
    }
    Ok(())
}
