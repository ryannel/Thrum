#!/usr/bin/env python3
"""Proof-only UDP packet conditioner. Each instance carries one client, in both directions."""
import argparse
import asyncio
import json
import random
import signal
import time
from pathlib import Path


async def run(args):
    loop = asyncio.get_running_loop()
    randomizer = random.Random(args.seed)
    stats = {"seed": args.seed, "one_way_ms": 50, "jitter_ms": 10, "loss": 0.02,
             "directions": [{"forwarded": 0, "dropped": 0, "cut": 0, "bytes": 0, "seen_bytes": 0,
                             "first_packet_us": None, "last_packet_us": None} for _ in range(2)]}
    backends = {}
    opening = {}
    cut = ""
    stats["cut_windows"] = []
    pending = set()
    stop = asyncio.Event()

    def forward(direction, data, send):
        row = stats["directions"][direction]
        now = time.monotonic_ns() // 1000
        if row["first_packet_us"] is None:
            row["first_packet_us"] = now
        row["last_packet_us"] = now
        row["seen_bytes"] += len(data)
        if cut == "cut":
            row["cut"] += 1
            return
        if randomizer.random() < 0.02:
            row["dropped"] += 1
            return
        def deliver():
            pending.discard(handle)
            if cut == "cut" or (direction == 1 and cut == "cut download"):
                row["cut"] += 1
                return
            send(data)
            row["forwarded"] += 1
            row["bytes"] += len(data)
        handle = loop.call_later(randomizer.uniform(0.04, 0.06), deliver)
        pending.add(handle)

    class Front(asyncio.DatagramProtocol):
        def datagram_received(self, data, address):
            if address in backends:
                forward(0,data,lambda packet:backends[address].sendto(packet))
                return
            if address in opening:
                if len(opening[address]) < 32:
                    opening[address].append(data)
                return
            if len(backends) + len(opening) >= 16:
                return
            opening[address] = [data]
            async def create():
                transport, _ = await loop.create_datagram_endpoint(lambda: Back(address),remote_addr=(host,int(port)))
                backends[address] = transport
                for packet in opening.pop(address):
                    forward(0,packet,lambda payload:transport.sendto(payload))
            asyncio.create_task(create())

    class Back(asyncio.DatagramProtocol):
        def __init__(self,address):
            self.address = address
        def datagram_received(self, data, _address):
            forward(1, data, lambda packet: front.sendto(packet,self.address))

    host, port = args.host.rsplit(":", 1)
    front, _ = await loop.create_datagram_endpoint(Front, local_addr=("127.0.0.1", 0))
    print(json.dumps({"address": "127.0.0.1:" + str(front.get_extra_info("sockname")[1])}), flush=True)
    for sig in (signal.SIGTERM, signal.SIGINT):
        loop.add_signal_handler(sig, stop.set)
    try:
        while not stop.is_set():
            if args.control and Path(args.control).exists():
                command = Path(args.control).read_text().strip()
                if command != cut:
                    cut = command
                    stats["cut_windows"].append({"command": cut, "wall_us": time.time_ns() // 1000})
            try:
                await asyncio.wait_for(stop.wait(), 0.01)
            except TimeoutError:
                pass
    finally:
        for handle in pending:
            handle.cancel()
        front.close()
        for transport in backends.values():
            transport.close()
        Path(args.output).write_text(json.dumps(stats, indent=2) + "\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--host", required=True)
    parser.add_argument("--seed", type=int, default=29721)
    parser.add_argument("--control")
    parser.add_argument("--output", required=True)
    asyncio.run(run(parser.parse_args()))
