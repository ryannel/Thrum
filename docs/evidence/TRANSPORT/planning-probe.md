# TRANSPORT planning probe, 13 November 2026

A throwaway measurement the planner ran before writing [the design](../../plans/TRANSPORT/DESIGN.md). It answers
questions the design had to settle and that reading alone could not. It is **not** engine code and proves nothing
about the engine: milestone M1 repeats every row through `thrum-transport`. The probe's source stayed in the
planner's scratch folder and is not kept, so nothing here is a baseline.

## What ran

- **Host.** One Quinn 0.11.12 endpoint on `127.0.0.1` (quinn-proto 0.11.19, rustls 0.23 with `ring`), offering two
  ALPNs on one UDP port: `h3`, handed to web-transport-quinn 0.13.6 for the WebTransport handshake, and a private
  one taken as raw QUIC. A self-signed ECDSA P-256 certificate from rcgen 0.14.10, valid 13 days, made at start.
  The host echoes datagrams and length-prefixed messages on one client-opened bidirectional stream, and answers a
  "push" request with a message of the requested size.
- **Clients.** A Rust program on the Mac (raw QUIC, the certificate's SHA-256 pinned by a custom verifier); the
  same program built for `aarch64-apple-ios-sim` and run in the booted iPhone 18 Pro simulator (iOS 27.2) with
  `simctl spawn`; and a page in headless Chrome 157.0.8103.51, started with no flags that touch certificates or
  QUIC, using the browser's `WebTransport` with `serverCertificateHashes`. The page was served from
  `http://127.0.0.1`. It used JavaScript directly, not a Wasm adapter.
- **Conditioner.** A UDP relay between client and host. Each packet, in each direction, is dropped with
  probability 2% or delayed by 50 ms ± 10 ms, uniform and independent, from a fixed seed: A0's 100 ms round trip,
  ±20 ms jitter and 2% loss. Independent delays reorder packets heavily.
- **Load.** For 40 s: one 189-byte datagram at 60 Hz (the size of A5's four-record input batch) and one 400-byte
  reliable message at 10 Hz, each echoed. At 10 s the host pushes one 256 KiB or 1 MiB message down the stream.
- This M3 Max, Rust 1.99.1, while other agents were building. Timings are contended and each row is one run.

## 1. Does each target reach the host, with datagrams?

Yes, in this probe.

| Row | Result |
|---|---|
| Mac, direct | 600 of 600 datagrams echoed at sizes from 64 to 1,162 bytes; 101 of 101 messages in order; handshake 0.5 ms |
| Chrome, direct | 720 of 720 datagrams echoed; handshake 16 ms; `reliability` is `supports-unreliable` |
| iPhone simulator, through the relay | 2,293 of 2,400 datagrams echoed; every message in order |
| A wrong certificate hash | Refused on the Mac (`UnknownIssuer`) and in Chrome ("Opening handshake failed") |
| Reconnect after a clean close | A new connection exchanged a datagram on every target |
| A Mac client frozen for 8 s with a 4 s idle timeout | The host closed it as timed out; on resume the client's connection was reset |
| Chrome's renderer processes frozen for 9 s with a 4 s idle timeout | The host kept the link open; on resume the page carried on, and 1,500 of 1,500 datagrams came back. Chrome's network process answers keep-alives without the page |

No datagram was duplicated or corrupted in any run. Through the relay 6% to 25% of datagrams arrived out of order.

**Datagram ceilings.** Quinn reported 1,162 bytes for raw QUIC at its 1,200-byte starting MTU and refused 1,163
(`TooLarge`). Chrome reported `maxDatagramSize` 1,211; datagrams up to 1,211 bytes came back and 1,212 did not, with
no error. Every ceiling is above A5's 1,000-byte budget.

**Chrome 157's surface.** `getStats` is not a function. The `congestionControl` attribute is absent. The incoming
and outgoing datagram high-water marks default to 1 and can be raised; a value of 64 changed nothing measurable
here. A hint of `low-latency` was passed and could not be read back.

## 2. What does 2% loss do to the link?

Round trips in milliseconds as p50 / p95 / max. "During" is the 30 s that follow the push. Host packets lost
are Quinn's count of packets it declared lost, which includes ones the relay only reordered.

| Client | Host's controller | Datagrams, steady | Datagrams, during | Echoed of 2,400 | Push time | Host UDP bytes sent | Host packets lost / sent |
|---|---|---|---|---|---|---|---|
| Mac | Cubic, 256 KiB | 105 / 130 / 169 | 109 / 161 / 316 | 2,299 | 5.0 s | 0.99 MB | 133 / 2,270 |
| Mac | Cubic, 1 MiB | 107 / 148 / 206 | 121 / 227 / 505 | 2,295 | 17.9 s | 1.81 MB | 140 / 2,201 |
| Mac | BBR, 256 KiB | 104 / 117 / 124 | 104 / 118 / 124 | 2,290 | 0.38 s | 1.68 MB | 671 / 3,288 |
| Mac | BBR, 1 MiB | 103 / 118 / 124 | 105 / 120 / 153 | 2,287 | 0.70 s | 5.76 MB | 2,946 / 6,131 |
| Chrome | Cubic, 256 KiB | 105 / 124 / 180 | 108 / 149 / 208 | 2,303 | 4.7 s | 0.99 MB | 149 / 2,953 |
| Chrome | BBR, 256 KiB | 103 / 118 / 128 | 106 / 121 / 191 | 2,291 | 0.31 s | 1.66 MB | 679 / 3,986 |
| iPhone simulator | BBR, 256 KiB | 103 / 118 / 125 | 104 / 118 / 182 | 2,293 | 0.52 s | 1.54 MB | 555 / 3,178 |

What follows for the design:

- **A loss-based controller cannot carry a join through this loss.** Quinn's default, Cubic, read the relay's
  random loss as congestion and held its window near 4 KB, about 50 KB/s. That fits the game's steady traffic and
  makes a 1 MiB baseline take 18 s. While the transfer ran, later reliable messages waited behind it for seconds
  (p95 3.1 s at 256 KiB, 15.9 s at 1 MiB). NewReno was slower again (15.7 s for a 256 KiB echo).
- **Quinn's BBR carried it** in under a second with the datagram round trip unchanged, and paid in bytes: 1.7 times
  Cubic's for the 256 KiB runs and 3.2 times for 1 MiB, because it declared up to half its packets lost. Quinn
  marks its BBR "Experimental! Use at your own risk."
- **Upload from Chrome is Chrome's own controller.** A 256 KiB message echoed up and down took 5.4 to 5.7 s whatever
  the host ran, and datagram p95 rose to between 0.29 and 0.73 s while it did. The game sends only input upward.
- Reliable messages under BBR: p50 104 to 107 ms, p95 176 to 217 ms. The handshake through the relay took 111 to
  115 ms for raw QUIC and 327 to 340 ms for WebTransport, which adds HTTP/3 settings and a CONNECT.

## 3. What does it link?

The probe's closure was 143 crates, rcgen and its date crate included. Every licence is permissive: MIT,
Apache-2.0, ISC, BSD-3-Clause, Zlib, Unicode-3.0 and "Unlicense OR MIT". web-transport-quinn switches on Quinn's
`platform-verifier` and `bloom` features and brings rustls-native-certs. Both rustls crypto providers, `ring` and
`aws-lc-rs`, built for the Mac and for the simulator target with no extra tools; every run above used `ring`.

## Not probed

A Wasm adapter; four clients at once; runs longer than 40 s; a hostile or stalled peer; the in-memory link; a
second machine or any path that is not loopback.
