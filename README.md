# Thrum

A native-first Rust 3D engine that aims to be production grade. It exists to learn, to practise the craft and to
show how far AI-driven engineering can push performance and visual quality. It will run interactive teaching
demos on the owner's website with public source, and later, games. Native platforms use the hardware each device
provides. Browsers run a portable WebGPU tier at showcase quality. Single-player fidelity leads; local and
multiplayer play share the same authoritative rules.

**Status:** the V1 game runs a provisional 10,000-entity simulation on macOS and Chrome WebGPU, plus a thin
iPhone simulator shell. An optional native cooked valley shares the physical sky and frame host; Chrome still uses
the diagnostic scene. The showcase and creator experience remain unfinished. See [the build plan](docs/BUILD-PLAN.md).

To run the native authored scene on this Mac after setup:

```sh
cargo run --locked -p thrum-cook -- cook examples/v1-game/assets/showcase tmp/v1-valley
THRUM_ASSET_STORE=tmp/v1-valley THRUM_SCENE=valley cargo run --locked -p v1-game
```

These are the real cooker and game binary. The scene is still a visual work in progress; transport remains after V1.

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
