use serde::{Serialize, de::DeserializeOwned};

use crate::error::{Error, Result};

/// Wire encoding used for Discord Gateway payloads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum GatewayEncoding {
    /// Plain-text JSON Gateway payloads.
    #[default]
    Json,
    /// Binary Erlang External Term Format Gateway payloads.
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
            Self::Etf => erltf_serde::from_bytes(bytes)
                .map_err(|error| Error::GatewayEncoding(error.to_string())),
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

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_json::{Value, json};

    use super::{EncodedGatewayPayload, GatewayEncoding};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Envelope {
        op: u8,
        d: Value,
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
        let EncodedGatewayPayload::Binary(bytes) = GatewayEncoding::Etf
            .encode(&payload)
            .expect("ETF payload")
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
    fn etf_round_trips_gateway_null_and_json_values() {
        let payload = Envelope {
            op: 0,
            d: json!({"nullable": null, "enabled": true, "count": 42}),
        };
        let EncodedGatewayPayload::Binary(bytes) = GatewayEncoding::Etf
            .encode(&payload)
            .expect("ETF payload")
        else {
            panic!("ETF must use a binary payload");
        };

        let decoded: Envelope = GatewayEncoding::Etf
            .decode_bytes(&bytes)
            .expect("ETF round trip");
        assert_eq!(decoded, payload);
    }
}
