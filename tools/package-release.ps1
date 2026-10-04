#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidatePattern('^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$')]
    [string]$Version
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'release-manifest-common.ps1')
$repoRoot = Split-Path $PSScriptRoot -Parent
$commit = (& git -C $repoRoot rev-parse HEAD).Trim()
Get-ReleaseIdentity $repoRoot $Version $commit
if ([version]$Version -ge [version]'0.1.3') { Assert-ThirdPartyNoticeInventory $repoRoot }
$manifest = Get-Content -Raw (Join-Path $repoRoot 'Cargo.toml')
if ($manifest -notmatch '(?ms)^\[workspace\.package\].*?^version\s*=\s*"([^"]+)"' -or $Matches[1] -ne $Version) {
    throw 'Release version does not match Cargo.toml.'
}
$architecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
if ($architecture -ne 'X64' -or (-not $IsWindows -and -not $IsLinux)) {
    throw 'Release packaging requires Windows x64 or Linux x64.'
}
$target = if ($IsWindows) { 'windows-x86_64' } else { 'linux-x86_64' }
$suffix = if ($IsWindows) { '.exe' } else { '' }
$extension = if ($IsWindows) { 'zip' } else { 'tar.gz' }
$name = "ond-$Version-$target"
$output = Join-Path $repoRoot 'target/packages'
New-Item -ItemType Directory -Force -Path $output | Out-Null
$archive = Join-Path $output "$name.$extension"
$descriptor = Join-Path $output "$name-descriptor.json"
if ((Test-Path $archive) -or (Test-Path "$archive.sha256") -or (Test-Path $descriptor)) {
    throw "Output already exists; preserve or move it before packaging again: $archive"
}
$staging = Join-Path ([IO.Path]::GetTempPath()) ("ond-package-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $staging | Out-Null
try {
    foreach ($binary in @("ond$suffix", "ond-lsp$suffix")) {
        Copy-Item -LiteralPath (Join-Path $repoRoot "target/release/$binary") -Destination $staging
    }
    $paths = @(Get-ReleaseArchivePaths $target $Version)
    foreach ($document in @($paths | Where-Object { $_ -cnotin @("ond$suffix", "ond-lsp$suffix") })) {
        $destination = Join-Path $staging $document
        New-Item -ItemType Directory -Force -Path (Split-Path $destination -Parent) | Out-Null
        Copy-Item -LiteralPath (Join-Path $repoRoot $document) -Destination $destination
    }
    if ($IsWindows) {
        # directory entryを追加せず、元の相対pathを持つ通常fileだけを収録する。
        $zip = [IO.Compression.ZipFile]::Open($archive, [IO.Compression.ZipArchiveMode]::Create)
        try {
            foreach ($path in $paths) {
                [IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, (Join-Path $staging $path), $path) | Out-Null
            }
        }
        finally { $zip.Dispose() }
    }
    else {
        & chmod 755 (Join-Path $staging 'ond') (Join-Path $staging 'ond-lsp')
        if ($LASTEXITCODE -ne 0) { throw 'Could not set executable permissions.' }
        & tar -czf $archive -C $staging -- @paths
        if ($LASTEXITCODE -ne 0) { throw 'Could not create Linux archive.' }
    }
    $checksum = (Get-FileHash -Algorithm SHA256 -LiteralPath $archive).Hash.ToLowerInvariant()
    "$checksum  $([IO.Path]::GetFileName($archive))" | Set-Content -Encoding utf8NoBOM -LiteralPath "$archive.sha256"
    $entry = [ordered]@{
        archive = [ordered]@{ name = [IO.Path]::GetFileName($archive); sha256 = $checksum; size = (Get-Item -LiteralPath $archive).Length }
        binary = [ordered]@{ name = "ond-lsp$suffix"; sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $staging "ond-lsp$suffix")).Hash.ToLowerInvariant() }
        rustTarget = $(if ($IsWindows) { 'x86_64-pc-windows-msvc' } else { 'x86_64-unknown-linux-gnu' })
        target = $target
    }
    $verified = Get-VerifiedReleaseTarget $entry $output $Version
    Write-ReleaseJson $descriptor (Get-ReleaseEnvelope $Version $commit @($verified))
    Write-Host "Release archive: $archive"
}
finally {
    Remove-Item -LiteralPath $staging -Recurse -Force
}
