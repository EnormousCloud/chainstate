use crate::State;
use cached::proc_macro::cached;
use ethereum_types::{H256, H160, U256};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tide::{Request, Response, Result};
use ureq::{Agent, AgentBuilder};

#[derive(Debug, Clone, Serialize)]
pub struct EvmTx {
    pub txid: H160,
    pub gas_used: u64,
    pub gas_limit: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvmBlock {
    pub gas_used: u64,
    pub gas_limit: u64,
    pub tx: Vec<EvmTx>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvmState {
    pub blocks: Vec<EvmBlock>,
}

#[derive(Debug, Clone, Deserialize)]
struct EvmNumericResult {
    pub result: U256,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all="camelCase")]
struct RpcResponseBlockInfo {
    base_fee_per_gas: U256,
    difficulty: U256,
    gas_limit: U256,
    gas_used: U256,
    hash: H256,
    miner: H160,
    number: U256,
    size: U256,
    timestamp: U256,
    total_difficulty: U256,
    transactions: Vec<H256>,
}

#[derive(Debug, Clone, Deserialize)]
struct RpcBatchResponse<T> {
    pub id: String,    
    pub result: T,
}

pub fn rpc_request(rpc_addr: &str) -> ureq::Request {
    let agent: Agent = AgentBuilder::new()
        .timeout_read(Duration::from_secs(25))
        .build();
    agent
        .post(rpc_addr)
        .set("Content-Type", "application/json")
}

#[cached(time = 30)]
pub fn get_evm_state(rpc_addr: String, num_blocks: usize) -> Option<EvmState> {
    let rq = rpc_request(&rpc_addr);
    // TODO: add authorization headers
    let payload = "{\"jsonrpc\":\"2.0\",\"method\":\"eth_blockNumber\",\"id\":1}";
    let response: String = rq.send_string(payload).unwrap().into_string().unwrap();
    tracing::info!("eth_blockNumber response {}", response);
    let numeric: EvmNumericResult = serde_json::from_str(&response).unwrap();
    tracing::info!("eth_blockNumber result {}", numeric.result.as_u64());

    // building batch to get the latest blocks
    let mut batch: Vec<String> = vec![];
    for i in 1..num_blocks {
        let block_num = numeric.result.as_u64() - (i as u64) + 1u64;
        batch.push(format!("{{\"jsonrpc\":\"2.0\",\"method\":\"eth_getBlockByNumber\",\"params\":[\"0x{:x?}\",false],\"id\":\"{}\"}}", block_num, block_num));
    }
    let payload = format!("[{}]", batch.join(","));
    tracing::info!("eth_blockNumber batch request {}", payload);
    let rq = rpc_request(&rpc_addr);
    let response: String = rq.send_string(&payload).unwrap().into_string().unwrap();
    
    let batches: Vec<RpcBatchResponse<RpcResponseBlockInfo>> = match serde_json::from_str(&response) {
        Ok(x) => x,
        Err(e) => {
            tracing::info!("eth_blockNumber batch response {}", response);
            tracing::error!("parse error {}", e);
            return None
        }
    };
    tracing::info!("eth_blockNumber {:#?}", batches);
    
    // building batch to get the latest receipts
    // TODO: diff versions for openethereum and geth
    Some(EvmState{ blocks: vec![] })
}

pub async fn get(req: Request<State>) -> Result {
    let mut res = Response::new(200);
    let rpc = req.state().eth1.clone();
    let out = get_evm_state(rpc, 3);

    res.set_content_type("application/json");
    res.set_body(serde_json::to_string(&out).unwrap());
    Ok(res)
}
