#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidateSet("Init", "Status", "Acquire", "Complete", "Recover")]
    [string]$Action,

    [string]$RunId,

    [ValidateSet("observed", "fixed", "no-issue", "blocked", "failed")]
    [string]$Outcome = "no-issue",

    [string]$Summary = "",

    [switch]$Force
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Write-Utf8NoBom {
    param([string]$Path, [string]$Content)
    [System.IO.File]::WriteAllText($Path, $Content, [System.Text.UTF8Encoding]::new($false))
}

function Write-Json {
    param([string]$Path, [object]$Value)
    Write-Utf8NoBom -Path $Path -Content (($Value | ConvertTo-Json -Depth 8) + "`n")
}

function Get-BootMarker {
    [DateTimeOffset]::UtcNow.AddMilliseconds(-[Environment]::TickCount64)
}

function Initialize-State {
    foreach ($name in @("queue", "active", "blocked", "archive", "runs")) {
        [System.IO.Directory]::CreateDirectory((Join-Path $script:StateRoot $name)) | Out-Null
    }
    if (-not (Test-Path -LiteralPath $script:StatePath)) {
        $state = [PSCustomObject]@{
            schema_version             = 1
            mode                       = "observe"
            observation_runs_remaining = 3
            last_completed_at_utc      = $null
            last_completed_commit      = $null
            last_outcome               = $null
            consecutive_failures       = 0
        }
        Write-Json -Path $script:StatePath -Value $state
    }
}

function Test-StaleLock {
    param([object]$Lock)

    $expired = [DateTimeOffset]::UtcNow -gt [DateTimeOffset]::Parse($Lock.lease_until_utc)
    $oldBoot = [DateTimeOffset]::Parse($Lock.boot_marker_utc)
    $bootChanged = [Math]::Abs(((Get-BootMarker) - $oldBoot).TotalMinutes) -gt 2
    $expired -or $bootChanged
}

function Archive-StaleLock {
    param([object]$Lock)

    $stamp = [DateTimeOffset]::Now.ToString("yyyyMMdd-HHmmss")
    $destination = Join-Path (Join-Path $script:StateRoot "runs") "$stamp-stale-lock.json"
    Move-Item -LiteralPath $script:LockPath -Destination $destination
    Write-Host "Archived stale lock for run $($Lock.run_id) at $destination."
}

$gitRoot = & git rev-parse --show-toplevel 2>$null
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($gitRoot)) {
    throw "tools/quality-loop.ps1 must be run inside the Ond Git repository."
}
$repoRoot = [System.IO.Path]::GetFullPath($gitRoot.Trim())
$script:StateRoot = Join-Path $repoRoot ".ond-quality"
$script:StatePath = Join-Path $script:StateRoot "state.json"
$script:LockPath = Join-Path $script:StateRoot "loop.lock"

Initialize-State

switch ($Action) {
    "Init" {
        Write-Host "Initialized local quality-loop state at $script:StateRoot."
    }
    "Status" {
        $state = Get-Content -Raw -LiteralPath $script:StatePath | ConvertFrom-Json
        $lock = if (Test-Path -LiteralPath $script:LockPath) {
            Get-Content -Raw -LiteralPath $script:LockPath | ConvertFrom-Json
        } else { $null }
        [PSCustomObject]@{
            state   = $state
            lock    = $lock
            queued  = @(Get-ChildItem (Join-Path $script:StateRoot "queue") -File -Filter "*.md").Count
            active  = @(Get-ChildItem (Join-Path $script:StateRoot "active") -File -Filter "*.md").Count
            blocked = @(Get-ChildItem (Join-Path $script:StateRoot "blocked") -File -Filter "*.md").Count
            archived = @(Get-ChildItem (Join-Path $script:StateRoot "archive") -File -Recurse -Filter "*.md").Count
        } | ConvertTo-Json -Depth 8
    }
    "Recover" {
        if (-not (Test-Path -LiteralPath $script:LockPath)) {
            Write-Host "No interrupted run was found."
            break
        }
        $lock = Get-Content -Raw -LiteralPath $script:LockPath | ConvertFrom-Json
        if (-not $Force -and -not (Test-StaleLock -Lock $lock)) {
            throw "Run $($lock.run_id) still has a valid lease until $($lock.lease_until_utc)."
        }
        Archive-StaleLock -Lock $lock
        Write-Host "Inspect .ond-quality/active and git status before acquiring a new run."
    }
    "Acquire" {
        if (Test-Path -LiteralPath $script:LockPath) {
            $lock = Get-Content -Raw -LiteralPath $script:LockPath | ConvertFrom-Json
            if (Test-StaleLock -Lock $lock) {
                Archive-StaleLock -Lock $lock
            }
            else {
                throw "Quality loop is already leased by run $($lock.run_id) until $($lock.lease_until_utc)."
            }
        }

        if ([string]::IsNullOrWhiteSpace($RunId)) {
            $RunId = [DateTimeOffset]::Now.ToString("yyyyMMdd-HHmmss") + "-" + [Guid]::NewGuid().ToString("N").Substring(0, 6)
        }
        $now = [DateTimeOffset]::UtcNow
        $lock = [PSCustomObject]@{
            schema_version   = 1
            run_id           = $RunId
            started_at_utc   = $now.ToString("o")
            lease_until_utc  = $now.AddHours(6).ToString("o")
            boot_marker_utc  = (Get-BootMarker).ToString("o")
            host             = [Environment]::MachineName
            base_commit      = (& git -C $repoRoot rev-parse HEAD).Trim()
        }

        $json = ($lock | ConvertTo-Json -Depth 5) + "`n"
        $bytes = [System.Text.UTF8Encoding]::new($false).GetBytes($json)
        $stream = [System.IO.File]::Open($script:LockPath, [System.IO.FileMode]::CreateNew, [System.IO.FileAccess]::Write, [System.IO.FileShare]::None)
        try { $stream.Write($bytes, 0, $bytes.Length) } finally { $stream.Dispose() }

        $runRoot = Join-Path (Join-Path $script:StateRoot "runs") $RunId
        [System.IO.Directory]::CreateDirectory($runRoot) | Out-Null
        Write-Json -Path (Join-Path $runRoot "run.json") -Value $lock
        Write-Output $RunId
    }
    "Complete" {
        if ([string]::IsNullOrWhiteSpace($RunId)) {
            throw "Complete requires -RunId."
        }
        if (-not (Test-Path -LiteralPath $script:LockPath)) {
            throw "No active loop lock exists."
        }
        $lock = Get-Content -Raw -LiteralPath $script:LockPath | ConvertFrom-Json
        if ($lock.run_id -ne $RunId) {
            throw "RunId does not own the active loop lock."
        }

        $state = Get-Content -Raw -LiteralPath $script:StatePath | ConvertFrom-Json
        if ($state.mode -eq "observe" -and $Outcome -in @("observed", "no-issue")) {
            $state.observation_runs_remaining = [Math]::Max(0, [int]$state.observation_runs_remaining - 1)
            if ($state.observation_runs_remaining -eq 0) {
                $state.mode = "guarded-fix"
            }
        }
        if ($Outcome -eq "failed") {
            $state.consecutive_failures = [int]$state.consecutive_failures + 1
        }
        else {
            $state.consecutive_failures = 0
        }
        $state.last_completed_at_utc = [DateTimeOffset]::UtcNow.ToString("o")
        $state.last_completed_commit = (& git -C $repoRoot rev-parse HEAD).Trim()
        $state.last_outcome = $Outcome
        Write-Json -Path $script:StatePath -Value $state

        $completion = [PSCustomObject]@{
            schema_version  = 1
            run_id          = $RunId
            outcome         = $Outcome
            summary         = $Summary
            finished_at_utc = [DateTimeOffset]::UtcNow.ToString("o")
            head_commit     = $state.last_completed_commit
        }
        $runRoot = Join-Path (Join-Path $script:StateRoot "runs") $RunId
        [System.IO.Directory]::CreateDirectory($runRoot) | Out-Null
        Write-Json -Path (Join-Path $runRoot "completion.json") -Value $completion
        Remove-Item -LiteralPath $script:LockPath
        Write-Host "Completed quality-loop run $RunId with outcome $Outcome."
    }
}
