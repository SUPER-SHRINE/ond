#requires -Version 7.0

param(
    [string] $Python,
    [switch] $Serve
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$website = Join-Path $repo 'website'
$targetRoot = Join-Path $repo 'target/website'
$sources = Join-Path $targetRoot 'sources'
$public = Join-Path $targetRoot 'public'

foreach ($path in @($targetRoot, $sources, $public)) {
    $resolved = [IO.Path]::GetFullPath($path)
    if (-not $resolved.StartsWith([IO.Path]::GetFullPath($targetRoot), [StringComparison]::OrdinalIgnoreCase)) {
        throw "生成先がtarget/websiteの外です: $resolved"
    }
}

if (-not $Python) {
    if ($IsWindows -or $env:OS -eq 'Windows_NT') {
        $Python = Join-Path $targetRoot 'venv/Scripts/python.exe'
    } else {
        $Python = Join-Path $targetRoot 'venv/bin/python'
    }
}
if (-not (Test-Path -LiteralPath $Python -PathType Leaf)) {
    throw "Pythonがありません: $Python`n先に次を実行してください: python -m venv target/website/venv; target/website/venv/Scripts/python -m pip install -r website/requirements.txt"
}
$Python = (Resolve-Path -LiteralPath $Python).Path
$check = & $Python -m zensical --version 2>&1
if ($LASTEXITCODE -ne 0) {
    throw "Zensicalが利用できません。`n必要な準備: `"$Python`" -m pip install -r website/requirements.txt`n$check"
}

$commit = (& git -C $repo rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) { throw 'GitのHEADを取得できません。' }
if (Test-Path -LiteralPath $sources) { Remove-Item -LiteralPath $sources -Recurse -Force }
New-Item -ItemType Directory -Path $sources -Force | Out-Null
if (Test-Path -LiteralPath $public) { Remove-Item -LiteralPath $public -Recurse -Force }
Copy-Item -LiteralPath (Join-Path $website 'index.md') -Destination (Join-Path $sources 'index.md')
Copy-Item -LiteralPath (Join-Path $website 'ond_markdown.py') -Destination (Join-Path $targetRoot 'ond_markdown.py')
$assets = Join-Path $website 'assets'
if (Test-Path -LiteralPath $assets -PathType Container) {
    Copy-Item -LiteralPath $assets -Destination (Join-Path $sources 'assets') -Recurse
}

$included = @()
$included += (& git -C $repo ls-files --cached --others --exclude-standard 'specs/**/*.md' 'specs/*.md' 'docs/releases/**/*.md' 'docs/releases/*.md' 'docs/manual/**/*.md' 'docs/manual/*.md')
foreach ($relative in $included) {
    if (-not $relative) { continue }
    $destination = Join-Path $sources $relative
    New-Item -ItemType Directory -Path (Split-Path $destination) -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $repo $relative) -Destination $destination
}

# 文書にある収集対象外のrepository file参照は、実在するGit blob/treeへ向ける。
$markdownLink = [regex]'(?<open>\]\()(?<url>[^\s\)]+)'
$gitHubBase = "https://github.com/SUPER-SHRINE/ond/blob/$commit/"
$treeBase = "https://github.com/SUPER-SHRINE/ond/tree/$commit/"
Get-ChildItem -LiteralPath $sources -Recurse -File -Filter '*.md' | ForEach-Object {
    $file = $_
    $relative = [IO.Path]::GetRelativePath($sources, $file.FullName).Replace('\', '/')
    $original = Join-Path $repo $relative
    $text = [IO.File]::ReadAllText($file.FullName)
    $text = $markdownLink.Replace($text, [System.Text.RegularExpressions.MatchEvaluator]{
        param($match)
        $url = $match.Groups['url'].Value
        if ($url -match '^(https?:|mailto:|#|data:)') { return $match.Value }
        $suffix = ''
        $path = $url
        $hash = $path.IndexOf('#')
        if ($hash -ge 0) { $suffix = $path.Substring($hash); $path = $path.Substring(0, $hash) }
        $query = $path.IndexOf('?')
        if ($query -ge 0) { $path = $path.Substring(0, $query) }
        $decoded = [Uri]::UnescapeDataString($path.Replace('/', [IO.Path]::DirectorySeparatorChar))
        $sourceResolved = if (-not $path) { $file.FullName } else { [IO.Path]::GetFullPath((Join-Path (Split-Path $file.FullName) $decoded)) }
        if ($sourceResolved.StartsWith($sources + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase) -and (Test-Path -LiteralPath $sourceResolved)) { return $match.Value }
        $resolved = if (-not $path) { $original } else { [IO.Path]::GetFullPath((Join-Path (Split-Path $original) $decoded)) }
        if (-not $resolved.StartsWith($repo, [StringComparison]::OrdinalIgnoreCase)) { return $match.Value }
        if (Test-Path -LiteralPath $resolved) {
            $repoRelative = [IO.Path]::GetRelativePath($repo, $resolved).Replace('\', '/')
            $encoded = (($repoRelative -split '/') | ForEach-Object { [Uri]::EscapeDataString($_) }) -join '/'
            $base = if ((Get-Item -LiteralPath $resolved).PSIsContainer) { $treeBase } else { $gitHubBase }
            return "$($match.Groups['open'].Value)$base$encoded$suffix"
        }
        $repoRelative = [IO.Path]::GetRelativePath($repo, $resolved).Replace('\', '/')
        if ($repoRelative -eq 'specs/ond/build/test-build.md' -and $relative -eq 'docs/releases/0.3.0-notes.md') {
            return "$($match.Groups['open'].Value)https://github.com/SUPER-SHRINE/ond/blob/0.3.0/$repoRelative$suffix"
        }
        return $match.Value
    })
    [IO.File]::WriteAllText($file.FullName, $text, [Text.UTF8Encoding]::new($false))
}

# Zensical requires docs_dir and site_dir to remain inside the project root.
# Copy the canonical config into target/website, where its relative paths resolve.
$buildConfig = Join-Path $targetRoot 'zensical.toml'
$configText = [IO.File]::ReadAllText((Join-Path $website 'zensical.toml'))
[IO.File]::WriteAllText($buildConfig, $configText, [Text.UTF8Encoding]::new($false))
Push-Location $repo
$oldPythonPath = $env:PYTHONPATH
$oldNoBytecode = $env:PYTHONDONTWRITEBYTECODE
$env:PYTHONPATH = if ($oldPythonPath) { "$targetRoot$([IO.Path]::PathSeparator)$oldPythonPath" } else { $targetRoot }
$env:PYTHONDONTWRITEBYTECODE = '1'
try {
    & $Python website/ond_markdown.py
    if ($LASTEXITCODE -ne 0) { throw "Markdownの入れ子list検査に失敗しました ($LASTEXITCODE)" }
    & $Python -m zensical build --config-file target/website/zensical.toml --clean --strict
    if ($LASTEXITCODE -ne 0) { throw "Zensical buildに失敗しました ($LASTEXITCODE)" }
    & $Python tools/check-docs-links.py target/website/public
    if ($LASTEXITCODE -ne 0) { throw "生成HTMLのリンク検査に失敗しました ($LASTEXITCODE)" }
} finally {
    $env:PYTHONPATH = $oldPythonPath
    $env:PYTHONDONTWRITEBYTECODE = $oldNoBytecode
    Pop-Location
}

if ($Serve) {
    & $Python -m http.server 8000 --bind 127.0.0.1 --directory $public
    if ($LASTEXITCODE -ne 0) { throw "ローカル配信に失敗しました ($LASTEXITCODE)" }
}


