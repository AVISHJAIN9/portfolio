use bytes::{Bytes, BytesMut};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mini_quic::wire::codec::PacketCodec;
use mini_quic::wire::frame::{AckFrame, Frame, StreamFrame};
use mini_quic::wire::packet::{ConnectionId, Header, Packet};

fn bench_quic_packet_encode(c: &mut Criterion) {
    let dcid = ConnectionId::new(b"12345678");
    let header = Header::Short {
        spin_bit: true,
        key_phase: false,
        dcid,
        packet_number: 100,
        packet_number_len: 2,
    };
    let stream_frame = Frame::Stream(StreamFrame {
        stream_id: 4,
        offset: 0,
        fin: false,
        data: Bytes::from_static(b"Sample HTTP/3 1-RTT Stream Chunk Data Payload"),
    });
    let ack_frame = Frame::Ack(AckFrame {
        largest_acknowledged: 99,
        ack_delay: 1,
        first_ack_range: 0,
        ack_ranges: vec![],
        ecn_counts: None,
    });
    let packet = Packet {
        header,
        frames: vec![stream_frame, ack_frame],
    };

    c.bench_function("quic_packet_encode", |b| {
        let mut buf = BytesMut::with_capacity(1500);
        b.iter(|| {
            buf.clear();
            PacketCodec::encode(black_box(&packet), &mut buf);
        });
    });
}

fn bench_quic_packet_decode(c: &mut Criterion) {
    let dcid = ConnectionId::new(b"12345678");
    let header = Header::Short {
        spin_bit: true,
        key_phase: false,
        dcid,
        packet_number: 100,
        packet_number_len: 2,
    };
    let stream_frame = Frame::Stream(StreamFrame {
        stream_id: 4,
        offset: 0,
        fin: false,
        data: Bytes::from_static(b"Sample HTTP/3 1-RTT Stream Chunk Data Payload"),
    });
    let packet = Packet {
        header,
        frames: vec![stream_frame],
    };

    let mut buf = BytesMut::new();
    PacketCodec::encode(&packet, &mut buf);
    let raw = buf.freeze();

    c.bench_function("quic_packet_decode", |b| {
        b.iter(|| {
            let mut read_slice = black_box(raw.clone());
            let _ = PacketCodec::decode(&mut read_slice).unwrap();
        });
    });
}

criterion_group!(benches, bench_quic_packet_encode, bench_quic_packet_decode);
criterion_main!(benches);
