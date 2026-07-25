param(
    [Parameter(Mandatory = $true)][string]$ExecutablePath,
    [Parameter(Mandatory = $true)][string]$OutputDirectory,
    [Parameter(Mandatory = $true)][string]$WebView2Version,
    [Parameter(Mandatory = $true)][string]$WebView2Url,
    [Parameter(Mandatory = $true)][string]$WebView2Sha256,
    [Parameter(Mandatory = $true)][string]$CommitSha
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Assert-X64Pe {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Label
    )

    $stream = [System.IO.File]::OpenRead($Path)
    $reader = [System.IO.BinaryReader]::new($stream)
    try {
        if ($reader.ReadUInt16() -ne 0x5A4D) {
            throw "$Label is not a Windows PE executable: $Path"
        }
        $stream.Position = 0x3C
        $peOffset = $reader.ReadInt32()
        $stream.Position = $peOffset
        if ($reader.ReadUInt32() -ne 0x00004550) {
            throw "$Label has an invalid PE header: $Path"
        }
        if ($reader.ReadUInt16() -ne 0x8664) {
            throw "$Label is not an x64 executable: $Path"
        }
    } finally {
        $reader.Dispose()
        $stream.Dispose()
    }
}

function Get-LowerSha256 {
    param([Parameter(Mandatory = $true)][string]$Path)
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

if (!(Test-Path -LiteralPath $ExecutablePath -PathType Leaf)) {
    throw "DBX executable is missing: $ExecutablePath"
}
Assert-X64Pe -Path $ExecutablePath -Label "DBX.exe"

$cargoMetadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
$appVersion = ($cargoMetadata.packages | Where-Object { $_.name -eq "dbx" } | Select-Object -First 1).version
if ([string]::IsNullOrWhiteSpace($appVersion)) {
    throw "Unable to read the DBX package version from Cargo metadata."
}

$shortCommit = $CommitSha.Substring(0, [Math]::Min(7, $CommitSha.Length))
$archiveBaseName = "DBX_${appVersion}-sky0458.${shortCommit}_ws2016_x64-fixed-webview2-portable"
$workingRoot = Join-Path $env:RUNNER_TEMP $archiveBaseName
$extractRoot = Join-Path $workingRoot "webview2-extracted"
$packageDir = Join-Path $workingRoot "DBX-portable"
$runtimeDir = Join-Path $packageDir "WebView2Runtime"
$cabName = "Microsoft.WebView2.FixedVersionRuntime.${WebView2Version}.x64.cab"
$cabPath = Join-Path $workingRoot $cabName
$zipPath = Join-Path $OutputDirectory "${archiveBaseName}.zip"
$zipHashPath = "${zipPath}.sha256"

if (Test-Path -LiteralPath $workingRoot) {
    Remove-Item -LiteralPath $workingRoot -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $workingRoot, $extractRoot, $packageDir, $OutputDirectory | Out-Null

Write-Host "Downloading Microsoft WebView2 Fixed Version Runtime $WebView2Version (x64)"
Invoke-WebRequest -UseBasicParsing -Uri $WebView2Url -OutFile $cabPath
$actualCabSha256 = Get-LowerSha256 -Path $cabPath
if ($actualCabSha256 -ne $WebView2Sha256.ToLowerInvariant()) {
    throw "WebView2 CAB SHA-256 mismatch. Expected $WebView2Sha256, got $actualCabSha256."
}

& expand.exe $cabPath "-F:*" $extractRoot
if ($LASTEXITCODE -ne 0) {
    throw "expand.exe failed with exit code $LASTEXITCODE."
}

$extractedRuntimeDir = Join-Path $extractRoot "Microsoft.WebView2.FixedVersionRuntime.${WebView2Version}.x64"
if (!(Test-Path -LiteralPath (Join-Path $extractedRuntimeDir "msedgewebview2.exe") -PathType Leaf)) {
    throw "The extracted WebView2 runtime does not contain msedgewebview2.exe."
}
Move-Item -LiteralPath $extractedRuntimeDir -Destination $runtimeDir
Assert-X64Pe -Path (Join-Path $runtimeDir "msedgewebview2.exe") -Label "msedgewebview2.exe"

Copy-Item -LiteralPath $ExecutablePath -Destination (Join-Path $packageDir "DBX.exe")
Copy-Item -LiteralPath "LICENSE" -Destination (Join-Path $packageDir "LICENSE")
Copy-Item -LiteralPath "docs/README-Server2016.txt" -Destination (Join-Path $packageDir "README-Server2016.txt")
New-Item -ItemType File -Force -Path (Join-Path $packageDir "portable.dbx") | Out-Null
New-Item -ItemType File -Force -Path (Join-Path $packageDir "fixed-webview2.dbx") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $packageDir "data") | Out-Null
Set-Content -LiteralPath (Join-Path $packageDir "data/README.txt") -Encoding utf8NoBOM -Value @"
DBX stores portable application data in this directory.
Keep this directory when replacing the complete package during an upgrade.
"@

$versions = [ordered]@{
    package_format = 1
    dbx = $appVersion
    commit = $CommitSha
    architecture = "x64"
    target_os = "Windows Server 2016 Desktop Experience"
    webview2 = $WebView2Version
    webview2_download = $WebView2Url
    webview2_cab_sha256 = $actualCabSha256
    sandbox_disabled_by_default = $true
    generated_at_utc = [DateTime]::UtcNow.ToString("o")
} | ConvertTo-Json
Set-Content -LiteralPath (Join-Path $packageDir "versions.json") -Encoding utf8NoBOM -Value $versions

$checksummedFiles = @(
    "DBX.exe",
    "WebView2Runtime/msedgewebview2.exe",
    "LICENSE",
    "README-Server2016.txt",
    "versions.json"
)
$checksumLines = foreach ($relativePath in $checksummedFiles) {
    $nativeRelativePath = $relativePath.Replace("/", [System.IO.Path]::DirectorySeparatorChar)
    $hash = Get-LowerSha256 -Path (Join-Path $packageDir $nativeRelativePath)
    "$hash  $relativePath"
}
Set-Content -LiteralPath (Join-Path $packageDir "SHA256SUMS.txt") -Encoding ascii -Value $checksumLines

if (Test-Path -LiteralPath $zipPath) {
    Remove-Item -LiteralPath $zipPath -Force
}
Compress-Archive -LiteralPath $packageDir -DestinationPath $zipPath -CompressionLevel Optimal

Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [System.IO.Compression.ZipFile]::OpenRead($zipPath)
try {
    $entries = @($zip.Entries | ForEach-Object { $_.FullName.Replace("\", "/") })
    $requiredEntries = @(
        "DBX-portable/DBX.exe",
        "DBX-portable/portable.dbx",
        "DBX-portable/fixed-webview2.dbx",
        "DBX-portable/WebView2Runtime/msedgewebview2.exe",
        "DBX-portable/data/README.txt",
        "DBX-portable/README-Server2016.txt",
        "DBX-portable/versions.json",
        "DBX-portable/SHA256SUMS.txt",
        "DBX-portable/LICENSE"
    )
    foreach ($entry in $requiredEntries) {
        if ($entries -notcontains $entry) {
            throw "Portable ZIP is missing required entry: $entry"
        }
    }
} finally {
    $zip.Dispose()
}

$zipSha256 = Get-LowerSha256 -Path $zipPath
Set-Content -LiteralPath $zipHashPath -Encoding ascii -Value "$zipSha256  $([System.IO.Path]::GetFileName($zipPath))"

Write-Host "Created $zipPath"
if ($env:GITHUB_OUTPUT) {
    "archive_name=$archiveBaseName" | Out-File -FilePath $env:GITHUB_OUTPUT -Encoding utf8 -Append
    "zip_path=$zipPath" | Out-File -FilePath $env:GITHUB_OUTPUT -Encoding utf8 -Append
    "zip_hash_path=$zipHashPath" | Out-File -FilePath $env:GITHUB_OUTPUT -Encoding utf8 -Append
}
