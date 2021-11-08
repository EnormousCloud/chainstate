pub mod args;
pub mod chainstate;
pub mod telemetry;

#[derive(Clone)]
pub struct State {
    pub eth1: String,
}

use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

pub fn read_networks(source: &str) -> Result<Vec<String>, &str> {
    let file = File::open(Path::new(source)).unwrap();
    let lines: Vec<String> = io::BufReader::new(file)
        .lines()
        .map(|row| row.unwrap())
        .filter(|row| row.len() > 0)
        .collect();
    Ok(lines)
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
    } else if args.networks_file.len() > 0 {
        let mut threads = vec![];
        for network in read_networks(&args.networks_file).unwrap() {
            // for each network spawn a thread that logs its status
            threads.push(std::thread::spawn(move || {
                crate::chainstate::get_evm_status(network.clone()).log_with_address(&network);
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
