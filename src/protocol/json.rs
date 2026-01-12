//! JSON codec - arRPC-compatible

use super::{Codec, CodecError};
use crate::types::{ActivityMessage, RpcCommand, RpcResponse};

/// JSON codec for arRPC compatibility
pub struct JsonCodec;

impl Codec for JsonCodec {
    fn encode_activity(&self, msg: &ActivityMessage) -> Result<Vec<u8>, CodecError> {
        Ok(serde_json::to_vec(msg)?)
    }

    fn decode_activity(&self, data: &[u8]) -> Result<ActivityMessage, CodecError> {
        Ok(serde_json::from_slice(data)?)
    }

    fn encode_response(&self, response: &RpcResponse) -> Result<Vec<u8>, CodecError> {
        Ok(serde_json::to_vec(response)?)
    }

    fn decode_command(&self, data: &[u8]) -> Result<RpcCommand, CodecError> {
        Ok(serde_json::from_slice(data)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Activity;

    #[test]
    fn test_encode_decode_activity() {
        let codec = JsonCodec;
        let msg = ActivityMessage {
            activity: Some(Activity {
                name: Some("Test Game".to_string()),
                ..Default::default()
            }),
            pid: 1234,
            socket_id: "0".to_string(),
        };

        let encoded = codec.encode_activity(&msg).unwrap();
        let decoded: ActivityMessage = codec.decode_activity(&encoded).unwrap();

        assert_eq!(decoded.pid, 1234);
        assert_eq!(decoded.socket_id, "0");
        assert!(decoded.activity.is_some());
    }

    #[test]
    fn test_json_format_compatibility() {
        let codec = JsonCodec;
        let msg = ActivityMessage {
            activity: None,
            pid: 5678,
            socket_id: "42".to_string(),
        };

        let encoded = codec.encode_activity(&msg).unwrap();
        let json_str = String::from_utf8(encoded).unwrap();

        // Verify arRPC-compatible format
        assert!(json_str.contains("\"socketId\""));
        assert!(json_str.contains("\"pid\""));
    }
}
