#!/usr/bin/env python3
"""
aioquic QUIC Client for Interoperability Testing.
Protocol: ALPN "mini-quic-test"
Behavior: Connects to server, opens bidirectional stream, sends payload,
reads SHA-256 digest reply, and checks equality.
"""
import argparse
import asyncio
import hashlib
import ssl
import time
from aioquic.asyncio.client import connect
from aioquic.quic.configuration import QuicConfiguration
from aioquic.quic.events import StreamDataReceived

async def run_client(host: str, port: int, payload_size: int = 1048576) -> dict:
    configuration = QuicConfiguration(
        is_client=True,
        alpn_protocols=["mini-quic-test"],
        verify_mode=ssl.CERT_NONE,
    )

    payload = bytes([(i % 251) for i in range(payload_size)])
    local_sha256 = hashlib.sha256(payload).hexdigest().lower()

    start_t = time.time()
    received_bytes = bytearray()
    complete_event = asyncio.Event()

    async with connect(host, port, configuration=configuration) as protocol:
        stream_id = protocol._quic.get_next_available_stream_id()

        def custom_event_received(event):
            nonlocal received_bytes
            if isinstance(event, StreamDataReceived) and event.stream_id == stream_id:
                received_bytes.extend(event.data)
                if event.end_stream:
                    complete_event.set()

        protocol.quic_event_received = custom_event_received

        # Send stream data
        protocol._quic.send_stream_data(stream_id, payload, end_stream=True)
        protocol.transmit()

        await asyncio.wait_for(complete_event.wait(), timeout=60.0)

    duration_ms = int((time.time() - start_t) * 1000)
    remote_sha256 = received_bytes.decode("ascii", errors="ignore").strip().lower()
    matched = (remote_sha256 == local_sha256)

    return {
        "completed": True,
        "bytes": payload_size,
        "sha256_match": matched,
        "local_sha256": local_sha256,
        "remote_sha256": remote_sha256,
        "duration_ms": duration_ms,
    }

def main():
    parser = argparse.ArgumentParser(description="aioquic Interop Client")
    parser.add_argument("--host", default="127.0.0.1", help="Target host")
    parser.add_argument("--port", type=int, default=4433, help="Target port")
    parser.add_argument("--size", type=int, default=1048576, help="Payload size in bytes")
    args = parser.parse_args()

    res = asyncio.run(run_client(args.host, args.port, args.size))
    print(res)

if __name__ == "__main__":
    main()
