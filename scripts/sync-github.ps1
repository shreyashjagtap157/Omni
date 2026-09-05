<#
.SYNOPSIS
    Convenience wrapper to bump Omni version and sync to git / GitHub remote.

.EXAMPLE
    .\scripts\sync-github.ps1 -Type patch
    .\scripts\sync-github.ps1 -Type patch -Number 1111
    .\scripts\sync-github.ps1 -Type minor
    .\scripts\sync-github.ps1 -Type major
    .\scripts\sync-github.ps1 -Type stable
    .\scripts\sync-github.ps1 -SetVersion "0.2.0.1111"
    .\scripts\sync-github.ps1 -Type patch -Remote github
#>

param(
    [ValidateSet("patch", "minor", "major", "stable", "current")]
    [string]$Type = "patch",

    [int]$Number = $null,

    [string]$SetVersion = "",

    [string]$Remote = "origin",

    [string]$Branch = "",

    [switch]$NoSync,

    [switch]$DryRun
)

$ErrorActionPreference = "Stop"
$RootDir = Split-Path -Parent $PSScriptRoot

if ($SetVersion -ne "") {
    $bumpArgs = @("scripts/bump-version.py", "set", $SetVersion)
} elseif ($Type -eq "patch" -and $PSBoundParameters.ContainsKey('Number')) {
    $bumpArgs = @("scripts/bump-version.py", "patch", "--number", $Number)
} else {
    $bumpArgs = @("scripts/bump-version.py", $Type)
}

if ($DryRun) {
    $bumpArgs += "--dry-run"
} elseif (-not $NoSync -and $Type -ne "current") {
    $bumpArgs += "--sync"
    $bumpArgs += @("--remote", $Remote)
    if ($Branch -ne "") {
        $bumpArgs += @("--branch", $Branch)
    }
}

Write-Host "Running: python $($bumpArgs -join ' ')" -ForegroundColor Cyan
& python $bumpArgs

if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

if (-not $DryRun -and $Type -ne "current") {
    Write-Host "Validating repository gates..." -ForegroundColor Cyan
    & python scripts/verify-source.py --worktree
    if ($LASTEXITCODE -ne 0) {
        Write-Error "verify-source.py failed"
        exit $LASTEXITCODE
    }
    & python scripts/audit-baseline.py --worktree
    if ($LASTEXITCODE -ne 0) {
        Write-Error "audit-baseline.py failed"
        exit $LASTEXITCODE
    }
    Write-Host "All source and baseline audit gates passed!" -ForegroundColor Green
}
