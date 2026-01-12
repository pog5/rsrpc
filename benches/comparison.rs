//! Benchmarks comparing rsrpc performance vs arRPC
//!
//! Run with: cargo bench

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_json_encode(c: &mut Criterion) {
    use rsrpc::protocol::json::JsonCodec;
    use rsrpc::protocol::Codec;
    use rsrpc::types::{Activity, ActivityMessage, Timestamps};

    let codec = JsonCodec;
    let msg = ActivityMessage {
        activity: Some(Activity {
            name: Some("Test Game".to_string()),
            details: Some("Playing level 5".to_string()),
            state: Some("In multiplayer".to_string()),
            timestamps: Some(Timestamps {
                start: Some(1704067200000),
                end: None,
            }),
            ..Default::default()
        }),
        pid: 12345,
        socket_id: "0".to_string(),
    };

    c.bench_function("json_encode_activity", |b| {
        b.iter(|| codec.encode_activity(black_box(&msg)))
    });
}

fn benchmark_msgpack_encode(c: &mut Criterion) {
    use rsrpc::protocol::msgpack::MsgPackCodec;
    use rsrpc::protocol::Codec;
    use rsrpc::types::{Activity, ActivityMessage, Timestamps};

    let codec = MsgPackCodec;
    let msg = ActivityMessage {
        activity: Some(Activity {
            name: Some("Test Game".to_string()),
            details: Some("Playing level 5".to_string()),
            state: Some("In multiplayer".to_string()),
            timestamps: Some(Timestamps {
                start: Some(1704067200000),
                end: None,
            }),
            ..Default::default()
        }),
        pid: 12345,
        socket_id: "0".to_string(),
    };

    c.bench_function("msgpack_encode_activity", |b| {
        b.iter(|| codec.encode_activity(black_box(&msg)))
    });
}

fn benchmark_json_decode(c: &mut Criterion) {
    use rsrpc::protocol::json::JsonCodec;
    use rsrpc::protocol::Codec;
    use rsrpc::types::{Activity, ActivityMessage, Timestamps};

    let codec = JsonCodec;
    let msg = ActivityMessage {
        activity: Some(Activity {
            name: Some("Test Game".to_string()),
            details: Some("Playing level 5".to_string()),
            state: Some("In multiplayer".to_string()),
            timestamps: Some(Timestamps {
                start: Some(1704067200000),
                end: None,
            }),
            ..Default::default()
        }),
        pid: 12345,
        socket_id: "0".to_string(),
    };
    let encoded = codec.encode_activity(&msg).unwrap();

    c.bench_function("json_decode_activity", |b| {
        b.iter(|| codec.decode_activity(black_box(&encoded)))
    });
}

fn benchmark_msgpack_decode(c: &mut Criterion) {
    use rsrpc::protocol::msgpack::MsgPackCodec;
    use rsrpc::protocol::Codec;
    use rsrpc::types::{Activity, ActivityMessage, Timestamps};

    let codec = MsgPackCodec;
    let msg = ActivityMessage {
        activity: Some(Activity {
            name: Some("Test Game".to_string()),
            details: Some("Playing level 5".to_string()),
            state: Some("In multiplayer".to_string()),
            timestamps: Some(Timestamps {
                start: Some(1704067200000),
                end: None,
            }),
            ..Default::default()
        }),
        pid: 12345,
        socket_id: "0".to_string(),
    };
    let encoded = codec.encode_activity(&msg).unwrap();

    c.bench_function("msgpack_decode_activity", |b| {
        b.iter(|| codec.decode_activity(black_box(&encoded)))
    });
}

fn benchmark_message_size(c: &mut Criterion) {
    use rsrpc::protocol::json::JsonCodec;
    use rsrpc::protocol::msgpack::MsgPackCodec;
    use rsrpc::protocol::Codec;
    use rsrpc::types::{Activity, ActivityMessage, Button, Timestamps};

    let json_codec = JsonCodec;
    let msgpack_codec = MsgPackCodec;

    let msg = ActivityMessage {
        activity: Some(Activity {
            name: Some("Awesome Game Title".to_string()),
            details: Some("Playing competitive ranked match".to_string()),
            state: Some("Score: 10-5 in Round 3".to_string()),
            timestamps: Some(Timestamps {
                start: Some(1704067200000),
                end: None,
            }),
            buttons: Some(vec![
                Button {
                    label: "Join Game".to_string(),
                    url: "https://example.com/join".to_string(),
                },
                Button {
                    label: "View Profile".to_string(),
                    url: "https://example.com/profile".to_string(),
                },
            ]),
            ..Default::default()
        }),
        pid: 12345,
        socket_id: "42".to_string(),
    };

    let json_size = json_codec.encode_activity(&msg).unwrap().len();
    let msgpack_size = msgpack_codec.encode_activity(&msg).unwrap().len();

    println!("\nMessage size comparison:");
    println!("  JSON:     {} bytes", json_size);
    println!("  MsgPack:  {} bytes", msgpack_size);
    println!(
        "  Savings:  {:.1}%",
        (1.0 - msgpack_size as f64 / json_size as f64) * 100.0
    );

    c.bench_function("size_comparison", |b| {
        b.iter(|| {
            let j = json_codec.encode_activity(black_box(&msg)).unwrap().len();
            let m = msgpack_codec
                .encode_activity(black_box(&msg))
                .unwrap()
                .len();
            (j, m)
        })
    });
}

criterion_group!(
    benches,
    benchmark_json_encode,
    benchmark_msgpack_encode,
    benchmark_json_decode,
    benchmark_msgpack_decode,
    benchmark_message_size,
);

criterion_main!(benches);
