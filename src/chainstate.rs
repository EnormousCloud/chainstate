use crate::State;
use cached::proc_macro::cached;
use ethereum_types::H160;
use serde::{Deserialize, Serialize};
use tide::{Request, Response, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmTx {
    pub txid: H160,
    pub gas_used: u64,
    pub gas_limit: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmBlock {
    pub gas_used: u64,
    pub gas_limit: u64,
    pub tx: Vec<EvmTx>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmState {
    pub blocks: Vec<EvmBlock>,
}


#[cached(time = 30)]
pub fn get_evm_state(endpoint: String) -> Option<EvmState> {
    // TODO:
    Some(EvmState {
        blocks: vec![],
    })
}


pub async fn get(req: Request<State>) -> Result {
    let mut res = Response::new(200);
    let rpc = req.state().eth1.clone();
    let out = get_evm_state(rpc);

    res.set_content_type("application/json");
    res.set_body(serde_json::to_string(&out).unwrap());
    Ok(res)
}
