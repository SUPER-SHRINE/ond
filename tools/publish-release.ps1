#requires -Version 7.0
# CI専用。候補assetを検証し、同一hashのassetだけ再利用する。
[CmdletBinding()]
param(
    [Parameter(Mandatory)] [string]$Version,
    [Parameter(Mandatory)] [string]$Commit,
    [Parameter(Mandatory)] [string]$AssetDirectory
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'release-manifest-common.ps1')
$repository = 'SUPER-SHRINE/ond'
& (Join-Path $PSScriptRoot 'release-manifest.ps1') -Mode Verify -Version $Version -Commit $Commit -AssetDirectory $AssetDirectory
$manifest = Read-ReleaseJson (Join-Path $AssetDirectory "ond-$Version-manifest.json")
$names = @($manifest.targets | ForEach-Object { $_.archive.name; "$($_.archive.name).sha256" }) + "ond-$Version-manifest.json"
$notes = Join-Path (Split-Path $PSScriptRoot -Parent) "docs/releases/$Version-notes.md"
if (-not (Test-Path -LiteralPath $notes -PathType Leaf)) { throw 'Release notes are missing.' }

function Invoke-ReleaseApi {
    param([string]$Endpoint)
    $json = & gh api $Endpoint
    if ($LASTEXITCODE -ne 0) { throw "GitHub API failed: $Endpoint" }
    return ($json -join "`n") | ConvertFrom-Json -AsHashtable
}
$ref = Invoke-ReleaseApi "repos/$repository/git/ref/tags/$Version"
$object = $ref.object
while ($object.type -ceq 'tag') { $object = (Invoke-ReleaseApi "repos/$repository/git/tags/$($object.sha)").object }
if ($object.type -cne 'commit' -or $object.sha -cne $Commit) { throw 'Remote release tag does not match candidate commit.' }

$errorPath = Join-Path ([IO.Path]::GetTempPath()) ("ond-release-error-" + [Guid]::NewGuid().ToString('N'))
$downloadRoot = Join-Path ([IO.Path]::GetTempPath()) ("ond-release-assets-" + [Guid]::NewGuid().ToString('N'))
try {
    $json = & gh api "repos/$repository/releases/tags/$Version" 2> $errorPath
    $lookupExit = $LASTEXITCODE
    if ($lookupExit -ne 0) {
        if ([IO.File]::ReadAllText($errorPath) -notmatch '\(HTTP 404\)') { throw 'Could not inspect existing GitHub Release.' }
        & gh release create $Version --repo $repository --verify-tag --notes-file $notes --title "Ond $Version"
        if ($LASTEXITCODE -ne 0) { throw 'Could not create GitHub Release.' }
        $release = Invoke-ReleaseApi "repos/$repository/releases/tags/$Version"
    }
    else { $release = ($json -join "`n") | ConvertFrom-Json -AsHashtable }
    if ($release.tag_name -cne $Version) { throw 'GitHub Release tag mismatch.' }
    New-Item -ItemType Directory -Path $downloadRoot | Out-Null
    $missing = [Collections.Generic.List[string]]::new()
    # 既存assetをすべて照合してから不足分を追加する。差分assetの置換は禁止。
    foreach ($name in $names) {
        $existing = @($release.assets | Where-Object { $_.name -ceq $name })
        if ($existing.Count -gt 1) { throw "Duplicate existing release asset: $name" }
        if ($existing.Count -eq 0) { $missing.Add($name); continue }
        & gh release download $Version --repo $repository --pattern $name --dir $downloadRoot
        if ($LASTEXITCODE -ne 0) { throw "Could not download existing asset: $name" }
        $candidateHash = (Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $AssetDirectory $name)).Hash
        $existingHash = (Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $downloadRoot $name)).Hash
        if ($existingHash -cne $candidateHash) { throw "Existing release asset has a different SHA-256: $name" }
        Write-Host "Same SHA-256; keep existing asset: $name"
    }
    foreach ($name in $missing) {
        & gh release upload $Version (Join-Path $AssetDirectory $name) --repo $repository
        if ($LASTEXITCODE -ne 0) { throw "Could not upload missing asset: $name" }
    }
}
finally {
    if (Test-Path -LiteralPath $errorPath) { Remove-Item -LiteralPath $errorPath -Force }
    if (Test-Path -LiteralPath $downloadRoot) { Remove-Item -LiteralPath $downloadRoot -Recurse -Force }
}
