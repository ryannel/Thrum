$ErrorActionPreference = "Stop"
Push-Location $PSScriptRoot
$previousPath = $env:PATH
try {
    $compiler = & rustup which --toolchain 1.99.1 rustc
    if ($LASTEXITCODE -ne 0) { throw "Install Rust 1.99.1 first; see README.md" }
    $env:PATH = "$(Split-Path $compiler)$([IO.Path]::PathSeparator)$previousPath"
    & rustup run 1.99.1 cargo run --locked --offline --quiet --package xtask -- @args
    $result = $LASTEXITCODE
} finally {
    $env:PATH = $previousPath
    Pop-Location
}
exit $result
