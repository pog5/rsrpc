//! Integration tests for rsrpc

#[tokio::test]
async fn test_config_from_env() {
    let config = rsrpc::Config::from_env();
    assert_eq!(config.bridge_port, 1337);
    assert_eq!(config.msgpack_port, 1338);
    assert!(config.process_scanning);
}

#[tokio::test]
async fn test_protocol_detection() {
    use rsrpc::protocol::Protocol;

    // JSON detection
    assert_eq!(Protocol::detect(b"{\"test\": 1}"), Protocol::Json);
    assert_eq!(Protocol::detect(b"[1, 2, 3]"), Protocol::Json);

    // MessagePack detection (binary)
    assert_eq!(Protocol::detect(&[0x82, 0xa3]), Protocol::MsgPack);

    // Empty defaults to JSON
    assert_eq!(Protocol::detect(&[]), Protocol::Json);
}

#[tokio::test]
async fn test_codec_roundtrip() {
    use rsrpc::protocol::{get_codec, Protocol};
    use rsrpc::types::{Activity, ActivityMessage};

    let msg = ActivityMessage {
        activity: Some(Activity {
            name: Some("Test".to_string()),
            ..Default::default()
        }),
        pid: 1234,
        socket_id: "0".to_string(),
    };

    // Test JSON codec
    let json_codec = get_codec(Protocol::Json);
    let json_encoded = json_codec.encode_activity(&msg).unwrap();
    let json_decoded = json_codec.decode_activity(&json_encoded).unwrap();
    assert_eq!(json_decoded.pid, msg.pid);

    // Test MessagePack codec
    let msgpack_codec = get_codec(Protocol::MsgPack);
    let msgpack_encoded = msgpack_codec.encode_activity(&msg).unwrap();
    let msgpack_decoded = msgpack_codec.decode_activity(&msgpack_encoded).unwrap();
    assert_eq!(msgpack_decoded.pid, msg.pid);

    // MessagePack should be smaller
    assert!(msgpack_encoded.len() < json_encoded.len());
}

#[test]
fn test_ipc_opcode_conversion() {
    use rsrpc::types::IpcOpcode;

    assert_eq!(IpcOpcode::try_from(0), Ok(IpcOpcode::Handshake));
    assert_eq!(IpcOpcode::try_from(1), Ok(IpcOpcode::Frame));
    assert_eq!(IpcOpcode::try_from(2), Ok(IpcOpcode::Close));
    assert_eq!(IpcOpcode::try_from(3), Ok(IpcOpcode::Ping));
    assert_eq!(IpcOpcode::try_from(4), Ok(IpcOpcode::Pong));
    assert!(IpcOpcode::try_from(99).is_err());
}

#[test]
fn test_ready_data_defaults() {
    use rsrpc::types::{DiscordConfig, User};

    let config = DiscordConfig::default();
    assert_eq!(config.cdn_host, "cdn.discordapp.com");
    assert_eq!(config.environment, "production");

    let user = User::default();
    assert_eq!(user.username, "rsrpc");
    assert!(!user.bot);
}
