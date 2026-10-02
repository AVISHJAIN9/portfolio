#!/usr/bin/env python3
"""
aioquic QUIC Server for Interoperability Testing.
Protocol: ALPN "mini-quic-test"
Behavior: On client-initiated bidi stream, reads payload until FIN,
computes lowercase SHA-256 hex digest, and responds with the digest + FIN.
Logs qlog and connection metrics.
"""
import argparse
import asyncio
import hashlib
import json
import ssl
from pathlib import Path
from aioquic.asyncio import QuicConnectionProtocol, serve
from aioquic.quic.configuration import QuicConfiguration
from aioquic.quic.events import HandshakeCompleted, StreamDataReceived, ConnectionTerminated
from aioquic.quic.logger import QuicFileLogger

class InteropServerProtocol(QuicConnectionProtocol):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._stream_hasher = {}
        self.received_bytes_count = 0
        self.negotiated_alpn = None
        self.negotiated_version = None

    def quic_event_received(self, event):
        if isinstance(event, HandshakeCompleted):
            self.negotiated_alpn = event.alpn_protocol
            self.negotiated_version = f"0x{self._quic.version:08x}" if hasattr(self._quic, "version") else "1"
        elif isinstance(event, StreamDataReceived):
            if event.stream_id not in self._stream_hasher:
                self._stream_hasher[event.stream_id] = hashlib.sha256()
            self._stream_hasher[event.stream_id].update(event.data)
            self.received_bytes_count += len(event.data)

            if event.end_stream:
                digest = self._stream_hasher[event.stream_id].hexdigest().lower().encode("ascii")
                self._quic.send_stream_data(event.stream_id, digest, end_stream=True)
                self.transmit()

async def run_server(host: str, port: int, cert_file: str, key_file: str, qlog_dir: str = None, alpn: str = "mini-quic-test"):
    quic_logger = QuicFileLogger(qlog_dir) if qlog_dir else None
    configuration = QuicConfiguration(
        is_client=False,
        alpn_protocols=[alpn],
        quic_logger=quic_logger,
    )
    configuration.load_cert_chain(cert_file, key_file)

    server = await serve(
        host=host,
        port=port,
        configuration=configuration,
        create_protocol=InteropServerProtocol,
    )
    await asyncio.Future()

def main():
    parser = argparse.ArgumentParser(description="aioquic Interop Server")
    parser.add_argument("--host", default="127.0.0.1", help="Bind host")
    parser.add_argument("--port", type=int, default=4433, help="Bind port")
    parser.add_argument("--cert", default=str(Path(__file__).parent / "cert.pem"), help="Cert PEM")
    parser.add_argument("--key", default=str(Path(__file__).parent / "key.pem"), help="Key PEM")
    parser.add_argument("--qlog-dir", default=None, help="Directory for qlogs")
    parser.add_argument("--alpn", default="mini-quic-test", help="ALPN protocol")
    args = parser.parse_args()

    asyncio.run(run_server(args.host, args.port, args.cert, args.key, args.qlog_dir, args.alpn))

if __name__ == "__main__":
    main()
