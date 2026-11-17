$ErrorActionPreference = "Stop"
Push-Location $PSScriptRoot
$previousPath = $env:PATH
$previousIncremental = $env:CARGO_INCREMENTAL
try {
    $compiler = & rustup which --toolchain 1.99.1 rustc
    if ($LASTEXITCODE -ne 0) { throw "Install Rust 1.99.1 first; see README.md" }
    $env:PATH = "$(Split-Path $compiler)$([IO.Path]::PathSeparator)$previousPath"
    # Keep the verification gate independent of rustc's revision-sensitive query cache.
    if ($args[0] -eq "check") { $env:CARGO_INCREMENTAL = "0" }
    & rustup run 1.99.1 cargo run --locked --offline --quiet --package xtask -- @args
    $result = $LASTEXITCODE
} finally {
    $env:PATH = $previousPath
    $env:CARGO_INCREMENTAL = $previousIncremental
    Pop-Location
}
exit $result
