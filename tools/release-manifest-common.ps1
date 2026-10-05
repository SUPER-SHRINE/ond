#requires -Version 7.0
# 配布物の検証とcanonical JSONを生成する共通処理。公開asset IDは含めない。
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-ReleaseIdentity {
    param([string]$RepoRoot, [string]$Version, [string]$Commit)
    if ($Version -cnotmatch '^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$') { throw 'Invalid release version.' }
    if ($Commit -cnotmatch '^[0-9a-f]{40}$') { throw 'Invalid release commit.' }
    $cargo = Get-Content -Raw -LiteralPath (Join-Path $RepoRoot 'Cargo.toml')
    if ($cargo -notmatch '(?ms)^\[workspace\.package\]\s*\n(?:(?!^\[).)*?^version\s*=\s*"([^"]+)"' -or $Matches[1] -cne $Version) {
        throw 'Release version does not match Cargo.toml.'
    }
    $head = & git -C $RepoRoot rev-parse HEAD
    if ($LASTEXITCODE -ne 0 -or $head.Trim() -cne $Commit) { throw 'Release commit does not match Git HEAD.' }
}

function Read-ReleaseJson {
    param([string]$Path)
    $text = [IO.File]::ReadAllText($Path, [Text.UTF8Encoding]::new($false, $true))
    $document = [System.Text.Json.JsonDocument]::Parse($text)
    try {
        Test-UniqueJsonKeys $document.RootElement
    }
    finally { $document.Dispose() }
    return ConvertFrom-Json -AsHashtable -InputObject $text
}

function Test-UniqueJsonKeys {
    param([System.Text.Json.JsonElement]$Element)
    if ($Element.ValueKind -eq [System.Text.Json.JsonValueKind]::Object) {
        $keys = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
        foreach ($property in $Element.EnumerateObject()) {
            if (-not $keys.Add($property.Name)) { throw "Duplicate JSON key: $($property.Name)" }
            Test-UniqueJsonKeys $property.Value
        }
    }
    elseif ($Element.ValueKind -eq [System.Text.Json.JsonValueKind]::Array) {
        foreach ($item in $Element.EnumerateArray()) { Test-UniqueJsonKeys $item }
    }
}

function Assert-ReleaseKeys {
    param($Value, [string[]]$Keys)
    if ($Value -isnot [Collections.IDictionary] -or $Value.Count -ne $Keys.Count) { throw 'Invalid release object keys.' }
    foreach ($key in $Keys) { if (-not $Value.Contains($key)) { throw "Missing release key: $key" } }
    foreach ($key in $Value.Keys) { if ($key -cnotin $Keys) { throw "Unknown release key: $key" } }
}

function Get-StreamSha256 {
    param([IO.Stream]$Stream)
    $algorithm = [Security.Cryptography.SHA256]::Create()
    try { return [BitConverter]::ToString($algorithm.ComputeHash($Stream)).Replace('-', '').ToLowerInvariant() }
    finally { $algorithm.Dispose() }
}

function Get-ReleaseArchivePaths {
    param([string]$Target, [string]$Version)
    $suffix = if ($Target -ceq 'linux-x86_64') { '' } else { '.exe' }
    @('CHANGELOG.md', 'LICENSE.md', 'README.md', "ond-lsp$suffix", "ond$suffix")
    # 公開済み0.1.1の5ファイル契約は変更しない。
    if ([version]$Version -ge [version]'0.1.3') { 'THIRD-PARTY-NOTICES.txt'; 'COPYRIGHT-library.html' }
}

function Assert-ThirdPartyNoticeInventory {
    param([string]$RepoRoot)
    $noticePath = Join-Path $RepoRoot 'THIRD-PARTY-NOTICES.txt'
    $copyrightPath = Join-Path $RepoRoot 'COPYRIGHT-library.html'
    foreach ($required in @($noticePath, $copyrightPath)) {
        if (-not (Test-Path -LiteralPath $required -PathType Leaf)) { throw "Missing required third-party notice file: $required" }
    }
    if ((Get-FileHash -Algorithm SHA256 -LiteralPath $noticePath).Hash.ToLowerInvariant() -cne
        'e73e809e1592c0fd90e83319943686c2df75dd24ea5bc2bf5d91756e10a67cc4') {
        throw 'Third-party notice contents changed; review the legal text and explicitly refresh its pinned SHA-256.'
    }
    if ((Get-FileHash -Algorithm SHA256 -LiteralPath $copyrightPath).Hash.ToLowerInvariant() -cne
        '3aa41caccecaeddad6fcf2f36ce14146ab7baae57064b05b12ecc6b52d5e917f') {
        throw 'Rust standard-library copyright report differs from the verified Rust 1.91.1 source.'
    }
    $lock = [IO.File]::ReadAllText((Join-Path $RepoRoot 'Cargo.lock'))
    $notice = [IO.File]::ReadAllText($noticePath)
    $blocks = [regex]::Matches($lock, '(?ms)^\[\[package\]\]\s*\n(?<body>.*?)(?=^\[\[package\]\]|\z)')
    $expectedCount = 0
    foreach ($block in $blocks) {
        $body = $block.Groups['body'].Value
        if ($body -notmatch '(?m)^name\s*=\s*"([^"]+)"' -or $Matches[1] -eq '') { throw 'Invalid Cargo.lock package name.' }
        $name = $Matches[1]
        if ($body -notmatch '(?m)^version\s*=\s*"([^"]+)"' -or $Matches[1] -eq '') { throw 'Invalid Cargo.lock package version.' }
        $version = $Matches[1]
        if ($body -notmatch '(?m)^source\s*=\s*"registry\+[^"\r\n]+"') { continue }
        if ($body -notmatch '(?m)^checksum\s*=\s*"([0-9a-f]{64})"') { throw "Registry package lacks a lock checksum: $name $version" }
        $checksum = $Matches[1]
        $record = "PACKAGE: $name $version`nLOCK CHECKSUM (SHA-256): $checksum`nDECLARED LICENSE: "
        if (-not $notice.Contains($record)) { throw "Third-party notice inventory is stale or missing: $name $version" }
        $expectedCount++
    }
    $actualCount = [regex]::Matches($notice, '(?m)^PACKAGE: ').Count
    if ($actualCount -ne $expectedCount) { throw 'Third-party notice inventory has unknown or duplicate registry packages.' }
}

function Get-ArchiveEntrySha256 {
    param([string]$Archive, [string]$Target, [string]$EntryName)
    if ($Target -ceq 'windows-x86_64') {
        $zip = [IO.Compression.ZipFile]::OpenRead($Archive)
        try {
            $entry = $zip.GetEntry($EntryName)
            if ($null -eq $entry) { throw "Missing ZIP entry: $EntryName" }
            $stream = $entry.Open()
            try { return (Get-StreamSha256 $stream) }
            finally { $stream.Dispose() }
        }
        finally { $zip.Dispose() }
    }
    $start = [Diagnostics.ProcessStartInfo]::new('tar')
    $start.UseShellExecute = $false
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    foreach ($argument in @('-xOzf', $Archive, '--', $EntryName)) { [void]$start.ArgumentList.Add($argument) }
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $start
    try {
        if (-not $process.Start()) { throw "Could not read archived entry: $EntryName" }
        $stderr = $process.StandardError.ReadToEndAsync()
        $hash = Get-StreamSha256 $process.StandardOutput.BaseStream
        $process.WaitForExit()
        if ($process.ExitCode -ne 0) { throw "Could not read archived entry $EntryName`: $($stderr.GetAwaiter().GetResult())" }
        return $hash
    }
    finally { $process.Dispose() }
}

function Assert-ArchiveNoticeHashes {
    param([string]$Archive, [string]$Target, [string]$Version)
    if ([version]$Version -lt [version]'0.1.3') { return }
    $expectedNotices = [ordered]@{
        'THIRD-PARTY-NOTICES.txt' = 'e73e809e1592c0fd90e83319943686c2df75dd24ea5bc2bf5d91756e10a67cc4'
        'COPYRIGHT-library.html' = '3aa41caccecaeddad6fcf2f36ce14146ab7baae57064b05b12ecc6b52d5e917f'
    }
    foreach ($name in $expectedNotices.Keys) {
        $expectedHash = $expectedNotices[$name]
        $archiveHash = Get-ArchiveEntrySha256 $Archive $Target $name
        if ($archiveHash -cne $expectedHash) { throw "Archived notice does not match source bytes: $name" }
    }
}

function Get-ArchiveBinaryHash {
    param([string]$Archive, [string]$Target, [string]$BinaryName, [string]$Version)
    $expected = @(Get-ReleaseArchivePaths $Target $Version)
    if ($Target -ceq 'windows-x86_64') {
        $zip = [IO.Compression.ZipFile]::OpenRead($Archive)
        try {
            $entries = @($zip.Entries)
            if ($entries.Count -ne $expected.Count) { throw 'Unexpected ZIP entries.' }
            $seen = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
            foreach ($entry in $entries) {
                if ($entry.FullName -cnotin $expected -or -not $seen.Add($entry.FullName) -or
                    (($entry.ExternalAttributes -shr 16) -band 0xf000) -notin @(0, 0x8000)) {
                    throw 'Invalid ZIP entry.'
                }
            }
            Assert-ArchiveNoticeHashes $Archive $Target $Version
            $stream = $zip.GetEntry($BinaryName).Open()
            try { return (Get-StreamSha256 $stream) }
            finally { $stream.Dispose() }
        }
        finally { $zip.Dispose() }
    }
    $names = @(& tar -tzf $Archive)
    if ($LASTEXITCODE -ne 0 -or $names.Count -ne $expected.Count) { throw 'Invalid tar archive.' }
    $seen = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    foreach ($name in $names) {
        if ($name -cnotin $expected -or -not $seen.Add($name)) { throw 'Invalid tar entry.' }
    }
    $listing = @(& tar -tvzf $Archive)
    if ($LASTEXITCODE -ne 0 -or @($listing | Where-Object { -not $_.StartsWith('-') }).Count -ne 0) { throw 'Tar entries must be regular files.' }
    Assert-ArchiveNoticeHashes $Archive $Target $Version
    $start = [Diagnostics.ProcessStartInfo]::new('tar')
    $start.UseShellExecute = $false
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    foreach ($argument in @('-xOzf', $Archive, '--', $BinaryName)) { [void]$start.ArgumentList.Add($argument) }
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $start
    try {
        if (-not $process.Start()) { throw 'Could not read archived LSP.' }
        $stderr = $process.StandardError.ReadToEndAsync()
        $hash = Get-StreamSha256 $process.StandardOutput.BaseStream
        $process.WaitForExit()
        if ($process.ExitCode -ne 0) { throw "Could not read archived LSP: $($stderr.GetAwaiter().GetResult())" }
        return $hash
    }
    finally { $process.Dispose() }
}

function Get-VerifiedReleaseTarget {
    param($Entry, [string]$AssetDirectory, [string]$Version)
    Assert-ReleaseKeys $Entry @('archive', 'binary', 'rustTarget', 'target')
    Assert-ReleaseKeys $Entry.archive @('name', 'sha256', 'size')
    Assert-ReleaseKeys $Entry.binary @('name', 'sha256')
    foreach ($name in @($Entry.rustTarget, $Entry.binary.name, $Entry.archive.name)) {
        if ($name -isnot [string]) { throw 'Invalid target name type.' }
    }
    if ($Entry.target -isnot [string] -or $Entry.target -cnotin @('linux-x86_64', 'windows-x86_64')) { throw 'Unknown release target.' }
    $linux = $Entry.target -ceq 'linux-x86_64'
    if ([version]$Version -ge [version]'0.1.3') { Assert-ThirdPartyNoticeInventory (Split-Path $PSScriptRoot -Parent) }
    $rustTarget = if ($linux) { 'x86_64-unknown-linux-gnu' } else { 'x86_64-pc-windows-msvc' }
    $binaryName = if ($linux) { 'ond-lsp' } else { 'ond-lsp.exe' }
    $extension = if ($linux) { 'tar.gz' } else { 'zip' }
    $archiveName = "ond-$Version-$($Entry.target).$extension"
    if ($Entry.rustTarget -cne $rustTarget -or $Entry.binary.name -cne $binaryName -or $Entry.archive.name -cne $archiveName) { throw 'Target names do not match release contract.' }
    foreach ($hash in @($Entry.archive.sha256, $Entry.binary.sha256)) {
        if ($hash -isnot [string] -or $hash -cnotmatch '^[0-9a-f]{64}$') { throw 'Invalid release SHA-256.' }
    }
    if (($Entry.archive.size -isnot [long] -and $Entry.archive.size -isnot [int]) -or $Entry.archive.size -le 0) { throw 'Invalid archive size.' }
    $archive = Join-Path $AssetDirectory $archiveName
    $size = (Get-Item -LiteralPath $archive).Length
    $sha = (Get-FileHash -Algorithm SHA256 -LiteralPath $archive).Hash.ToLowerInvariant()
    $checksum = [IO.File]::ReadAllText("$archive.sha256")
    if ($size -ne $Entry.archive.size -or $sha -cne $Entry.archive.sha256 -or $checksum -cnotmatch "\A$sha  $([regex]::Escape($archiveName))\r?\n\z") { throw 'Archive checksum or size mismatch.' }
    $binaryHash = Get-ArchiveBinaryHash $archive $Entry.target $binaryName $Version
    if ($binaryHash -cne $Entry.binary.sha256) { throw 'Archived LSP checksum mismatch.' }
    return [ordered]@{
        archive = [ordered]@{ name = $archiveName; sha256 = $sha; size = $size }
        binary = [ordered]@{ name = $binaryName; sha256 = $binaryHash }
        rustTarget = $rustTarget
        target = $Entry.target
    }
}

function Get-ReleaseEnvelope {
    param([string]$Version, [string]$Commit, [object[]]$Targets)
    return [ordered]@{ commit = $Commit; repository = 'SUPER-SHRINE/ond'; schemaVersion = 1; targets = @($Targets); version = $Version }
}

function Assert-ReleaseEnvelope {
    param($Value, [string]$Version, [string]$Commit, [int]$TargetCount)
    Assert-ReleaseKeys $Value @('commit', 'repository', 'schemaVersion', 'targets', 'version')
    if ($Value.schemaVersion -isnot [long] -and $Value.schemaVersion -isnot [int]) { throw 'Invalid schemaVersion type.' }
    foreach ($key in @('repository', 'version', 'commit')) { if ($Value[$key] -isnot [string]) { throw 'Invalid release identity type.' } }
    if ($Value.schemaVersion -ne 1 -or $Value.repository -cne 'SUPER-SHRINE/ond' -or
        $Value.version -cne $Version -or $Value.commit -cne $Commit -or
        $Value.targets -isnot [array] -or $Value.targets.Count -ne $TargetCount) { throw 'Release identity or targets mismatch.' }
}

function ConvertTo-ReleaseJson {
    param($Value)
    return (ConvertTo-Json -Depth 8 -Compress -InputObject $Value) + "`n"
}

function Write-ReleaseJson {
    param([string]$Path, $Value)
    $bytes = [Text.UTF8Encoding]::new($false).GetBytes((ConvertTo-ReleaseJson $Value))
    if (Test-Path -LiteralPath $Path) {
        if ([Convert]::ToBase64String([IO.File]::ReadAllBytes($Path)) -cne [Convert]::ToBase64String($bytes)) { throw "Output already exists with different contents: $Path" }
        return
    }
    [IO.File]::WriteAllBytes($Path, $bytes)
}
