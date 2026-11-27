use std::mem::size_of;

use erltf::OwnedTerm;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Map, Number, Value};

use crate::error::{Error, Result};

/// Wire encoding used for Discord Gateway payloads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum GatewayEncoding {
    /// Plain-text JSON Gateway payloads.
    #[default]
    Json,
    /// Binary Erlang External Term Format Gateway payloads.
    /// Internally compressed ETF terms and non-JSON terms are rejected; use
    /// Gateway transport compression instead.
    Etf,
}

impl GatewayEncoding {
    pub(crate) const fn query_value(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Etf => "etf",
        }
    }

    pub(crate) fn encode<T>(self, value: &T) -> Result<EncodedGatewayPayload>
    where
        T: Serialize,
    {
        match self {
            Self::Json => serde_json::to_string(value)
                .map(EncodedGatewayPayload::Text)
                .map_err(Error::from),
            Self::Etf => erltf_serde::to_bytes(value)
                .map(EncodedGatewayPayload::Binary)
                .map_err(|error| Error::GatewayEncoding(error.to_string())),
        }
    }

    pub(crate) fn decode_bytes<T>(self, bytes: &[u8]) -> Result<T>
    where
        T: DeserializeOwned,
    {
        match self {
            Self::Json => Ok(serde_json::from_slice(bytes)?),
            Self::Etf => {
                validate_etf(bytes)?;
                let term = erltf::decode(bytes)
                    .map_err(|error| Error::GatewayEncoding(error.to_string()))?;
                Ok(serde_json::from_value(term_to_json(term)?)?)
            }
        }
    }

    pub(crate) fn decode_text<T>(self, text: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        match self {
            Self::Json => Ok(serde_json::from_str(text)?),
            Self::Etf => Err(Error::GatewayProtocol(
                "received a text WebSocket frame while ETF encoding was configured".to_owned(),
            )),
        }
    }
}

pub(crate) enum EncodedGatewayPayload {
    Text(String),
    Binary(Vec<u8>),
}

impl EncodedGatewayPayload {
    pub(crate) const fn len(&self) -> usize {
        match self {
            Self::Text(text) => text.len(),
            Self::Binary(bytes) => bytes.len(),
        }
    }
}

// Validate the JSON-compatible ETF subset before erltf performs recursive
// parsing or trusts any collection lengths. No allocation depends on wire counts.
const MAX_ETF_DEPTH: usize = 64;
const MAX_ETF_NODES: usize = 250_000;
const MAX_ETF_BYTES: usize = 32 * 1024 * 1024;

fn invalid_etf() -> Error {
    Error::GatewayEncoding(
        "invalid or unsupported ETF structure, or resource limit exceeded".into(),
    )
}

fn validate_etf(bytes: &[u8]) -> Result<()> {
    if bytes.len() > MAX_ETF_BYTES || bytes.first() != Some(&131) {
        return Err(invalid_etf());
    }
    let mut input = &bytes[1..];
    let mut nodes = MAX_ETF_NODES;
    validate_etf_term(&mut input, &mut nodes, 0)?;
    if !input.is_empty() {
        return Err(invalid_etf());
    }
    Ok(())
}

fn take_etf<'a>(input: &mut &'a [u8], count: usize) -> Result<&'a [u8]> {
    let (head, tail) = input.split_at_checked(count).ok_or_else(invalid_etf)?;
    *input = tail;
    Ok(head)
}

fn etf_count(input: &mut &[u8], width: usize) -> Result<usize> {
    let mut count = 0_usize;
    for &byte in take_etf(input, width)? {
        count = count
            .checked_mul(256)
            .and_then(|n| n.checked_add(usize::from(byte)))
            .ok_or_else(invalid_etf)?;
    }
    Ok(count)
}

fn validate_etf_term(input: &mut &[u8], nodes: &mut usize, depth: usize) -> Result<()> {
    if depth > MAX_ETF_DEPTH || *nodes == 0 {
        return Err(invalid_etf());
    }
    *nodes -= 1;
    let tag = take_etf(input, 1)?[0];
    match tag {
        70 => {
            take_etf(input, 8)?;
        } // NEW_FLOAT_EXT
        97 => {
            take_etf(input, 1)?;
        } // SMALL_INTEGER_EXT
        98 => {
            take_etf(input, 4)?;
        } // INTEGER_EXT
        99 => {
            take_etf(input, 31)?;
        } // FLOAT_EXT
        100 | 118 | 107 => {
            // atoms and STRING_EXT
            let count = etf_count(input, 2)?;
            take_etf(input, count)?;
        }
        115 | 119 => {
            let count = etf_count(input, 1)?;
            take_etf(input, count)?;
        }
        109 => {
            let count = etf_count(input, 4)?;
            take_etf(input, count)?;
        }
        110 | 111 => {
            let count = etf_count(input, if tag == 110 { 1 } else { 4 })?;
            if count > 8 || take_etf(input, 1)?[0] > 1 {
                return Err(invalid_etf());
            }
            take_etf(input, count)?;
        }
        106 => {} // NIL_EXT
        104 | 105 | 108 | 116 => {
            let count = etf_count(input, if tag == 104 { 1 } else { 4 })?;
            let children = if tag == 116 {
                count.checked_mul(2).ok_or_else(invalid_etf)?
            } else {
                count
            };
            // Every child requires at least a tag byte.
            if children > *nodes || children > input.len() {
                return Err(invalid_etf());
            }
            for _ in 0..children {
                validate_etf_term(input, nodes, depth + 1)?;
            }
            if tag == 108 {
                // Only proper lists can be represented as JSON arrays.
                if take_etf(input, 1)?[0] != 106 {
                    return Err(invalid_etf());
                }
            }
        }
        // Includes COMPRESSED, distribution terms, functions, pids and references.
        _ => return Err(invalid_etf()),
    }
    Ok(())
}

fn term_to_json(term: OwnedTerm) -> Result<Value> {
    match term {
        OwnedTerm::Atom(atom) => match atom.to_string().as_str() {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            "nil" | "undefined" => Ok(Value::Null),
            value => Ok(Value::String(value.to_owned())),
        },
        OwnedTerm::Integer(value) => Ok(Value::Number(Number::from(value))),
        OwnedTerm::BigInt(value) => bigint_to_json(value.sign.is_positive(), &value.digits),
        OwnedTerm::Float(value) => Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| Error::GatewayEncoding("ETF contained a non-finite float".to_owned())),
        OwnedTerm::Binary(bytes) => String::from_utf8(bytes)
            .map(Value::String)
            .map_err(|_| Error::GatewayEncoding("ETF binary was not valid UTF-8".to_owned())),
        OwnedTerm::String(value) => Ok(Value::String(value)),
        OwnedTerm::List(values) | OwnedTerm::Tuple(values) => values
            .into_iter()
            .map(term_to_json)
            .collect::<Result<Vec<_>>>()
            .map(Value::Array),
        OwnedTerm::Map(entries) => {
            let mut object = Map::with_capacity(entries.len());
            for (key, value) in entries {
                let key = term_to_json_key(key)?;
                let value = term_to_json(value)?;
                if object.insert(key, value).is_some() {
                    return Err(Error::GatewayEncoding(
                        "ETF map contained duplicate keys after string normalization".to_owned(),
                    ));
                }
            }
            Ok(Value::Object(object))
        }
        OwnedTerm::Nil => Ok(Value::Array(Vec::new())),
        OwnedTerm::BitBinary { .. }
        | OwnedTerm::ImproperList { .. }
        | OwnedTerm::Pid(_)
        | OwnedTerm::Port(_)
        | OwnedTerm::Reference(_)
        | OwnedTerm::ExternalFun(_)
        | OwnedTerm::InternalFun(_) => Err(Error::GatewayEncoding(
            "ETF contained a term that cannot represent Discord Gateway JSON data".to_owned(),
        )),
    }
}

fn term_to_json_key(term: OwnedTerm) -> Result<String> {
    match term {
        OwnedTerm::Atom(atom) => Ok(atom.to_string()),
        OwnedTerm::Binary(bytes) => String::from_utf8(bytes)
            .map_err(|_| Error::GatewayEncoding("ETF map key was not valid UTF-8".to_owned())),
        OwnedTerm::String(value) => Ok(value),
        _ => Err(Error::GatewayEncoding(
            "ETF Gateway map keys must be strings".to_owned(),
        )),
    }
}

fn bigint_to_json(positive: bool, digits: &[u8]) -> Result<Value> {
    if digits.len() > size_of::<u64>() {
        return Err(Error::GatewayEncoding(
            "ETF integer exceeded Discord's 64-bit Gateway integer range".to_owned(),
        ));
    }

    let mut bytes = [0_u8; size_of::<u64>()];
    bytes[..digits.len()].copy_from_slice(digits);
    let magnitude = u64::from_le_bytes(bytes);

    if positive {
        return Ok(Value::Number(Number::from(magnitude)));
    }

    if magnitude == 1_u64 << 63 {
        return Ok(Value::Number(Number::from(i64::MIN)));
    }
    if magnitude <= i64::MAX as u64 {
        return Ok(Value::Number(Number::from(-(magnitude as i64))));
    }

    Err(Error::GatewayEncoding(
        "negative ETF integer exceeded the signed 64-bit Gateway range".to_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use serde::{Deserialize, Serialize};
    use serde_json::{Number, Value, json};

    use super::{EncodedGatewayPayload, GatewayEncoding};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Envelope {
        op: u8,
        d: Value,
    }

    #[test]
    fn etf_preflight_rejects_excessive_depth_and_declared_lengths() {
        let mut nested = vec![131];
        for _ in 0..65 {
            nested.extend([104, 1]);
        }
        nested.push(106);
        assert!(GatewayEncoding::Etf.decode_bytes::<Value>(&nested).is_err());
        assert!(
            GatewayEncoding::Etf
                .decode_bytes::<Value>(&[131, 108, 255, 255, 255, 255, 106])
                .is_err()
        );
        assert!(
            GatewayEncoding::Etf
                .decode_bytes::<Value>(&[131, 80, 0, 0, 0, 1])
                .is_err()
        );
        assert!(
            GatewayEncoding::Etf
                .decode_bytes::<Value>(&[131, 106, 106])
                .is_err()
        );
        assert!(
            GatewayEncoding::Etf
                .decode_bytes::<Value>(&[131, 104, 1, 80, 0, 0, 0, 1])
                .is_err()
        );
    }

    #[test]
    fn encoding_query_values_match_discord() {
        assert_eq!(GatewayEncoding::Json.query_value(), "json");
        assert_eq!(GatewayEncoding::Etf.query_value(), "etf");
    }

    #[test]
    fn json_encodes_as_text() {
        let payload = Envelope {
            op: 1,
            d: Value::Null,
        };
        let EncodedGatewayPayload::Text(text) = GatewayEncoding::Json
            .encode(&payload)
            .expect("JSON payload")
        else {
            panic!("JSON must use a text payload");
        };

        assert_eq!(text, r#"{"op":1,"d":null}"#);
    }

    #[test]
    fn etf_encodes_struct_keys_as_binaries() {
        let payload = Envelope {
            op: 1,
            d: Value::Null,
        };
        let EncodedGatewayPayload::Binary(bytes) =
            GatewayEncoding::Etf.encode(&payload).expect("ETF payload")
        else {
            panic!("ETF must use a binary payload");
        };

        // ETF version, MAP_EXT, four-byte arity, then the first map key.
        // BINARY_EXT (109) confirms Discord-compatible string keys rather than atoms.
        assert_eq!(bytes[0], 131);
        assert_eq!(bytes[1], 116);
        assert_eq!(bytes[6], 109);
    }

    #[test]
    fn etf_round_trips_gateway_json_values_and_u64_snowflakes() {
        let payload = Envelope {
            op: 0,
            d: json!({
                "nullable": null,
                "enabled": true,
                "count": 42,
                "snowflake": u64::MAX
            }),
        };
        let EncodedGatewayPayload::Binary(bytes) =
            GatewayEncoding::Etf.encode(&payload).expect("ETF payload")
        else {
            panic!("ETF must use a binary payload");
        };

        let decoded: Envelope = GatewayEncoding::Etf
            .decode_bytes(&bytes)
            .expect("ETF round trip");
        assert_eq!(decoded, payload);
    }

    fn gateway_json_value() -> impl Strategy<Value = Value> {
        let leaf = prop_oneof![
            Just(Value::Null),
            any::<bool>().prop_map(Value::Bool),
            any::<i64>().prop_map(|value| Value::Number(Number::from(value))),
            any::<u64>().prop_map(|value| Value::Number(Number::from(value))),
            "[ -~]{0,48}".prop_map(Value::String),
        ];

        leaf.prop_recursive(3, 64, 8, |inner| {
            prop_oneof![
                prop::collection::vec(inner.clone(), 0..8).prop_map(Value::Array),
                prop::collection::btree_map("[A-Za-z0-9_]{1,16}", inner, 0..8)
                    .prop_map(|entries| Value::Object(entries.into_iter().collect()),),
            ]
        })
    }

    proptest! {
        #[test]
        fn arbitrary_etf_input_never_panics(bytes in prop::collection::vec(any::<u8>(), 0..1024)) {
            let _ = GatewayEncoding::Etf.decode_bytes::<Value>(&bytes);
        }

        #[test]
        fn json_round_trips_arbitrary_gateway_values(
            op in any::<u8>(),
            data in gateway_json_value(),
        ) {
            let payload = Envelope { op, d: data };
            let EncodedGatewayPayload::Text(text) = GatewayEncoding::Json
                .encode(&payload)
                .expect("JSON payload")
            else {
                return Err(TestCaseError::fail("JSON encoding produced a binary payload"));
            };

            let decoded: Envelope = GatewayEncoding::Json
                .decode_text(&text)
                .expect("JSON round trip");
            prop_assert_eq!(decoded, payload);
        }

        #[test]
        fn etf_round_trips_arbitrary_gateway_values(
            op in any::<u8>(),
            data in gateway_json_value(),
        ) {
            let payload = Envelope { op, d: data };
            let EncodedGatewayPayload::Binary(bytes) = GatewayEncoding::Etf
                .encode(&payload)
                .expect("ETF payload")
            else {
                return Err(TestCaseError::fail("ETF encoding produced a text payload"));
            };

            let decoded: Envelope = GatewayEncoding::Etf
                .decode_bytes(&bytes)
                .expect("ETF round trip");
            prop_assert_eq!(decoded, payload);
        }
    }
}
