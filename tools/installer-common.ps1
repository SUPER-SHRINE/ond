#requires -Version 7.0
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
function Assert-InstallerChecksum {
    param([string]$AssetDirectory, [string]$Version)
    if ($Version -cnotmatch '^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$') { throw 'Invalid installer version.' }
    $name = "ond-$Version-windows-x86_64-setup.exe"
    $path = Join-Path $AssetDirectory $name
    if ((Get-Item -LiteralPath $path).Length -le 0) { throw 'Empty installer.' }
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant()
    $checksum = [IO.File]::ReadAllText("$path.sha256")
    if ($checksum -cnotmatch "\A$hash  $([regex]::Escape($name))\r?\n\z") { throw 'Installer checksum mismatch.' }
    return @($name, "$name.sha256")
}
