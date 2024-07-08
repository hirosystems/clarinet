use ::clarity::vm::events::{FTEventType, NFTEventType, STXEventType, StacksTransactionEvent};

use crate::repl::clarity_values::value_to_string;

pub fn serialize_event(event: &StacksTransactionEvent) -> serde_json::Value {
    match event {
        StacksTransactionEvent::SmartContractEvent(event_data) => json!({
            "type": "contract_event",
            "contract_event": {
                "contract_identifier": event_data.key.0.to_string(),
                "topic": event_data.key.1,
                "value": value_to_string(&event_data.value),
            }
        }),
        StacksTransactionEvent::STXEvent(STXEventType::STXTransferEvent(event_data)) => json!({
            "type": "stx_transfer_event",
            "stx_transfer_event": event_data.json_serialize()
        }),
        StacksTransactionEvent::STXEvent(STXEventType::STXMintEvent(event_data)) => json!({
            "type": "stx_mint_event",
            "stx_mint_event": event_data.json_serialize()
        }),
        StacksTransactionEvent::STXEvent(STXEventType::STXBurnEvent(event_data)) => json!({
            "type": "stx_burn_event",
            "stx_burn_event": event_data.json_serialize()
        }),
        StacksTransactionEvent::STXEvent(STXEventType::STXLockEvent(event_data)) => json!({
            "type": "stx_lock_event",
            "stx_lock_event": event_data.json_serialize()
        }),
        StacksTransactionEvent::NFTEvent(NFTEventType::NFTTransferEvent(event_data)) => json!({
            "type": "nft_transfer_event",
            "nft_transfer_event": {
                "asset_identifier": format!("{}", event_data.asset_identifier),
                "sender": format!("{}", event_data.sender),
                "recipient": format!("{}", event_data.recipient),
                "value": value_to_string(&event_data.value),
            }
        }),
        StacksTransactionEvent::NFTEvent(NFTEventType::NFTMintEvent(event_data)) => json!({
            "type": "nft_mint_event",
            "nft_mint_event": {
                "asset_identifier": format!("{}", event_data.asset_identifier),
                "recipient": format!("{}", event_data.recipient),
                "value": value_to_string(&event_data.value),
            }
        }),
        StacksTransactionEvent::NFTEvent(NFTEventType::NFTBurnEvent(event_data)) => json!({
            "type": "nft_burn_event",
            "nft_burn_event": {
                "asset_identifier": format!("{}", event_data.asset_identifier),
                "sender": format!("{}",event_data.sender),
                "value": value_to_string(&event_data.value),
            }
        }),
        StacksTransactionEvent::FTEvent(FTEventType::FTTransferEvent(event_data)) => json!({
            "type": "ft_transfer_event",
            "ft_transfer_event": event_data.json_serialize()
        }),
        StacksTransactionEvent::FTEvent(FTEventType::FTMintEvent(event_data)) => json!({
            "type": "ft_mint_event",
            "ft_mint_event": event_data.json_serialize()
        }),
        StacksTransactionEvent::FTEvent(FTEventType::FTBurnEvent(event_data)) => json!({
            "type": "ft_burn_event",
            "ft_burn_event": event_data.json_serialize()
        }),
    }
}
