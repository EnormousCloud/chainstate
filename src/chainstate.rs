use crate::State;
use bytes::Bytes;
use cached::proc_macro::cached;
use ethereum_types::{H160, H256, U256, U64};
use hex_literal::hex;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::time::Duration;
use tide::{Request, Response, Result};
use ureq::{Agent, AgentBuilder};

#[derive(Debug, Clone, Serialize)]
pub struct EvmTx {
    pub txid: H256,
    pub used: u64,
    pub price: U256,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    pub status: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EvmSync {
    Progress {
        #[serde(rename = "startingBlock")]
        starting_block: U64,
        #[serde(rename = "currentBlock")]
        current_block: U64,
        #[serde(rename = "highestBlock")]
        highest_block: U64,
    },
    Done(bool),
}

impl EvmSync {
    pub fn to_string(&self) -> String {
        match *self {
            Self::Done(_) => "false".to_owned(),
            Self::Progress {
                current_block,
                highest_block,
                ..
            } => {
                format!(
                    "{}% {} out of {}",
                    current_block * 100 / highest_block,
                    current_block,
                    highest_block
                )
            }
        }
    }
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct ReceiptLog {
    pub data: Bytes,
    pub log_index: U256,
    pub removed: Option<bool>,
    pub topics: Vec<H256>,
    pub transaction_log_index: U256,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
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
struct RpcError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RpcErrorResponse {
    pub id: serde_json::Value,
    pub error: RpcError,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlockGaps {
    #[serde(rename = "blockGap")]
    pub block_gap: Vec<U256>,
}

impl BlockGaps {
    pub fn to_string(&self) -> String {
        let s: Vec<String> = self.block_gap.iter().map(|x| format!("{}", x)).collect();
        s.join("..")
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "level", content = "msg")]
pub enum EvmStatus {
    Ok(String),
    Warn(String),
    Fail(String),
}

impl EvmStatus {
    pub fn log(&self) {
        match self {
            Self::Ok(msg) => tracing::info!("{}", msg),
            Self::Warn(msg) => tracing::warn!("{}", msg),
            Self::Fail(msg) => tracing::error!("{}", msg),
        }
    }

    pub fn log_with_address(&self, addr: &str) {
        match self {
            Self::Ok(msg) => tracing::info!("{}: {}", addr, msg),
            Self::Warn(msg) => tracing::warn!("{}: {}", addr, msg),
            Self::Fail(msg) => tracing::error!("{}: {}", addr, msg),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RpcResponse<T> {
    pub id: serde_json::Value,
    pub result: T,
}

pub fn rpc_request(rpc_addr: &str) -> ureq::Request {
    let agent: Agent = AgentBuilder::new()
        .timeout_read(Duration::from_secs(25))
        .build();
    agent.post(rpc_addr).set("Content-Type", "application/json")
}

#[cached(time = 3000)]
pub fn get_evm_chain_id(rpc_addr: String) -> std::result::Result<u64, String> {
    let payload = format!("{{\"jsonrpc\":\"2.0\",\"method\":\"net_version\",\"id\":\"1\"}}");
    let rq = rpc_request(&rpc_addr);
    let response: String = match rq.send_string(&payload) {
        Ok(x) => x.into_string().unwrap(),
        Err(e) => return Err(format!("{}", e)),
    };
    if let Ok(err) = serde_json::from_str::<RpcErrorResponse>(&response) {
        return Err(err.error.message.clone());
    }
    let out: RpcResponse<serde_json::Value> = match serde_json::from_str(&response) {
        Ok(x) => x,
        Err(x) => return Err(x.to_string()),
    };
    match out.result {
        serde_json::Value::Number(n) => return Ok(n.as_u64().unwrap()),
        serde_json::Value::String(s) => return Ok(s.parse().unwrap()),
        _ => return Err("result convertion failure".to_owned()),
    }
}

#[cached(time = 15)]
pub fn get_evm_syncing(rpc_addr: String) -> std::result::Result<EvmSync, String> {
    let payload = format!("{{\"jsonrpc\":\"2.0\",\"method\":\"eth_syncing\",\"id\":\"1\"}}");
    let rq = rpc_request(&rpc_addr);
    let response: String = match rq.send_string(&payload) {
        Ok(x) => x.into_string().unwrap(),
        Err(e) => return Err(format!("{}", e)),
    };
    if let Ok(err) = serde_json::from_str::<RpcErrorResponse>(&response) {
        return Err(err.error.message.clone());
    }
    let out: RpcResponse<EvmSync> = match serde_json::from_str(&response) {
        Ok(x) => x,
        Err(x) => return Err(format!("{}. RESPONSE: {}", x.to_string(), response)),
    };
    Ok(out.result)
}

#[cached(time = 5)]
pub fn get_evm_block_number(rpc_addr: String) -> std::result::Result<u64, String> {
    let payload = format!("{{\"jsonrpc\":\"2.0\",\"method\":\"eth_blockNumber\",\"id\":\"1\"}}");
    let rq = rpc_request(&rpc_addr);
    let response: String = match rq.send_string(&payload) {
        Ok(x) => x.into_string().unwrap(),
        Err(e) => return Err(format!("{}", e)),
    };
    if let Ok(err) = serde_json::from_str::<RpcErrorResponse>(&response) {
        return Err(err.error.message.clone());
    }
    let out: RpcResponse<U64> = match serde_json::from_str(&response) {
        Ok(x) => x,
        Err(x) => return Err(format!("{}. RESPONSE: {}", x.to_string(), response)),
    };
    Ok(out.result.as_u64())
}

#[cached(time = 5)]
pub fn get_evm_gaps(rpc_addr: String) -> std::result::Result<BlockGaps, String> {
    let payload = format!(
        "{{\"jsonrpc\":\"2.0\",\"method\":\"parity_chainStatus\",\"id\":\"1\",\"params\":[]}}"
    );
    let rq = rpc_request(&rpc_addr);
    let response: String = match rq.send_string(&payload) {
        Ok(x) => x.into_string().unwrap(),
        Err(e) => return Err(format!("{}", e)),
    };
    if let Ok(err) = serde_json::from_str::<RpcErrorResponse>(&response) {
        return Err(err.error.message.clone());
    }
    let out: RpcResponse<BlockGaps> = match serde_json::from_str(&response) {
        Ok(x) => x,
        Err(x) => return Err(format!("{}. RESPONSE: {}", x.to_string(), response)),
    };
    Ok(out.result)
}

pub fn get_evm_status(rpc_addr: String) -> EvmStatus {
    let chain_id = match get_evm_chain_id(rpc_addr.clone()) {
        Ok(x) => x,
        Err(err) => return EvmStatus::Fail(err.to_owned()),
    };
    match get_evm_syncing(rpc_addr.clone()) {
        Ok(x) => {
            if let EvmSync::Progress { .. } = x {
                return EvmStatus::Warn(format!("chain {}, {}", chain_id, x.to_string()));
            }
        }
        Err(err) => {
            let msg = err.to_owned();
            // Some RPC APIs (i.e. arbitrum) don't have this method - and we will allow that
            if !msg.contains("method eth_syncing") {
                return EvmStatus::Fail(msg);
            }
        },
    };
    let head_block = match get_evm_block_number(rpc_addr.clone()) {
        Ok(x) => x,
        Err(err) => return EvmStatus::Fail(err.to_owned()),
    };
    EvmStatus::Ok(match get_evm_gaps(rpc_addr.clone()) {
        Ok(x) => format!(
            "chain {}, block {}, gaps {}",
            chain_id,
            head_block,
            x.to_string()
        ),
        Err(_) => {
            if head_block == 0 {
                return EvmStatus::Warn(format!("chain {}, zero head block", chain_id));
            }
            // tracing::error!("parse error {}", err);
            format!("chain {}, block {}", chain_id, head_block)
        }
    })
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
            return None;
        }
    };
    // println!("{:#?}", r1);

    let payload2 = format!("{{\"jsonrpc\":\"2.0\",\"method\":\"parity_getBlockReceipts\",\"params\":[\"0x{:x?}\"],\"id\":\"r{}\"}}", block_num, block_num);
    // tracing::debug!("RQ {}", payload2);
    let rq2 = rpc_request(&rpc_addr);
    let response2: String = rq2.send_string(&payload2).unwrap().into_string().unwrap();
    let r2: RpcResponse<Vec<RpcResponseBlockReceiptsInfo>> = match serde_json::from_str(&response2)
    {
        Ok(x) => x,
        Err(e) => {
            tracing::info!("parity_getBlockReceipts response {}", response2);
            tracing::error!("parse error {}", e);
            return None;
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
            if topic
                == hex!("ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef").into()
            {
                class = Some("Transfer".to_owned());
            }
        }
        if let Some(_) = receipt.contract_address {
            class = Some("Publish".to_owned());
        }
        tx.push(EvmTx {
            txid: receipt.transaction_hash,
            used: receipt.gas_used.as_u64(),
            price: receipt.effective_gas_price,
            class,
            status: receipt.status.as_u64(),
            contract_address: receipt.contract_address,
        })
    }

    Some(EvmBlock {
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

    let payload = "{\"jsonrpc\":\"2.0\",\"method\":\"eth_blockNumber\",\"id\":1}";
    let response: String = rq.send_string(payload).unwrap().into_string().unwrap();
    let numeric: EvmNumericResult = serde_json::from_str(&response).unwrap();
    tracing::info!("eth_blockNumber={}", numeric.result.as_u64());
    if numeric.result == U256::from(0) {
        // node that is not in sync will return 0
        return None;
    }

    // building batch to get the latest blocks
    let mut blocks = vec![];
    for i in 1..num_blocks {
        let block_num = numeric.result.as_u64() - (i as u64) + 1u64;
        if let Some(b) = get_evm_block(rpc_addr.clone(), block_num) {
            blocks.push(b)
        }
    }
    Some(EvmState { blocks }) //, syncing: EvmSync::Done})
}

pub async fn get(req: Request<State>) -> Result {
    let mut res = Response::new(200);
    let rpc = req.state().eth1.clone();
    let out = get_evm_state(rpc, 5);

    res.set_content_type("application/json");
    res.set_body(serde_json::to_string(&out).unwrap());
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethereum_types::U64;
    use std::matches;

    #[test]
    pub fn it_parses_done() {
        let input = r#"{"jsonrpc":"2.0","id":1,"result":false}"#;
        let output: RpcResponse<EvmSync> = serde_json::from_str(&input).unwrap();
        assert!(matches!(output.result, EvmSync::Done(false)));
    }

    #[test]
    pub fn it_parses_geth_syncing() {
        let input = r#"
    {"jsonrpc":"2.0","id":1,"result":{"currentBlock":"0xceb358","highestBlock":"0xcf219e","knownStates":"0x0","pulledStates":"0x0","startingBlock":"0xceb358"}}
    "#;
        let output: RpcResponse<EvmSync> = serde_json::from_str(&input).unwrap();
        match output.result {
            EvmSync::Progress {
                starting_block,
                current_block,
                highest_block,
            } => {
                assert_eq!(starting_block, U64::from(13546328));
                assert_eq!(current_block, U64::from(13546328));
                assert_eq!(highest_block, U64::from(13574558));
            }
            EvmSync::Done(_) => panic!("expected progress"),
        }
    }

    #[test]
    pub fn it_parses_openethereum_syncing() {
        let input = r#"
    {"jsonrpc":"2.0","result":{"currentBlock":"0xd1c504","highestBlock":"0x121534f","startingBlock":"0x0","warpChunksAmount":null,"warpChunksProcessed":null},"id":1}
    "#;
        let output: RpcResponse<EvmSync> = serde_json::from_str(&input).unwrap();
        match output.result {
            EvmSync::Progress {
                starting_block,
                current_block,
                highest_block,
            } => {
                assert_eq!(starting_block, U64::from(0));
                assert_eq!(current_block, U64::from(13747460));
                assert_eq!(highest_block, U64::from(18961231));
            }
            EvmSync::Done(_) => panic!("expected progress"),
        }
    }

    #[test]
    pub fn it_reads_chain_id() {
        let chain_id = get_evm_chain_id("https://dai.poa.network/".to_owned()).unwrap();
        assert_eq!(chain_id, 100);
    }
}
