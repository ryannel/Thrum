# Thrum

A Rust 3D game engine being built for beautiful, flexible games on modern desktops,
WebGPU browsers and native mobile platforms. Features and visual quality are selected
per game; local and multiplayer play share the same authoritative rules.

**Status:** architecture and initial specifications are recorded; this repository contains
a compilable scaffold and contributor checks. It does not run a game yet.

Install Rust 1.99.1 through rustup (with rustfmt and Clippy) and Python 3.11 or newer:

```sh
rustup toolchain install 1.99.1 --profile minimal --component rustfmt --component clippy
./dev check
```

On Windows, use Git Bash for `./dev`, or run
`./dev.ps1 check` from PowerShell. Set `THRUM_PYTHON=python`
if Python is not installed as `python3`. No third-party Rust crates are needed yet.

- [Build plan](docs/BUILD-PLAN.md): the next complete interaction and its acceptance criteria.
- [Engine design](docs/design/README.md): requirements, architecture, modules and research.
- [Contributing](CONTRIBUTING.md): commands, conventions and review evidence.
- [Repository layout](docs/REPOSITORY.md): where work belongs and how it is retained.
- [Maturity ledger](docs/design/MATURITY.md): design is separate from running proof.

Bevy is a reference, not a dependency. Reuse decisions are recorded before adding libraries.
Plans were copied from Broadsides; [provenance](docs/provenance/IMPORT.md) records the import.
