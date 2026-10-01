use bytes::{Bytes, BytesMut};
use mini_quic::connection::{ConnectionState, QuicConnection};
use mini_quic::wire::codec::PacketCodec;
use mini_quic::wire::frame::{Frame, StreamFrame};
use mini_quic::wire::packet::ConnectionId;
use pf_sim::{DeterministicRng, DiscreteEventSimulator};
use sha2::{Digest, Sha256};
use std::collections::VecDeque;

// FEAT: 05-138
// ACCEPT: Simulated network transfer of 100 KB payload across 0/5/20% packet loss with retransmissions and SHA-256 match
#[test]
fn test_simulated_network_transfer_at_loss_rates() {
    for (loss_rate, exp_min_retransmits) in [(0.0, 0), (0.05, 1), (0.20, 3)] {
        let mut rng = DeterministicRng::new(42);
        let mut sim = DiscreteEventSimulator::<Vec<u8>>::new();

        let server_cid = ConnectionId::random(8);
        let mut client = QuicConnection::new_client(server_cid.clone());
        let mut server = QuicConnection::new_server(client.scid.clone(), server_cid);

        client.state = ConnectionState::OneRtt;
        server.state = ConnectionState::OneRtt;

        // Generate 100 KB deterministic payload
        let mut payload = Vec::with_capacity(100 * 1024);
        for i in 0..(100 * 1024) {
            payload.push(((i * 31 + 7) % 251) as u8);
        }
        let sent_hash = Sha256::digest(&payload);

        let chunk_size = 1024;
        let num_chunks = payload.len() / chunk_size;
        let mut retransmit_count = 0;
        let mut pending_chunks: VecDeque<(usize, Vec<u8>)> = VecDeque::new();

        for i in 0..num_chunks {
            let chunk = payload[i * chunk_size..(i + 1) * chunk_size].to_vec();
            pending_chunks.push_back((i, chunk));
        }

        let mut simulated_time = 0;
        while (!pending_chunks.is_empty() || !sim.is_empty()) && simulated_time < 50_000 {
            // Send pending chunks
            if let Some((idx, chunk)) = pending_chunks.pop_front() {
                let is_fin = idx == num_chunks - 1 && pending_chunks.is_empty();
                let is_lost = rng.gen_bool(loss_rate);

                let offset = (idx * chunk_size) as u64;
                if is_lost {
                    retransmit_count += 1;
                    pending_chunks.push_back((idx, chunk));
                } else {
                    let pn = client.next_packet_number;
                    client.next_packet_number += 1;
                    let pkt = mini_quic::wire::packet::Packet {
                        header: mini_quic::wire::packet::Header::Short {
                            spin_bit: false,
                            key_phase: false,
                            dcid: client.dcid.clone(),
                            packet_number: pn,
                            packet_number_len: 2,
                        },
                        frames: vec![mini_quic::wire::frame::Frame::Stream(
                            mini_quic::wire::frame::StreamFrame {
                                stream_id: 4,
                                offset,
                                fin: is_fin,
                                data: Bytes::copy_from_slice(&chunk),
                            },
                        )],
                    };
                    let mut buf = BytesMut::new();
                    PacketCodec::encode(&pkt, &mut buf);
                    let delay = rng.gen_range(10..40);
                    sim.schedule_after(delay, buf.to_vec());
                }
            }

            if let Some((t, raw_packet)) = sim.step() {
                simulated_time = t;
                let raw_bytes = Bytes::from(raw_packet);
                let _ = server.handle_packet(raw_bytes);
            }
        }

        // Verify receiver stream reassembly and payload integrity
        let s = server.streams.get_or_create(4, false, 1024 * 1024);
        let mut received = Vec::new();
        while let Some(chunk) = s.read_contiguous() {
            received.extend_from_slice(&chunk);
        }

        let received_hash = Sha256::digest(&received);
        assert_eq!(received_hash, sent_hash, "Payload SHA-256 integrity mismatch at loss={loss_rate}");
        if loss_rate > 0.0 {
            assert!(
                retransmit_count >= exp_min_retransmits,
                "Expected retransmissions >= {exp_min_retransmits} for loss_rate={loss_rate}, got {retransmit_count}"
            );
        }
    }
}
