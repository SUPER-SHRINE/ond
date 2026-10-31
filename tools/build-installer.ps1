#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)] [string]$Version,
    [string]$AssetDirectory = (Join-Path $PSScriptRoot '../target/packages'),
    [string]$Compiler
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if (-not $IsWindows) { throw 'Installer compilation requires Windows.' }
if (-not $Compiler) { $Compiler = Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6/ISCC.exe' }
if (-not (Test-Path -LiteralPath $Compiler -PathType Leaf)) { throw "Inno Setup 6 compiler is required: $Compiler" }
. (Join-Path $PSScriptRoot 'release-manifest-common.ps1')
$AssetDirectory = (Resolve-Path -LiteralPath $AssetDirectory).Path
$descriptor = Read-ReleaseJson (Join-Path $AssetDirectory "ond-$Version-windows-x86_64-descriptor.json")
$commit = (& git -C (Split-Path $PSScriptRoot -Parent) rev-parse HEAD).Trim()
Get-ReleaseIdentity (Split-Path $PSScriptRoot -Parent) $Version $commit
Assert-ReleaseEnvelope $descriptor $Version $commit 1
$verified = Get-VerifiedReleaseTarget $descriptor.targets[0] $AssetDirectory $Version
if ($verified.target -cne 'windows-x86_64') { throw 'Expected Windows release archive.' }
$output = Join-Path $AssetDirectory "ond-$Version-windows-x86_64-setup.exe"
if ((Test-Path -LiteralPath $output) -or (Test-Path -LiteralPath "$output.sha256")) { throw "Output already exists: $output" }
$staging = Join-Path ([IO.Path]::GetTempPath()) ('ond-installer-' + [guid]::NewGuid().ToString('N'))
try {
    # Entry names, file types, inventory and checksums have been verified before extraction.
    [IO.Compression.ZipFile]::ExtractToDirectory((Join-Path $AssetDirectory $verified.archive.name), $staging)
    & $Compiler "/DReleaseVersion=$Version" "/DPayloadDirectory=$staging" "/DOutputDirectory=$AssetDirectory" (Join-Path $PSScriptRoot 'installer/ond.iss')
    if ($LASTEXITCODE -ne 0) { throw 'Inno Setup compilation failed.' }
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $output).Hash.ToLowerInvariant()
    "$hash  $([IO.Path]::GetFileName($output))" | Set-Content -Encoding utf8NoBOM -LiteralPath "$output.sha256"
}
finally { if (Test-Path -LiteralPath $staging) { Remove-Item -LiteralPath $staging -Recurse -Force } }
