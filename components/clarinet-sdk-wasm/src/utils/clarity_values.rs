use clarity_repl::clarity::{
    codec::StacksMessageCodec,
    util::hash,
    vm::types::{CharType, SequenceData},
    Value,
};

pub fn to_raw_value(value: &Value) -> String {
    let mut bytes = vec![];
    value
        .consensus_serialize(&mut bytes)
        .unwrap_or_else(|e| panic!("failed to parse clarity value: {}", e));
    let raw_value = bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<String>>();
    format!("0x{}", raw_value.join(""))
}

pub fn uint8_to_string(value: &[u8]) -> String {
    value_to_string(&uint8_to_value(value))
}

pub fn uint8_to_value(mut value: &[u8]) -> Value {
    Value::consensus_deserialize(&mut value)
        .unwrap_or_else(|e| panic!("failed to parse clarity value: {}", e))
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::Principal(principal_data) => {
            format!("'{principal_data}")
        }
        Value::Tuple(tup_data) => {
            let mut data = Vec::new();
            for (name, value) in tup_data.data_map.iter() {
                data.push(format!("{}: {}", &**name, value_to_string(value)))
            }
            format!("{{ {} }}", data.join(", "))
        }
        Value::Optional(opt_data) => match opt_data.data {
            Some(ref x) => format!("(some {})", value_to_string(x)),
            None => "none".to_string(),
        },
        Value::Response(res_data) => match res_data.committed {
            true => format!("(ok {})", value_to_string(&res_data.data)),
            false => format!("(err {})", value_to_string(&res_data.data)),
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
            format!("u\"{result}\"")
        }
        Value::Sequence(SequenceData::List(list_data)) => {
            let mut data = Vec::new();
            for value in list_data.data.iter() {
                data.push(value_to_string(value))
            }
            format!("(list {})", data.join(" "))
        }
        _ => format!("{value}"),
    }
}

#[cfg(test)]
mod tests {
    use super::value_to_string;
    use clarity_repl::clarity::vm::types::{
        ASCIIData, CharType, ListData, ListTypeData, OptionalData, PrincipalData,
        QualifiedContractIdentifier, ResponseData, SequenceData, SequencedValue,
        StandardPrincipalData, TupleData, TypeSignature, UTF8Data, NONE,
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

        s = value_to_string(&Value::Principal(PrincipalData::Standard(
            StandardPrincipalData::transient(),
        )));
        assert_eq!(s, "'S1G2081040G2081040G2081040G208105NK8PE5");

        s = value_to_string(&Value::Principal(PrincipalData::Contract(
            QualifiedContractIdentifier::transient(),
        )));
        assert_eq!(s, "'S1G2081040G2081040G2081040G208105NK8PE5.__transient");

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
