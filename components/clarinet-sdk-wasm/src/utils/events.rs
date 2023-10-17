use clarity_repl::clarity::{
    events::{FTEventType, NFTEventType, STXEventType},
    vm::events::StacksTransactionEvent,
};
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

#[allow(unused_macros)]
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
