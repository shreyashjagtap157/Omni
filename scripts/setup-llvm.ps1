Param(
  [string]$PreferredVersion = '19.1.7'
)

if (Get-Command llvm-config -ErrorAction SilentlyContinue) {
  $prefix = & llvm-config --prefix
  $ver = & llvm-config --version
  $targetHeader = Join-Path $prefix 'include\llvm-c\Target.h'
  if ($ver -match '(\d+)\.(\d+)' -and (Test-Path $targetHeader)) {
    $maj = $Matches[1]
    $min = $Matches[2]
    $varName = "LLVM_SYS_${maj}${min}_PREFIX"
    [System.Environment]::SetEnvironmentVariable($varName, $prefix, 'Process')
    Write-Output "Detected existing LLVM at $prefix"
    Write-Output "Set $varName for current session to $prefix"
    return
  }
}

$repoRoot = Split-Path $PSScriptRoot -Parent
$localPrefix = Join-Path (Join-Path $repoRoot 'third_party\llvm19') "clang+llvm-$PreferredVersion-x86_64-pc-windows-msvc"
$localConfig = Join-Path $localPrefix 'bin\llvm-config.exe'
$localHeader = Join-Path $localPrefix 'include\llvm-c\Target.h'
if ((Test-Path $localConfig) -and (Test-Path $localHeader)) {
  $ver = & $localConfig --version
  if ($ver -match '(\d+)\.(\d+)') {
    $maj = $Matches[1]
    $min = $Matches[2]
    $varName = "LLVM_SYS_${maj}${min}_PREFIX"
    [System.Environment]::SetEnvironmentVariable($varName, $localPrefix, 'Process')
    Write-Output "Detected local LLVM archive at $localPrefix"
    Write-Output "Set $varName for current session to $localPrefix"
    return
  }
}

Write-Output "Attempting to install LLVM (PowerShell script). Requires administrative privileges."

try {
  choco install -y llvm --version=$PreferredVersion | Out-Null
} catch {
  Write-Output "choco install failed or choco not available. Please install LLVM manually."
}

$prefix = $null
$installedPrefix = 'C:\Program Files\LLVM'
if (Test-Path $installedPrefix) {
  $clang = Join-Path $installedPrefix 'bin\clang.exe'
  $targetHeader = Join-Path $installedPrefix 'include\llvm-c\Target.h'
  if (Test-Path $clang) {
    $verOut = & $clang --version
    if ($verOut -match '(\d+)\.(\d+)') {
      $maj = $Matches[1]
      if ($maj -eq '19' -and (Test-Path $targetHeader)) {
        $prefix = $installedPrefix
      }
    }
  }
}

if (-not $prefix) {
  $fallback = Join-Path $PSScriptRoot 'download-llvm-win.ps1'
  if (Test-Path $fallback) {
    $result = & $fallback -Versions @($PreferredVersion)
    if ($result -and $result.prefix) {
      $prefix = $result.prefix
    }
  }
}

if ($prefix -and (Test-Path $prefix)) {
  $clang = Join-Path $prefix 'bin\clang.exe'
  if (Test-Path $clang) {
    $verOut = & $clang --version
    Write-Output $verOut
    if ($verOut -match '(\d+)\.(\d+)') {
      $maj = $Matches[1]; $min = $Matches[2]
      $varName = "LLVM_SYS_${maj}${min}_PREFIX"
      [System.Environment]::SetEnvironmentVariable($varName, $prefix, 'Process')
      Write-Output "Set $varName for current session to $prefix"
      Write-Output "To persist this, set a system environment variable or update your profile."
    }
  } else {
    Write-Error "clang.exe not found under $prefix"
  }
} else {
  Write-Error "LLVM 19.1.7 not found; please install LLVM and retry."
}
