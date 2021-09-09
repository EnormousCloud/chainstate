pub mod args;
pub mod chainstate;
pub mod telemetry;

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

    let state = State {
        eth1: args.network.clone(),
    };
    let mut app = tide::with_state(state);
    app.with(telemetry::TraceMiddleware::new());
    // app.with(ServeMiddleware {});
    app.at("/api/chainstate").get(chainstate::get);
    app.listen(args.addr.as_str()).await?;
    Ok(())
}
