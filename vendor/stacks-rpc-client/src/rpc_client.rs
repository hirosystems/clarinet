use clarity_repl::clarity::codec::{StacksMessageCodec, StacksTransaction};
use clarity_repl::clarity::types::Value;
use clarity_repl::clarity::util::hash::{bytes_to_hex, hex_bytes};
use reqwest::blocking::Client;
use std::io::Cursor;

#[derive(Debug)]
pub enum RpcError {
    Generic,
}

pub struct StacksRpc {
    pub url: String,
    pub client: Client,
}

pub struct PostTransactionResult {
    pub txid: String,
}

pub struct CallReadOnlyFnResult {
    pub result: Value,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct NodeInfo {
    pub peer_version: u64,
    pub pox_consensus: String,
    pub burn_block_height: u64,
    pub stable_pox_consensus: String,
    pub stable_burn_block_height: u64,
    pub server_version: String,
    pub network_id: u32,
    pub parent_network_id: u32,
    pub stacks_tip_height: u64,
    pub stacks_tip: String,
    pub stacks_tip_consensus_hash: String,
    pub genesis_chainstate_hash: String,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct PoxInfo {
    pub contract_id: String,
    pub pox_activation_threshold_ustx: u64,
    pub first_burnchain_block_height: u64,
    pub prepare_phase_block_length: u32,
    pub reward_phase_block_length: u32,
    pub reward_slots: u32,
    pub reward_cycle_id: u32,
    pub total_liquid_supply_ustx: u64,
    pub next_cycle: PoxCycle,
}

impl PoxInfo {
    pub fn default() -> PoxInfo {
        PoxInfo {
            contract_id: "ST000000000000000000002AMW42H.pox".into(),
            pox_activation_threshold_ustx: 0,
            first_burnchain_block_height: 100,
            prepare_phase_block_length: 5,
            reward_phase_block_length: 10,
            reward_slots: 20,
            total_liquid_supply_ustx: 1000000000000000,
            ..Default::default()
        }
    }
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct PoxCycle {
    pub min_threshold_ustx: u64,
}

#[derive(Deserialize, Debug)]
struct Balance {
    balance: String,
    nonce: u64,
    balance_proof: String,
    nonce_proof: String,
}

#[derive(Deserialize, Debug)]
pub struct Contract {
    source: String,
    publish_height: u64,
}

impl StacksRpc {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.into(),
            client: Client::builder().build().unwrap(),
        }
    }

    pub fn post_transaction(
        &self,
        transaction: StacksTransaction,
    ) -> Result<PostTransactionResult, RpcError> {
        let tx = transaction.serialize_to_vec();
        let path = format!("{}/v2/transactions", self.url);
        let res = self
            .client
            .post(&path)
            .header("Content-Type", "application/octet-stream")
            .body(tx)
            .send()
            .unwrap();

        if !res.status().is_success() {
            println!("{}", res.text().unwrap());
            return Err(RpcError::Generic);
        }

        let txid: String = res.json().unwrap();
        let res = PostTransactionResult { txid };
        Ok(res)
    }

    pub fn get_nonce(&self, address: &str) -> Result<u64, RpcError> {
        let request_url = format!("{}/v2/accounts/{addr}", self.url, addr = address,);

        let res: Balance = self
            .client
            .get(&request_url)
            .send()
            .expect("Unable to retrieve account")
            .json()
            .expect("Unable to parse contract");
        let nonce = res.nonce;
        Ok(nonce)
    }

    pub fn get_pox_info(&self) -> Result<PoxInfo, RpcError> {
        let request_url = format!("{}/v2/pox", self.url);

        let res: PoxInfo = self
            .client
            .get(&request_url)
            .send()
            .expect("Unable to retrieve account")
            .json()
            .expect("Unable to parse contract");
        Ok(res)
    }

    pub fn get_info(&self) -> Result<NodeInfo, RpcError> {
        let request_url = format!("{}/v2/info", self.url);

        let res: NodeInfo = self
            .client
            .get(&request_url)
            .send()
            .expect("Unable to retrieve account")
            .json()
            .expect("Unable to parse contract");
        Ok(res)
    }

    pub fn get_contract_source(
        &self,
        principal: &str,
        contract_name: &str,
    ) -> Result<Contract, RpcError> {
        let request_url = format!(
            "{}/v2/contracts/source/{}/{}",
            self.url, principal, contract_name
        );

        let res = self.client.get(&request_url).send();

        match res {
            Ok(response) => match response.json() {
                Ok(value) => Ok(value),
                _ => Err(RpcError::Generic),
            },
            _ => Err(RpcError::Generic),
        }
    }

    pub fn call_read_only_fn(
        &self,
        contract_addr: &str,
        contract_name: &str,
        method: &str,
        args: Vec<Value>,
        sender: &str,
    ) -> Result<Value, RpcError> {
        let path = format!(
            "{}/v2/contracts/call-read/{}/{}/{}",
            self.url, contract_addr, contract_name, method
        );

        let arguments = args
            .iter()
            .map(|a| bytes_to_hex(&a.serialize_to_vec()))
            .collect::<Vec<_>>();
        let res = self
            .client
            .post(&path)
            .json(&json!({
                "sender": sender,
                "arguments": arguments,
            }))
            .send()
            .unwrap();

        if !res.status().is_success() {
            println!("{}", res.text().unwrap());
            return Err(RpcError::Generic);
        }

        #[derive(Deserialize, Debug)]
        struct ReadOnlyCallResult {
            okay: bool,
            result: String,
        }

        let response: ReadOnlyCallResult = res.json().unwrap();
        if response.okay {
            // Removing the 0x prefix
            let raw_value = match response.result.strip_prefix("0x") {
                Some(raw_value) => raw_value,
                _ => panic!(),
            };
            let bytes = hex_bytes(&raw_value).unwrap();
            let mut cursor = Cursor::new(&bytes);
            let value = Value::consensus_deserialize(&mut cursor).unwrap();
            Ok(value)
        } else {
            Err(RpcError::Generic)
        }
    }
}
