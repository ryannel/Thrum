# Thrum

A native-first Rust 3D engine that aims to be production grade. It exists to learn, to practise the craft and to
show how far AI-driven engineering can push performance and visual quality. It will run interactive teaching
demos on the owner's website with public source, and later, games. Native platforms use the hardware each device
provides. Browsers run a portable WebGPU tier at showcase quality. Single-player fidelity leads; local and
multiplayer play share the same authoritative rules.

**Status:** a provisional 10,000-entity simulation feeds a synthetic lit scene in the
`examples/v1-game` test consumer on macOS and Chrome WebGPU; a thin iPhone simulator shell
also exists. There is no finished scene, demo, real transport or shipping runtime yet. V1 ends in a
single-player showcase scene. The [V1 build plan](docs/BUILD-PLAN.md)
is the active work queue.

Install Rust 1.99.1 through rustup (with rustfmt and Clippy) and Python 3.11 or newer:

```sh
rustup toolchain install 1.99.1 --profile minimal --component rustfmt --component clippy
rustup run 1.99.1 cargo fetch --locked
./dev check
```

On Windows, use Git Bash for `./dev`, or run
`./dev.ps1 check` from PowerShell. Set `THRUM_PYTHON=python`
if Python is not installed as `python3`. Fetch the locked Rust dependencies once before the offline checks on a clean copy.

- [Central work list](docs/BUILD-PLAN.md): checked-off slices, next actions and milestone status.
- [Engine design](docs/design/README.md): requirements, architecture, modules and research.
- [Contributing](CONTRIBUTING.md): commands, conventions and review evidence.
- [Repository layout](docs/REPOSITORY.md): where work belongs and how it is retained.
- [Maturity ledger](docs/design/MATURITY.md): design is separate from running proof.

Thrum aims to use the hardware each device provides; see [the hardware-native decision](docs/design/decisions/HARDWARE-NATIVE-ENGINE.md).
Reuse decisions are recorded before adding libraries.
Plans were copied from Broadsides; [provenance](docs/provenance/IMPORT.md) records the import.
