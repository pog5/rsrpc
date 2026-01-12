//! MessagePack codec - optimized for performance

use super::{Codec, CodecError};
use crate::types::{ActivityMessage, RpcCommand, RpcResponse};

/// MessagePack codec for optimized performance
///
/// Uses rmp-serde for ~25% smaller messages and faster serialization.
/// Compatible with browser's @msgpack/msgpack library.
pub struct MsgPackCodec;

impl Codec for MsgPackCodec {
    fn encode_activity(&self, msg: &ActivityMessage) -> Result<Vec<u8>, CodecError> {
        Ok(rmp_serde::to_vec_named(msg)?)
    }

    fn decode_activity(&self, data: &[u8]) -> Result<ActivityMessage, CodecError> {
        Ok(rmp_serde::from_slice(data)?)
    }

    fn encode_response(&self, response: &RpcResponse) -> Result<Vec<u8>, CodecError> {
        Ok(rmp_serde::to_vec_named(response)?)
    }

    fn decode_command(&self, data: &[u8]) -> Result<RpcCommand, CodecError> {
        Ok(rmp_serde::from_slice(data)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::json::JsonCodec;
    use crate::types::Activity;

    #[test]
    fn test_encode_decode_activity() {
        let codec = MsgPackCodec;
        let msg = ActivityMessage {
            activity: Some(Activity {
                name: Some("Test Game".to_string()),
                details: Some("Playing level 5".to_string()),
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
    fn test_msgpack_smaller_than_json() {
        let json_codec = JsonCodec;
        let msgpack_codec = MsgPackCodec;

        let msg = ActivityMessage {
            activity: Some(Activity {
                name: Some("Test Game With Longer Name".to_string()),
                details: Some("Playing a detailed game session".to_string()),
                state: Some("In multiplayer lobby".to_string()),
                ..Default::default()
            }),
            pid: 12345,
            socket_id: "999".to_string(),
        };

        let json_encoded = json_codec.encode_activity(&msg).unwrap();
        let msgpack_encoded = msgpack_codec.encode_activity(&msg).unwrap();

        // MessagePack should be smaller
        assert!(
            msgpack_encoded.len() < json_encoded.len(),
            "MsgPack ({} bytes) should be smaller than JSON ({} bytes)",
            msgpack_encoded.len(),
            json_encoded.len()
        );
    }
}
