#requires -Version 7.0
# native Release binaryの包装と、第三者ライセンス情報のbyte一致を公開せず検査する。
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path $PSScriptRoot -Parent
$cargo = Get-Content -Raw (Join-Path $repoRoot 'Cargo.toml')
if ($cargo -notmatch '(?ms)^\[workspace\.package\].*?^version\s*=\s*"([^"]+)"') { throw 'Missing workspace version.' }
$version = $Matches[1]
& (Join-Path $PSScriptRoot 'package-release.ps1') -Version $version
$target = if ($IsWindows) { 'windows-x86_64' } else { 'linux-x86_64' }
$extension = if ($IsWindows) { 'zip' } else { 'tar.gz' }
$archive = Join-Path $repoRoot "target/packages/ond-$version-$target.$extension"
$root = Join-Path ([IO.Path]::GetTempPath()) ("ond-package-test-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
try {
    # package-releaseが通常fileの厳密な一覧とhashを検証済みのarchiveだけを展開する。
    if ($IsWindows) { [IO.Compression.ZipFile]::ExtractToDirectory($archive, $root) }
    else {
        & tar -xzf $archive -C $root
        if ($LASTEXITCODE -ne 0) { throw 'Could not extract test archive.' }
    }
    $noticePath = Join-Path $root 'THIRD-PARTY-NOTICES.txt'
    if (-not (Test-Path -LiteralPath $noticePath)) { throw 'Archive is missing third-party notices.' }
    $sourceNoticeHash = (Get-FileHash -LiteralPath (Join-Path $repoRoot 'THIRD-PARTY-NOTICES.txt') -Algorithm SHA256).Hash
    $archiveNoticeHash = (Get-FileHash -LiteralPath $noticePath -Algorithm SHA256).Hash
    if ($sourceNoticeHash -cne $archiveNoticeHash) { throw 'Archived third-party notices differ from source bytes.' }
    $copyrightPath = Join-Path $root 'COPYRIGHT-library.html'
    if (-not (Test-Path -LiteralPath $copyrightPath)) { throw 'Archive is missing the Rust standard-library copyright report.' }
    $sourceCopyrightHash = (Get-FileHash -LiteralPath (Join-Path $repoRoot 'COPYRIGHT-library.html') -Algorithm SHA256).Hash
    $archiveCopyrightHash = (Get-FileHash -LiteralPath $copyrightPath -Algorithm SHA256).Hash
    if ($sourceCopyrightHash -cne $archiveCopyrightHash) { throw 'Archived Rust standard-library copyright report differs from source bytes.' }
    Write-Host "Native release archive verified: $target; both third-party notice files match source bytes."
}
finally { Remove-Item -LiteralPath $root -Recurse -Force }
