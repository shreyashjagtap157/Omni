Param(
  [string]$PreferredVersion = '14.0.6'
)

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
  if (Test-Path $clang) {
    $verOut = & $clang --version
    if ($verOut -match '(\d+)\.(\d+)') {
      $maj = $Matches[1]
      if ($maj -eq '14') {
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
  Write-Error "LLVM 14.0.6 not found; please install LLVM and retry."
}
