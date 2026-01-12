//! Protocol codecs for JSON (arRPC-compatible) and MessagePack (optimized)

pub mod json;
pub mod msgpack;

use crate::types::ActivityMessage;
use thiserror::Error;

/// Protocol format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    /// JSON format (arRPC-compatible)
    Json,
    /// MessagePack format (optimized)
    MsgPack,
}

impl Protocol {
    /// Detect protocol from first byte of message
    pub fn detect(data: &[u8]) -> Self {
        if data.is_empty() {
            return Self::Json;
        }
        // JSON messages start with '{' or '['
        match data[0] {
            b'{' | b'[' => Self::Json,
            _ => Self::MsgPack,
        }
    }

    /// Parse protocol from query parameter
    pub fn from_query(query: &str) -> Option<Self> {
        if query.contains("format=msgpack") || query.contains("format=messagepack") {
            Some(Self::MsgPack)
        } else if query.contains("format=json") {
            Some(Self::Json)
        } else {
            None
        }
    }
}

/// Codec errors
#[derive(Debug, Error)]
pub enum CodecError {
    #[error("JSON encoding error: {0}")]
    JsonEncode(#[from] serde_json::Error),
    #[error("MessagePack encode error: {0}")]
    MsgPackEncode(#[from] rmp_serde::encode::Error),
    #[error("MessagePack decode error: {0}")]
    MsgPackDecode(#[from] rmp_serde::decode::Error),
}

/// Trait for protocol codecs
pub trait Codec: Send + Sync {
    /// Encode activity message to bytes
    fn encode_activity(&self, msg: &ActivityMessage) -> Result<Vec<u8>, CodecError>;

    /// Decode activity message from bytes
    fn decode_activity(&self, data: &[u8]) -> Result<ActivityMessage, CodecError>;

    /// Encode RPC response to bytes
    fn encode_response(&self, response: &crate::types::RpcResponse) -> Result<Vec<u8>, CodecError>;

    /// Decode RPC command from bytes
    fn decode_command(&self, data: &[u8]) -> Result<crate::types::RpcCommand, CodecError>;
}

/// Get codec for protocol
pub fn get_codec(protocol: Protocol) -> Box<dyn Codec> {
    match protocol {
        Protocol::Json => Box::new(json::JsonCodec),
        Protocol::MsgPack => Box::new(msgpack::MsgPackCodec),
    }
}
