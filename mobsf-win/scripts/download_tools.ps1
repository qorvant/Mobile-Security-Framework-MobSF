# Download the external reverse-engineering tools (jadx, apktool) into
# ./tools so that MobSF-Win's decompile feature works. The binaries are
# git-ignored (see ../../.gitignore) and only needed locally.
#
# Requires: PowerShell 7+ and network access to github.com.
# jadx/apktool are Java apps, so a JRE (java on PATH) is also required.

$ErrorActionPreference = 'Stop'

$root = Resolve-Path (Join-Path $PSScriptRoot '..')
$tools = Join-Path $root 'tools'
New-Item -ItemType Directory -Force -Path $tools | Out-Null

function Get-GitHubAssetUrl {
    param([string] $repo, [string] $pattern)
    $rel = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/latest" `
        -Headers @{ 'User-Agent' = 'mobsf-win' }
    $asset = $rel.assets `
        | Where-Object { $_.name -match $pattern } `
        | Select-Object -First 1
    if (-not $asset) {
        throw "No asset matching '$pattern' found in latest release of $repo"
    }
    return $asset.browser_download_url
}

# --- jadx (Java/Kotlin source decompiler) -----------------------------------
Write-Host '[1/2] Downloading jadx ...'
$jadxUrl = Get-GitHubAssetUrl 'skylot/jadx' '^jadx-\d.*\.zip$'
$zip = Join-Path $tools 'jadx-bin.zip'
Invoke-WebRequest -Uri $jadxUrl -OutFile $zip

$tmp = Join-Path $tools 'jadx-tmp'
Expand-Archive -Path $zip -DestinationPath $tmp -Force
$inner = Get-ChildItem -Path $tmp -Directory | Select-Object -First 1
Copy-Item -Path (Join-Path $inner.FullName '*') `
    -Destination (Join-Path $tools 'jadx') -Recurse -Force
Remove-Item -Path $tmp -Recurse -Force
Remove-Item -Path $zip -Force
$jadxBat = Join-Path $tools 'jadx\bin\jadx.bat'
if (-not (Test-Path $jadxBat)) { throw "jadx did not extract as expected ($jadxBat missing)" }
Write-Host "      jadx -> $jadxBat"

# --- apktool (resources + smali) --------------------------------------------
Write-Host '[2/2] Downloading apktool ...'
$apkUrl = Get-GitHubAssetUrl 'ibotpeaches/apktool' 'apktool.*\.jar$'
$apkJar = Join-Path $tools 'apktool.jar'
Invoke-WebRequest -Uri $apkUrl -OutFile $apkJar
Write-Host "      apktool -> $apkJar"

Write-Host ''
Write-Host "Done. Tools installed under: $tools"
Write-Host 'Note: both tools require Java (javac/java) on PATH.'
