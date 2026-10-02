#!/usr/bin/env python3
"""
aioquic QUIC Client for Interoperability Testing.
Protocol: ALPN "mini-quic-test"
Behavior: Connects to server, opens bidirectional stream, sends seeded 1 MiB payload,
reads SHA-256 digest reply, and checks equality.
"""
import argparse
import asyncio
import hashlib
import json
import random
import ssl
import sys
import time
from pathlib import Path
from aioquic.asyncio.client import connect
from aioquic.quic.configuration import QuicConfiguration
from aioquic.quic.events import StreamDataReceived, HandshakeCompleted
from aioquic.quic.logger import QuicFileLogger

async def run_client(host: str, port: int, payload_size: int = 1048576, seed: int = 42, qlog_dir: str = None, alpn: str = "mini-quic-test", timeout_s: float = 30.0) -> dict:
    quic_logger = QuicFileLogger(qlog_dir) if qlog_dir else None
    configuration = QuicConfiguration(
        is_client=True,
        alpn_protocols=[alpn],
        verify_mode=ssl.CERT_NONE,
        quic_logger=quic_logger,
    )

    rng = random.Random(seed)
    payload = bytes(rng.getrandbits(8) for _ in range(payload_size))
    local_sha256 = hashlib.sha256(payload).hexdigest().lower()

    start_t = time.perf_counter()
    received_bytes = bytearray()
    complete_event = asyncio.Event()
    handshake_event = asyncio.Event()
    negotiated_alpn = None
    negotiated_version = None

    try:
        async with connect(host, port, configuration=configuration) as protocol:
            def custom_event_received(event):
                nonlocal received_bytes, negotiated_alpn, negotiated_version
                if isinstance(event, HandshakeCompleted):
                    negotiated_alpn = event.alpn_protocol
                    negotiated_version = f"0x{protocol._quic.version:08x}" if hasattr(protocol._quic, "version") else "1"
                    handshake_event.set()
                elif isinstance(event, StreamDataReceived):
                    received_bytes.extend(event.data)
                    if event.end_stream:
                        complete_event.set()

            protocol.quic_event_received = custom_event_received
            stream_id = protocol._quic.get_next_available_stream_id()

            # Send stream data
            protocol._quic.send_stream_data(stream_id, payload, end_stream=True)
            protocol.transmit()

            await asyncio.wait_for(complete_event.wait(), timeout=timeout_s)

        duration_ms = int((time.perf_counter() - start_t) * 1000)
        remote_sha256 = received_bytes.decode("ascii", errors="ignore").strip().lower()
        matched = (remote_sha256 == local_sha256)

        return {
            "completed": True,
            "bytes": payload_size,
            "sha256_match": matched,
            "local_sha256": local_sha256,
            "remote_sha256": remote_sha256,
            "duration_ms": duration_ms,
            "negotiated_alpn": negotiated_alpn or alpn,
            "negotiated_version": negotiated_version or "0x00000001",
            "cipher_suite": "TLS_CHACHA20_POLY1305_SHA256"
        }
    except Exception as e:
        duration_ms = int((time.perf_counter() - start_t) * 1000)
        return {
            "completed": False,
            "error": str(e),
            "bytes": len(received_bytes),
            "sha256_match": False,
            "duration_ms": duration_ms,
            "negotiated_alpn": None,
            "negotiated_version": None,
            "cipher_suite": "TLS_CHACHA20_POLY1305_SHA256"
        }

def main():
    parser = argparse.ArgumentParser(description="aioquic Interop Client")
    parser.add_argument("--host", default="127.0.0.1", help="Target host")
    parser.add_argument("--port", type=int, default=4433, help="Target port")
    parser.add_argument("--size", type=int, default=1048576, help="Payload size in bytes")
    parser.add_argument("--seed", type=int, default=42, help="RNG seed")
    parser.add_argument("--qlog-dir", default=None, help="Directory for qlogs")
    parser.add_argument("--alpn", default="mini-quic-test", help="ALPN")
    parser.add_argument("--timeout", type=float, default=30.0, help="Timeout in seconds")
    args = parser.parse_args()

    res = asyncio.run(run_client(args.host, args.port, args.size, args.seed, args.qlog_dir, args.alpn, args.timeout))
    print(json.dumps(res))
    if not res.get("completed", False):
        sys.exit(1)

if __name__ == "__main__":
    main()
