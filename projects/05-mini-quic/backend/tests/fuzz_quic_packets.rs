// FEAT: FEAT-05-001
use bytes::Bytes;
use mini_quic::wire::codec::PacketCodec;
use rand::Rng;

#[test]
fn test_fuzz_10000_quic_packets_no_panic() {
    let mut rng = rand::thread_rng();

    for _ in 0..10_000 {
        let len = rng.gen_range(0..1500);
        let mut data = vec![0u8; len];
        rng.fill(&mut data[..]);

        let mut buf = Bytes::from(data);
        // Ensure that decoding random fuzz data returns Err or Ok without panicking
        let _ = PacketCodec::decode(&mut buf);
    }
}
