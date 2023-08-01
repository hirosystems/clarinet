use clarity_repl::clarity::{
    codec::StacksMessageCodec,
    events::{FTEventType, NFTEventType, STXEventType},
    util::hash,
    vm::{
        events::StacksTransactionEvent,
        types::{CharType, SequenceData},
    },
    Value,
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

pub fn to_raw_value(value: &Value) -> String {
    let mut bytes = vec![];

    value.consensus_serialize(&mut bytes).unwrap();
    let raw_value = bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<String>>();
    format!("0x{}", raw_value.join(""))
}

pub fn uint8_to_string(mut value: &[u8]) -> String {
    let value = Value::consensus_deserialize(&mut value).expect("failed to parse clarity value");
    value_to_string(&value)
}

pub fn uint8_to_value(mut value: &[u8]) -> Value {
    let value = Value::consensus_deserialize(&mut value).expect("failed to parse clarity value");
    value
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::Tuple(tup_data) => {
            let mut data = Vec::new();
            for (name, value) in tup_data.data_map.iter() {
                data.push(format!("{}: {}", &**name, value_to_string(value)))
            }
            format!("{{ {} }}", data.join(", "))
        }
        Value::Optional(opt_data) => match opt_data.data {
            Some(ref x) => format!("(some {})", value_to_string(&**x)),
            None => "none".to_string(),
        },
        Value::Response(res_data) => match res_data.committed {
            true => format!("(ok {})", value_to_string(&*res_data.data)),
            false => format!("(err {})", value_to_string(&*res_data.data)),
        },
        Value::Sequence(SequenceData::String(CharType::ASCII(data))) => {
            format!("\"{}\"", String::from_utf8(data.data.clone()).unwrap())
        }
        Value::Sequence(SequenceData::String(CharType::UTF8(data))) => {
            let mut result = String::new();
            for c in data.data.iter() {
                if c.len() > 1 {
                    // escape extended charset
                    result.push_str(&format!("\\u{{{}}}", hash::to_hex(&c[..])));
                } else {
                    result.push(c[0] as char)
                }
            }
            format!("u\"{}\"", result)
        }
        Value::Sequence(SequenceData::List(list_data)) => {
            let mut data = Vec::new();
            for value in list_data.data.iter() {
                data.push(value_to_string(value))
            }
            format!("(list {})", data.join(" "))
        }
        _ => format!("{}", value),
    }
}

#[cfg(test)]
mod tests {
    use super::value_to_string;
    use clarity_repl::clarity::vm::types::{
        ASCIIData, CharType, ListData, ListTypeData, OptionalData, ResponseData, SequenceData,
        SequencedValue, TupleData, TypeSignature, UTF8Data, NONE,
    };
    use clarity_repl::clarity::vm::{ClarityName, Value};
    use std::convert::TryFrom;

    #[test]
    fn test_value_to_string() {
        let mut s = value_to_string(&Value::Int(42));
        assert_eq!(s, "42");

        s = value_to_string(&Value::UInt(12345678909876));
        assert_eq!(s, "u12345678909876");

        s = value_to_string(&Value::Bool(true));
        assert_eq!(s, "true");

        s = value_to_string(&Value::buff_from(vec![1, 2, 3]).unwrap());
        assert_eq!(s, "0x010203");

        s = value_to_string(&Value::buff_from(vec![1, 2, 3]).unwrap());
        assert_eq!(s, "0x010203");

        s = value_to_string(&Value::Tuple(
            TupleData::from_data(vec![(
                ClarityName::try_from("foo".to_string()).unwrap(),
                Value::Bool(true),
            )])
            .unwrap(),
        ));
        assert_eq!(s, "{ foo: true }");

        s = value_to_string(&Value::Optional(OptionalData {
            data: Some(Box::new(Value::UInt(42))),
        }));
        assert_eq!(s, "(some u42)");

        s = value_to_string(&NONE);
        assert_eq!(s, "none");

        s = value_to_string(&Value::Response(ResponseData {
            committed: true,
            data: Box::new(Value::Int(-321)),
        }));
        assert_eq!(s, "(ok -321)");

        s = value_to_string(&Value::Response(ResponseData {
            committed: false,
            data: Box::new(Value::Sequence(SequenceData::String(CharType::ASCII(
                ASCIIData {
                    data: "'foo'".as_bytes().to_vec(),
                },
            )))),
        }));
        assert_eq!(s, "(err \"'foo'\")");

        s = value_to_string(&Value::Sequence(SequenceData::String(CharType::ASCII(
            ASCIIData {
                data: "Hello, \"world\"\n".as_bytes().to_vec(),
            },
        ))));
        assert_eq!(s, "\"Hello, \"world\"\n\"");

        s = value_to_string(&UTF8Data::to_value(&"Hello, 'world'\n".as_bytes().to_vec()));
        assert_eq!(s, "u\"Hello, 'world'\n\"");

        s = value_to_string(&Value::Sequence(SequenceData::List(ListData {
            data: vec![Value::Int(-321)],
            type_signature: ListTypeData::new_list(TypeSignature::IntType, 2).unwrap(),
        })));
        assert_eq!(s, "(list -321)");
    }
}
