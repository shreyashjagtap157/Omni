$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

# The normative v0.1.0 release gate is the deterministic 60-second black-box
# lexer/parser fuzz run. cargo-fuzz is an optional deeper oracle.
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "SKIP optional cargo-fuzz oracle: cargo is unavailable"
    exit 0
}
try {
    cargo fuzz --help *> $null
} catch {
    Write-Host "SKIP optional cargo-fuzz oracle: cargo-fuzz is not installed"
    exit 0
}
if (-not (Test-Path "crates/omni-fuzz")) {
    Write-Host "SKIP optional cargo-fuzz oracle: crates/omni-fuzz is not present"
    exit 0
}
Push-Location crates/omni-fuzz
try {
    cargo fuzz run lexer_parser -- -max_total_time=60
} finally {
    Pop-Location
}
Write-Host "Omni optional cargo-fuzz lexer/parser oracle PASS"
