#requires -Version 7.0
# 実binaryや公開APIを使わず、固定fixtureでproducer契約を検査する。
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'release-manifest-common.ps1')
$repoRoot = Split-Path $PSScriptRoot -Parent
$cargo = Get-Content -Raw (Join-Path $repoRoot 'Cargo.toml')
if ($cargo -notmatch '(?ms)^\[workspace\.package\].*?^version\s*=\s*"([^"]+)"') { throw 'Missing workspace version.' }
$version = $Matches[1]
$commit = (& git -C $repoRoot rev-parse HEAD).Trim()
$root = Join-Path ([IO.Path]::GetTempPath()) ("ond-manifest-test-" + [Guid]::NewGuid().ToString('N'))
$assets = Join-Path $root 'assets'
$staging = Join-Path $root 'staging'
New-Item -ItemType Directory -Path $assets, $staging | Out-Null
$script:testCount = 0
function Assert-Rejected {
    param([string]$Name, [scriptblock]$Action)
    $rejected = $false
    try { & $Action | Out-Null } catch { $rejected = $true }
    if (-not $rejected) { throw "Fixture was not rejected: $Name" }
    $script:testCount++
    Write-Host "Passed rejection: $Name"
}
function Invoke-Manifest {
    param([string]$Mode = 'Generate', [string]$Version = $version, [string]$Commit = $commit)
    & (Join-Path $PSScriptRoot 'release-manifest.ps1') -Mode $Mode -Version $Version -Commit $Commit -AssetDirectory $assets
}
function Test-DescriptorMutation {
    param([string]$Name, [scriptblock]$Mutation)
    $path = Join-Path $assets "ond-$version-linux-x86_64-descriptor.json"
    $backup = [IO.File]::ReadAllBytes($path)
    try {
        $value = Read-ReleaseJson $path
        & $Mutation $value
        [IO.File]::WriteAllText($path, (ConvertTo-ReleaseJson $value), [Text.UTF8Encoding]::new($false))
        Assert-Rejected $Name { Invoke-Manifest }
    }
    finally { [IO.File]::WriteAllBytes($path, $backup) }
}
$noticePath = 'THIRD-PARTY-NOTICES.txt'
$copyrightPath = 'COPYRIGHT-library.html'
function New-FixtureArchive {
    param([string]$Archive, [string]$Target, [string[]]$Paths)
    if (Test-Path -LiteralPath $Archive) { Remove-Item -LiteralPath $Archive }
    if ($Target -eq 'linux-x86_64') {
        & tar -czf $Archive -C $staging -- @Paths
        if ($LASTEXITCODE -ne 0) { throw 'Fixture tar failed.' }
    }
    else {
        $zip = [IO.Compression.ZipFile]::Open($Archive, [IO.Compression.ZipArchiveMode]::Create)
        try {
            foreach ($path in $Paths) {
                [IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, (Join-Path $staging $path), $path) | Out-Null
            }
        }
        finally { $zip.Dispose() }
    }
}
try {
    foreach ($target in @('linux-x86_64', 'windows-x86_64')) {
        $linux = $target -eq 'linux-x86_64'
        $suffix = if ($linux) { '' } else { '.exe' }
        $extension = if ($linux) { 'tar.gz' } else { 'zip' }
        $legacyPaths = @("ond$suffix", "ond-lsp$suffix", 'README.md', 'CHANGELOG.md', 'LICENSE.md')
        $paths = $legacyPaths + @($noticePath, $copyrightPath)
        foreach ($name in $paths + @('unexpected.md', 'machine-roadmap.md', 'docs/design/extra.md', 'docs/design/README.md')) {
            $file = Join-Path $staging $name
            New-Item -ItemType Directory -Force -Path (Split-Path $file -Parent) | Out-Null
            if ($name -ceq $noticePath -or $name -ceq $copyrightPath) {
                Copy-Item -LiteralPath (Join-Path $repoRoot $name) -Destination $file
            }
            else { [IO.File]::WriteAllText($file, "fixture-$name`n", [Text.UTF8Encoding]::new($false)) }
        }
        $badArchive = Join-Path $root "negative-$target.$extension"
        New-FixtureArchive $badArchive $target $legacyPaths
        $legacyHash = Get-ArchiveBinaryHash $badArchive $target "ond-lsp$suffix" '0.1.1'
        if ($legacyHash -cne (Get-FileHash (Join-Path $staging "ond-lsp$suffix") -Algorithm SHA256).Hash.ToLowerInvariant()) { throw 'Legacy LSP hash mismatch.' }
        $script:testCount++
        Assert-Rejected "$target new release without third-party notices" { Get-ArchiveBinaryHash $badArchive $target "ond-lsp$suffix" $version }
        New-FixtureArchive $badArchive $target ($paths | Where-Object { $_ -cne $noticePath })
        Assert-Rejected "$target new release missing one third-party notice" { Get-ArchiveBinaryHash $badArchive $target "ond-lsp$suffix" $version }
        New-FixtureArchive $badArchive $target ($paths | Where-Object { $_ -cne $copyrightPath })
        Assert-Rejected "$target new release without Rust copyright report" { Get-ArchiveBinaryHash $badArchive $target "ond-lsp$suffix" $version }
        foreach ($notice in @($noticePath, $copyrightPath)) {
            $noticeFixture = Join-Path $staging $notice
            $noticeBackup = [IO.File]::ReadAllBytes($noticeFixture)
            try {
                [IO.File]::WriteAllText($noticeFixture, "altered notice fixture`n", [Text.UTF8Encoding]::new($false))
                New-FixtureArchive $badArchive $target $paths
                Assert-Rejected "$target altered $notice contents" { Get-ArchiveBinaryHash $badArchive $target "ond-lsp$suffix" $version }
                [IO.File]::WriteAllBytes($noticeFixture, [byte[]]@())
                New-FixtureArchive $badArchive $target $paths
                Assert-Rejected "$target empty $notice contents" { Get-ArchiveBinaryHash $badArchive $target "ond-lsp$suffix" $version }
            }
            finally { [IO.File]::WriteAllBytes($noticeFixture, $noticeBackup) }
        }
        foreach ($extra in @('unexpected.md', 'docs/design/extra.md', 'docs/design/README.md')) {
            New-FixtureArchive $badArchive $target ($paths + $extra)
            Assert-Rejected "$target extra or duplicate $extra" { Get-ArchiveBinaryHash $badArchive $target "ond-lsp$suffix" $version }
        }
        New-FixtureArchive $badArchive $target $paths
        Assert-Rejected "$target legacy with extra docs" { Get-ArchiveBinaryHash $badArchive $target "ond-lsp$suffix" '0.1.1' }
        $archive = Join-Path $assets "ond-$version-$target.$extension"
        New-FixtureArchive $archive $target $paths
        $sha = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
        [IO.File]::WriteAllText("$archive.sha256", "$sha  $([IO.Path]::GetFileName($archive))`n", [Text.UTF8Encoding]::new($false))
        $entry = [ordered]@{
            archive = [ordered]@{ name = [IO.Path]::GetFileName($archive); sha256 = $sha; size = (Get-Item $archive).Length }
            binary = [ordered]@{ name = "ond-lsp$suffix"; sha256 = (Get-FileHash -LiteralPath (Join-Path $staging "ond-lsp$suffix") -Algorithm SHA256).Hash.ToLowerInvariant() }
            rustTarget = $(if ($linux) { 'x86_64-unknown-linux-gnu' } else { 'x86_64-pc-windows-msvc' })
            target = $target
        }
        Write-ReleaseJson (Join-Path $assets "ond-$version-$target-descriptor.json") (Get-ReleaseEnvelope $version $commit @($entry))
    }
    Invoke-Manifest
    Invoke-Manifest 'Verify'
    $manifestPath = Join-Path $assets "ond-$version-manifest.json"
    $first = [IO.File]::ReadAllBytes($manifestPath)
    Remove-Item -LiteralPath $manifestPath
    Invoke-Manifest
    if ([Convert]::ToBase64String($first) -cne [Convert]::ToBase64String([IO.File]::ReadAllBytes($manifestPath))) { throw 'Manifest generation is not deterministic.' }
    $script:testCount++
    Test-DescriptorMutation 'unknown root key' { param($v) $v['unexpected'] = 1 }
    Test-DescriptorMutation 'unknown nested key' { param($v) $v.targets[0].binary['unexpected'] = 1 }
    Test-DescriptorMutation 'missing key' { param($v) $v.targets[0].archive.Remove('size') }
    Test-DescriptorMutation 'unknown target' { param($v) $v.targets[0].target = 'macos-x86_64' }
    Test-DescriptorMutation 'duplicate target' { param($v) $v.targets[0] = (Read-ReleaseJson (Join-Path $assets "ond-$version-windows-x86_64-descriptor.json")).targets[0] }
    Test-DescriptorMutation 'wrong archive SHA' { param($v) $v.targets[0].archive.sha256 = '0' * 64 }
    Test-DescriptorMutation 'wrong binary SHA' { param($v) $v.targets[0].binary.sha256 = '0' * 64 }
    Test-DescriptorMutation 'wrong size' { param($v) $v.targets[0].archive.size++ }
    Test-DescriptorMutation 'noninteger size' { param($v) $v.targets[0].archive.size = 1.5 }
    Test-DescriptorMutation 'wrong version' { param($v) $v.version = '9.9.9' }
    Test-DescriptorMutation 'wrong commit' { param($v) $v.commit = '0' * 40 }
    Test-DescriptorMutation 'wrong repository' { param($v) $v.repository = 'example/ond' }
    Assert-Rejected 'version does not match Cargo' { Invoke-Manifest -Version '9.9.9' }
    Assert-Rejected 'commit does not match Git' { Invoke-Manifest -Commit ('0' * 40) }
    Assert-Rejected 'leading zero version' { Invoke-Manifest -Version '00.1.2' }
    $descriptor = Join-Path $assets "ond-$version-linux-x86_64-descriptor.json"
    $backup = [IO.File]::ReadAllBytes($descriptor)
    try {
        [IO.File]::WriteAllText($descriptor, '{"commit":"x","commit":"y"}')
        Assert-Rejected 'duplicate JSON key' { Invoke-Manifest }
        Remove-Item $descriptor
        Assert-Rejected 'missing descriptor' { Invoke-Manifest }
    }
    finally { [IO.File]::WriteAllBytes($descriptor, $backup) }
    [IO.File]::WriteAllText($manifestPath, ([IO.File]::ReadAllText($manifestPath)).Replace('"commit":', '"commit" :'))
    Assert-Rejected 'noncanonical manifest' { Invoke-Manifest 'Verify' }
    Assert-Rejected 'different existing manifest' { Invoke-Manifest }
    [IO.File]::WriteAllBytes($manifestPath, $first)
    # 公開scriptのghをモックし、ネットワークも公開も行わず再実行契約を検査する。
    $global:OndReleaseMock = @{
        assets = $assets; names = @((Read-ReleaseJson $manifestPath).targets | ForEach-Object { $_.archive.name; "$($_.archive.name).sha256" }) + "ond-$version-manifest.json"
        version = $version; commit = $commit; uploads = [Collections.Generic.List[string]]::new(); missing = @(); corrupt = $false
    }
    function global:gh {
        param([Parameter(ValueFromRemainingArguments)] [string[]]$Arguments)
        $global:LASTEXITCODE = 0
        $mock = $global:OndReleaseMock
        if ($Arguments[0] -eq 'api') {
            if ($Arguments[1] -match '/git/ref/tags/') { return (@{ object = @{ type = 'commit'; sha = $mock.commit } } | ConvertTo-Json -Depth 4 -Compress) }
            return (@{ tag_name = $mock.version; assets = @($mock.names | Where-Object { $_ -cnotin $mock.missing } | ForEach-Object { @{ name = $_ } }) } | ConvertTo-Json -Depth 4 -Compress)
        }
        if ($Arguments[0] -eq 'release' -and $Arguments[1] -eq 'download') {
            $name = $Arguments[([array]::IndexOf($Arguments, '--pattern') + 1)]
            $directory = $Arguments[([array]::IndexOf($Arguments, '--dir') + 1)]
            Copy-Item -LiteralPath (Join-Path $mock.assets $name) -Destination $directory
            if ($mock.corrupt) { [IO.File]::AppendAllText((Join-Path $directory $name), 'changed') }
            return
        }
        if ($Arguments[0] -eq 'release' -and $Arguments[1] -eq 'upload') { $mock.uploads.Add([IO.Path]::GetFileName($Arguments[3])); return }
        throw 'Unexpected mocked gh invocation.'
    }
    try {
        & (Join-Path $PSScriptRoot 'publish-release.ps1') -Version $version -Commit $commit -AssetDirectory $assets
        if ($global:OndReleaseMock.uploads.Count -ne 0) { throw 'Identical assets must not be uploaded.' }
        $script:testCount++
        $global:OndReleaseMock.corrupt = $true
        Assert-Rejected 'existing asset differs' { & (Join-Path $PSScriptRoot 'publish-release.ps1') -Version $version -Commit $commit -AssetDirectory $assets }
        if ($global:OndReleaseMock.uploads.Count -ne 0) { throw 'Different assets must stop before any upload.' }
        $global:OndReleaseMock.corrupt = $false
        $global:OndReleaseMock.missing = @("ond-$version-manifest.json")
        & (Join-Path $PSScriptRoot 'publish-release.ps1') -Version $version -Commit $commit -AssetDirectory $assets
        if ($global:OndReleaseMock.uploads.Count -ne 1 -or $global:OndReleaseMock.uploads[0] -cne "ond-$version-manifest.json") { throw 'Only missing assets may be uploaded.' }
        $script:testCount++
    }
    finally { Remove-Item Function:/gh; Remove-Variable OndReleaseMock -Scope Global }
    $checksum = Join-Path $assets "ond-$version-linux-x86_64.tar.gz.sha256"
    [IO.File]::WriteAllText($checksum, "$('0' * 64)  ond-$version-linux-x86_64.tar.gz`n")
    Assert-Rejected 'wrong sidecar SHA' { Invoke-Manifest 'Verify' }
    Write-Host "Release manifest fixture tests passed ($script:testCount checks)."
}
finally { Remove-Item -LiteralPath $root -Recurse -Force }
