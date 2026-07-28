#Requires -Version 5.1
$ErrorActionPreference = 'Stop'

$Repo        = if ($env:STATUSLINE_REPO)        { $env:STATUSLINE_REPO }        else { 'Darkwing4/statusline-rs-cc' }
$InstallDir  = if ($env:STATUSLINE_INSTALL_DIR) { $env:STATUSLINE_INSTALL_DIR } else { Join-Path $env:USERPROFILE '.claude\bin' }
$Settings    = if ($env:STATUSLINE_SETTINGS)    { $env:STATUSLINE_SETTINGS }    else { Join-Path $env:USERPROFILE '.claude\settings.json' }
$Tag         = if ($env:STATUSLINE_TAG)         { $env:STATUSLINE_TAG }         else { 'latest' }
$SkipSettings = [bool]$env:STATUSLINE_SKIP_SETTINGS

[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$arch = $env:PROCESSOR_ARCHITECTURE
if ($env:PROCESSOR_ARCHITEW6432) { $arch = $env:PROCESSOR_ARCHITEW6432 }
switch ($arch.ToUpper()) {
    'AMD64' { $target = 'x86_64-pc-windows-msvc' }
    'ARM64' { $target = 'aarch64-pc-windows-msvc' }
    default { throw "unsupported architecture: $arch" }
}

$asset = "statusline-$target.zip"

if ($Tag -eq 'latest') {
    $apiUrl = "https://api.github.com/repos/$Repo/releases/latest"
} else {
    $apiUrl = "https://api.github.com/repos/$Repo/releases/tags/$Tag"
}

Write-Host "fetching release metadata: $apiUrl"
$headers = @{ 'Accept' = 'application/vnd.github+json'; 'User-Agent' = 'statusline-installer' }
$release = Invoke-RestMethod -Uri $apiUrl -Headers $headers

$assetObj = $release.assets | Where-Object { $_.name -eq $asset } | Select-Object -First 1
if (-not $assetObj) {
    throw "asset $asset not found in release ($apiUrl)"
}
$checksumAsset = "$asset.sha256"
$checksumAssetObj = $release.assets | Where-Object { $_.name -eq $checksumAsset } | Select-Object -First 1
if (-not $checksumAssetObj) {
    throw "asset $checksumAsset not found in release ($apiUrl)"
}

$tmp = New-Item -ItemType Directory -Path (Join-Path $env:TEMP ("statusline-" + [guid]::NewGuid().ToString('N')))
try {
    $zipPath = Join-Path $tmp $asset
    $checksumPath = Join-Path $tmp $checksumAsset
    Write-Host "downloading $asset"
    Invoke-WebRequest -Uri $assetObj.browser_download_url -OutFile $zipPath -UseBasicParsing -Headers @{ 'User-Agent' = 'statusline-installer' }
    Write-Host "downloading $checksumAsset"
    Invoke-WebRequest -Uri $checksumAssetObj.browser_download_url -OutFile $checksumPath -UseBasicParsing -Headers @{ 'User-Agent' = 'statusline-installer' }

    $checksumText = [System.IO.File]::ReadAllText($checksumPath)
    $checksumPattern = '\A(?<hash>[0-9A-Fa-f]{64})[ \t]+\*?' + [regex]::Escape($asset) + '(?:\r\n|\n)?\z'
    $checksumMatch = [regex]::Match($checksumText, $checksumPattern, [System.Text.RegularExpressions.RegexOptions]::CultureInvariant)
    if (-not $checksumMatch.Success) {
        throw "invalid checksum file ${checksumAsset}: expected exactly one sha256sum line for $asset"
    }

    $expectedHash = $checksumMatch.Groups['hash'].Value
    $actualHash = (Get-FileHash -LiteralPath $zipPath -Algorithm SHA256).Hash
    if (-not [string]::Equals($actualHash, $expectedHash, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "checksum mismatch for $asset"
    }

    Write-Host "verified $asset"
    Expand-Archive -Path $zipPath -DestinationPath $tmp -Force

    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }
    $dest = Join-Path $InstallDir 'statusline.exe'
    Move-Item -Path (Join-Path $tmp 'statusline.exe') -Destination $dest -Force

    Write-Host ""
    Write-Host "installed: $dest"
} finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}

$cmdPath = $dest -replace '\\', '/'

function Write-Snippet {
    Write-Host ""
    Write-Host "add this to $Settings manually:"
    Write-Host ""
    $snippet = [ordered]@{
        statusLine = [ordered]@{
            type    = 'command'
            command = $cmdPath
        }
    } | ConvertTo-Json -Depth 5
    Write-Host $snippet
}

if ($SkipSettings) {
    Write-Snippet
    exit 0
}

$settingsDir = Split-Path -Parent $Settings
if ($settingsDir -and -not (Test-Path $settingsDir)) {
    New-Item -ItemType Directory -Path $settingsDir -Force | Out-Null
}

$obj = $null
if (Test-Path $Settings) {
    try {
        $raw = Get-Content -Path $Settings -Raw -ErrorAction Stop
        if ($raw -and $raw.Trim()) {
            $obj = $raw | ConvertFrom-Json -ErrorAction Stop
        }
    } catch {
        Write-Warning "could not parse ${Settings}: $($_.Exception.Message)"
        Write-Warning "leaving $Settings untouched"
        Write-Snippet
        exit 0
    }
}
if (-not $obj) { $obj = [PSCustomObject]@{} }

$prev = $null
if ($obj.PSObject.Properties.Name -contains 'statusLine' -and $obj.statusLine) {
    if ($obj.statusLine.PSObject.Properties.Name -contains 'command') {
        $prev = $obj.statusLine.command
    }
}

if ($prev -eq $cmdPath) {
    Write-Host "${Settings}: statusLine already points at $cmdPath"
    exit 0
}

if (Test-Path $Settings) {
    Copy-Item -Path $Settings -Destination ($Settings + '.bak') -Force
    Write-Host "backup written: $($Settings).bak"
}

$newStatusLine = [PSCustomObject]@{
    type    = 'command'
    command = $cmdPath
}

if ($obj.PSObject.Properties.Name -contains 'statusLine') {
    $obj.statusLine = $newStatusLine
} else {
    $obj | Add-Member -MemberType NoteProperty -Name statusLine -Value $newStatusLine
}

$json = $obj | ConvertTo-Json -Depth 64

$fullPath = if (Test-Path $Settings) {
    (Resolve-Path $Settings).Path
} else {
    [System.IO.Path]::GetFullPath($Settings)
}
$utf8NoBom = New-Object System.Text.UTF8Encoding $false
[System.IO.File]::WriteAllText($fullPath, $json + [Environment]::NewLine, $utf8NoBom)

if ($prev) {
    Write-Host "replaced previous statusLine command: $prev"
}
Write-Host "updated: $Settings"
Write-Host ""
Write-Host "done. statusline will refresh on the next Claude Code event."
