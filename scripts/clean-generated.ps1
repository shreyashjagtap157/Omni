param(
    [switch]$LocalLlvm
)
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

function Remove-Generated([string]$Path) {
    if (Test-Path $Path) {
        Write-Host "remove $Path"
        Remove-Item -Recurse -Force $Path
    }
}

Remove-Generated "target"
Get-ChildItem "crates" -Directory -Recurse -Filter "target" -ErrorAction SilentlyContinue |
    ForEach-Object { Remove-Generated $_.FullName }
Remove-Generated "crates/omni-fuzz/artifacts"
Remove-Generated ".fuzz-cache"
Remove-Generated "coverage"
Remove-Generated "tmp"

if ($LocalLlvm) {
    Remove-Generated "llvm-build"
    Remove-Generated "toolchains/llvm"
    Remove-Generated ".llvm-cache"
}

Write-Host "Generated-output cleanup complete. Fuzz source and codegen-llvm source were preserved."
