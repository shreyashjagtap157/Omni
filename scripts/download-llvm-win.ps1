# Attempt to download prebuilt Windows LLVM releases and extract for local use
# Tries multiple LLVM versions and sets an appropriate LLVM_SYS_<MAJOR><MINOR>_PREFIX env var

param(
    [string[]] $Versions = @('14.0.6'),
    [string] $OutDir = "third_party/llvm"
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path $OutDir)) { New-Item -ItemType Directory -Path $OutDir | Out-Null }
$OutDir = (Resolve-Path $OutDir).Path

function TryDownload($ver) {
    # Query GitHub release assets to find a Windows archive/executable
    $api = "https://api.github.com/repos/llvm/llvm-project/releases/tags/llvmorg-$ver"
    Write-Host ("Querying GitHub API for release llvmorg-{0}" -f $ver)
    try {
        $rel = Invoke-RestMethod -Uri $api -Headers @{ 'User-Agent' = 'Omni-Agent' } -ErrorAction Stop
    } catch {
        Write-Host ("Failed to query GitHub API for {0}: {1}" -f $ver, $_.ToString())
        return $null
    }

    $asset = $null
    foreach ($a in $rel.assets) {
        $n = $a.name
        if ($n -match '(?i)win' -and ($n -match '(?i)zip' -or $n -match '(?i)exe' -or $n -match '(?i)7z')) {
            $asset = $a
            break
        }
    }
    if (-not $asset) {
        Write-Host ("No suitable Windows asset found for release {0}" -f $ver)
        return $null
    }

    $url = $asset.browser_download_url
    $fileName = $asset.name
    $outPath = Join-Path $OutDir $fileName
    Write-Host ("Downloading asset {0} from {1}" -f $fileName, $url)
    try {
        Invoke-WebRequest -Uri $url -OutFile $outPath -UseBasicParsing -ErrorAction Stop
        Write-Host ("Downloaded {0}" -f $outPath)
        return $outPath
    } catch {
        Write-Host ("Download failed for {0}: {1}" -f $url, $_.ToString())
        if (Test-Path $outPath) { Remove-Item $outPath -Force -ErrorAction SilentlyContinue }
        return $null
    }
}

function Test-LLVMPrefix($root) {
    if (-not $root) {
        return $null
    }

    $hasClang = Test-Path (Join-Path $root 'bin\clang.exe')
    $hasLlvmConfig = Test-Path (Join-Path $root 'bin\llvm-config.exe')
    if ($hasClang -or $hasLlvmConfig) {
        return $root
    }

    return $null
}

foreach ($v in $Versions) {
    $z = TryDownload $v
        if ($z) {
            if (-not (Test-Path $z)) {
                Write-Output ("Downloaded file missing: {0}; skipping" -f $z)
                continue
            }
        $dest = Join-Path $OutDir ("llvm-$v")
        if (Test-Path $dest) { Remove-Item $dest -Recurse -Force -ErrorAction SilentlyContinue }
        Write-Output ("Installing {0} to {1}" -f $z, $dest)
        try {
            $ext = [System.IO.Path]::GetExtension($z).ToLowerInvariant()
            if ($ext -eq '.exe') {
                $args = @('/S', ('/D=' + $dest))
                Start-Process -FilePath $z -ArgumentList $args -Wait -NoNewWindow -ErrorAction Stop | Out-Null
            } else {
                Expand-Archive -Path $z -DestinationPath $dest -Force -ErrorAction Stop
            }
        } catch {
            Write-Output ("Failed to install {0}: {1}" -f $z, $_.ToString())
            continue
        }
        # Determine prefix (some installers/archives use a top-level folder or
        # the default Program Files location instead of the requested target).
        $prefix = $null
        $candidateRoots = @($dest)
        if ($env:ProgramFiles) {
            $candidateRoots += (Join-Path $env:ProgramFiles 'LLVM')
        }
        if (${env:ProgramFiles(x86)}) {
            $candidateRoots += (Join-Path ${env:ProgramFiles(x86)} 'LLVM')
        }

        foreach ($candidate in $candidateRoots) {
            $maybe = Test-LLVMPrefix $candidate
            if ($maybe) {
                $prefix = $maybe
                break
            }
        }

        if (-not $prefix) {
            $children = Get-ChildItem -Path $dest -Directory -ErrorAction SilentlyContinue
            foreach ($c in $children) {
                $maybe = Test-LLVMPrefix $c.FullName
                if ($maybe) {
                    $prefix = $maybe
                    break
                }
            }
        }
        if ($prefix) {
            Write-Output ("Checking for clang/llvm-config under {0}" -f $prefix)
            # Determine major/minor
            $maj = ($v -split '\.')[0]
            $min = ($v -split '\.')[1]
            $varName = "LLVM_SYS_${maj}${min}_PREFIX"
            Write-Output ("Setting environment variable {0} = {1}" -f $varName, $prefix)
            [System.Environment]::SetEnvironmentVariable($varName, $prefix, 'Process')
            Write-Output ("Success: extracted LLVM {0} to {1}" -f $v, $prefix)
            return @{ prefix = $prefix; var = $varName; version = $v }
        } else {
            Write-Output ("No clang/llvm-config found under extracted tree for {0}, continuing" -f $v)
        }
    }
}

Write-Output "Failed to download and extract any LLVM prebuilt releases."
return $null
