use bytes::Bytes;
use crate::State;
use cached::proc_macro::cached;
use ethereum_types::{H256, H160, U256};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tide::{Request, Response, Result};
use ureq::{Agent, AgentBuilder};
use std::str::FromStr;
use hex_literal::hex;

#[derive(Debug, Clone, Serialize)]
pub struct EvmTx {
    pub txid: H256,
    pub used: u64,
    pub price: U256,
    #[serde(skip_serializing_if="Option::is_none")]
    pub class: Option<String>,
    pub status: u64,
    #[serde(skip_serializing_if="Option::is_none")]
    pub contract_address: Option<H160>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvmBlock {
    pub block_num: u64,
    pub block_hash: H256,
    pub miner: H160,
    pub used: u64,
    pub limit: u64,
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
struct RpcResponse<T> {
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
pub fn get_evm_block(rpc_addr: String, block_num: u64) -> Option<EvmBlock> {
    let payload1 = format!("{{\"jsonrpc\":\"2.0\",\"method\":\"eth_getBlockByNumber\",\"params\":[\"0x{:x?}\",false],\"id\":\"i{}\"}}", block_num, block_num);
    // tracing::debug!("RQ {}", payload1);
    let rq1 = rpc_request(&rpc_addr);
    let response1: String = rq1.send_string(&payload1).unwrap().into_string().unwrap();
    let r1: RpcResponse<RpcResponseBlockInfo> = match serde_json::from_str(&response1) {
        Ok(x) => x,
        Err(e) => {
            tracing::info!("eth_getBlockByNumber response {}", response1);
            tracing::error!("parse error {}", e);
            return None
        }
    };
    // println!("{:#?}", r1);
    
    let payload2 = format!("{{\"jsonrpc\":\"2.0\",\"method\":\"parity_getBlockReceipts\",\"params\":[\"0x{:x?}\"],\"id\":\"r{}\"}}", block_num, block_num);
    // tracing::debug!("RQ {}", payload2);
    let rq2 = rpc_request(&rpc_addr);
    let response2: String = rq2.send_string(&payload2).unwrap().into_string().unwrap();
    let r2: RpcResponse<Vec<RpcResponseBlockReceiptsInfo>> = match serde_json::from_str(&response2) {
        Ok(x) => x,
        Err(e) => {
            tracing::info!("parity_getBlockReceipts response {}", response2);
            tracing::error!("parse error {}", e);
            return None
        }
    };
    // println!("{:#?}", r2);
    let mut tx = vec![]; // TODO: map tx
    for receipt in r2.result {
        let mut class = None;
        if receipt.logs.len() > 1 && receipt.logs[0].data.len() > 32 {
            let hex_str = hex::encode(&receipt.logs[0].data);
            let u256_str: String = hex_str.chars().take(64).collect();
            let topic = U256::from_str(&u256_str).unwrap();
            if topic == hex!("ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef").into() {
                class = Some("Transfer".to_owned());
            }
        }
        if let Some(_) = receipt.contract_address {
            class = Some("Publish".to_owned());
        }
        tx.push(EvmTx{
            txid: receipt.transaction_hash,
            used: receipt.gas_used.as_u64(),
            price: receipt.effective_gas_price,
            class,
            status: receipt.status.as_u64(),
            contract_address: receipt.contract_address,
        })
    }

    Some(EvmBlock{
        block_num: r1.result.number.as_u64(),
        block_hash: r1.result.hash,
        miner: r1.result.miner,
        limit: r1.result.gas_limit.as_u64(),
        used: r1.result.gas_used.as_u64(),
        tx,
    })
}

#[cached(time = 10)]
pub fn get_evm_state(rpc_addr: String, num_blocks: usize) -> Option<EvmState> {
    let rq = rpc_request(&rpc_addr);

    // TODO: add authorization headers 
    let payload = "{\"jsonrpc\":\"2.0\",\"method\":\"eth_blockNumber\",\"id\":1}";
    let response: String = rq.send_string(payload).unwrap().into_string().unwrap();
    let numeric: EvmNumericResult = serde_json::from_str(&response).unwrap();
    tracing::info!("eth_blockNumber={}", numeric.result.as_u64());
    if numeric.result == U256::from(0) {
        // node that is not in sync will return 0
        return None
    }

    // building batch to get the latest blocks
    let mut blocks = vec![];
    for i in 1..num_blocks {
        let block_num = numeric.result.as_u64() - (i as u64) + 1u64;
        if let Some(b) = get_evm_block(rpc_addr.clone(), block_num) {
            blocks.push(b)
        }
    }   
    Some(EvmState{ blocks })
}

pub async fn get(req: Request<State>) -> Result {
    let mut res = Response::new(200);
    let rpc = req.state().eth1.clone();
    let out = get_evm_state(rpc, 5);

    res.set_content_type("application/json");
    res.set_body(serde_json::to_string(&out).unwrap());
    Ok(res)
}
