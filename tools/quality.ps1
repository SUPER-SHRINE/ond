#requires -Version 7.0
[CmdletBinding()]
param(
    [ValidateSet("Quick", "Full")]
    [string]$Profile = "Full",

    [switch]$UpdateClippyBaseline
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Invoke-CapturedProcess {
    param(
        [Parameter(Mandatory)] [string]$FileName,
        [Parameter(Mandatory)] [string[]]$Arguments,
        [Parameter(Mandatory)] [string]$WorkingDirectory
    )

    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $FileName
    $startInfo.WorkingDirectory = $WorkingDirectory
    $startInfo.UseShellExecute = $false
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    foreach ($argument in $Arguments) {
        [void]$startInfo.ArgumentList.Add($argument)
    }

    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    if (-not $process.Start()) {
        throw "Failed to start $FileName."
    }

    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    $process.WaitForExit()

    [PSCustomObject]@{
        ExitCode = $process.ExitCode
        Stdout   = $stdoutTask.GetAwaiter().GetResult()
        Stderr   = $stderrTask.GetAwaiter().GetResult()
    }
}

function Write-Utf8NoBom {
    param(
        [Parameter(Mandatory)] [string]$Path,
        [Parameter(Mandatory)] [AllowEmptyString()] [string]$Content
    )

    [System.IO.File]::WriteAllText($Path, $Content, [System.Text.UTF8Encoding]::new($false))
}

function Invoke-QualityStep {
    param(
        [Parameter(Mandatory)] [string]$Name,
        [Parameter(Mandatory)] [string[]]$Arguments,
        [Parameter(Mandatory)] [string]$LogPath,
        [switch]$QuietOutput,
        [string]$FileName = "cargo"
    )

    Write-Host "==> $Name"
    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
    $result = Invoke-CapturedProcess -FileName $FileName -Arguments $Arguments -WorkingDirectory $script:RepoRoot
    $combined = @($result.Stdout, $result.Stderr) -join ""
    Write-Utf8NoBom -Path $LogPath -Content $combined
    if (-not $QuietOutput -and $combined.Length -gt 0) {
        Write-Host $combined.TrimEnd()
    }
    elseif ($QuietOutput -and $result.Stderr.Length -gt 0) {
        Write-Host $result.Stderr.TrimEnd()
    }

    $stopwatch.Stop()
    $step = [PSCustomObject]@{
        name             = $Name
        exit_code        = $result.ExitCode
        duration_seconds = [Math]::Round($stopwatch.Elapsed.TotalSeconds, 3)
        log              = [System.IO.Path]::GetRelativePath($script:RepoRoot, $LogPath).Replace("\", "/")
    }
    $script:Steps.Add($step)

    if ($result.ExitCode -ne 0) {
        throw "$Name failed with exit code $($result.ExitCode)."
    }

    return $result
}

function Get-ClippyWarnings {
    param([Parameter(Mandatory)] [string]$JsonLines)

    $warnings = foreach ($line in ($JsonLines -split "`r?`n")) {
        if ([string]::IsNullOrWhiteSpace($line)) {
            continue
        }
        try {
            $item = $line | ConvertFrom-Json -ErrorAction Stop
        }
        catch {
            continue
        }
        if ($item.reason -ne "compiler-message" -or $item.message.level -ne "warning") {
            continue
        }

        $code = if ($null -ne $item.message.code) { [string]$item.message.code.code } else { "rustc" }
        $span = @($item.message.spans | Where-Object { $_.is_primary }) | Select-Object -First 1
        if ($null -eq $span) {
            $span = @($item.message.spans) | Select-Object -First 1
        }
        $file = if ($null -ne $span) { [string]$span.file_name } else { "<unknown>" }
        $file = $file.Replace("\", "/")
        $message = ([string]$item.message.message).Trim()

        [PSCustomObject]@{
            code    = $code
            file    = $file
            message = $message
            key     = "$code|$file|$message"
        }
    }

    @($warnings)
}

$gitRoot = & git rev-parse --show-toplevel 2>$null
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($gitRoot)) {
    throw "tools/quality.ps1 must be run inside the Ond Git repository."
}
$script:RepoRoot = [System.IO.Path]::GetFullPath($gitRoot.Trim())
$stateRoot = Join-Path $script:RepoRoot ".ond-quality"
$runId = [DateTimeOffset]::Now.ToString("yyyyMMdd-HHmmss")
$runRoot = Join-Path (Join-Path $stateRoot "runs") "$runId-quality"
[System.IO.Directory]::CreateDirectory($runRoot) | Out-Null
$script:Steps = [System.Collections.Generic.List[object]]::new()
$summaryPath = Join-Path $runRoot "summary.json"
$succeeded = $false

try {
    Invoke-QualityStep -Name "release-manifest-fixtures" -FileName "pwsh" -Arguments @("-NoProfile", "-File", (Join-Path $script:RepoRoot "tools/test-release-manifest.ps1")) -LogPath (Join-Path $runRoot "release-manifest.log") | Out-Null
    Invoke-QualityStep -Name "cargo-fmt" -Arguments @("fmt", "--all", "--", "--check") -LogPath (Join-Path $runRoot "fmt.log") | Out-Null
    Invoke-QualityStep -Name "cargo-test" -Arguments @("test", "--workspace", "--locked", "--quiet") -LogPath (Join-Path $runRoot "test.log") | Out-Null

    if ($Profile -eq "Full") {
        $clippyResult = Invoke-QualityStep -Name "cargo-clippy" -Arguments @(
            "clippy", "--workspace", "--all-targets", "--locked", "--message-format=json"
        ) -LogPath (Join-Path $runRoot "clippy.log") -QuietOutput
        $warnings = Get-ClippyWarnings -JsonLines $clippyResult.Stdout
        $groups = @($warnings | Group-Object key | Sort-Object Name)
        $current = @($groups | ForEach-Object {
            $sample = $_.Group[0]
            [PSCustomObject]@{
                code    = $sample.code
                file    = $sample.file
                message = $sample.message
                count   = $_.Count
            }
        })

        $baselinePath = Join-Path $script:RepoRoot "tools\clippy-baseline.json"
        if ($UpdateClippyBaseline) {
            $baseline = [PSCustomObject]@{
                schema_version   = 1
                generated_at_utc = [DateTimeOffset]::UtcNow.ToString("o")
                rustc            = (& rustc --version).Trim()
                warnings         = $current
            }
            Write-Utf8NoBom -Path $baselinePath -Content (($baseline | ConvertTo-Json -Depth 6) + "`n")
            Write-Host "Updated tools/clippy-baseline.json with $($warnings.Count) warning occurrence(s)."
        }
        else {
            if (-not (Test-Path -LiteralPath $baselinePath)) {
                throw "Missing tools/clippy-baseline.json. Create it only through an approved baseline update."
            }
            $baseline = Get-Content -Raw -LiteralPath $baselinePath | ConvertFrom-Json
            $allowed = @{}
            foreach ($warning in @($baseline.warnings)) {
                $key = "$($warning.code)|$($warning.file)|$($warning.message)"
                $allowed[$key] = [int]$warning.count
            }

            $newWarnings = @()
            foreach ($group in $groups) {
                $allowedCount = if ($allowed.ContainsKey($group.Name)) { $allowed[$group.Name] } else { 0 }
                if ($group.Count -gt $allowedCount) {
                    $newWarnings += [PSCustomObject]@{
                        key       = $group.Name
                        added     = $group.Count - $allowedCount
                        locations = @($group.Group | ForEach-Object { $_.file })
                    }
                }
            }
            if ($newWarnings.Count -gt 0) {
                Write-Host ($newWarnings | ConvertTo-Json -Depth 5)
                throw "Clippy produced $($newWarnings.Count) new warning fingerprint(s) beyond the approved baseline."
            }
            Write-Host "Clippy baseline check passed ($($warnings.Count) known warning occurrence(s))."
        }
    }

    $succeeded = $true
}
finally {
    $summary = [PSCustomObject]@{
        schema_version = 1
        run_id         = $runId
        profile        = $Profile
        started_at     = $runId
        finished_at_utc = [DateTimeOffset]::UtcNow.ToString("o")
        succeeded      = $succeeded
        head           = (& git -C $script:RepoRoot rev-parse HEAD).Trim()
        steps          = $script:Steps
    }
    Write-Utf8NoBom -Path $summaryPath -Content (($summary | ConvertTo-Json -Depth 8) + "`n")
    Write-Host "Quality summary: $summaryPath"
}

if (-not $succeeded) {
    exit 1
}
