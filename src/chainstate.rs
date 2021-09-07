use bytes::Bytes;
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
    pub base_fee_per_gas: Option<U256>,
    pub difficulty: U256,
    pub gas_limit: U256,
    pub gas_used: U256,
    pub hash: H256,
    pub miner: H160,
    pub number: U256,
    pub size: U256,
    pub timestamp: U256,
    pub total_difficulty: U256,
    pub transactions: Vec<H256>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct ReceiptLog {
    pub data: Bytes,
    pub log_index: U256,
    pub removed: Option<bool>,
    pub topics: Vec<H256>,
    pub transaction_log_index: U256,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all="camelCase")]
struct RpcResponseBlockReceiptsInfo {
    pub block_hash: H256,
    pub block_number: U256,
    pub contract_address: Option<H160>,
    pub cumulative_gas_used: U256,
    pub effective_gas_price: U256,
    pub from: H160,
    pub gas_used: U256,
    pub logs: Vec<ReceiptLog>,
    pub status: U256,
    pub to: Option<H160>,
    pub transaction_hash: H256,
    pub transaction_index: U256,
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

// #[cached(time=1000)]
// pub fn get_receipts(tx: Vec<H256>) -> EvmTx

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
    let mut rbatch: Vec<String> = vec![];
    for i in 1..num_blocks {
        let block_num = numeric.result.as_u64() - (i as u64) + 1u64;
        batch.push(format!("{{\"jsonrpc\":\"2.0\",\"method\":\"eth_getBlockByNumber\",\"params\":[\"0x{:x?}\",false],\"id\":\"i{}\"}}", block_num, block_num));
        rbatch.push(format!("{{\"jsonrpc\":\"2.0\",\"method\":\"parity_getBlockReceipts\",\"params\":[\"0x{:x?}\"],\"id\":\"r{}\"}}", block_num, block_num));
    }
    let payload = format!("[{}]", batch.join(","));
    tracing::info!("eth_blockNumber batch request {}", payload);
    let rq = rpc_request(&rpc_addr);
    let response: String = rq.send_string(&payload).unwrap().into_string().unwrap();

    let rpayload = format!("[{}]", rbatch.join(","));
    let rrq = rpc_request(&rpc_addr);
    let rresponse: String = rrq.send_string(&rpayload).unwrap().into_string().unwrap();
    tracing::info!("parity_getBlockReceipts batch response {}", rresponse);

    let batches: Vec<RpcBatchResponse<RpcResponseBlockInfo>> = match serde_json::from_str(&response) {
        Ok(x) => x,
        Err(e) => {
            tracing::info!("eth_blockNumber batch response {}", response);
            tracing::error!("parse error {}", e);
            return None
        }
    };
    
    let rbatches: RpcBatchResponse<RpcResponseBlockReceiptsInfo> = match serde_json::from_str(&rresponse) {
        Ok(x) => x,
        Err(e) => {
            tracing::info!("parity_getBlockReceipts batch response {}", response);
            tracing::error!("parse error {}", e);
            return None
        }
    };
    tracing::info!("{:#?}", batches);
    tracing::info!("{:#?}", rbatches);

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
