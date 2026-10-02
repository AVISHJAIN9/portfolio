#!/usr/bin/env python3
"""
lossy_proxy.py: Seeded UDP lossy proxy for QUIC packet drop simulation.
Can simulate independent packet drop probabilities in client->server and server->client directions.
"""
import argparse
import random
import socket
import select
import sys
import threading
import time

class LossyProxy:
    def __init__(self, listen_port: int, target_host: str, target_port: int, drop_rate: float, seed: int):
        self.listen_port = listen_port
        self.target_host = target_host
        self.target_port = target_port
        self.drop_rate = drop_rate
        self.rng = random.Random(seed)
        self.running = False
        self.sock = None
        self.client_addr = None

    def start(self):
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        self.sock.bind(("127.0.0.1", self.listen_port))
        self.sock.setblocking(False)
        self.running = True

        target_addr = (self.target_host, self.target_port)

        while self.running:
            r, _, _ = select.select([self.sock], [], [], 0.05)
            if not r:
                continue

            try:
                data, addr = self.sock.recvfrom(65535)
            except Exception:
                break

            # If packet is from target server, forward to client
            if addr == target_addr:
                if self.client_addr:
                    if self.rng.random() >= self.drop_rate:
                        self.sock.sendto(data, self.client_addr)
            else:
                # Packet is from client, remember client addr and forward to target
                self.client_addr = addr
                if self.rng.random() >= self.drop_rate:
                    self.sock.sendto(data, target_addr)

    def stop(self):
        self.running = False
        if self.sock:
            self.sock.close()

def main():
    parser = argparse.ArgumentParser(description="Lossy UDP Proxy")
    parser.add_argument("--listen-port", type=int, default=4434, help="Proxy listen port")
    parser.add_argument("--target-host", default="127.0.0.1", help="Target server host")
    parser.add_argument("--target-port", type=int, default=4433, help="Target server port")
    parser.add_argument("--drop-rate", type=float, default=0.05, help="Packet drop rate (0.0 to 1.0)")
    parser.add_argument("--seed", type=int, default=42, help="RNG seed")
    args = parser.parse_args()

    proxy = LossyProxy(args.listen_port, args.target_host, args.target_port, args.drop_rate, args.seed)
    try:
        proxy.start()
    except KeyboardInterrupt:
        proxy.stop()

if __name__ == "__main__":
    main()
