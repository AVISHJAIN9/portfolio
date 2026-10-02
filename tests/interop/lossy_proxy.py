#!/usr/bin/env python3
"""
lossy_proxy.py: Seeded UDP lossy proxy for QUIC packet drop simulation and logging.
Writes detailed trace to evidence/interop/<run_id>/proxy.jsonl.
Supports bit-flip corruption for negative control test (ii).
"""
import argparse
import json
import random
import socket
import select
import sys
import time
from pathlib import Path

class LossyProxy:
    def __init__(self, listen_port: int, target_host: str, target_port: int, drop_rate: float, seed: int, log_dir: str = None, corrupt_one_bit: bool = False):
        self.listen_port = listen_port
        self.target_host = target_host
        self.target_port = target_port
        self.drop_rate = drop_rate
        self.rng = random.Random(seed)
        self.running = False
        self.sock = None
        self.client_addr = None
        self.log_dir = Path(log_dir) if log_dir else None
        self.corrupt_one_bit = corrupt_one_bit
        self.bit_flipped = False
        
        self.stats = {
            "c2s_total": 0,
            "c2s_dropped": 0,
            "c2s_forwarded": 0,
            "s2c_total": 0,
            "s2c_dropped": 0,
            "s2c_forwarded": 0,
        }

    def start(self, duration_s: float = 60.0):
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        self.sock.bind(("127.0.0.1", self.listen_port))
        self.sock.setblocking(False)
        self.running = True

        target_addr = (self.target_host, self.target_port)
        jsonl_f = None
        if self.log_dir:
            self.log_dir.mkdir(parents=True, exist_ok=True)
            jsonl_f = open(self.log_dir / "proxy.jsonl", "w", encoding="utf-8")

        start_time = time.time()
        try:
            while self.running and (time.time() - start_time < duration_s):
                r, _, _ = select.select([self.sock], [], [], 0.02)
                if not r:
                    continue

                try:
                    data, addr = self.sock.recvfrom(65535)
                except Exception:
                    break

                t_ns = time.time_ns()
                first_4_hex = data[:4].hex() if len(data) >= 4 else data.hex()

                if addr == target_addr:
                    # Server to Client
                    self.stats["s2c_total"] += 1
                    dropped = (self.rng.random() < self.drop_rate)
                    if dropped:
                        self.stats["s2c_dropped"] += 1
                    else:
                        self.stats["s2c_forwarded"] += 1
                        if self.client_addr:
                            self.sock.sendto(data, self.client_addr)
                    direction = "s2c"
                else:
                    # Client to Server
                    self.client_addr = addr
                    self.stats["c2s_total"] += 1
                    dropped = (self.rng.random() < self.drop_rate)
                    if dropped:
                        self.stats["c2s_dropped"] += 1
                    else:
                        # Check if bit flip requested on a 1-RTT packet (not initial)
                        if self.corrupt_one_bit and not self.bit_flipped and len(data) > 30 and (data[0] & 0x80 == 0):
                            # Flip 1 bit in ciphertext
                            b = bytearray(data)
                            b[20] ^= 0x01
                            data = bytes(b)
                            self.bit_flipped = True

                        self.stats["c2s_forwarded"] += 1
                        self.sock.sendto(data, target_addr)
                    direction = "c2s"

                if jsonl_f:
                    rec = {
                        "t_ns": t_ns,
                        "direction": direction,
                        "length": len(data),
                        "dropped": dropped,
                        "first_4_hex": first_4_hex
                    }
                    jsonl_f.write(json.dumps(rec) + "\n")
                    jsonl_f.flush()
        finally:
            if jsonl_f:
                jsonl_f.close()
            if self.log_dir:
                with open(self.log_dir / "proxy_summary.json", "w", encoding="utf-8") as sf:
                    total_pkts = self.stats["c2s_total"] + self.stats["s2c_total"]
                    total_dropped = self.stats["c2s_dropped"] + self.stats["s2c_dropped"]
                    ratio = (total_dropped / total_pkts) if total_pkts > 0 else 0.0
                    self.stats["measured_drop_ratio"] = round(ratio, 4)
                    json.dump(self.stats, sf, indent=2)

    def stop(self):
        self.running = False
        if self.sock:
            self.sock.close()

def main():
    parser = argparse.ArgumentParser(description="Lossy UDP Proxy")
    parser.add_argument("--listen-port", type=int, default=4434, help="Proxy listen port")
    parser.add_argument("--target-host", default="127.0.0.1", help="Target server host")
    parser.add_argument("--target-port", type=int, default=4433, help="Target server port")
    parser.add_argument("--drop-rate", type=float, default=0.05, help="Packet drop rate")
    parser.add_argument("--seed", type=int, default=42, help="RNG seed")
    parser.add_argument("--log-dir", default=None, help="Directory to output proxy.jsonl")
    parser.add_argument("--corrupt-one-bit", action="store_true", help="Corrupt one bit in 1-RTT packet")
    args = parser.parse_args()

    proxy = LossyProxy(args.listen_port, args.target_host, args.target_port, args.drop_rate, args.seed, args.log_dir, args.corrupt_one_bit)
    try:
        proxy.start()
    except KeyboardInterrupt:
        proxy.stop()

if __name__ == "__main__":
    main()
