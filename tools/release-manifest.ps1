#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)] [ValidateSet('Generate', 'Verify')] [string]$Mode,
    [Parameter(Mandatory)] [string]$Version,
    [Parameter(Mandatory)] [string]$Commit,
    [Parameter(Mandatory)] [string]$AssetDirectory
)
. (Join-Path $PSScriptRoot 'release-manifest-common.ps1')
$repoRoot = Split-Path $PSScriptRoot -Parent
Get-ReleaseIdentity $repoRoot $Version $Commit
$path = Join-Path $AssetDirectory "ond-$Version-manifest.json"
$entries = if ($Mode -eq 'Generate') {
    $descriptors = @(Get-ChildItem -LiteralPath $AssetDirectory -Filter '*-descriptor.json' -File)
    if ($descriptors.Count -ne 2) { throw 'Exactly two target descriptors are required.' }
    foreach ($descriptor in $descriptors) {
        $value = Read-ReleaseJson $descriptor.FullName
        Assert-ReleaseEnvelope $value $Version $Commit 1
        $value.targets[0]
    }
}
else {
    $value = Read-ReleaseJson $path
    Assert-ReleaseEnvelope $value $Version $Commit 2
    $value.targets
}
$targets = @($entries | ForEach-Object { Get-VerifiedReleaseTarget $_ $AssetDirectory $Version } | Sort-Object { $_.target })
if ($targets[0].target -cne 'linux-x86_64' -or $targets[1].target -cne 'windows-x86_64') { throw 'Both release targets are required exactly once.' }
$manifest = Get-ReleaseEnvelope $Version $Commit $targets
if ($Mode -eq 'Generate') { Write-ReleaseJson $path $manifest }
elseif ([Convert]::ToBase64String([IO.File]::ReadAllBytes($path)) -cne [Convert]::ToBase64String([Text.UTF8Encoding]::new($false).GetBytes((ConvertTo-ReleaseJson $manifest)))) {
    throw 'Release manifest is not canonical.'
}
Write-Host "Release manifest $Mode passed: $path"
