#requires -Version 7.0
[CmdletBinding()]
param(
    [ValidateSet("Debug", "Release")]
    [string]$Profile = "Debug",

    [switch]$SkipBuild
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Invoke-Checked {
    param(
        [Parameter(Mandatory)] [string]$FileName,
        [Parameter(Mandatory)] [string[]]$Arguments
    )

    & $FileName @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$FileName failed with exit code $LASTEXITCODE."
    }
}

$repoRoot = (& git rev-parse --show-toplevel 2>$null).Trim()
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($repoRoot)) {
    throw "tools/release-smoke.ps1 must be run inside the Ond Git repository."
}
$repoRoot = [System.IO.Path]::GetFullPath($repoRoot)
$cargoProfile = $Profile.ToLowerInvariant()
$binaryDirectory = Join-Path (Join-Path $repoRoot "target") $cargoProfile
$executableSuffix = if ($IsWindows) { ".exe" } else { "" }
$ond = Join-Path $binaryDirectory "ond$executableSuffix"
$ondLsp = Join-Path $binaryDirectory "ond-lsp$executableSuffix"

if (-not $SkipBuild) {
    $buildArguments = @("build", "--workspace", "--locked")
    if ($Profile -eq "Release") {
        $buildArguments += "--release"
    }
    Invoke-Checked -FileName "cargo" -Arguments $buildArguments
}

foreach ($binary in @($ond, $ondLsp)) {
    if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
        throw "Missing release smoke-test binary: $binary"
    }
}

$tempBase = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
$tempRoot = Join-Path $tempBase ("ond-release-smoke-" + [Guid]::NewGuid().ToString("N"))
$project = Join-Path $tempRoot "hello"
[System.IO.Directory]::CreateDirectory($project) | Out-Null

$previousOndPath = [Environment]::GetEnvironmentVariable("OND_PATH", "Process")
try {
    Get-ChildItem -LiteralPath (Join-Path $repoRoot "examples/hello") | Copy-Item -Destination $project -Recurse
    [Environment]::SetEnvironmentVariable("OND_PATH", $null, "Process")

    Invoke-Checked -FileName $ond -Arguments @("compile", $project)
    $manifest = Join-Path $project "target/build.ondbuild"
    if (-not (Test-Path -LiteralPath $manifest -PathType Leaf)) {
        throw "Compile smoke test did not create $manifest."
    }

    $image = Join-Path $tempRoot "hello.ondimage"
    Invoke-Checked -FileName $ond -Arguments @(
        "link", $manifest,
        "--ram", "4096:65536",
        "--stack", "8192",
        "--return-to", "0",
        "-o", $image
    )
    if (-not (Test-Path -LiteralPath $image -PathType Leaf) -or (Get-Item -LiteralPath $image).Length -eq 0) {
        throw "Link smoke test did not create a non-empty LinkedImage."
    }

    $initialize = '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'
    $shutdown = '{"jsonrpc":"2.0","id":2,"method":"shutdown","params":{}}'
    $exit = '{"jsonrpc":"2.0","method":"exit"}'
    $input = @($initialize, $shutdown, $exit) | ForEach-Object {
        "Content-Length: $([System.Text.Encoding]::UTF8.GetByteCount($_))`r`n`r`n$_"
    }
    $input = $input -join ""

    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $ondLsp
    $startInfo.UseShellExecute = $false
    $startInfo.RedirectStandardInput = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    if (-not $process.Start()) {
        throw "Failed to start ond-lsp."
    }
    $process.StandardInput.Write($input)
    $process.StandardInput.Close()
    $stdout = $process.StandardOutput.ReadToEnd()
    $stderr = $process.StandardError.ReadToEnd()
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) {
        throw "ond-lsp smoke test failed with exit code $($process.ExitCode): $stderr"
    }
    if ($stdout -notmatch '"id":1' -or $stdout -notmatch '"id":2') {
        throw "ond-lsp smoke test did not return initialize and shutdown responses."
    }

    $cargoManifest = Get-Content -Raw -LiteralPath (Join-Path $repoRoot 'Cargo.toml')
    if ($cargoManifest -notmatch '(?ms)^\[workspace\.package\].*?^version\s*=\s*"([^"]+)"') { throw 'Workspace version is missing.' }
    $version = $Matches[1]
    $responses = [regex]::Matches($stdout, 'Content-Length: ([0-9]+)\r\n\r\n')
    $initialized = $false
    foreach ($frame in $responses) {
        $start = $frame.Index + $frame.Length
        $tail = [Text.Encoding]::UTF8.GetBytes($stdout.Substring($start))
        $payload = [Text.Encoding]::UTF8.GetString($tail, 0, [int]$frame.Groups[1].Value) | ConvertFrom-Json
        if ($payload.id -eq 1) {
            if ($payload.result.serverInfo.version -cne $version) { throw 'Archived build LSP version does not match Cargo.toml.' }
            $initialized = $true
        }
    }
    if (-not $initialized) { throw 'Initialize response is missing.' }
    Write-Host "Release smoke test passed ($Profile, LSP $version)."
}
finally {
    [Environment]::SetEnvironmentVariable("OND_PATH", $previousOndPath, "Process")
    $resolvedTempRoot = [System.IO.Path]::GetFullPath($tempRoot)
    $safePrefix = $tempBase.TrimEnd([System.IO.Path]::DirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
    if ($resolvedTempRoot.StartsWith($safePrefix, [System.StringComparison]::OrdinalIgnoreCase) -and
        [System.IO.Path]::GetFileName($resolvedTempRoot).StartsWith("ond-release-smoke-", [System.StringComparison]::Ordinal)) {
        [System.IO.Directory]::Delete($resolvedTempRoot, $true)
    }
}
