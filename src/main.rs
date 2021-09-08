pub mod args;
pub mod chainstate;
pub mod telemetry;

#[derive(Clone)]
pub struct State {
    pub static_dir: String,
    pub eth1: String,
}

// use std::path::{Path, PathBuf};
// use std::{ffi::OsStr, io};
// use tide::{Body, Middleware, Next, Request, Response, StatusCode};

#[async_std::main]
async fn main() -> tide::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("info"))
        .init();
    let args = match args::parse() {
        Ok(x) => x,
        Err(e) => {
            panic!("Args parsing error: {}", e);
        }
    };

    let state = State {
        static_dir: args.static_dir.clone(),
        eth1: args.eth1.clone(),
    };
    let mut app = tide::with_state(state);
    app.with(telemetry::TraceMiddleware::new());
    // app.with(ServeMiddleware {});
    app.at("/api/chainstate").get(chainstate::get);
    app.listen(args.addr.as_str()).await?;
    Ok(())
}
