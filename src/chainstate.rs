// use serde_json;
use crate::State;
use tide::{Request, Response, Result};

pub async fn get(_req: Request<State>) -> Result {
    let mut res = Response::new(200);
    res.set_content_type("application/json");
    res.set_body("{}");
    Ok(res)
}
