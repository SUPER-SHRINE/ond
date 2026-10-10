#requires -Version 7.0
[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$gitRoot = & git rev-parse --show-toplevel 2>$null
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($gitRoot)) {
    throw "tools/test-spec-ids.ps1 must be run inside the Ond Git repository."
}

$repoRoot = [System.IO.Path]::GetFullPath($gitRoot.Trim())
$specRoot = Join-Path $repoRoot "specs/ond"
$idPattern = '^[a-z0-9]+(?:-[a-z0-9]+)*(?:/[a-z0-9]+(?:-[a-z0-9]+)*)*$'
$markerPattern = '^<!-- ond-spec-id: (?<id>[^ ]+) -->$'
$seen = @{}
$failures = [System.Collections.Generic.List[string]]::new()
$headingCount = 0

foreach ($file in Get-ChildItem -LiteralPath $specRoot -Recurse -File -Filter "*.md" | Sort-Object FullName) {
    $relative = [System.IO.Path]::GetRelativePath($repoRoot, $file.FullName).Replace("\", "/")
    $lines = @(Get-Content -LiteralPath $file.FullName)
    $inFence = $false

    for ($index = 0; $index -lt $lines.Count; $index++) {
        $line = [string]$lines[$index]
        if ($line -match '^\s*(```|~~~)') {
            $inFence = -not $inFence
            continue
        }
        if ($inFence -or $line -notmatch '^#{1,6} ') {
            continue
        }

        $headingCount++
        $lineNumber = $index + 1
        if ($index + 1 -ge $lines.Count -or [string]$lines[$index + 1] -notmatch $markerPattern) {
            $failures.Add("${relative}:${lineNumber}: heading must be followed immediately by an ond-spec-id marker")
            continue
        }

        $id = [string]$Matches.id
        if ($id -notmatch $idPattern) {
            $failures.Add("${relative}:$($lineNumber + 1): invalid ond-spec-id '$id'")
            continue
        }
        if ($seen.ContainsKey($id)) {
            $failures.Add("${relative}:$($lineNumber + 1): duplicate ond-spec-id '$id' (first used at $($seen[$id]))")
            continue
        }
        $seen[$id] = "${relative}:$($lineNumber + 1)"
    }
}

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    exit 1
}

Write-Host "Validated $headingCount unique ond-spec-id marker(s)."
