#!/usr/bin/env python3
"""
aioquic QUIC Server for Interoperability Testing.
Protocol: ALPN "mini-quic-test"
Behavior: On client-initiated bidi stream, reads payload until FIN,
computes lowercase SHA-256 hex digest, and responds with the digest + FIN.
"""
import argparse
import asyncio
import hashlib
import ssl
from pathlib import Path
from aioquic.asyncio import QuicConnectionProtocol, serve
from aioquic.quic.configuration import QuicConfiguration
from aioquic.quic.events import HandshakeCompleted, StreamDataReceived, ConnectionTerminated

class InteropServerProtocol(QuicConnectionProtocol):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._stream_buffers = {}
        self._stream_hasher = {}

    def quic_event_received(self, event):
        if isinstance(event, StreamDataReceived):
            if event.stream_id not in self._stream_hasher:
                self._stream_hasher[event.stream_id] = hashlib.sha256()
            self._stream_hasher[event.stream_id].update(event.data)

            if event.end_stream:
                digest = self._stream_hasher[event.stream_id].hexdigest().lower().encode("ascii")
                self._quic.send_stream_data(event.stream_id, digest, end_stream=True)
                self.transmit()

async def run_server(host: str, port: int, cert_file: str, key_file: str, ready_event: asyncio.Event = None):
    configuration = QuicConfiguration(
        is_client=False,
        alpn_protocols=["mini-quic-test"],
    )
    configuration.load_cert_chain(cert_file, key_file)

    server = await serve(
        host=host,
        port=port,
        configuration=configuration,
        create_protocol=InteropServerProtocol,
    )
    if ready_event:
        ready_event.set()
    await asyncio.Future()

def main():
    parser = argparse.ArgumentParser(description="aioquic Interop Server")
    parser.add_argument("--host", default="127.0.0.1", help="Bind host")
    parser.add_argument("--port", type=int, default=4433, help="Bind port")
    parser.add_argument("--cert", default=str(Path(__file__).parent / "cert.pem"), help="Cert PEM")
    parser.add_argument("--key", default=str(Path(__file__).parent / "key.pem"), help="Key PEM")
    args = parser.parse_args()

    asyncio.run(run_server(args.host, args.port, args.cert, args.key))

if __name__ == "__main__":
    main()
