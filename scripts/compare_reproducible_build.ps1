param(
    [string]$BinaryA = "target\parity1\release\omni-stage0.exe",
    [string]$BinaryB = "target\parity2\release\omni-stage0.exe",
    [string]$NormalizedPath = "target/stable1",
    [switch]$InspectDiff
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Replace-AsciiBytes {
    param(
        [byte[]]$Data,
        [string]$OldText,
        [string]$NewText
    )

    $oldBytes = [System.Text.Encoding]::ASCII.GetBytes($OldText)
    $newBytes = [System.Text.Encoding]::ASCII.GetBytes($NewText)

    if ($oldBytes.Length -ne $newBytes.Length) {
        throw "Replacement text must be the same length as the original text."
    }

    for ($index = 0; $index -le $Data.Length - $oldBytes.Length; $index++) {
        $matches = $true
        for ($offset = 0; $offset -lt $oldBytes.Length; $offset++) {
            if ($Data[$index + $offset] -ne $oldBytes[$offset]) {
                $matches = $false
                break
            }
        }

        if ($matches) {
            for ($offset = 0; $offset -lt $oldBytes.Length; $offset++) {
                $Data[$index + $offset] = $newBytes[$offset]
            }
        }
    }

    return $Data
}

function Get-Sha256 {
    param([byte[]]$Data)

    $sha256 = [System.Security.Cryptography.SHA256]::Create()
    try {
        return ([System.BitConverter]::ToString($sha256.ComputeHash($Data))).Replace("-", "")
    }
    finally {
        $sha256.Dispose()
    }
}

function Normalize-Binary {
    param(
        [string]$Path,
        [string]$OldPath,
        [string]$NewPath
    )

    $bytes = [System.IO.File]::ReadAllBytes($Path)

    # Normalize the PE COFF timestamp so the comparison ignores linker metadata.
    for ($index = 0x100; $index -lt 0x104 -and $index -lt $bytes.Length; $index++) {
        $bytes[$index] = 0
    }

    $bytes = Replace-AsciiBytes -Data $bytes -OldText $OldPath -NewText $NewPath
    $bytes = Replace-AsciiBytes -Data $bytes -OldText ($OldPath -replace '\\', '/') -NewText $NewPath
    return $bytes
}

$binaryABytes = [System.IO.File]::ReadAllBytes($BinaryA)
$binaryBBytes = [System.IO.File]::ReadAllBytes($BinaryB)
$rawHashA = Get-Sha256 -Data $binaryABytes
$rawHashB = Get-Sha256 -Data $binaryBBytes

$normalizedA = Normalize-Binary -Path $BinaryA -OldPath "target\parity1" -NewPath $NormalizedPath
$normalizedB = Normalize-Binary -Path $BinaryB -OldPath "target\parity2" -NewPath $NormalizedPath
$normalizedHashA = Get-Sha256 -Data $normalizedA
$normalizedHashB = Get-Sha256 -Data $normalizedB

$result = [pscustomobject]@{
    BinaryA = $BinaryA
    BinaryB = $BinaryB
    RawHashA = $rawHashA
    RawHashB = $rawHashB
    NormalizedHashA = $normalizedHashA
    NormalizedHashB = $normalizedHashB
    MatchAfterNormalization = ($normalizedHashA -eq $normalizedHashB)
}

if ($InspectDiff) {
    $limit = [Math]::Min($normalizedA.Length, $normalizedB.Length)
    $firstDiff = -1
    for ($index = 0; $index -lt $limit; $index++) {
        if ($normalizedA[$index] -ne $normalizedB[$index]) {
            $firstDiff = $index
            break
        }
    }

    $result | Add-Member -NotePropertyName FirstDiffOffset -NotePropertyValue $firstDiff
    if ($firstDiff -ge 0) {
        $start = [Math]::Max(0, $firstDiff - 16)
        $end = [Math]::Min($limit - 1, $firstDiff + 32)
        $window = for ($index = $start; $index -le $end; $index++) {
            [pscustomobject]@{
                Offset = ('{0:X8}' -f $index)
                ByteA = ('{0:X2}' -f $normalizedA[$index])
                ByteB = ('{0:X2}' -f $normalizedB[$index])
                CharA = [char]$normalizedA[$index]
                CharB = [char]$normalizedB[$index]
            }
        }
        $result | Add-Member -NotePropertyName DiffWindow -NotePropertyValue $window
    }
}

$result
