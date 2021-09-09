use rand::Rng;
use std::time::Instant;
use tide::{Middleware, Next, Request};
use tracing::{error, error_span, field, info, info_span, warn};
use tracing_futures::Instrument;

const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
const RQID_LEN: usize = 8;

/// Log all incoming requests and responses with tracing spans.
///
/// ```
/// let mut app = tide::Server::new();
/// app.with(tide_tracing::TraceMiddleware::new());
/// ```
#[derive(Debug, Default, Clone)]
pub struct TraceMiddleware;

impl TraceMiddleware {
    /// Create a new instance of `TraceMiddleware`.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Log a request and a response.
    async fn log<'a, State: Clone + Send + Sync + 'static>(
        &'a self,
        ctx: Request<State>,
        next: Next<'a, State>,
    ) -> tide::Result {
        let path = ctx.url().path().to_owned();
        let method = ctx.method();

        let rqid = match ctx.header("x-request-id") {
            Some(x) => x.to_string(),
            None => {
                let mut rng = rand::thread_rng();
                (0..RQID_LEN)
                    .map(|_| {
                        let idx = rng.gen_range(0..CHARSET.len());
                        CHARSET[idx] as char
                    })
                    .collect()
            }
        };

        let ip = match ctx.header("x-forwarded-for") {
            Some(x) => x.to_string(),
            None => match ctx.header("x-real-ip") {
                Some(x) => x.to_string(),
                None => ctx.remote().unwrap_or("").to_owned(),
            },
        };
        let ua = match ctx.header("user-agent") {
            Some(user_agent) => user_agent.to_string(),
            None => "".to_owned(),
        };

        let geo = vec![
            match ctx.header("x-country-code") {
                Some(hv) => Some(hv.to_string()),
                None => None,
            },
            match ctx.header("x-city-en-name") {
                Some(hv) => Some(hv.to_string()),
                None => None,
            },
            match ctx.header("x-location-accuracy") {
                Some(hv) => Some(hv.to_string()),
                None => None,
            },
        ]
        .iter()
        .flatten()
        .cloned()
        .collect::<Vec<String>>()
        .join(",");

        Ok(async {
            let start = Instant::now();
            let response = next.run(ctx).await;
            let duration = start.elapsed();
            let status = response.status();

            info_span!("Response", rq = %rqid, status = status as u16, took = ?duration).in_scope(
                || {
                    if status.is_server_error() {
                        let span = error_span!("Internal", error = field::Empty);
                        if let Some(error) = response.error() {
                            span.record("error", &field::display(error));
                        }
                        span.in_scope(|| error!("ok"));
                    } else if status.is_client_error() {
                        // warn_span!("error").in_scope(|| warn!("fail"));
                        warn!("fail")
                    } else {
                        info!("ok")
                    }
                },
            );
            response
        }
        .instrument(
            if geo.len() > 0 {
                info_span!("Request", rq = %rqid, m = %method, u = %path, ip = %ip, agent = %ua, geo = %geo)
            } else {
                info_span!("Request", rq = %rqid, m = %method, u = %path, ip = %ip, agent = %ua)
            }
        )
        .await)
    }
}

#[async_trait::async_trait]
impl<State: Clone + Send + Sync + 'static> Middleware<State> for TraceMiddleware {
    async fn handle(&self, req: Request<State>, next: Next<'_, State>) -> tide::Result {
        self.log(req, next).await
    }
}
