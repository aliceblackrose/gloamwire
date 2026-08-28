#[cfg(feature = "compression-zlib")]
use flate2::{Decompress, FlushDecompress};
#[cfg(feature = "compression-zstd")]
use zstd::stream::raw::{Decoder as ZstdDecoder, Operation};

use crate::error::{Error, Result};

#[cfg(feature = "compression-zlib")]
const ZLIB_SUFFIX: [u8; 4] = [0x00, 0x00, 0xff, 0xff];
#[cfg(any(feature = "compression-zlib", feature = "compression-zstd"))]
const DECODE_CHUNK_BYTES: usize = 16 * 1024;

/// Transport compression requested from Discord's Gateway.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum GatewayCompression {
    /// Do not use Gateway transport compression.
    #[default]
    None,
    /// Use Discord's shared-context `zlib-stream` transport compression.
    ///
    /// Constructing a Gateway connection with this variant requires the
    /// `compression-zlib` Cargo feature.
    ZlibStream,
    /// Use Discord's shared-context `zstd-stream` transport compression.
    ///
    /// Constructing a Gateway connection with this variant requires the
    /// `compression-zstd` Cargo feature.
    ZstdStream,
}

impl GatewayCompression {
    pub(crate) const fn query_value(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::ZlibStream => Some("zlib-stream"),
            Self::ZstdStream => Some("zstd-stream"),
        }
    }
}

pub(crate) enum GatewayDecoder {
    Plain,
    #[cfg(feature = "compression-zlib")]
    Zlib(ZlibStreamDecoder),
    #[cfg(feature = "compression-zstd")]
    Zstd(ZstdStreamDecoder),
}

impl GatewayDecoder {
    pub(crate) fn new(compression: GatewayCompression) -> Result<Self> {
        match compression {
            GatewayCompression::None => Ok(Self::Plain),
            GatewayCompression::ZlibStream => {
                #[cfg(feature = "compression-zlib")]
                {
                    Ok(Self::Zlib(ZlibStreamDecoder::new()))
                }
                #[cfg(not(feature = "compression-zlib"))]
                {
                    Err(Error::GatewayCompression(
                        "zlib-stream support is disabled; enable the `compression-zlib` Cargo feature"
                            .to_owned(),
                    ))
                }
            }
            GatewayCompression::ZstdStream => {
                #[cfg(feature = "compression-zstd")]
                {
                    Ok(Self::Zstd(ZstdStreamDecoder::new()?))
                }
                #[cfg(not(feature = "compression-zstd"))]
                {
                    Err(Error::GatewayCompression(
                        "zstd-stream support is disabled; enable the `compression-zstd` Cargo feature"
                            .to_owned(),
                    ))
                }
            }
        }
    }

    /// Decodes one WebSocket binary message.
    ///
    /// `zlib-stream` may require multiple WebSocket messages before a complete
    /// Gateway payload is available, so `None` means more compressed bytes are
    /// required. Plain and zstd messages always return one decoded payload.
    pub(crate) fn decode(&mut self, bytes: &[u8]) -> Result<Option<Vec<u8>>> {
        match self {
            Self::Plain => Ok(Some(bytes.to_vec())),
            #[cfg(feature = "compression-zlib")]
            Self::Zlib(decoder) => decoder.decode(bytes),
            #[cfg(feature = "compression-zstd")]
            Self::Zstd(decoder) => decoder.decode(bytes).map(Some),
        }
    }
}

#[cfg(feature = "compression-zlib")]
pub(crate) struct ZlibStreamDecoder {
    inflater: Decompress,
    compressed: Vec<u8>,
}

#[cfg(feature = "compression-zlib")]
impl ZlibStreamDecoder {
    fn new() -> Self {
        Self {
            inflater: Decompress::new(true),
            compressed: Vec::new(),
        }
    }

    fn decode(&mut self, bytes: &[u8]) -> Result<Option<Vec<u8>>> {
        self.compressed.extend_from_slice(bytes);
        if !self.compressed.ends_with(&ZLIB_SUFFIX) {
            return Ok(None);
        }

        let compressed = std::mem::take(&mut self.compressed);
        self.inflate(&compressed).map(Some)
    }

    fn inflate(&mut self, compressed: &[u8]) -> Result<Vec<u8>> {
        let mut remaining = compressed;
        let mut decoded = Vec::new();

        loop {
            let mut chunk = [0_u8; DECODE_CHUNK_BYTES];
            let input_before = self.inflater.total_in();
            let output_before = self.inflater.total_out();
            self.inflater
                .decompress(remaining, &mut chunk, FlushDecompress::Sync)
                .map_err(|error| Error::GatewayCompression(error.to_string()))?;

            let consumed = (self.inflater.total_in() - input_before) as usize;
            let written = (self.inflater.total_out() - output_before) as usize;
            decoded.extend_from_slice(&chunk[..written]);
            remaining = &remaining[consumed..];

            if remaining.is_empty() && written < chunk.len() {
                return Ok(decoded);
            }
            if consumed == 0 && written == 0 {
                return Err(Error::GatewayCompression(
                    "zlib decoder made no progress".to_owned(),
                ));
            }
        }
    }
}

#[cfg(feature = "compression-zstd")]
pub(crate) struct ZstdStreamDecoder {
    decoder: ZstdDecoder<'static>,
}

#[cfg(feature = "compression-zstd")]
impl ZstdStreamDecoder {
    fn new() -> Result<Self> {
        let decoder =
            ZstdDecoder::new().map_err(|error| Error::GatewayCompression(error.to_string()))?;
        Ok(Self { decoder })
    }

    fn decode(&mut self, bytes: &[u8]) -> Result<Vec<u8>> {
        let mut remaining = bytes;
        let mut decoded = Vec::new();

        loop {
            let mut chunk = [0_u8; DECODE_CHUNK_BYTES];
            let status = self
                .decoder
                .run_on_buffers(remaining, &mut chunk)
                .map_err(|error| Error::GatewayCompression(error.to_string()))?;

            decoded.extend_from_slice(&chunk[..status.bytes_written]);
            remaining = &remaining[status.bytes_read..];

            if remaining.is_empty() && status.bytes_written < chunk.len() {
                return Ok(decoded);
            }
            if status.bytes_read == 0 && status.bytes_written == 0 {
                return Err(Error::GatewayCompression(
                    "zstd decoder made no progress".to_owned(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "compression-zstd")]
    use std::io::Cursor;

    #[cfg(feature = "compression-zlib")]
    use flate2::{Compress, Compression, FlushCompress};

    use super::{GatewayCompression, GatewayDecoder};

    #[test]
    fn compression_query_values_match_discord() {
        assert_eq!(GatewayCompression::None.query_value(), None);
        assert_eq!(
            GatewayCompression::ZlibStream.query_value(),
            Some("zlib-stream")
        );
        assert_eq!(
            GatewayCompression::ZstdStream.query_value(),
            Some("zstd-stream")
        );
    }

    #[cfg(feature = "compression-zlib")]
    #[test]
    fn zlib_stream_buffers_fragments_and_reuses_context() {
        let mut compressor = Compress::new(Compression::fast(), true);
        let first = compress_zlib_message(&mut compressor, br#"{"op":10,"d":{}}"#);
        let second = compress_zlib_message(&mut compressor, br#"{"op":11,"d":null}"#);
        assert!(first.ends_with(&super::ZLIB_SUFFIX));
        assert!(second.ends_with(&super::ZLIB_SUFFIX));

        let mut decoder = GatewayDecoder::new(GatewayCompression::ZlibStream).expect("decoder");
        let split = first.len() / 2;
        assert!(decoder.decode(&first[..split]).expect("fragment").is_none());
        assert_eq!(
            decoder
                .decode(&first[split..])
                .expect("first")
                .expect("complete"),
            br#"{"op":10,"d":{}}"#
        );
        assert_eq!(
            decoder.decode(&second).expect("second").expect("complete"),
            br#"{"op":11,"d":null}"#
        );
    }

    #[cfg(feature = "compression-zstd")]
    #[test]
    fn zstd_decoder_accepts_gateway_message_bytes() {
        let payload = br#"{"op":10,"d":{"heartbeat_interval":45000}}"#;
        let compressed = zstd::stream::encode_all(Cursor::new(payload), 1).expect("compress");
        let mut decoder = GatewayDecoder::new(GatewayCompression::ZstdStream).expect("decoder");
        assert_eq!(
            decoder.decode(&compressed).expect("decode"),
            Some(payload.to_vec())
        );
    }

    #[cfg(not(feature = "compression-zlib"))]
    #[test]
    fn zlib_stream_reports_disabled_feature() {
        let error = GatewayDecoder::new(GatewayCompression::ZlibStream)
            .err()
            .expect("zlib should be disabled");
        assert!(error.to_string().contains("compression-zlib"));
    }

    #[cfg(not(feature = "compression-zstd"))]
    #[test]
    fn zstd_stream_reports_disabled_feature() {
        let error = GatewayDecoder::new(GatewayCompression::ZstdStream)
            .err()
            .expect("zstd should be disabled");
        assert!(error.to_string().contains("compression-zstd"));
    }

    #[cfg(feature = "compression-zlib")]
    fn compress_zlib_message(compressor: &mut Compress, payload: &[u8]) -> Vec<u8> {
        let mut output = Vec::with_capacity(payload.len() * 2 + 128);
        compressor
            .compress_vec(payload, &mut output, FlushCompress::Sync)
            .expect("compress");
        output
    }
}
