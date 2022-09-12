use clarity::util::hash;
use clarity::vm::Value;
use std::fmt::Write;

pub fn value_to_string(value: &Value) -> String {
    use clarity::vm::types::{CharType, SequenceData};

    match value {
        Value::Tuple(tup_data) => {
            let mut out = String::new();
            let _ = write!(out, "{{");
            for (i, (name, value)) in tup_data.data_map.iter().enumerate() {
                let _ = write!(out, "{}: {}", &**name, value_to_string(value));
                if i < tup_data.data_map.len() - 1 {
                    let _ = write!(out, ", ");
                }
            }
            let _ = write!(out, "}}");
            out
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
                    // We escape extended charset
                    result.push_str(&format!("\\u{{{}}}", hash::to_hex(&c[..])));
                } else {
                    result.push(c[0] as char)
                }
            }
            format!("u\"{}\"", result)
        }
        Value::Sequence(SequenceData::List(list_data)) => {
            let mut out = String::new();
            let _ = write!(out, "[");
            for (ix, v) in list_data.data.iter().enumerate() {
                if ix > 0 {
                    let _ = write!(out, ", ");
                }
                let _ = write!(out, "{}", value_to_string(v));
            }
            let _ = write!(out, "]");
            out
        }
        _ => format!("{}", value),
    }
}

#[cfg(test)]
mod tests {
    use super::value_to_string;
    use clarity::vm::types::{
        ASCIIData, CharType, ListData, ListTypeData, OptionalData, ResponseData, SequenceData,
        SequencedValue, TupleData, TypeSignature, UTF8Data, NONE,
    };
    use clarity::vm::{ClarityName, Value};

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
        assert_eq!(s, "{foo: true}");

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
        assert_eq!(s, "[-321]");
    }
}
