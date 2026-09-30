use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pf_bytes::{QuicVarInt, ULEB128};

fn bench_varint(c: &mut Criterion) {
    c.bench_function("quic_varint_encode_4byte", |b| {
        let mut buf = Vec::with_capacity(8);
        b.iter(|| {
            buf.clear();
            QuicVarInt::encode(black_box(100_000_000), &mut buf).unwrap();
        })
    });

    c.bench_function("quic_varint_decode_4byte", |b| {
        let mut buf = Vec::new();
        QuicVarInt::encode(100_000_000, &mut buf).unwrap();
        b.iter(|| {
            let mut cursor = &buf[..];
            QuicVarInt::decode(black_box(&mut cursor)).unwrap()
        })
    });
}

criterion_group!(benches, bench_varint);
criterion_main!(benches);
