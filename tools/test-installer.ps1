#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)] [string]$Version,
    [string]$AssetDirectory = (Join-Path $PSScriptRoot '../target/packages'),
    [switch]$RunIntegration
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'installer-common.ps1')
$names = @(Assert-InstallerChecksum $AssetDirectory $Version)
if (-not $RunIntegration) { Write-Host 'Installer checksum verified.'; return }
# Live installation is restricted to disposable GitHub-hosted Windows runners.
if (-not $IsWindows -or $env:GITHUB_ACTIONS -cne 'true' -or $env:RUNNER_ENVIRONMENT -cne 'github-hosted') {
    throw 'Integration requires a disposable GitHub-hosted Windows runner.'
}
$defaultRoot = Join-Path $env:ProgramFiles 'SUPER-SHRINE/Ond'
$customRoot = Join-Path $env:ProgramFiles 'Ond Installer Test/Custom Ond'
$noPathRoot = Join-Path $env:ProgramFiles 'Ond Installer Test/No;Path'
$invalidRoot = Join-Path $env:ProgramFiles 'Ond Installer Test/Invalid;Path'
$uninstallKey = 'SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{2C11CC72-539E-4EAA-A57B-CA7C5E4F94BB}_is1'
$environmentKey = 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment'
$registry = [Microsoft.Win32.RegistryKey]::OpenBaseKey('LocalMachine', 'Registry64')
$environment = $registry.OpenSubKey($environmentKey, $true)
$original = $environment.GetValue('Path', $null, 'DoNotExpandEnvironmentNames')
$originalKind = $environment.GetValueKind('Path')
$kind = [Microsoft.Win32.RegistryValueKind]::ExpandString
foreach ($directory in @($defaultRoot, $customRoot, $noPathRoot, $invalidRoot)) {
    if (Test-Path -LiteralPath $directory) { throw "Runner already has directory: $directory" }
}
if ($registry.OpenSubKey($uninstallKey)) { throw 'Runner already has Ond installed.' }
$setup = (Resolve-Path -LiteralPath (Join-Path $AssetDirectory $names[0])).Path
$quiet = '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /LANG=english'
function Invoke-CheckedInstaller([string]$Path, [string]$Arguments) {
    $process = Start-Process -FilePath $Path -ArgumentList $Arguments -WindowStyle Hidden -PassThru -Wait
    if ($process.ExitCode -ne 0) { throw "Installer exit code: $($process.ExitCode)" }
    if ([IO.Path]::GetFileName($Path) -ieq 'unins000.exe') {
        for ($attempt = 0; $attempt -lt 50 -and (Test-Path -LiteralPath $Path); $attempt++) { Start-Sleep -Milliseconds 100 }
        if (Test-Path -LiteralPath $Path) { throw 'Uninstaller executable remains.' }
    }
}
function Assert-RejectedSetup([string]$Arguments) {
    $process = Start-Process -FilePath $setup -ArgumentList "$quiet $Arguments" -WindowStyle Hidden -PassThru -Wait
    if ($process.ExitCode -eq 0) { throw "Setup unexpectedly accepted: $Arguments" }
}
function Read-SystemPath { $environment.GetValue('Path', $null, 'DoNotExpandEnvironmentNames') }
function Get-InstallationSnapshot([string]$Directory) {
    $files = @(Get-ChildItem -LiteralPath $Directory -File -Recurse | Sort-Object FullName | ForEach-Object {
        [ordered]@{ name = $_.FullName; hash = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash }
    })
    $registration = $registry.OpenSubKey($uninstallKey)
    try {
        $values = @($registration.GetValueNames() | Sort-Object | ForEach-Object {
            [ordered]@{ name = $_; kind = $registration.GetValueKind($_).ToString(); value = $registration.GetValue($_, $null, 'DoNotExpandEnvironmentNames') }
        })
    }
    finally { $registration.Dispose() }
    ConvertTo-Json -Depth 8 -Compress -InputObject ([ordered]@{ files = $files; registration = $values; path = (Read-SystemPath); pathKind = $environment.GetValueKind('Path').ToString() })
}
function Assert-LockedUpdatePreservesInstallation([string]$Directory, [string]$Binary, [string]$Arguments) {
    $before = Get-InstallationSnapshot $Directory
    $lock = [IO.File]::Open((Join-Path $Directory $Binary), 'Open', 'Read', 'None')
    try { Assert-RejectedSetup $Arguments }
    finally { $lock.Dispose() }
    if ((Get-InstallationSnapshot $Directory) -cne $before) { throw 'Rejected locked update changed files, registration, or SYSTEM PATH.' }
}
function Assert-Installed([string]$Directory, [bool]$OnPath = $true) {
    foreach ($file in @('ond.exe', 'ond-lsp.exe', 'LICENSE.md', 'THIRD-PARTY-NOTICES.txt', 'COPYRIGHT-library.html')) {
        if (-not (Test-Path -LiteralPath (Join-Path $Directory $file))) { throw "Missing installed file: $file" }
    }
    $registration = $registry.OpenSubKey($uninstallKey)
    try {
        if ($registration.GetValue('InstallLocation').TrimEnd('\') -ine $Directory) { throw 'Installation location mismatch.' }
    }
    finally { $registration.Dispose() }
    if ($OnPath) {
        $tokens = @((Read-SystemPath).Split(';') | Where-Object { $_ -ieq $Directory })
        if ($tokens.Count -ne 1 -or $environment.GetValueKind('Path') -ne $kind) { throw 'SYSTEM PATH contract failed.' }
    }
}
function Uninstall-Ond([string]$Directory) {
    Invoke-CheckedInstaller (Join-Path $Directory 'unins000.exe') '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART'
    if ($registry.OpenSubKey($uninstallKey)) { throw 'Uninstall registration remains.' }
}
try {
    $fixture = $original + ';%SystemRoot%\OndInstallerTest;;' + $defaultRoot + '-other'
    $environment.SetValue('Path', $fixture, $kind)
    Invoke-CheckedInstaller $setup $quiet
    Assert-Installed $defaultRoot
    Assert-LockedUpdatePreservesInstallation $defaultRoot 'ond.exe' ''
    Invoke-CheckedInstaller $setup $quiet
    Assert-Installed $defaultRoot
    # Invalid PATH choices must fail before removing the existing installation.
    Assert-RejectedSetup "/DIR=`"$invalidRoot`" /TASKS=systempath"
    Assert-Installed $defaultRoot
    # Move the installation, removing only the old owned PATH token.
    Assert-LockedUpdatePreservesInstallation $defaultRoot 'ond-lsp.exe' "/DIR=`"$customRoot`""
    if (Test-Path -LiteralPath $customRoot) { throw 'Rejected locked move created its destination.' }
    Invoke-CheckedInstaller $setup "$quiet /DIR=`"$customRoot`""
    Assert-Installed $customRoot
    if (Test-Path -LiteralPath (Join-Path $defaultRoot 'ond.exe')) { throw 'Old installation remains after move.' }
    if (@((Read-SystemPath).Split(';') | Where-Object { $_ -ieq $defaultRoot }).Count) { throw 'Old PATH entry remains after move.' }
    Uninstall-Ond $customRoot
    if ((Read-SystemPath) -cne $fixture) { throw 'Uninstall did not restore PATH.' }
    # Keep a pre-existing case-insensitive exact token and the registry value type.
    $kind = [Microsoft.Win32.RegistryValueKind]::String
    $preexisting = $fixture + ';' + $customRoot.ToUpperInvariant()
    $environment.SetValue('Path', $preexisting, $kind)
    Invoke-CheckedInstaller $setup "$quiet /DIR=`"$customRoot`""
    Assert-Installed $customRoot
    Uninstall-Ond $customRoot
    if ((Read-SystemPath) -cne $preexisting) { throw 'Uninstall removed a pre-existing PATH entry.' }
    # A PATH-disabled custom directory may contain semicolons; update restores that choice.
    $environment.SetValue('Path', $fixture, $kind)
    Invoke-CheckedInstaller $setup "$quiet /DIR=`"$noPathRoot`" /TASKS=`"`""
    Assert-Installed $noPathRoot $false
    Invoke-CheckedInstaller $setup $quiet
    Assert-Installed $noPathRoot $false
    $registration = $registry.OpenSubKey($uninstallKey)
    try { if ($registration.GetValue('OndPathOwned') -ne 0) { throw 'PATH-disabled update recorded ownership.' } }
    finally { $registration.Dispose() }
    if ((Read-SystemPath) -cne $fixture -or $environment.GetValueKind('Path') -ne $kind) { throw 'PATH-disabled setup changed PATH.' }
    Uninstall-Ond $noPathRoot
    if ((Read-SystemPath) -cne $fixture) { throw 'PATH-disabled uninstall changed PATH.' }
    # Existing user files remain; known unregistered binaries must not be overwritten.
    New-Item -ItemType Directory -Force -Path $customRoot | Out-Null
    $sentinel = Join-Path $customRoot 'user-file.txt'
    'keep' | Set-Content -LiteralPath $sentinel
    $binary = Join-Path $customRoot 'ond.exe'
    'unregistered' | Set-Content -LiteralPath $binary
    Assert-RejectedSetup "/DIR=`"$customRoot`""
    if ((Get-Content -LiteralPath $binary) -cne 'unregistered') { throw 'Unregistered binary was overwritten.' }
    Remove-Item -LiteralPath $binary
    # A broken HKLM registration must not launch arbitrary commands.
    $registration = $registry.CreateSubKey($uninstallKey)
    try {
        $registration.SetValue('InstallLocation', $customRoot)
        $registration.SetValue('UninstallString', 'C:\Windows\System32\cmd.exe /c exit 0')
    }
    finally { $registration.Dispose() }
    Assert-RejectedSetup "/DIR=`"$customRoot`""
    $registry.DeleteSubKeyTree($uninstallKey)
    if (-not (Test-Path -LiteralPath $sentinel)) { throw 'User file was removed.' }
    # A normal existing directory is allowed; unrelated files survive uninstall.
    Invoke-CheckedInstaller $setup "$quiet /DIR=`"$customRoot`""
    Assert-Installed $customRoot
    Uninstall-Ond $customRoot
    if (-not (Test-Path -LiteralPath $sentinel)) { throw 'Uninstall removed user data.' }
    Remove-Item -LiteralPath $sentinel
    Remove-Item -LiteralPath $customRoot
    Write-Host 'Installer lifecycle and optional PATH verified.'
}
finally {
    $environment.SetValue('Path', $original, $originalKind)
    $environment.Dispose()
    $registry.Dispose()
}
