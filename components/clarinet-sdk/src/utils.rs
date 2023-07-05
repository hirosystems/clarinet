use clarity_repl::clarity::{
    codec::StacksMessageCodec,
    events::{FTEventType, NFTEventType, STXEventType},
    vm::events::StacksTransactionEvent,
    Value,
};
use js_sys::Uint8Array;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SmartContractEvent {
    contract_identifier: String,
    topic: String,
    value: String,
}

#[derive(Deserialize, Serialize)]
pub struct StacksEvent {
    pub event: String,
    pub data: serde_json::Value,
}

macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

pub fn serialize_event(event: &StacksTransactionEvent) -> StacksEvent {
    match event {
        StacksTransactionEvent::SmartContractEvent(data) => StacksEvent {
            event: "print_event".into(),
            data: data.json_serialize(),
        },
        StacksTransactionEvent::STXEvent(STXEventType::STXTransferEvent(data)) => StacksEvent {
            event: "stx_transfer_event".into(),
            data: data.json_serialize(),
        },
        StacksTransactionEvent::STXEvent(STXEventType::STXMintEvent(data)) => StacksEvent {
            event: "stx_mint_event".into(),
            data: data.json_serialize(),
        },
        StacksTransactionEvent::STXEvent(STXEventType::STXBurnEvent(data)) => StacksEvent {
            event: "stx_burn_event".into(),
            data: data.json_serialize(),
        },
        StacksTransactionEvent::STXEvent(STXEventType::STXLockEvent(data)) => StacksEvent {
            event: "stx_lock_event".into(),
            data: data.json_serialize(),
        },
        StacksTransactionEvent::NFTEvent(NFTEventType::NFTTransferEvent(data)) => StacksEvent {
            event: "nft_transfer_event".into(),
            data: data.json_serialize(),
        },
        StacksTransactionEvent::NFTEvent(NFTEventType::NFTMintEvent(data)) => StacksEvent {
            event: "nft_mint_event".into(),
            data: data.json_serialize(),
        },
        StacksTransactionEvent::NFTEvent(NFTEventType::NFTBurnEvent(data)) => StacksEvent {
            event: "nft_burn_event".into(),
            data: data.json_serialize(),
        },
        StacksTransactionEvent::FTEvent(FTEventType::FTTransferEvent(data)) => StacksEvent {
            event: "ft_transfer_event".into(),
            data: data.json_serialize(),
        },
        StacksTransactionEvent::FTEvent(FTEventType::FTMintEvent(data)) => StacksEvent {
            event: "ft_mint_event".into(),
            data: data.json_serialize(),
        },
        StacksTransactionEvent::FTEvent(FTEventType::FTBurnEvent(data)) => StacksEvent {
            event: "ft_burn_event".into(),
            data: data.json_serialize(),
        },
    }
}

pub fn to_raw_value(value: &Value) -> String {
    let mut bytes = vec![];

    value.consensus_serialize(&mut bytes).unwrap();
    let raw_value = bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<String>>();
    format!("0x{}", raw_value.join(""))
}

pub fn raw_value_to_string(value: &Uint8Array) -> String {
    let value = Value::consensus_deserialize(&mut &value.to_vec()[..])
        .expect("failed to parse clarity value");

    value.to_string()
}
